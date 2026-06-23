# Layer System (CR 613) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `characteristics()` — the pure function that computes an object's current power/toughness/types/colors/abilities by applying continuous effects in the CR 613 layer order — over explicit inputs, with NO CardDb or effect-IR yet (those feed it in a later plan).

**Architecture:** A new module `magicbot-engine/src/layers.rs`. The core is `fn compute(base: BaseChars, effects: &[ContinuousEffect], counters: &Counters, status: Status) -> Chars`. It applies layers 1→7 in fixed cross-layer order (CR 613.1), with layer-7 sublayers 7a→7b→7c→7d (CR 613.4), ordering effects within a layer by timestamp (CR 613.7), with dependency ordering (CR 613.8) deferred to a clearly-marked stub. Inputs are plain value types supplied by the test/caller; wiring this into `GameState` (reading real objects, gathering real effects) is a LATER plan. This isolates the hardest, most bug-prone subsystem (the thing that produced the "wrong P/T" bugs in XMage/Forge) so it can be tested directly against conformance cases F1–F14.

**Tech Stack:** Rust 1.93+, `im` (for any shared collections), `proptest` (dev). No new deps.

**Source spec:** `prior-art/layers-and-characteristics.md` (the verified CR 613/707/604.3 research) and `prior-art/conformance-cases.md` section F (F1–F14) + section G (copy/copiable, the parts achievable without a CardDb).

## Global Constraints

- **Rust edition 2021**, toolchain `1.93+`.
- **`compute()` is a PURE function** — no `GameState`, no I/O, no mutation of inputs. Same inputs → same output. (CR 613.5: characteristics are continuously, automatically recomputed.)
- **Cross-layer order is FIXED regardless of timestamp** (CR 613.1): 1 copy → 2 control → 3 text → 4 type → 5 color → 6 abilities → 7 P/T. This slice implements layers **4 (type), 5 (color), 6 (abilities), 7 (P/T)** and the layer-1 copiable base; layers 2 (control) and 3 (text) are represented in the enum but their application is a documented no-op stub (control lives in GameState, text-changing is rare) — DEFERRED, not silently dropped.
- **Layer 7 sublayer order is FIXED** (CR 613.4): 7a CDA-P/T → 7b set → 7c modify (counters folded in here, per post-2021 CR) → 7d switch. A set in 7b NEVER overwrites a later 7c counter.
- **Within a layer, order by timestamp** (CR 613.7), ascending. Dependency ordering (CR 613.8) is a marked stub returning timestamp order — DEFERRED.
- **No CardDb, no effect IR, no parsing.** Inputs are constructed value types.
- **TDD**; every conformance test cites its CR rule; `cargo test` + `cargo clippy --all-targets -- -D warnings` clean after every task.

---

### Task 1: Characteristic value types

**Files:**
- Create: `magicbot-engine/src/layers.rs`
- Modify: `magicbot-engine/src/lib.rs` (add `pub mod layers;`)
- Test: in-file `#[cfg(test)]` in `layers.rs`

**Interfaces:**
- Consumes: nothing (self-contained value types).
- Produces:
  - `pub struct Pt { pub power: i32, pub toughness: i32 }` — `Copy, Clone, PartialEq, Eq, Debug`.
  - `pub struct TypeSet { creature: bool, land: bool, artifact: bool, enchantment: bool }` — `Copy, Clone, PartialEq, Eq, Debug`, with `TypeSet::none()`, `with_creature(self)`, `is_creature(&self)`, `is_land(&self)`, `is_artifact(&self)`, `is_enchantment(&self)`. (A minimal card-type set — enough to test type-changing; subtypes deferred.)
  - `pub struct ColorSet(u8)` — bitflags W=1,U=2,B=4,R=8,G=16; `Copy,Clone,PartialEq,Eq,Debug`; `ColorSet::colorless()`, `with(self, Color)`, `has(&self, Color)`.
  - `pub enum Color { White, Blue, Black, Red, Green }` — `Copy,Clone,PartialEq,Eq,Debug`.
  - `pub struct Chars { pub pt: Option<Pt>, pub types: TypeSet, pub colors: ColorSet, pub abilities: AbilitySet }` — `Clone, PartialEq, Debug`. (`pt` is `Option` because noncreatures have no P/T.)
  - `pub struct AbilitySet(im::HashSet<Ability>)` with `new()`, `add(&mut, Ability)`, `remove(&mut, Ability)`, `has(&self, Ability) -> bool`, `clear(&mut)`.
  - `pub enum Ability { Flying, Trample, Deathtouch, Lifelink, FirstStrike, Vigilance, Haste, Defender }` — `Copy,Clone,PartialEq,Eq,Hash,Debug`. (A small fixed keyword set; full ability model deferred.)

- [ ] **Step 1: Write the failing test**

Create `magicbot-engine/src/layers.rs`:

