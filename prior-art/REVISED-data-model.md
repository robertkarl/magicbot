I'll write this design doc directly as my response. This is a synthesis task — the research corpus is comprehensive and I have everything I need. No tools required.

# MTG Rules-Engine State Data Model (Revised, Prior-Art-Informed)

This replaces the conversational sketch. The defining constraint is **constant-time copy-on-write forking** of `GameState`. Every design decision below is in service of that, validated against the verified CR corpus and the XMage/Forge/Argentum prior art.

---

## 0. Architectural spine (the five invariants everything rests on)

1. **Object identity is per-instance, per-zone.** A zone change mints a new object id (CR 400.7), with an enumerated exception set carried across.
2. **Behavior is static and shared; state is flat and value-typed.** Card behavior (the effect IR) lives once in a `&'static CardDb`. `GameState` leaves carry only ids, enums, small value types, and counts.
3. **Current characteristics are never stored** — they are a pure function of (copiable base, ordered continuous effects, counters, status) recomputed via the layer system (CR 613, 613.5).
4. **Forking is structural sharing.** Persistent/`Arc`-shared containers (HAMT maps, RRB/`im` vectors) make `fork()` an `Arc` bump — O(1) amortized, not a deep copy.
5. **One ground truth + `observe(player)` filter** for hidden info. Determinization/belief deferred.

```rust
/// The whole game. Cloning this is the fork; it is cheap by construction.
#[derive(Clone)]
pub struct GameState {
    db: &'static CardDb,                  // shared, never copied

    objects: ObjMap<ObjectId, Object>,    // im::HashMap — the universal object table
    zones:   Zones,                       // per-player + shared, ordered where CR requires
    stack:   im::Vector<ObjectId>,        // LIFO; entries are Objects in Zone::Stack (CR 405.2)

    players: im::Vector<PlayerState>,     // life, mana pool, hand-order pref, counters
    cont_effects: im::Vector<ContinuousEffect>, // active layer/RE effects (CR 611, 613, 614)
    turn:    TurnState,                   // phase/step, active player, priority, APNAP cursor
    linkage: Linkage,                     // linked-ability records (CR 607), exile sets
    lki:     ObjMap<ObjectId, Lki>,       // last-known-info snapshots (CR 608.2h, 603.10, 704.8)

    next_obj_id: u64,                     // monotonic id minter
    next_timestamp: u64,                  // monotonic layer-timestamp minter (CR 613.7)
    rng: DeterministicRng,                // seeded; replay/determinism
}
type ObjMap<K, V> = im::HashMap<K, V>;
```

`im::HashMap`/`im::Vector` give structural sharing: `GameState::clone()` clones a handful of `Arc`-backed root pointers. A mutation in the fork copies only the path to the changed node (O(log n)), leaving the parent untouched. This is the difference between the XMage/Forge approach (mutate-in-place, expensive deep clones for search) and what we need (cheap forking for tree search / determinization later).

---

## 1. The static `CardDb` / `CardDef` (behavior stored once)

### 1.1 Stable def-id

Per the schema research, **`oracle_id` is the canonical "same card" key** — reprint-invariant, name-collision-safe. We key the `CardDb` on a newtype over it.

```rust
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct DefId(u64);   // interned from Scryfall oracle_id / MTGJSON scryfallOracleId

pub struct CardDb {
    defs: HashMap<DefId, CardDef>,
    tokens: HashMap<DefId, CardDef>,   // token defs share the CardDef shape (schema §3)
}
```

Printing-level ids (`scryfallId`/`uuid`/`multiverse_ids`) are **not** in the rules model; if needed they live on an optional side `Printing` record for art/provenance only.

### 1.2 `CardDef` and faces

A card is a **list of faces** plus a **layout** that tells the engine how the faces relate (schema §2). Single-faced cards are the degenerate one-face case. Behavior (the effect IR) lives **per face**, because split/MDFC/adventure/room faces are independently castable.

