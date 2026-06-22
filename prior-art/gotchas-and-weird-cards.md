# MTG Rules-Engine Conformance Hazards: The Canonical GOTCHA Checklist

A checklist of interactions that break naive object/state data models. CR references are to the Comprehensive Rules (current numbering as of the 2024–2026 rules). Each item: **the interaction → why naive models fail → CR basis → data-model implication.**

> Note on rule numbers: Magic's rule numbering shifts between rules updates (new keyword actions/abilities are inserted, renumbering later rules). Section/rule numbers below reflect the current (2026) numbering; treat the cited *topic* as authoritative even if a future update nudges a sub-number.

The single deepest meta-lesson: **a game object is not a stable pointer.** Objects change zones (and thereby become *new objects* with no memory), get copied, merge, phase, go face-down, and resolve into effects that outlive them. Almost every gotcha below is a symptom of treating "the card" or "the permanent" as one persistent identity.

---

## 1. Last Known Information (LKI)

- [ ] **Abilities resolve using info about objects that have already left their zone.** A naive model that reads the live object's current state (or null, if it's gone) returns wrong values. Classic cases: "draw cards equal to that creature's power" when the creature died before resolution; Boros Reckoner's damage-dealt trigger after it's already in the graveyard; "sacrifice a creature, gain life equal to its toughness."
- **Why naive fails:** the engine looks up the object by ID, finds it in the graveyard (a new object) or finds nothing, and reads 0 / wrong characteristics. Layer effects, counters, and attachments that modified it on the battlefield are gone.
- **CR basis:** CR 608.2h (resolution uses last known information when info is needed about an object that's no longer in the public zone it was expected to be in); CR 603.6d and 603.10 (leaves-the-battlefield-style triggered abilities, which "look back in time" / use LKI). The phrase "last known information" appears throughout the rules. LKI = the object's characteristics *as they last existed in the relevant zone, with all continuous effects/counters applied.*
- **Data-model implication:** When an object leaves a zone, **snapshot its full characteristic state** (all layers applied, P/T, counters, attachments, controller) and make that snapshot resolvable by abilities that were already on the stack or that triggered off the leave event. Stack objects and triggered abilities must carry references that gracefully fall back to the snapshot, not to a live lookup. LKI is frozen — it does not keep updating after the object leaves.

---

## 2. Linked Abilities (CR 607)

- [ ] **Two abilities on one object reference each other and must be paired correctly.** "Exile target creature" + "Return the exiled card" (Banisher Priest / O-Ring style); "you may cast the exiled card" (Chandra, Torch of Defiance / impulse-draw); "the chosen color/card/type." With multiple instances or multiple exiled cards, the engine must know *which* exiled set belongs to *which* ability instance.
- **Why naive fails:** A model with a global "exiled-by-this-card" list breaks when one source exiles multiple cards across multiple activations, or when text-changing/copy effects create ambiguity. It also breaks if the exiling ability never happened (e.g., countered, or the object entered without triggering) — the second ability then refers to *nothing*, not to "all cards in exile."
- **CR basis:** CR 607.1–607.5. Two abilities are linked when one refers to "the exiled card[s]" / "the chosen ___" that the other ability acted on. CR 607.4: if the first ability didn't happen for a given object, the second does nothing for it. CR 607.3: an object can have multiple linked-ability pairs.
- **Data-model implication:** Store linkage as **per-instance association records** tied to the specific ability resolution that created them (e.g., an exile event records "exiled-with object X, by ability-instance Y"), not as a global property of the card. The "exiled card" reference must survive even though the exiled card itself changes object identity in exile. If the linked second ability has no record, it references nothing.

---

## 3. "As ~ enters" Replacement Effects, Enters-With Choices, and "As It Leaves"

- [ ] **Some choices/state are locked in *as part of entering*, before the permanent legally exists on the battlefield.** Enters tapped; enters as a copy (Clone — choose what to copy *as it enters*); enters with N +1/+1 counters; "as ~ enters, choose a color/creature type"; "as ~ enters, choose an opponent." These are static abilities whose effect is part of the entering event (replacement effects on the entering event), not triggers, so they never use the stack and cannot be responded to.
- [ ] **"As it leaves the battlefield" effects** read state at the moment of leaving (e.g., "when ~ leaves, do X based on counters on it"). These are self-replacement effects on the leave event.
- **Why naive fails:** A model that (a) puts the permanent on the battlefield first, then (b) runs triggers, will tap/counter/choose too late, allowing illegal windows (responding to a Clone's copy choice; state-based actions firing on a 0-toughness creature before its counters are applied). Treating enters-with-counters as a normal trigger causes a creature with toughness from counters to momentarily have lethal/zero toughness.
- **CR basis:** CR 603.6e ("As ~ enters" / "~ enters as" text is a static ability whose effect happens as part of the entering event — not a triggered ability); CR 614.12 and 614.1c–d (replacement effects that modify how a permanent enters the battlefield); CR 614 / 614.1c ("as it leaves the battlefield" self-replacement, also 603.6d); CR 616 for ordering multiple replacements affecting one event; CR 706/707.2 for copy-as-enters (Clone) interacting with copiable values. (Note: CR 614.13 is unrelated — it merely says one object can't be chosen to change zones more than once when applying entry-modifying replacements.)
- **Data-model implication:** Model entry as a **transformable event** with a replacement-effect pipeline that runs *before* the object is considered on the battlefield and before SBAs are checked. Counters/tapped-status/copy-choices/"as" choices must be applied atomically with placement. "As it leaves" needs the LKI snapshot from gotcha #1.

---

## 4. Copiable Values vs. the Layer System

- [ ] **A "copy" copies only the copiable values, not the final printed-on-the-table characteristics.** When Clone copies a creature, it copies that creature's *copiable values* (name, mana cost, color indicator, card types, subtypes, supertypes, rules text, P/T as defined by characteristic-defining abilities and copy effects already applied to it) — but **not** counters, **not** auras/equipment, **not** most other continuous effects, **not** control/status. Then layer effects re-apply on top.
- **Why naive fails:** A model that copies the live, fully-computed state will wrongly carry over +1/+1 counters, "gets +3/+3 until end of turn" buffs, tapped status, and control effects. Conversely it will *miss* copy effects and CDAs that ARE part of copiable values (e.g., copying something that's itself a copy of a third thing; copying a face-up creature that has a CDA P/T like `*/*`).
- **CR basis:** CR 706.2 (the copy *acquires* the copiable values); CR 707.2 (the *definition* of copiable values: printed values as modified by other copy effects, by "as ~ enters"/"as ~ is turned face up" abilities that set characteristics, and by face-down status); CR 707.2 sub-rules (face-down copiable values); CR 613 (layer system) — copies feed into layer 1 of the type/CDA computation; copy effects are applied in layer 1, then other layers stack on top. CDAs are part of copiable values; counters and external buffs are not.
- **Data-model implication:** Separate **copiable values** (base layer) from **applied continuous effects**. Copy = clone the copiable-value record only, then recompute layers fresh. Do not snapshot the rendered characteristics. CDAs must be classified as part of copiable values (and applied in their proper layer), while counters and external buffs are not.

---

## 5. Face-Down Permanents (Morph / Manifest / Disguise / Cloak)

- [ ] **A face-down permanent is (by default) a 2/2 colorless creature with no name, no types beyond "creature," no mana cost, no abilities — regardless of what card it actually is.** (Cloak's face-down profile is a 2/2 with ward {2}.) Turning it face up is a *special action* (not on the stack, can't be responded to) that reveals the real card. Manifest can put a *land* or *noncreature* card face down (still a 2/2 creature while face down). The hidden information (which card it is) must not leak, but the game still tracks it.
- **Why naive fails:** Reading the underlying card's characteristics while face-down (showing its real name/P/T/abilities), or losing the underlying card's identity when computing the 2/2 face-down state. Also: copy effects copy the *face-down* copiable values (a plain 2/2), not the hidden card; and combat/triggers must use the 2/2 stats. Turning face up mid-combat keeps it the same object (counters/damage/attachments persist) but suddenly its real characteristics apply.
- **CR basis:** CR 707.2 sub-rules (face-down spells/permanents have specific copiable values: 2/2 creature, no name/types/abilities/mana cost); CR 708 (face-down spells and permanents); CR 702 (morph / disguise keyword abilities); CR 701 keyword actions for manifest (~701.40) and cloak (~701.58, grants ward {2}); CR 116.2 (turning face up is a special action that doesn't use the stack), with the actual turn-face-up procedures in 702 (morph/disguise) and 701 (manifest/cloak). The card's real characteristics are tracked privately but are not the object's characteristics while face down. (Note: rule 711 is "Leveler Cards," not manifest/cloak.)
- **Data-model implication:** Model face-down as an **override of copiable values** to the canonical 2/2 blank profile (or 2/2-with-ward for cloak), with the real card stored as private hidden state (not contributing to public characteristics or to opponents' visible info). Turning face up swaps the copiable-value source *on the same object identity* — counters, attachments, summoning sickness, and damage marked must persist across the flip.

---

## 6. Merged Permanents (Mutate)

- [ ] **A merged permanent is a single object whose characteristics come from the *top* card/token, but whose *abilities* come from *all* components.** It's one permanent, one object on the battlefield, but it is physically a stack of cards that must un-merge correctly when it leaves.
- **Why naive fails:** Treating each merged card as a separate permanent (it isn't — it's one), or taking abilities only from the top, or characteristics from all. When the permanent dies/exiles/bounces, *one* object leaves but *each component card goes to the appropriate zone separately* (and the owner of each component decides — components can have different owners). Copying a merged creature copies the *top* card's copiable values plus the merge's granted abilities-from-all, not the whole stack.
- **CR basis:** CR 730 (Merging with Permanents) is the merge rule in current numbering (not 721 and not the "707/711 region"). A merged permanent has the characteristics of the topmost component; if merged via mutate it also has all abilities of all components; on leaving, one permanent leaves and each component goes to its owner's appropriate zone; replacement effects applied to the leave/zone-change apply to all components. Transform interactions are governed alongside CR 712 (double-faced/transforming permanents).
- **Data-model implication:** Model the permanent as a **single object with an ordered component list**. Characteristics resolve from `components[top]`; abilities aggregate over all components. Zone-change must fan out: one "leaves" event, then per-component placement honoring each component's owner. Counters/attachments/damage live on the merged object, not on components, and must be discarded (counters) on un-merge per zone-change rules.

---

## 7. Phasing

- [ ] **A phased-out permanent still exists and is the *same object* — it is merely "treated as though it doesn't exist."** It does not change zones, does not change controller, is not a new object on phase-in, retains all counters, and its attachments/Auras/Equipment phase out *with* it ("indirectly") and return attached.
- **Why naive fails:** Modeling phase-out as removal-to-a-zone (loses object identity, triggers leaves-the-battlefield abilities — which it must NOT), or as a status flag that other queries ignore (so combat/targeting/SBAs wrongly still see it). Also: continuous effects and Auras that left while it was phased out, or new objects, can have weird interactions on return. Phase-in/out happens at the untap step (or via instant-speed effects like Teferi's Protection) and is *not* a zone change, so "enters the battlefield" triggers must NOT fire on phase-in.
- **CR basis:** CR 702.26 (phasing): phased-out permanents are treated as though they don't exist; phasing is not a zone change; same object retains counters; attached permanents phase indirectly; "enters/leaves the battlefield" do not trigger from phasing.
- **Data-model implication:** Phasing is a **visibility/existence flag on the same object**, not a zone move. All game queries (targets, attackers, SBAs, "creatures you control") must filter out phased-out objects, but the object, its counters, and its attachment links remain intact. Phase events must NOT emit ETB/LTB triggers. Indirect phasing must cascade to attachments.

---

## 8. Tokens Ceasing to Exist + Copies of Tokens

- [ ] **A token in any zone other than the battlefield ceases to exist as a state-based action — but only after one "look."** When a token leaves the battlefield (dies, exiled, bounced, drawn), it goes to that zone momentarily (so leaves-the-battlefield/dies triggers fire and can see it), then the next SBA check removes it. A bounced token therefore "exists" in hand long enough for triggers but then vanishes.
- [ ] **Copying a token copies the token's copiable values, not "tokenness" inherently** — but the copy *is itself a token* only if created by a token-creating effect; "copy" effects (e.g., a populate/copy spell) create token copies. A copy of a token uses the token's copiable values as defined by the effect that created it (not its current modified state).
- **Why naive fails:** Removing the token instantly on zone change (so dies/LTB triggers see nothing — wrong); or letting it persist in the graveyard/hand. Also miscomputing copies of tokens: copying the token's *current* P/T instead of its as-created copiable values.
- **CR basis:** CR 111.7 / 111.8 (a token that's in a zone other than the battlefield ceases to exist as an SBA; it exists there momentarily for triggers; a token that has left can't move to another zone or come back); CR 704 (SBA timing); CR 706.2 / 707.2 (copying a token: copy the token's copiable values).
- **Data-model implication:** Tokens get a **"cease to exist" SBA** that runs *after* the zone-change triggers are put on the stack/logged, not synchronously with the move. LKI applies to the now-gone token (gotcha #1). Token copies derive copiable values from the originating effect's definition.

---

## 9. The Stack: Spells and Abilities as Objects Distinct from Their Source

- [ ] **An ability on the stack is its own object, independent of its source.** If the source leaves the battlefield, the ability still resolves (using LKI). A spell on the stack is an object on the stack, *not* a permanent — it's not the card in any zone. Activated/triggered abilities are not the permanent.
- [ ] **Modes, targets, X, divisions, and most choices are locked in when the spell/ability is put on the stack (cast/activated), not at resolution.** You cannot change targets or X at resolution. Costs and X are fixed at announcement.
- **Why naive fails:** Modeling an ability as a method-call on the live source object (so it fizzles or errors if the source is gone — wrong); re-reading X or re-choosing targets at resolution; treating the spell as still being "the card." Also: a spell countered or that leaves the stack does NOT trigger leaves-the-battlefield (it was never a permanent).
- **CR basis:** CR 113.7a (once activated or triggered, an ability exists on the stack independently of its source); CR 112 (spells) / 113 (abilities); CR 608.2 (resolution); CR 601.2 (casting locks in modes/targets/X/costs); CR 601.2b–2i (announcement sequence). Targets are chosen at cast and only re-evaluated for legality at resolution (and the spell is countered by game rules only if *all* its targets are illegal).
- **Data-model implication:** Stack entries are **first-class objects** carrying their own copy of: source reference (with LKI fallback), chosen modes, locked targets, locked X, cost info, and any divisions. Resolution reads these frozen values, not the source. Source departure must not invalidate the stack object.

---

## 10. Continuous Effects: Resolved-Spell Effects vs. Static Abilities vs. One-Shots

- [ ] **Three distinct kinds of "effects" with different lifetimes and layer behavior.** (a) A one-shot effect from a resolving spell/ability happens once and is done (draw a card, deal damage). (b) A *continuous* effect from a resolving spell/ability ("creatures get +1/+1 until end of turn") locks in its affected set at the moment it begins — it does not re-scan for new objects (unless it's a "for as long as" effect). (c) A *static* ability ("other creatures you control get +1/+1") continuously re-applies and dynamically picks up new objects.
- **Why naive fails:** Treating a resolved "until end of turn" pump as if it re-scans the battlefield (so creatures that enter later wrongly get the buff), or treating an anthem static ability as a one-time snapshot (so new creatures don't get it). Also: timestamps and dependency ordering in the layer system.
- **CR basis:** CR 611 (continuous effects from spells/abilities; 611.2c — the affected set is locked in when the effect begins and won't change, unless "for as long as"); CR 604 (static abilities, continuously applied); CR 613.7 (timestamps) and CR 613.8 (dependency) for ordering.
- **Data-model implication:** Distinguish **one-shot**, **fixed-set continuous** (snapshot the affected object set + characteristics when the effect begins), and **static/recomputed continuous** (re-evaluate every layer pass). The layer engine must hold all active continuous effects with timestamps and a dependency resolver, recomputing characteristics from copiable values upward on every state change.

---

## 11. Replacement-Effect Ordering, Self-Replacement, and "Skip"/"Instead"

- [ ] **Replacement effects don't use the stack, can't be responded to, and each applies at most once to a given event.** When multiple replacements apply to one event, the *affected player or controller of the affected object* chooses the order — and applying one can change which others now apply. Self-replacement effects (those modifying the very event they're part of, e.g., a card's own ETB "enters with counters" or "if you would draw, instead...") are applied first / handled specially. "Skip" effects and "instead" effects are replacements.
- **Why naive fails:** Applying replacements in a fixed engine-defined order (wrong — it's a player choice); applying the same replacement twice; looping (two "if you would die, exile instead" effects); not re-evaluating applicability after each replacement; treating prevention/redirection or "skip your draw step" as triggers.
- **CR basis:** CR 614 (replacement effects); CR 616 (interaction of replacement/prevention effects — ordering chosen by the affected entity, each applies once, re-check after each); CR 614.5 (a self-replacement effect is applied before other applicable replacement effects); CR 614.15 ("skip" effects are replacement effects). Replacement effects "wait" and watch for their event; they modify the event before it happens.
- **Data-model implication:** Model the game's mutating events through a **replacement pipeline**: collect all applicable replacements for an event, let the appropriate player order them, apply one, re-collect (an applied effect is marked used-for-this-event so it can't reapply), repeat until none apply, then execute. Self-replacement and "as enters" are part of this same pipeline (ties to #3). Prevention shields and damage redirection live here too.

---

## 12. Additional Notorious Hazards

- [ ] **Control-change effects vs. ownership; objects becoming new objects on zone change.** Every time an object changes zones it becomes a **new object** with no memory of prior counters, auras, control effects, or "until end of turn" buffs (CR 400.7). Auras/control effects generally don't carry. A naive model keying off card identity across zones is wrong; key off *object instances*. (Exceptions are explicitly enumerated, e.g., LKI, some "exiled and return" effects, foretell/adventure tracking.)

- [ ] **State-Based Actions are checked continuously and as a batch, before any player gets priority — not via triggers.** 0-toughness death, lethal damage, legend rule, +1/+1 & -1/-1 counter annihilation (CR 704.5q), token/copy cleanup, empty-library loss; "the game checks all at once and repeats until none apply." Modeling these as triggers (which use the stack) is wrong; they happen as a state correction with no stack, repeatedly, and can cascade. SBAs are checked *before* triggered abilities are put on the stack.
  - **CR basis:** CR 704. **Data-model implication:** a dedicated SBA loop that runs to fixpoint whenever a player would receive priority and before placing triggers on the stack.

- [ ] **Triggered abilities trigger off the game *state/event* even if the source/ability text is no longer present at resolution; "intervening if" clauses are re-checked at trigger AND resolution.** "Leaves the battlefield" / "dies" triggers must look back (LKI). Triggers are gathered and put on the stack the next time a player would get priority, in APNAP order, controller-ordered. Reflexive and delayed triggers add wrinkles.
  - **CR basis:** CR 603 (603.10 for "look back in time" triggers; 603.4 for intervening-if). **Implication:** event bus that records what triggered *and the LKI context*, with APNAP ordering and intervening-if double-checks.

- [ ] **Damage is marked on creatures (not subtracted from toughness) and wiped at cleanup; lethal-damage and deathtouch are SBA-driven.** Marked damage, -1/-1 effects, and toughness reduction are three different mechanisms with different layer/cleanup behavior. Naive "subtract from toughness" loses the distinction (e.g., regeneration, "damage can't be prevented," indestructible vs. lethal damage vs. 0 toughness).

- [ ] **"Until end of turn" / "this turn" effects end at cleanup, but cleanup can itself trigger SBA/ability loops; and some effects are "for as long as."** Duration tracking must be tied to the cleanup step and to the specific source/duration, including "for as long as" effects that *do* re-check (unlike normal fixed-set continuous effects in #10).

- [ ] **Targeting legality is re-checked only at resolution; a spell with multiple targets is countered by game rules only if ALL its targets are illegal; "target" vs. "choose" vs. "any number of targets."** Changing targets (redirect/Misdirection) is a separate operation. Hexproof/shroud/protection/ward alter legality at *cast* time and at *re-check* time differently.

- [ ] **Mana abilities don't use the stack and resolve immediately; loyalty/planeswalker abilities, saga chapter abilities (lore counters), and "can't be activated" restrictions** each break the simple "all abilities go on the stack" assumption (CR 605 mana abilities; CR 606 loyalty abilities; CR 714 sagas).

- [ ] **Layer-system dependency and timestamp loops** (CR 613.8): effect A's existence/text can depend on effect B (e.g., a text-changing effect that determines what another effect grants). Dependency ordering overrides timestamp ordering, and the engine must detect dependency loops and fall back to timestamps. This is a common source of XMage/Forge "wrong P/T or wrong abilities" bugs.

- [ ] **Characteristic-Defining Abilities (CDAs) apply in all zones and are part of copiable values** (CR 604.3, 707.2) — e.g., a creature with `*/*` defined by cards in graveyard has that P/T even in your hand/graveyard/stack/on the stack. Naive models that only compute P/T on the battlefield get hand/graveyard counts and copy interactions wrong.

- [ ] **"Becomes the target," "becomes blocked," "is dealt damage," and combat assignment ordering** are distinct events with their own triggers; combat damage is dealt simultaneously by all combatants in a step, and first strike creates an extra combat damage step only when needed (CR 509–510). Trample, banding, multiple blockers, and damage-assignment order must be locked in before damage.

---

### Cross-Cutting Design Principles (the conformance test boils down to these)

1. **Object identity is per-instance and per-zone.** Zone change = new object (CR 400.7). Keep instance IDs; don't key on card identity for state.
2. **Snapshot LKI on every zone/existence change** so already-committed stack objects and triggers can resolve correctly.
3. **Compute characteristics through the layer engine from copiable values upward, every recomputation** — never store rendered characteristics as ground truth.
4. **Replacement effects and "as enters" run as a player-ordered, re-evaluated pipeline before the event, off-stack.**
5. **SBAs run to fixpoint off-stack before triggers and before priority.**
6. **Stack entries freeze their choices (modes/targets/X) at announcement and resolve independently of their source.**
7. **Static abilities recompute dynamically; resolved continuous effects fix their affected set; one-shots happen once.**
8. **Special structures — face-down, merged, phased — keep one object identity while changing what characteristics/existence the game sees.**

---

**Sources:**
- [MTG Comprehensive Rules (Yawgatog hyperlinked)](https://yawgatog.com/resources/magic-rules/)
- [MTG Comprehensive Rules (current TXT, Wizards)](https://media.wizards.com/2026/downloads/MagicCompRules%2020260227.txt)
- [Resolving spells and abilities — MTG Wiki](https://mtg.fandom.com/wiki/Resolving_spells_and_abilities)
- [Layer — MTG Wiki](https://mtg.fandom.com/wiki/Layer)
- [Merge — MTG Wiki](https://mtg.fandom.com/wiki/Merge)
- [Mutate — MTG Wiki](https://mtg.fandom.com/wiki/Mutate)
- [Phasing — MTG Wiki](https://mtg.fandom.com/wiki/Phasing)
- [Manifest — MTG Wiki](https://mtg.fandom.com/wiki/Manifest)
- [Cloak — MTG Wiki](https://mtg.fandom.com/wiki/Cloak)
- [Face down — MTG Wiki](https://mtg.fandom.com/wiki/Face_down)
- [Last Known Information — Magic Judges Forum](https://apps.magicjudges.org/forum/topic/23752/)