```rust
//! CR 613 layer system: compute current characteristics as a pure function.
//! No CardDb / effect IR yet — inputs are explicit value types (a later plan
//! gathers these from GameState).

#[cfg(test)]
mod value_tests {
    use super::*;

    #[test]
    fn typeset_creature_flag() {
        let t = TypeSet::none().with_creature();
        assert!(t.is_creature());
        assert!(!t.is_land());
    }

    #[test]
    fn colorset_bits() {
        let c = ColorSet::colorless().with(Color::White).with(Color::Blue);
        assert!(c.has(Color::White));
        assert!(c.has(Color::Blue));
        assert!(!c.has(Color::Black));
    }

    #[test]
    fn ability_set_add_remove_has() {
        let mut a = AbilitySet::new();
        a.add(Ability::Flying);
        assert!(a.has(Ability::Flying));
        a.remove(Ability::Flying);
        assert!(!a.has(Ability::Flying));
    }
}
```

- [ ] **Step 2: Add the module declaration**

In `magicbot-engine/src/lib.rs`, add after the existing `pub mod` lines:

```rust
pub mod layers;
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd magicbot-engine && cargo test --lib layers`
Expected: FAIL — `TypeSet`, `ColorSet`, `AbilitySet` not defined.

- [ ] **Step 4: Write minimal implementation**

At the top of `magicbot-engine/src/layers.rs` (above the test module):

```rust
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Pt {
    pub power: i32,
    pub toughness: i32,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct TypeSet {
    creature: bool,
    land: bool,
    artifact: bool,
    enchantment: bool,
}

impl TypeSet {
    pub fn none() -> Self {
        TypeSet { creature: false, land: false, artifact: false, enchantment: false }
    }
    pub fn with_creature(mut self) -> Self {
        self.creature = true;
        self
    }
    pub fn with_land(mut self) -> Self {
        self.land = true;
        self
    }
    pub fn is_creature(&self) -> bool {
        self.creature
    }
    pub fn is_land(&self) -> bool {
        self.land
    }
    pub fn is_artifact(&self) -> bool {
        self.artifact
    }
    pub fn is_enchantment(&self) -> bool {
        self.enchantment
    }
    pub fn set_creature(&mut self, v: bool) {
        self.creature = v;
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ColorSet(u8);

impl ColorSet {
    pub fn colorless() -> Self {
        ColorSet(0)
    }
    fn bit(c: Color) -> u8 {
        match c {
            Color::White => 1,
            Color::Blue => 2,
            Color::Black => 4,
            Color::Red => 8,
            Color::Green => 16,
        }
    }
    pub fn with(mut self, c: Color) -> Self {
        self.0 |= Self::bit(c);
        self
    }
    pub fn has(&self, c: Color) -> bool {
        self.0 & Self::bit(c) != 0
    }
    pub fn set_to(&mut self, other: ColorSet) {
        self.0 = other.0;
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Ability {
    Flying,
    Trample,
    Deathtouch,
    Lifelink,
    FirstStrike,
    Vigilance,
    Haste,
    Defender,
}

#[derive(Clone, PartialEq, Debug)]
pub struct AbilitySet(im::HashSet<Ability>);

impl AbilitySet {
    pub fn new() -> Self {
        AbilitySet(im::HashSet::new())
    }
    pub fn add(&mut self, a: Ability) {
        self.0.insert(a);
    }
    pub fn remove(&mut self, a: Ability) {
        self.0.remove(&a);
    }
    pub fn has(&self, a: Ability) -> bool {
        self.0.contains(&a)
    }
    pub fn clear(&mut self) {
        self.0 = im::HashSet::new();
    }
}

impl Default for AbilitySet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Chars {
    pub pt: Option<Pt>,
    pub types: TypeSet,
    pub colors: ColorSet,
    pub abilities: AbilitySet,
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd magicbot-engine && cargo test --lib layers`
Expected: PASS — all three value tests pass.

- [ ] **Step 6: Commit**

```bash
git add magicbot-engine/src/layers.rs magicbot-engine/src/lib.rs
git commit -m "feat(layers): characteristic value types (Pt/TypeSet/ColorSet/AbilitySet/Chars)"
```

---

### Task 2: `ContinuousEffect`, `BaseChars`, `Status`, and the `compute()` skeleton

**Files:**
- Modify: `magicbot-engine/src/layers.rs`
- Test: in-file `#[cfg(test)]` in `layers.rs`