```rust
pub struct CardDef {
    def_id: DefId,
    layout: Layout,            // drives multi-face semantics (degenerate = Normal)
    faces:  Vec<FaceDef>,      // index 0 = front; order is load-bearing (schema §2)
    related: Vec<Related>,     // tokens made, meld partner/result (CR via all_parts)
}

pub enum Layout {
    Normal,
    Split, Aftermath,          // both halves castable, share one object
    Flip,                      // Kamigawa
    Transform { front: usize, back: usize },   // DFC, physical transform
    ModalDfc   { front: usize, back: usize },  // MDFC
    Adventure  { creature: usize, spell: usize },
    Room       { left: usize, right: usize },  // two doors (CR room rules)
    Meld,                      // back face is a SEPARATE physical card -> see `related`
    Leveler, Saga, Class, Case,// single-face, multi-region; regions live in IR
}

pub struct FaceDef {
    name: Interned,
    mana_cost: ManaCost,           // parsed {..} vector
    mana_value: f32,               // CR 202.3 (f32 only for Un {1/2})
    supertypes: TypeMask, types: TypeMask, subtypes: SubtypeSet,
    colors: ColorMask,
    color_indicator: Option<ColorMask>,
    printed_pt: Option<PtPrinted>, // strings normalized: Num(i32) | Star | StarPlus(i32) | XVar
    printed_loyalty: Option<LoyVal>,
    printed_defense: Option<DefVal>,

    /// THE BEHAVIOR. Data-driven effect IR; Custom(fn) escape hatch.
    abilities: Vec<AbilityDef>,
}

pub struct Related { component: Component, def: DefId } // Token | MeldPart | MeldResult | ComboPiece
```

### 1.3 The effect IR + `Custom(fn)` escape hatch

Behavior is a data-driven IR so the engine can introspect (targets, costs, replacement shapes for the 614/616 pipeline) without running code. The `Custom` arm handles the long tail.

```rust
pub struct AbilityDef {
    kind: AbilityKind,
    // classification flags the engine needs WITHOUT executing the body:
    is_cda: bool,              // CR 604.3 — affects layer/copiable handling
    layer: Option<Layer>,     // for static continuous abilities (CR 613.1)
    intervening_if: Option<Predicate>, // CR 603.4 (double-checked)
    look_back: bool,          // CR 603.10 (uses pre-event snapshot)
}

pub enum AbilityKind {
    Static(StaticEffectIr),         // continuous; feeds layer engine
    Activated { cost: Cost, effect: EffectIr },
    Triggered { event: TriggerSpec, effect: EffectIr },
    Replacement(ReplacementIr),     // CR 614 event-interceptor
    Spell(EffectIr),                // instant/sorcery resolution
    Mana(ManaEffectIr),             // CR 605 — resolves immediately, off-stack
    Custom(fn(&mut EffectCtx) -> Result<(), Fizzle>),  // escape hatch
}
```

The `is_cda`, `layer`, `intervening_if`, `look_back` flags are *metadata about behavior* and so legitimately live in the static `CardDef` — they describe the printed ability, not per-instance state.

---

## 2. The forked `GameState` shape

### 2.1 Object id scheme

```rust
#[derive(Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ObjectId(u64);   // minted monotonically; NEVER reused
```

A single monotonic counter (`next_obj_id`) mints ids. **Every zone change allocates a new `ObjectId`** (CR 400.7). This is our analogue of XMage's `(UUID, zone-change-counter)` `MageObjectReference`, but simpler: because we mint a fresh id on every move, "is this the same object as before?" is just `id == id`. There is no separate ZCC because the id *is* the per-instance-per-zone identity. The XMage TODO-littered ZCC scheme exists precisely because XMage reuses the card's UUID; we sidestep it.

References that must survive the source leaving (stack targets, triggered-ability sources, linked abilities) hold an `ObjectId` plus an **LKI fallback** (§4.4, gotcha #1, #9).

### 2.2 The universal `Object`

One struct models everything that can be an object in any zone — permanents, cards in hand/library/graveyard/exile, spells/abilities on the stack, emblems, tokens, copies. This mirrors Forge's "one `Card` class, kind via enum" choice (which the research found more robust than XMage's subclass tree for runtime identity fluidity), but with flat value-typed instance state for cheap forking.