**Interfaces:**
- Consumes: Task 1 types; `Timestamp` from `crate::ids`; `CounterKind` from `crate::object`.
- Produces:
  - `pub struct BaseChars { pub pt: Option<Pt>, pub types: TypeSet, pub colors: ColorSet, pub abilities: AbilitySet }` — the layer-1 copiable base (printed values; a later plan derives this from CardDb). `Clone, Debug`.
  - `pub enum Layer { Type, Color, Ability, Pt }` — the layers this slice applies (1/2/3 handled as base/no-op). `Copy,Clone,PartialEq,Eq,Debug`.
  - `pub enum Effect` — the effect bodies:
    - `AddType(TypeSet)` (layer 4 — OR the flags in)
    - `SetColors(ColorSet)` (layer 5)
    - `AddAbility(Ability)` (layer 6)
    - `RemoveAllAbilities` (layer 6)
    - `SetBasePt(Pt)` (layer 7b)
    - `ModifyPt { power: i32, toughness: i32 }` (layer 7c)
    - `SwitchPt` (layer 7d)
  - `pub struct ContinuousEffect { pub effect: Effect, pub timestamp: Timestamp, pub is_cda: bool }` — `Clone, Debug`. `is_cda` flags characteristic-defining (affects 7a/ordering; used in later tasks).
  - `pub struct Counters { plus1: i32, minus1: i32 }` with `Counters::new()`, `with_plus1(self, n)`, `with_minus1(self, n)`, `net_pt(&self) -> (i32, i32)` returning `(plus1 - minus1, plus1 - minus1)`.
  - `pub struct Status { pub tapped: bool }` — `Copy,Clone,Debug` (face-down/phased deferred).
  - `pub fn compute(base: &BaseChars, effects: &[ContinuousEffect], counters: &Counters, _status: Status) -> Chars` — skeleton that, for now, just copies base into Chars and ignores effects/counters (filled by Tasks 3–6).
  - `fn layer_of(e: &Effect) -> Layer` helper.

- [ ] **Step 1: Write the failing test**

Add to the implementation area of `magicbot-engine/src/layers.rs` a new test module:

```rust
#[cfg(test)]
mod compute_skeleton_tests {
    use super::*;

    fn base_creature() -> BaseChars {
        BaseChars {
            pt: Some(Pt { power: 2, toughness: 2 }),
            types: TypeSet::none().with_creature(),
            colors: ColorSet::colorless().with(Color::Green),
            abilities: AbilitySet::new(),
        }
    }

    #[test]
    fn compute_with_no_effects_returns_base() {
        let base = base_creature();
        let out = compute(&base, &[], &Counters::new(), Status { tapped: false });
        assert_eq!(out.pt, Some(Pt { power: 2, toughness: 2 }));
        assert!(out.types.is_creature());
        assert!(out.colors.has(Color::Green));
    }

    #[test]
    fn layer_of_maps_effects() {
        assert_eq!(layer_of(&Effect::AddType(TypeSet::none())), Layer::Type);
        assert_eq!(layer_of(&Effect::SetColors(ColorSet::colorless())), Layer::Color);
        assert_eq!(layer_of(&Effect::AddAbility(Ability::Flying)), Layer::Ability);
        assert_eq!(layer_of(&Effect::RemoveAllAbilities), Layer::Ability);
        assert_eq!(layer_of(&Effect::SetBasePt(Pt { power: 1, toughness: 1 })), Layer::Pt);
        assert_eq!(layer_of(&Effect::ModifyPt { power: 1, toughness: 1 }), Layer::Pt);
        assert_eq!(layer_of(&Effect::SwitchPt), Layer::Pt);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd magicbot-engine && cargo test --lib layers::compute_skeleton`
Expected: FAIL — `BaseChars`, `compute`, `layer_of` not defined.

- [ ] **Step 3: Write minimal implementation**

Add to `magicbot-engine/src/layers.rs` (after the Task 1 types):

```rust
use crate::ids::Timestamp;

#[derive(Clone, Debug)]
pub struct BaseChars {
    pub pt: Option<Pt>,
    pub types: TypeSet,
    pub colors: ColorSet,
    pub abilities: AbilitySet,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Layer {
    Type,
    Color,
    Ability,
    Pt,
}

#[derive(Clone, Debug)]
pub enum Effect {
    AddType(TypeSet),
    SetColors(ColorSet),
    AddAbility(Ability),
    RemoveAllAbilities,
    SetBasePt(Pt),
    ModifyPt { power: i32, toughness: i32 },
    SwitchPt,
}

#[derive(Clone, Debug)]
pub struct ContinuousEffect {
    pub effect: Effect,
    pub timestamp: Timestamp,
    pub is_cda: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct Counters {
    plus1: i32,
    minus1: i32,
}

impl Counters {
    pub fn new() -> Self {
        Counters { plus1: 0, minus1: 0 }
    }
    pub fn with_plus1(mut self, n: i32) -> Self {
        self.plus1 += n;
        self
    }
    pub fn with_minus1(mut self, n: i32) -> Self {
        self.minus1 += n;
        self
    }
    /// CR 122 / 704.5q net for P/T purposes (annihilation handled elsewhere as SBA).
    pub fn net_pt(&self) -> (i32, i32) {
        let net = self.plus1 - self.minus1;
        (net, net)
    }
}

impl Default for Counters {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Status {
    pub tapped: bool,
}

pub fn layer_of(e: &Effect) -> Layer {
    match e {
        Effect::AddType(_) => Layer::Type,
        Effect::SetColors(_) => Layer::Color,
        Effect::AddAbility(_) | Effect::RemoveAllAbilities => Layer::Ability,
        Effect::SetBasePt(_) | Effect::ModifyPt { .. } | Effect::SwitchPt => Layer::Pt,
    }
}

/// CR 613.5: characteristics are a pure function recomputed on demand.
/// Skeleton — Tasks 3-6 fill in the per-layer application.
pub fn compute(
    base: &BaseChars,
    _effects: &[ContinuousEffect],
    _counters: &Counters,
    _status: Status,
) -> Chars {
    Chars {
        pt: base.pt,
        types: base.types,
        colors: base.colors,
        abilities: base.abilities.clone(),
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd magicbot-engine && cargo test --lib layers::compute_skeleton`
Expected: PASS — both tests pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/layers.rs
git commit -m "feat(layers): ContinuousEffect/BaseChars/Counters/Status + compute() skeleton"
```

---

### Task 3: Apply layers 4 (type) and 5 (color), timestamp-ordered

**Files:**
- Modify: `magicbot-engine/src/layers.rs`
- Test: in-file `#[cfg(test)]` in `layers.rs`

**Interfaces:**
- Consumes: all prior types.
- Produces: `compute()` now applies `AddType` (layer 4: OR the type flags onto base) and `SetColors` (layer 5: last-timestamp wins) in the fixed cross-layer order Type→Color. Adds a private helper `effects_in_layer(effects, layer) -> Vec<&ContinuousEffect>` returning effects of that layer sorted ascending by timestamp (CR 613.7).

- [ ] **Step 1: Write the failing test**

Add a test module to `magicbot-engine/src/layers.rs`:

```rust
#[cfg(test)]
mod layer_4_5_tests {
    use super::*;
    use crate::ids::Timestamp;

    fn land_base() -> BaseChars {
        BaseChars {
            pt: None,
            types: TypeSet::none().with_land(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        }
    }

    #[test]
    fn animate_land_adds_creature_type_layer_4() {
        // CR 613.1d layer 4: type-changing. "Target land becomes a creature."
        let base = land_base();
        let effects = vec![ContinuousEffect {
            effect: Effect::AddType(TypeSet::none().with_creature()),
            timestamp: Timestamp(1),
            is_cda: false,
        }];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert!(out.types.is_creature(), "creature type added");
        assert!(out.types.is_land(), "still a land");
    }

    #[test]
    fn color_set_last_timestamp_wins_layer_5() {
        // CR 613.1e layer 5, CR 613.7 timestamp order within a layer.
        let base = land_base();
        let effects = vec![
            ContinuousEffect {
                effect: Effect::SetColors(ColorSet::colorless().with(Color::White)),
                timestamp: Timestamp(1),
                is_cda: false,
            },
            ContinuousEffect {
                effect: Effect::SetColors(ColorSet::colorless().with(Color::Blue)),
                timestamp: Timestamp(2),
                is_cda: false,
            },
        ];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert!(out.colors.has(Color::Blue), "later timestamp wins");
        assert!(!out.colors.has(Color::White), "earlier set overwritten");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd magicbot-engine && cargo test --lib layers::layer_4_5`
Expected: FAIL — the skeleton ignores effects, so `is_creature()` is false and color is colorless.

- [ ] **Step 3: Write the implementation**

In `magicbot-engine/src/layers.rs`, add the helper and rewrite `compute`'s body (replace the skeleton `compute`):

```rust
fn effects_in_layer<'a>(
    effects: &'a [ContinuousEffect],
    layer: Layer,
) -> Vec<&'a ContinuousEffect> {
    let mut v: Vec<&ContinuousEffect> =
        effects.iter().filter(|e| layer_of(&e.effect) == layer).collect();
    // CR 613.7: ascending timestamp order within a layer.
    // CR 613.8 dependency ordering is DEFERRED (stub: timestamp only).
    v.sort_by_key(|e| e.timestamp.0);
    v
}

pub fn compute(
    base: &BaseChars,
    effects: &[ContinuousEffect],
    counters: &Counters,
    _status: Status,
) -> Chars {
    // Start from the layer-1 copiable base (copy/face-down handled in a later task).
    let mut types = base.types;
    let mut colors = base.colors;
    let abilities = base.abilities.clone();
    let pt = base.pt;

    // Layer 4: type-changing (OR new types in).
    for ce in effects_in_layer(effects, Layer::Type) {
        if let Effect::AddType(t) = &ce.effect {
            if t.is_creature() {
                types.set_creature(true);
            }
        }
    }

    // Layer 5: color-changing (set; last timestamp wins).
    for ce in effects_in_layer(effects, Layer::Color) {
        if let Effect::SetColors(c) = &ce.effect {
            colors.set_to(*c);
        }
    }

    // Layers 6 and 7 added in later tasks.
    let _ = counters;

    Chars { pt, types, colors, abilities }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd magicbot-engine && cargo test --lib layers`