```rust
pub struct Object {
    id: ObjectId,
    owner: PlayerId,                 // fixed (CR 400, 108.3)
    controller: Option<PlayerId>,    // mutable; None off-battlefield (CR 405.4, layer 2)
    zone: Zone,

    origin: Origin,                  // WHAT this object is printed-from (the copiable source)
    instance: InstanceState,         // per-object mutable state (the "flat leaf")
    stack: Option<StackPayload>,     // Some(..) iff zone == Stack
    timestamp: Timestamp,            // CR 613.7 zone-entry / attach / transform timestamp
}
```

#### 2.2.1 `Origin` — printed base + composition (copiable source, merge, faces, tokens, copies)

`Origin` answers "where do this object's *copiable values* come from before layers?" It must express every degenerate case as one general mechanism: a **component stack** of "what cards/defs physically compose this object," plus per-component face selection, plus an optional baked-in copy.

```rust
pub struct Origin {
    /// Ordered components. len==1 for ordinary permanents/cards/tokens.
    /// len>1 ONLY for merged permanents (Mutate/meld-as-merge). Index 0 = TOP.
    components: SmallVec<[Component; 1]>,

    /// Layer 1a "becomes a copy" override (CR 707.2/3, 613.2a).
    /// Some(..) replaces the TOP component's copiable values with this baked copy.
    copy_of: Option<Box<CopiableValues>>,
}

pub struct Component {
    def: DefId,            // points into &'static CardDb (behavior + printed base)
    face: FaceSel,         // which face is up / which mode
    is_token: bool,        // CR 111.7 cease-to-exist SBA applies
    token_pt: Option<Pt>,  // tokens' as-created P/T when defined by the creating effect
    component_owner: PlayerId, // merge: each component may have a DIFFERENT owner (CR 730)
}

pub enum FaceSel {
    Front, Back,                 // transform/MDFC/flip (CR 712)
    SplitHalf(usize),            // split/aftermath/room door
    AdventureSide(AdvSide),
    FaceDown(FaceDownKind),      // morph/manifest/disguise/cloak (CR 708) -> 2/2 blank profile
}
pub enum FaceDownKind { Morph, Manifest, Disguise, Cloak } // Cloak = 2/2 ward {2}
```

**How each card-construct is a degenerate case of this one model:**