Expected: PASS — all layer tests plus the earlier ones pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/layers.rs
git commit -m "feat(layers): apply layer 4 (type) and layer 5 (color), timestamp-ordered"
```

---

### Task 4: Apply layer 6 (abilities) — add / remove-all, CR 613.3 CDA-first

**Files:**
- Modify: `magicbot-engine/src/layers.rs`
- Test: in-file `#[cfg(test)]` in `layers.rs`

**Interfaces:**
- Consumes: all prior.
- Produces: `compute()` applies layer 6 between layer 5 and layer 7. Within layer 6: CDAs first (CR 613.3), then by timestamp; `AddAbility` adds, `RemoveAllAbilities` clears. Add/remove interleave by the ordering so a later `RemoveAllAbilities` wipes an earlier-added ability (conformance F6).

- [ ] **Step 1: Write the failing test**

Add a test module:

```rust
#[cfg(test)]
mod layer_6_tests {
    use super::*;
    use crate::ids::Timestamp;

    fn vanilla() -> BaseChars {
        BaseChars {
            pt: Some(Pt { power: 2, toughness: 2 }),
            types: TypeSet::none().with_creature(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        }
    }

    #[test]
    fn add_ability_layer_6() {
        let base = vanilla();
        let effects = vec![ContinuousEffect {
            effect: Effect::AddAbility(Ability::Flying),
            timestamp: Timestamp(1),
            is_cda: false,
        }];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert!(out.abilities.has(Ability::Flying));
    }

    #[test]
    fn add_then_remove_all_by_timestamp_f6() {
        // CR 613.1f, 613.3: grant flying (ts1), then remove all abilities (ts2) -> no flying.
        let base = vanilla();
        let effects = vec![
            ContinuousEffect {
                effect: Effect::AddAbility(Ability::Flying),
                timestamp: Timestamp(1),
                is_cda: false,
            },
            ContinuousEffect {
                effect: Effect::RemoveAllAbilities,
                timestamp: Timestamp(2),
                is_cda: false,
            },
        ];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert!(!out.abilities.has(Ability::Flying), "later remove-all wins");
    }

    #[test]
    fn remove_all_then_add_by_timestamp_f6_reverse() {
        // Reverse timestamps: remove all (ts1) then grant flying (ts2) -> flying present.
        let base = vanilla();
        let effects = vec![
            ContinuousEffect {
                effect: Effect::RemoveAllAbilities,
                timestamp: Timestamp(1),
                is_cda: false,
            },
            ContinuousEffect {
                effect: Effect::AddAbility(Ability::Flying),
                timestamp: Timestamp(2),
                is_cda: false,
            },
        ];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert!(out.abilities.has(Ability::Flying), "later add wins");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd magicbot-engine && cargo test --lib layers::layer_6`
Expected: FAIL — abilities currently pass through unchanged.

- [ ] **Step 3: Write the implementation**

In `compute()`, change `let abilities = base.abilities.clone();` to `let mut abilities = base.abilities.clone();`, and insert this block after the layer-5 loop and before the `let _ = counters;` line:

```rust
    // Layer 6: ability add / remove-all. CR 613.3: CDAs first, then timestamp.
    for ce in effects_in_layer_cda_first(effects, Layer::Ability) {
        match &ce.effect {
            Effect::AddAbility(a) => abilities.add(*a),
            Effect::RemoveAllAbilities => abilities.clear(),
            _ => {}
        }
    }
```

Then add this helper next to `effects_in_layer`:

```rust
/// Like effects_in_layer but applies CR 613.3: CDA-sourced effects first,
/// then non-CDA, each group ascending by timestamp.
fn effects_in_layer_cda_first<'a>(
    effects: &'a [ContinuousEffect],
    layer: Layer,
) -> Vec<&'a ContinuousEffect> {
    let mut v = effects_in_layer(effects, layer);
    // stable partition: CDAs (true) before non-CDAs (false), preserving timestamp order.
    v.sort_by_key(|e| !e.is_cda);
    v
}
```

NOTE: `sort_by_key` is stable in Rust, so the prior timestamp ordering is preserved within each CDA/non-CDA group.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd magicbot-engine && cargo test --lib layers`
Expected: PASS — all three layer-6 tests plus prior tests pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/layers.rs
git commit -m "feat(layers): apply layer 6 (abilities) with CR 613.3 CDA-first ordering"
```

---

### Task 5: Apply layer 7 (P/T) sublayers 7a→7b→7c→7d + counters

**Files:**
- Modify: `magicbot-engine/src/layers.rs`
- Test: in-file `#[cfg(test)]` in `layers.rs`

**Interfaces:**
- Consumes: all prior.
- Produces: `compute()` applies layer 7 last, in sublayer order: 7a CDA-P/T (effects with `is_cda` setting P/T — modeled as `SetBasePt` with `is_cda: true`), 7b set (`SetBasePt`, non-CDA), 7c modify (`ModifyPt` AND counters folded in), 7d switch (`SwitchPt`). The critical invariant (conformance F3): a 7b set NEVER overwrites a later 7c counter — set runs first, then modify/counters add on top.

- [ ] **Step 1: Write the failing test**

Add a test module:

```rust
#[cfg(test)]
mod layer_7_tests {
    use super::*;
    use crate::ids::Timestamp;

    fn base_3_3() -> BaseChars {
        BaseChars {
            pt: Some(Pt { power: 3, toughness: 3 }),
            types: TypeSet::none().with_creature(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        }
    }

    #[test]
    fn set_7b_before_modify_7c_f3() {
        // CR 613.4b/c: base 3/3, set to 1/1 (7b), +1/+1 counter (7c) -> 2/2,
        // regardless of timestamp. The set must NOT erase the later counter.
        let base = base_3_3();
        let effects = vec![ContinuousEffect {
            effect: Effect::SetBasePt(Pt { power: 1, toughness: 1 }),
            timestamp: Timestamp(5), // later than the counter conceptually; must not matter
            is_cda: false,
        }];
        let counters = Counters::new().with_plus1(1);
        let out = compute(&base, &effects, &counters, Status { tapped: false });
        assert_eq!(out.pt, Some(Pt { power: 2, toughness: 2 }));
    }

    #[test]
    fn modify_7c_stacks_on_base() {
        // base 3/3, +2/+2 modify, +1/+1 counter -> 6/6.
        let base = base_3_3();
        let effects = vec![ContinuousEffect {
            effect: Effect::ModifyPt { power: 2, toughness: 2 },
            timestamp: Timestamp(1),
            is_cda: false,
        }];
        let counters = Counters::new().with_plus1(1);
        let out = compute(&base, &effects, &counters, Status { tapped: false });
        assert_eq!(out.pt, Some(Pt { power: 6, toughness: 6 }));
    }

    #[test]
    fn switch_7d_is_last_f11() {
        // CR 613.4d: 1/4, +2/+0 (7c) -> 3/4, then switch -> 4/3.
        let base = BaseChars {
            pt: Some(Pt { power: 1, toughness: 4 }),
            types: TypeSet::none().with_creature(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        };
        let effects = vec![
            ContinuousEffect {
                effect: Effect::ModifyPt { power: 2, toughness: 0 },
                timestamp: Timestamp(1),
                is_cda: false,
            },
            ContinuousEffect {
                effect: Effect::SwitchPt,
                timestamp: Timestamp(2),
                is_cda: false,
            },
        ];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert_eq!(out.pt, Some(Pt { power: 4, toughness: 3 }));
    }

    #[test]
    fn cda_pt_7a_before_set_7b_f4() {
        // CR 613.4a/b: a CDA defining P/T applies in 7a, then a non-CDA set in 7b overrides.
        let base = BaseChars {
            pt: Some(Pt { power: 0, toughness: 0 }),
            types: TypeSet::none().with_creature(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        };
        let effects = vec![
            // CDA sets 5/5 in 7a
            ContinuousEffect {
                effect: Effect::SetBasePt(Pt { power: 5, toughness: 5 }),
                timestamp: Timestamp(1),
                is_cda: true,
            },
            // non-CDA set 1/1 in 7b overrides the CDA
            ContinuousEffect {
                effect: Effect::SetBasePt(Pt { power: 1, toughness: 1 }),
                timestamp: Timestamp(2),
                is_cda: false,
            },
        ];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert_eq!(out.pt, Some(Pt { power: 1, toughness: 1 }), "7b set overrides 7a CDA");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd magicbot-engine && cargo test --lib layers::layer_7`
Expected: FAIL — P/T currently passes base through, ignoring effects and counters.

- [ ] **Step 3: Write the implementation**

In `compute()`, replace the `let pt = base.pt;` binding with `let mut pt = base.pt;`, remove the placeholder `let _ = counters;` line, and insert this block at the END of `compute` (just before building `Chars`):