| Construct | `Origin` shape |
|---|---|
| Ordinary permanent / card | `components=[one]`, `copy_of=None`, `face=Front` |
| Token | same, `is_token=true`, def from token table |
| Copy (Clone) | `copy_of=Some(copiable values of target)` — Layer 1a (CR 707.2, gotcha #4) |
| Merged (Mutate) | `components=[top, .., bottom]` — **chars from top, abilities union of all** (CR 730, gotcha #6) |
| Meld | melded object's component points at the `MeldResult` def (or two components, layout-dependent) |
| Multi-face (split/MDFC/transform/adventure/room) | one component, `face` selects (CR 712, schema §2) |
| Face-down | one component, `face=FaceDown(kind)` → copiable values overridden to 2/2 blank (CR 708, 707.2) |

Crucially, **characteristics resolve from `components[0]` (top); abilities aggregate over all components** — exactly the merge rule (CR 730), and the degenerate single-component case gives ordinary behavior for free.

#### 2.2.2 `InstanceState` — the flat, value-typed leaf

Everything per-object and mutable that is *not* a current characteristic (those are computed). This is the data that gets dropped on zone change (CR 400.7) unless an exception carries it.

```rust
pub struct InstanceState {
    status: Status,                 // bitflags: TAPPED, FLIPPED, FACE_DOWN, PHASED_OUT, SUMMONING_SICK
    counters: Counters,            // im::HashMap<CounterKind, i32> (+1/+1, loyalty, keyword, etc.)
    marked_damage: u32,            // CR 120.3 — MARKED, not subtracted (gotcha §12)
    deathtouch_marked: bool,       // CR 704.5h
    regen_shield: u8,              // CR 614.8 pending regeneration replacements
    attached_to: Option<ObjectId>, // Aura/Equipment/Fortification host
    attachments: im::Vector<ObjectId>, // what is attached to this
    damage_assignment_order: Option<im::Vector<ObjectId>>, // combat (CR 509)
    entered_this_turn: bool,       // summoning sickness / "this turn" memory
    chosen: ChosenState,           // "as enters" choices: color, type, opponent (CR 603.6e)
    phased_companions: im::Vector<ObjectId>, // indirect phasing (CR 702.26, gotcha §7)
}
```

`Counters`, `attachments`, etc. are persistent containers so forking stays cheap even for objects with lots of state.

#### 2.2.3 `StackPayload` — stack objects as first-class (CR 405, 113.7a, gotcha §9)

A spell/ability on the stack is its own object, independent of its source, with **frozen choices**.

```rust
pub struct StackPayload {
    kind: StackKind,               // Spell | Activated | Triggered | DelayedTriggered
    source: ObjectRef,             // id + LKI fallback (source may leave; CR 608.2b)
    text: AbilityDef,             // CR 405.4: ability "has the text that created it and no other characteristics"
    // FROZEN at announcement (CR 601.2, gotcha §9):
    targets: SmallVec<[Target; 2]>,
    modes: SmallVec<[ModeId; 1]>,
    x_value: Option<u32>,
    chosen_costs: CostsPaid,       // CR 400.7d — referencable post-resolution
    is_reflexive: bool,            // CR 603.12
    intervening_if: Option<Predicate>, // re-checked on resolve (CR 603.4 / 608.2a)
}
```

`ObjectRef` is the universal "points at an object but tolerates its departure" type:

```rust
pub struct ObjectRef { id: ObjectId, lki: Option<Arc<Lki>> } // CR 608.2b/2h
```

#### 2.2.4 Emblems / non-permanent continuous objects

Emblems are objects in the **command zone** (CR 400, 113.x) that only ever contribute a static ability to the layer engine. They are just `Object`s with a token-like `Origin` (a synthetic `DefId`) whose single ability is `Static(..)`. No special-casing.

### 2.3 Zones

```rust
pub struct Zones {
    // Per-player, owner-fixed (CR 400.1). Ordered where CR requires (400.5):
    library:   im::Vector<im::Vector<ObjectId>>,   // ORDERED (per player)
    hand:      im::Vector<im::Vector<ObjectId>>,    // positionally free
    graveyard: im::Vector<im::Vector<ObjectId>>,    // ORDERED
    // Shared singletons:
    battlefield: im::HashSet<ObjectId>,             // positionally free
    exile:       im::Vector<ObjectId>,              // can carry order / face-down piles (400.5)
    command:     im::Vector<ObjectId>,              // emblems, commanders, etc.
    // stack lives on GameState directly (LIFO vector)
}
```

`Zone::is_public()` is a property of the zone (CR 400.2); face-down is a per-object `Status` override on a public zone. Ordering is modeled where the rules fix it (library, graveyard, stack, face-down piles) and as free sets/positions otherwise. Redirect-to-owner (CR 400.3) and type-based restrictions (CR 400.4) are normalized in the move pipeline (§4).

---

## 3. Computing current characteristics (the layer function)

**Never stored.** `characteristics(state, id) -> Characteristics` is a pure function recomputed on query (CR 613.5), memoized only with invalidation on any change to effects/counters/status/timestamps.

### 3.1 Inputs (exactly the four the layer research mandates)

```rust
fn characteristics(s: &GameState, id: ObjectId) -> Characteristics {
    // INPUT 1: copiable base (layer 1 fixed point, CR 613.2c)
    let base = copiable_values(s, id);
    //   = layer 1a: apply Origin.copy_of, then merge top-component copiable values
    //               (merge characteristics from top per CR 730);
    //     layer 1b: if FaceDown -> override to 2/2 blank profile (CR 708.2, 707.2);
    //     freeze (613.2c).

    // INPUT 2: all applicable continuous effects, each tagged
    //   { layer/sublayer, timestamp, is_cda, locked object-set (CR 613.6), predicate }
    let effects = applicable_continuous_effects(s, id);

    // INPUT 3: counters (layer 6 keyword; layer 7c numeric) from InstanceState
    // INPUT 4: status (tapped/flipped/face-down/phased) from InstanceState

    apply_layers(base, effects, &s.objects[&id].instance)
}
```

### 3.2 The layer algorithm (CR 613, fixed cross-layer order)

1. **Layer 1** (sublayers): 1a copy/merge → 1b face-down (CR 613.2a/b) → freeze copiable values (613.2c).
2. **Layers 2→7** in fixed order (613.1): control → text → type → color → abilities → P/T. Cross-layer order is **fixed regardless of timestamp** (613.1).
3. **Within a layer:** CDAs first (613.3; for P/T, sublayer 7a), then **dependency (613.8) before timestamp (613.7) as fallback**, **re-evaluating dependency after each application** (613.8c), breaking mutual loops by timestamp (613.8b).
4. **P/T sublayers (current CR 613.4):** 7a CDA P/T → 7b set → 7c modify (**counters folded in here**, matching the post-2021 CR and Forge's single `MODIFYPT`) → 7d switch.
5. **Ability aggregation in layer 6** unions abilities from **all merge components** (CR 730) plus add/remove/keyword-counter/"can't have" effects.
6. CDAs apply **in all zones** (CR 604.3) — so `characteristics` is callable on a card in hand/graveyard/stack, not just the battlefield (gotcha §12, Tarmogoyf-in-hand).

Effects on players (613.10) and on game rules (613.11) are applied in timestamp order after object characteristics.

This is the Forge/Argentum "base vs projected" split with the layer ordering done per-evaluation (not cached) because dependency/applicability are state-dependent (613.8c). Memoization keyed on `(applyEffectsCounter, instance-version)` is permitted with invalidation — analogous to XMage's `applyEffectsCounter`.

---

## 4. Zone change as events; dies/LTB predicates; the new-object rule

### 4.1 The move pipeline (CR 400.6 + 614 + 616)

Every state mutation that could be replaced/trigger flows through one event pipeline — the choke point.

```rust
fn move_object(s: &mut GameState, mv: MoveEvent) -> ObjectId {
    // 1. normalize destination: redirect-to-owner (400.3), type restrictions (400.4)
    // 2. owner/controller looks for move-affecting abilities (400.6 step 1)
    // 3. REPLACEMENT pipeline (614/616): collect applicable REs, order via 616.1
    //    priority loop (self-repl -> control -> copy -> back-face -> free), apply ONE,
    //    mark used-for-this-event (614.5), re-collect (616.1f), respect outer-before-inner
    //    (616.1g), until none apply.
    // 4. SNAPSHOT LKI of the leaving object BEFORE the move (608.2h, 603.10, 704.8).
    // 5. MINT NEW ObjectId; build destination Object; drop InstanceState per 400.7,
    //    then apply the 400.7a-m carry-across exceptions.
    // 6. emit post-event; gather triggers against post-event objects (603.10 default),
    //    EXCEPT the look-back set which uses the pre-event snapshot.
}
```

### 4.2 Event distinctions: dies vs LTB vs bounce (CR 700.4, 603.6c)

These are **predicates over the `MoveEvent`**, not stored object kinds. This is exactly the distinction the prompt demands.

```rust
fn dies(mv: &MoveEvent) -> bool       // CR 700.4
    { mv.from == Zone::Battlefield && mv.to_kind == ZoneKind::Graveyard }

fn leaves_battlefield(mv: &MoveEvent) -> bool  // CR 603.6c
    { mv.from == Zone::Battlefield }           // to ANY other zone
    // ...plus a phased-in permanent leaving the GAME with its owner

fn bounced(mv: &MoveEvent) -> bool
    { mv.from == Zone::Battlefield && mv.to_kind == ZoneKind::Hand }

// "destroyed" is a CAUSE, not a path: a flag on the MoveEvent (Forge's wasDestroyed),
// distinguishing destruction (CR 701.7) from lethal-damage SBA / 0-toughness / sacrifice.
```

LTB triggers look back in time (CR 603.10a) and find the object **only in the first zone it reached** (603.6c). "From anywhere" is never an LTB ability (603.6c).

### 4.3 New-object rule (CR 400.7) and its exceptions

Default on every zone change: new id; **drop** marked damage, counters, attachments, continuous-effect links, "this turn" memory, control. Then apply carry-across exceptions, each modeled explicitly:

- **400.7a–d, g, h** (permanent-spell→permanent continuity; "may cast"; cost/mana referencing): carried via the resolving `StackPayload` → new `Object` linkage; `chosen_costs` retained for 400.7d.
- **400.7e/f** (zone-change-trigger / Aura-LTB finds the new object in a **public** destination): the trigger resolution resolves `ObjectRef` to the freshly-minted destination id.
- **400.7i/j/k** (play-land / cost-moves-to-public / madness find the new object): same `ObjectRef`-to-new-id resolution, gated on destination being public.
- **400.7m** (stickers retained public→public): `InstanceState.chosen`/stickers carried.
- **400.8/.9/.10** (exile-already-in-exile, command face-down, command→command): "new object, same zone" — mint new id without a zone vector move.
- **Token exception** (CR 111.7/704.5d): a token that leaves the battlefield gets the new id and momentarily exists in the destination (so LTB/dies triggers see it), then the **cease-to-exist SBA** removes it (§5, gotcha §8).

### 4.4 LKI store

```rust
pub struct Lki {                 // frozen snapshot; never updated after capture
    chars: Characteristics,      // all layers applied at moment of leaving
    counters: Counters, controller: Option<PlayerId>,
    attachments: im::Vector<ObjectId>, zone_before: Zone, timestamp: Timestamp,
}
```

Captured before the move (and, per CR 704.8, **before the simultaneous SBA event** for SBA-driven LTB). Resolvable by stack objects (608.2b/2h) and look-back triggers (603.10).

---

## 5. SBA + priority loop (CR 117.5 ≈ 704.3)

```rust
fn before_grant_priority(s: &mut GameState) {
    loop {
        while sba_applicable(s) { perform_all_sba_as_single_event(s); } // to fixpoint
        let trigs = collect_triggers_apnap(s);                          // 603.3b two-phase
        if trigs.is_empty() { break; }
        put_on_stack(s, trigs);
    }
    // then the appropriate player gets priority
}
```

SBAs (CR 704.5) are predicates over current state, performed simultaneously as one event (704.4 ignores mid-resolution transients; 704.7 single RE for same-result; 704.8 LKI before the batch). The token cease-to-exist SBA (704.5d) runs **after** zone-change triggers are logged (gotcha §8).

---

## 6. Where each named gotcha is handled

| Gotcha (CR) | Location in model |
|---|---|
| **LKI** (608.2h, 603.10, 704.8) | §4.4 `Lki` store, snapshot-before-move; `ObjectRef` fallback (§2.2.3) |
| **Linked abilities** (607) | `Linkage` table: per-instance association records keyed by the ability-resolution that created them, not global card properties (§0, gotcha #2) |
| **"As enters" / "as leaves" / enters-with** (603.6e, 614.12, 616) | Replacement pipeline §4.1 step 3 runs *before* object is on battlefield; `InstanceState.chosen` + counters applied atomically with placement |
| **Copiable values vs layers** (707.2, 613) | `Origin.copy_of` is Layer 1a only; counters/auras/buffs live in `InstanceState`/`cont_effects` and re-apply in their own layers (§2.2.1, §3) |
| **Face-down** (708, 707.2) | `FaceSel::FaceDown` overrides copiable values to 2/2 blank in layer 1b on the **same** `ObjectId`; real card = `components[0].def` kept private; flip = `FaceSel` swap, instance state persists (§2.2.1) |
| **Merged / Mutate** (730) | `Origin.components` (top=chars, all=abilities); un-merge fans out per-component to each `component_owner`'s zone; counters on merged object discarded (§2.2.1, §4) |
| **Phasing** (702.26) | `Status::PHASED_OUT` flag on same object; not a zone change; no ETB/LTB; `phased_companions` cascades to attachments; all queries filter phased-out (§2.2.2, gotcha #7) |
| **Tokens cease to exist** (111.7, 704.5d) | `is_token` + cease-to-exist SBA *after* triggers logged (§4.3, §5) |
| **Stack object ≠ source** (113.7a, 601.2) | `StackPayload` first-class with frozen targets/modes/X/costs; `ObjectRef` source with LKI fallback (§2.2.3) |
| **Continuous: one-shot vs fixed-set vs static** (611.2c, 604, 613) | `ContinuousEffect` carries a locked object-set (613.6) for fixed-set; statics re-scan each layer pass; one-shots never enter `cont_effects` (§3) |
| **Replacement ordering / self-repl / skip** (614, 616) | §4.1 step 3 player-ordered, re-evaluated, used-flagged pipeline |
| **New object on zone change** (400.7) | §4.3, new id + drop + enumerated exceptions |
| **SBA batch before triggers** (704) | §5 fixpoint loop |
| **Intervening-if double-check** (603.4) | `intervening_if` checked at trigger time and on resolve (608.2a) |
| **Marked damage / deathtouch / -1/-1 distinct** (120.3, 704.5g/h) | `marked_damage` + `deathtouch_marked` separate from P/T-modifying effects (§2.2.2) |
| **Target legality re-checked on resolve** (608.2b) | resolution re-validates every `Target`; all-illegal → countered by rules |
| **CDAs in all zones** (604.3, 707.2) | `characteristics()` callable in any zone; CDAs applied 7a/613.3 (§3.2) |
| **Layer dependency/timestamp loops** (613.8) | per-evaluation dependency resolution with timestamp loop fallback (§3.2) |

---

## 7. DEFERRED (explicitly out of scope for this data model)

1. **Hidden-information belief / determinization / ISMCTS sampling.** Only ground-truth state + `observe(player)` filter is modeled now. No per-player belief states, no card-counting inference, no determinized sampling.
2. **`observe(player)` filter implementation details** beyond "zone-public-ness gates visibility; face-down/hidden-zone cards expose only public characteristics." The exact redaction projection is named but not specified.
3. **Full ability-IR vocabulary.** The `EffectIr`/`TriggerSpec`/`ReplacementIr`/`Predicate`/`Cost` grammars are sketched as enums but not enumerated; `Custom(fn)` covers the tail. The loader must boot the whole card pool with static characteristics and only *missing behavior* for unencoded cards.
4. **AI / evaluation / search policy.** The cheap-fork design exists to enable search later, but no search, heuristic, or evaluation is part of the state model.
5. **Networking / serialization wire format.** Determinism (seeded `DeterministicRng`, monotonic id/timestamp minters) is in place to *enable* replay/netcode; the protocol is deferred.
6. **Variant formats** (2HG, Commander 21-damage & commander-tax, Archenemy, Planechase, ante) — CR 704.6 SBAs and command-zone variants noted but not modeled.
7. **Combat sub-engine details** (banding, first-strike step creation, trample assignment) — `damage_assignment_order` field reserved; the step machinery is deferred.
8. **Memoization/invalidation cache for `characteristics()`** — the pure function is specified; the concrete cache keying (analogous to XMage's `applyEffectsCounter`) is an optimization deferred to implementation.
9. **Un-set / silver-border corner cases** beyond accommodating fractional mana value and `∞` P/T as value types.
10. **Sticker/attraction/contraption** mechanics beyond the `400.7m` carry-across note.

---

### Key prior-art deltas baked in

- **vs XMage:** no UUID+ZCC `MageObjectReference` — a fresh `ObjectId` per move makes identity comparison trivial and avoids the documented `// TODO: wtf` ZCC fragility. We adopt XMage's "dies = typed Battlefield→Graveyard" but as a *predicate over events*, not a trigger subclass.
- **vs Forge:** we keep Forge's "one object struct, kind via enum/data" (robust for runtime identity fluidity) and its single-`MODIFYPT`-7c current-CR layer model and JGraphT-style dependency resolution — but make all instance state value-typed in persistent containers so **forking is O(1)**, which Forge's mutable `Card` cannot do.
- **vs Argentum:** we adopt the immutable base-vs-projected split and pure-data behavior, but use a flat `Object` table with persistent maps rather than full ECS, trading ECS generality for predictable cheap forking and direct CR-shaped pipelines.

Relevant file for follow-up implementation work: this design would live at `/Users/robertkarl/Code/magicbot/` (the repo whose only commit is the research notes).