```rust
    // Layer 7 (P/T), sublayers in fixed order 7a -> 7b -> 7c -> 7d (CR 613.4).
    if pt.is_some() {
        let pt_effects = effects_in_layer(effects, Layer::Pt);

        // 7a: CDA P/T sets (is_cda && SetBasePt), timestamp order.
        for ce in pt_effects.iter().filter(|e| e.is_cda) {
            if let Effect::SetBasePt(p) = &ce.effect {
                pt = Some(*p);
            }
        }
        // 7b: non-CDA sets, timestamp order.
        for ce in pt_effects.iter().filter(|e| !e.is_cda) {
            if let Effect::SetBasePt(p) = &ce.effect {
                pt = Some(*p);
            }
        }
        // 7c: modify effects + counters (CR 613.4c folds counters here).
        let mut p = pt.unwrap();
        for ce in pt_effects.iter() {
            if let Effect::ModifyPt { power, toughness } = &ce.effect {
                p.power += power;
                p.toughness += toughness;
            }
        }
        let (cp, ct) = counters.net_pt();
        p.power += cp;
        p.toughness += ct;
        pt = Some(p);

        // 7d: switch P/T, timestamp order (each switch flips).
        for ce in pt_effects.iter() {
            if let Effect::SwitchPt = &ce.effect {
                let cur = pt.unwrap();
                pt = Some(Pt { power: cur.toughness, toughness: cur.power });
            }
        }
    }
```

NOTE on 7a/7b: `effects_in_layer` already sorts by timestamp ascending, so iterating the filtered subsets preserves timestamp order. 7c iterates all Pt effects but only matches `ModifyPt`. Counters fold into 7c per current CR (post-2021 merge).

- [ ] **Step 4: Run test to verify it passes**

Run: `cd magicbot-engine && cargo test --lib layers`
Expected: PASS — all four layer-7 tests plus all prior tests pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/layers.rs
git commit -m "feat(layers): apply layer 7 P/T sublayers 7a-7d with counters in 7c"
```

---

### Task 6: Integration conformance tests + Humility (F5) + dependency-stub documentation

**Files:**
- Create: `magicbot-engine/tests/layers_conformance.rs`
- Modify: `magicbot-engine/src/layers.rs` (add a doc comment to the dependency stub)

**Interfaces:**
- Consumes: full public API of `layers`.
- Produces: an integration test file exercising the headline cross-layer conformance cases as black-box tests through the public `compute()` — the Humility+anthem set-vs-modify case (F5), the cross-layer fixed-order case (F2), and a no-effects identity (F1). Plus a doc comment on `effects_in_layer` recording that CR 613.8 dependency ordering is deliberately deferred.

- [ ] **Step 1: Write the failing test**

Create `magicbot-engine/tests/layers_conformance.rs`:

```rust
// CR 613 conformance, black-box through compute(). Cases from
// prior-art/conformance-cases.md section F.
use magicbot_engine::ids::Timestamp;
use magicbot_engine::layers::{
    compute, Ability, BaseChars, Chars, Color, ColorSet, ContinuousEffect, Counters, Effect,
    Pt, Status, TypeSet,
};

fn creature(power: i32, toughness: i32) -> BaseChars {
    BaseChars {
        pt: Some(Pt { power, toughness }),
        types: TypeSet::none().with_creature(),
        colors: ColorSet::colorless(),
        abilities: {
            let mut a = magicbot_engine::layers::AbilitySet::new();
            a.add(Ability::Flying);
            a
        },
    }
}

fn untapped() -> Status {
    Status { tapped: false }
}

#[test]
fn f1_no_effects_is_identity() {
    let base = creature(2, 2);
    let out = compute(&base, &[], &Counters::new(), untapped());
    assert_eq!(out.pt, Some(Pt { power: 2, toughness: 2 }));
    assert!(out.abilities.has(Ability::Flying));
}

#[test]
fn f5_humility_plus_anthem_set_vs_modify() {
    // Humility: base P/T 1/1 (7b set) + remove all abilities (layer 6).
    // Glorious Anthem: +1/+1 (7c modify). Result: 2/2 with NO abilities.
    let base = creature(3, 3); // base has Flying
    let effects = vec![
        ContinuousEffect {
            effect: Effect::RemoveAllAbilities,
            timestamp: Timestamp(1),
            is_cda: false,
        },
        ContinuousEffect {
            effect: Effect::SetBasePt(Pt { power: 1, toughness: 1 }),
            timestamp: Timestamp(1),
            is_cda: false,
        },
        ContinuousEffect {
            effect: Effect::ModifyPt { power: 1, toughness: 1 },
            timestamp: Timestamp(2),
            is_cda: false,
        },
    ];
    let out: Chars = compute(&base, &effects, &Counters::new(), untapped());
    assert_eq!(out.pt, Some(Pt { power: 2, toughness: 2 }), "set 1/1 then +1/+1 = 2/2");
    assert!(!out.abilities.has(Ability::Flying), "Humility stripped abilities");
}

#[test]
fn f2_cross_layer_order_type_before_color_independent_of_timestamp() {
    // Type change (layer 4) at a LATER timestamp than a color change (layer 5)
    // still applies first because cross-layer order is fixed (CR 613.1).
    let base = BaseChars {
        pt: None,
        types: TypeSet::none().with_land(),
        colors: ColorSet::colorless(),
        abilities: magicbot_engine::layers::AbilitySet::new(),
    };
    let effects = vec![
        ContinuousEffect {
            effect: Effect::SetColors(ColorSet::colorless().with(Color::Blue)),
            timestamp: Timestamp(1),
            is_cda: false,
        },
        ContinuousEffect {
            effect: Effect::AddType(TypeSet::none().with_creature()),
            timestamp: Timestamp(9),
            is_cda: false,
        },
    ];
    let out = compute(&base, &effects, &Counters::new(), untapped());
    assert!(out.types.is_creature());
    assert!(out.colors.has(Color::Blue));
}
```

- [ ] **Step 2: Run test to verify it fails or passes**

Run: `cd magicbot-engine && cargo test --test layers_conformance`
Expected: PASS if Tasks 3–5 are correct (these are black-box checks of already-built behavior). If `f5` FAILS, the set-vs-modify ordering in Task 5 has a bug — fix `compute()` so 7b set precedes 7c counters/modify, then re-run. (This is the integration check that the sublayers compose correctly.)

- [ ] **Step 3: Document the dependency-ordering deferral**

In `magicbot-engine/src/layers.rs`, replace the comment inside `effects_in_layer` that currently says `// CR 613.8 dependency ordering is DEFERRED (stub: timestamp only).` with this fuller doc note directly above the `fn effects_in_layer` definition:

```rust
/// Returns the effects in `layer`, ordered for application.
///
/// CR 613.7 timestamp ordering (ascending) is implemented. CR 613.8 DEPENDENCY
/// ordering is DEFERRED to a later plan: when one same-layer effect changes what
/// another applies to (e.g. Blood Moon vs. an Urborg-style effect in layer 4),
/// dependency order overrides timestamp, with mutual loops falling back to
/// timestamp (613.8b) and re-evaluation after each application (613.8c). This
/// stub applies pure timestamp order, which is correct for all non-dependent
/// effect sets (the common case and every conformance case in this slice).
```

- [ ] **Step 4: Verify whole crate green + lint-clean**

Run: `cd magicbot-engine && cargo test && cargo clippy --all-targets -- -D warnings`
Expected: all tests pass (unit + both integration files); clippy no warnings.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/tests/layers_conformance.rs magicbot-engine/src/layers.rs
git commit -m "test(layers): cross-layer conformance (F1/F2/F5) + document 613.8 deferral"
```

---

## Out of scope for this plan (deferred to later plans)

1. **CR 613.8 dependency ordering** (Blood Moon vs Urborg-style). Stub applies timestamp order; documented in Task 6.
2. **Layers 1 (copy/copiable values), 2 (control), 3 (text-changing).** Copy is a big subsystem (CR 707); control lives in GameState; text-changing is rare. Enum has room; application deferred.
3. **Wiring into `GameState`** — gathering real `BaseChars` from a CardDb and real `ContinuousEffect`s from active sources, and a `GameState::characteristics(id)` method. This slice is the pure algorithm only.
4. **Subtypes, supertypes, full keyword/ability vocabulary, P/T as `*`/CDA-from-game-state** (e.g. Tarmogoyf counting card types in graveyards). `is_cda` flag exists; the CDA's value here is a fixed `SetBasePt`, not a game-state-derived count.
5. **613.6 effect-set lock-in, 613.9/.10/.11 (player/rules effects), face-down (1b), CDAs-in-all-zones.** Status has only `tapped`.
6. **Memoization/invalidation cache.** `compute()` is a pure function recomputed each call; caching is a later optimization.

## Self-review notes

- **Spec coverage:** layers-and-characteristics.md §1 (7 layers, fixed cross-layer order) → Tasks 3–5 + constraint. §1 layer-7 sublayers 7a–7d, set-before-modify, counters-in-7c → Task 5. §1 613.3 CDA-first in layer 6 → Task 4. §2 613.7 timestamp → Task 3 helper. 613.8 dependency → explicitly deferred + documented (Task 6). Conformance F1 (identity) → Task 6. F2 (cross-layer fixed) → Task 6. F3 (set vs modify) → Task 5. F4 (7a before 7b) → Task 5. F5 (Humility+anthem) → Task 6. F6 (add/remove-all) → Task 4. F11 (switch last) → Task 5. F7/F8/F9 (dependency) → deferred. F10 (613.6 lock-in), F12 (re-timestamp), F13 (CDA all zones), F14 (player/rules) → deferred (need GameState/CardDb).
- **Type consistency:** `compute(&BaseChars, &[ContinuousEffect], &Counters, Status) -> Chars` signature identical across Tasks 2–6. `Effect` variants identical Tasks 2–5. `is_cda` field used consistently (Task 2 defines, Tasks 4/5 read). `effects_in_layer`/`effects_in_layer_cda_first` helper names consistent Tasks 3–5.
- **Mutability note:** Task 3 introduces `compute` with `let mut types/colors`; Task 4 changes `abilities` to `mut`; Task 5 changes `pt` to `mut` and removes the `let _ = counters;` placeholder. Each task's step text says exactly what to change so an out-of-order reader isn't lost.
