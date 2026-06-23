# Wire CardDb into GameState: `characteristics()` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `GameState::characteristics(&self, id, &CardDb) -> Option<Chars>` — ask a live object on the battlefield for its CURRENT characteristics, composing all three subsystems: look up its printed base from the CardDb, gather the static continuous effects produced by OTHER permanents (scoping an anthem's "other creatures you control"), read its counters, and run the CR 613 layer engine.

**Architecture:** Adds methods to `GameState` (in `state.rs`) and small accessors to `Object`/`CardDef`. `CardDb` is passed as a `&CardDb` ARGUMENT — never stored in `GameState` (preserving the "static data is not forked" invariant). The affected-set scoping for `AnthemOtherCreatures` ("other creatures you control") is implemented here: when computing object X's characteristics, scan the battlefield for permanents whose def has an anthem static, and include that anthem's `ModifyPt` effect iff the anthem's controller equals X's controller and the anthem is not X itself and X is a creature. This is the first slice where state + carddb + layers compose into a real query.

**Tech Stack:** Rust 1.93+. No new deps.

**Source spec:** `prior-art/REVISED-data-model.md` §3 (computing characteristics: the inputs to the layer function) and §0 invariant 3 (characteristics never stored, always computed). Bridges `state.rs`, `object.rs`, `carddb.rs`, `layers.rs` — all already on master.

## Global Constraints

- **Rust edition 2021**, toolchain `1.93+`.
- **`CardDb` is NEVER stored in `GameState`.** It is a `&CardDb` parameter to `characteristics()`. Adding a `CardDb` field to `GameState` is a plan violation (it would put non-forked data on the fork path / bloat clones).
- **Characteristics are NEVER stored on the object** (CR 613.5 / REVISED §0 inv. 3) — `characteristics()` computes fresh every call by delegating to `layers::compute()`. This slice must NOT add a cached P/T field to `Object`.
- **Reuse, don't reimplement:** `characteristics()` MUST build `BaseChars` via `CardDef::base_chars()` and apply effects via `layers::compute()`. It must NOT duplicate any layer logic.
- **Anthem affected-set scoping** is: include an anthem's effect on target X iff (anthem.controller == X.controller) AND (anthem.id != X.id) AND (X is a creature per its base types). "Other creatures you control."
- **Only battlefield objects** contribute static abilities and only battlefield objects have meaningful characteristics for this slice; `characteristics()` returns `None` if the id is unknown.
- **TDD**; `cargo test` + `cargo clippy --all-targets -- -D warnings` clean after every task.

---

### Task 1: Expose the inputs `characteristics()` needs — `Object` counters + `CardDef` static introspection

**Files:**
- Modify: `magicbot-engine/src/object.rs`
- Modify: `magicbot-engine/src/carddb.rs`
- Test: in-file `#[cfg(test)]` in each

**Interfaces:**
- Consumes: existing `Object`, `CardDef`, `CounterKind`, `layers::Counters`, `carddb::StaticAbility`.
- Produces:
  - `impl Object { pub fn layer_counters(&self) -> crate::layers::Counters }` — builds a `layers::Counters` from this object's `PlusOnePlusOne` and `MinusOneMinusOne` counter counts.
  - `impl CardDef { pub fn static_abilities(&self) -> &[StaticAbility] }` — read access to the raw static abilities (needed for affected-set scoping; `static_effects()` loses the kind). Requires `statics` to be readable — add this accessor returning a slice.
  - `impl CardDef { pub fn is_creature(&self) -> bool }` — convenience: whether the printed base types include creature (used for "other CREATURES you control"). Reads `self.types.is_creature()`.

- [ ] **Step 1: Write the failing test**

Add to the test module in `magicbot-engine/src/object.rs`:

```rust
    #[test]
    fn layer_counters_reflects_plus_and_minus() {
        use crate::ids::{IdMinter, PlayerId};
        let mut m = IdMinter::new();
        let mut o = Object::new(m.mint(), PlayerId(0), Zone::Battlefield, 1);
        o.add_counter(CounterKind::PlusOnePlusOne, 3);
        o.add_counter(CounterKind::MinusOneMinusOne, 1);
        let c = o.layer_counters();
        // net +2/+2
        assert_eq!(c.net_pt(), (2, 2));
    }
```

Add to the test module in `magicbot-engine/src/carddb.rs`:

```rust
    #[test]
    fn static_abilities_and_is_creature_accessors() {
        let anthem = CardDef::new(DefId(3), "Glorious Anthem", 3)
            .with_types(TypeSet::none().with_enchantment())
            .with_static(StaticAbility::AnthemOtherCreatures { power: 1, toughness: 1 });
        assert_eq!(anthem.static_abilities().len(), 1);
        assert!(!anthem.is_creature());

        let bears = CardDef::new(DefId(1), "Grizzly Bears", 2)
            .with_types(TypeSet::none().with_creature())
            .with_pt(2, 2);
        assert!(bears.is_creature());
        assert_eq!(bears.static_abilities().len(), 0);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd magicbot-engine && cargo test --lib object::tests::layer_counters carddb::tests::static_abilities`
Expected: FAIL — `layer_counters`, `static_abilities`, `is_creature` not defined.

- [ ] **Step 3: Write the implementations**

Add to `impl Object` in `magicbot-engine/src/object.rs`:

```rust
    /// Build the layer engine's Counters view from this object's +1/+1 and
    /// -1/-1 counters (the counters that affect P/T in layer 7c).
    pub fn layer_counters(&self) -> crate::layers::Counters {
        crate::layers::Counters::new()
            .with_plus1(self.counter_count(CounterKind::PlusOnePlusOne))
            .with_minus1(self.counter_count(CounterKind::MinusOneMinusOne))
    }
```

Add to `impl CardDef` in `magicbot-engine/src/carddb.rs`:

```rust
    /// The raw static abilities (kind preserved, for affected-set scoping).
    pub fn static_abilities(&self) -> &[StaticAbility] {
        &self.statics
    }

    /// Whether the printed base types include creature.
    pub fn is_creature(&self) -> bool {
        self.types.is_creature()
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd magicbot-engine && cargo test --lib object carddb`
Expected: PASS — the two new tests plus all prior pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/object.rs magicbot-engine/src/carddb.rs
git commit -m "feat(engine): expose Object::layer_counters + CardDef static introspection accessors"
```

---

### Task 2: `GameState::base_chars_of` — resolve one object's printed base via CardDb

**Files:**
- Modify: `magicbot-engine/src/state.rs`
- Test: in-file `#[cfg(test)]` in `state.rs`

**Interfaces:**
- Consumes: `GameState`, `Object`, `carddb::{CardDb, DefId}`, `layers::BaseChars`.
- Produces: `impl GameState { pub fn base_chars_of(&self, id: ObjectId, db: &crate::carddb::CardDb) -> Option<crate::layers::BaseChars> }` — looks up the object, then its `CardDef` (by `DefId(object.def_id())`), returns `def.base_chars()`. `None` if the object or its def is unknown.

- [ ] **Step 1: Write the failing test**

Add a test module to `magicbot-engine/src/state.rs` (or extend the existing one):

```rust
#[cfg(test)]
mod characteristics_tests {
    use super::*;
    use crate::carddb::{sample_pool, DefId};
    use crate::ids::PlayerId;
    use crate::layers::Pt;
    use crate::object::Zone;

    #[test]
    fn base_chars_of_resolves_printed_pt() {
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        // Grizzly Bears is DefId(1), a 2/2.
        let id = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        let base = s.base_chars_of(id, &db).expect("base chars");
        assert_eq!(base.pt, Some(Pt { power: 2, toughness: 2 }));
        assert!(base.types.is_creature());
    }

    #[test]
    fn base_chars_of_unknown_object_is_none() {
        let db = sample_pool();
        let s = GameState::new(2, 0);
        assert!(s.base_chars_of(crate::ids::ObjectId(999), &db).is_none());
    }

    #[test]
    fn base_chars_of_unknown_def_is_none() {
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        // def_id 777 is not in sample_pool.
        let id = s.create_object(PlayerId(0), Zone::Battlefield, 777);
        assert!(s.base_chars_of(id, &db).is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd magicbot-engine && cargo test --lib state::characteristics_tests::base_chars`
Expected: FAIL — `base_chars_of` not defined.

- [ ] **Step 3: Write the implementation**

Add to `impl GameState` in `magicbot-engine/src/state.rs`:

```rust
    /// The layer-1 printed base for an object, resolved through the CardDb.
    /// `None` if the object or its definition is unknown. CardDb is passed in —
    /// it is static shared data, never stored on GameState.
    pub fn base_chars_of(
        &self,
        id: ObjectId,
        db: &crate::carddb::CardDb,
    ) -> Option<crate::layers::BaseChars> {
        let obj = self.objects.get(&id)?;
        let def = db.get(crate::carddb::DefId(obj.def_id()))?;
        Some(def.base_chars())
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd magicbot-engine && cargo test --lib state`
Expected: PASS — all three new tests plus prior state tests pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/state.rs
git commit -m "feat(engine): GameState::base_chars_of resolves printed base via CardDb"
```

---

### Task 3: `GameState::active_static_effects_for` — gather + scope anthem effects

**Files:**
- Modify: `magicbot-engine/src/state.rs`
- Test: in-file `#[cfg(test)]` in `state.rs`

**Interfaces:**
- Consumes: `GameState`, `carddb::{CardDb, DefId, StaticAbility}`, `layers::{Effect, ContinuousEffect}`, `ids::Timestamp`.
- Produces: `impl GameState { pub fn active_static_effects_for(&self, target: ObjectId, db: &CardDb) -> Vec<ContinuousEffect> }` — for the given target object, scan all battlefield objects; for each that has an `AnthemOtherCreatures` static in its def, include the corresponding `Effect::ModifyPt` as a `ContinuousEffect` (timestamp from a simple per-source counter; `is_cda: false`) IF the scoping holds: anthem source's controller == target's controller, source.id != target.id, and the target is a creature. Returns the effects to feed `compute()`.

- [ ] **Step 1: Write the failing test**

Add to the `characteristics_tests` module in `magicbot-engine/src/state.rs`:

```rust
    #[test]
    fn anthem_effect_applies_to_other_creature_same_controller() {
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        // Bears (creature, def 1) and Anthem (enchantment, def 3), both controller 0.
        let bears = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        let _anthem = s.create_object(PlayerId(0), Zone::Battlefield, 3);
        // Set controllers explicitly (create_object leaves controller None; we set owner=controller for battlefield here).
        s.get_mut(bears).unwrap().set_controller(Some(PlayerId(0)));
        s.get_mut(_anthem).unwrap().set_controller(Some(PlayerId(0)));

        let effects = s.active_static_effects_for(bears, &db);
        assert_eq!(effects.len(), 1, "anthem applies to the bears");
    }

    #[test]
    fn anthem_does_not_apply_to_itself() {
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        let anthem = s.create_object(PlayerId(0), Zone::Battlefield, 3);
        s.get_mut(anthem).unwrap().set_controller(Some(PlayerId(0)));
        // The anthem is an enchantment (not a creature) and is itself — no effect on itself.
        let effects = s.active_static_effects_for(anthem, &db);
        assert_eq!(effects.len(), 0);
    }

    #[test]
    fn anthem_does_not_apply_across_controllers() {
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        let bears = s.create_object(PlayerId(1), Zone::Battlefield, 1);
        let anthem = s.create_object(PlayerId(0), Zone::Battlefield, 3);
        s.get_mut(bears).unwrap().set_controller(Some(PlayerId(1)));
        s.get_mut(anthem).unwrap().set_controller(Some(PlayerId(0)));
        // bears controlled by P1, anthem by P0 -> "other creatures YOU control" excludes it.
        let effects = s.active_static_effects_for(bears, &db);
        assert_eq!(effects.len(), 0);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd magicbot-engine && cargo test --lib state::characteristics_tests::anthem`
Expected: FAIL — `active_static_effects_for` not defined.

- [ ] **Step 3: Write the implementation**

Add to `impl GameState` in `magicbot-engine/src/state.rs`:

```rust
    /// Gather the static continuous effects that currently apply to `target`,
    /// scoped per their affected-set. This slice handles AnthemOtherCreatures:
    /// "other creatures you control get +X/+Y". CardDb is passed in.
    pub fn active_static_effects_for(
        &self,
        target: ObjectId,
        db: &crate::carddb::CardDb,
    ) -> Vec<crate::layers::ContinuousEffect> {
        use crate::carddb::{DefId, StaticAbility};
        use crate::layers::{ContinuousEffect, Effect};
        use crate::ids::Timestamp;

        let mut out = Vec::new();

        // The target must be a creature for "other creatures" anthems to apply.
        let target_obj = match self.objects.get(&target) {
            Some(o) => o,
            None => return out,
        };
        let target_is_creature = db
            .get(DefId(target_obj.def_id()))
            .map(|d| d.is_creature())
            .unwrap_or(false);
        if !target_is_creature {
            return out;
        }
        let target_controller = target_obj.controller();

        // Scan all objects on the battlefield for anthem sources.
        let mut ts: u64 = 0;
        for (sid, src) in self.objects.iter() {
            if src.zone() != crate::object::Zone::Battlefield {
                continue;
            }
            if *sid == target {
                continue; // "other"
            }
            let def = match db.get(DefId(src.def_id())) {
                Some(d) => d,
                None => continue,
            };
            for sa in def.static_abilities() {
                match sa {
                    StaticAbility::AnthemOtherCreatures { power, toughness } => {
                        // "you control": source controller == target controller.
                        if src.controller() == target_controller && target_controller.is_some() {
                            ts += 1;
                            out.push(ContinuousEffect {
                                effect: Effect::ModifyPt {
                                    power: *power,
                                    toughness: *toughness,
                                },
                                timestamp: Timestamp(ts),
                                is_cda: false,
                            });
                        }
                    }
                }
            }
        }
        out
    }
```

NOTE: `self.objects.iter()` yields `(&ObjectId, &Object)`. Iteration order of `im::HashMap` is unspecified; since all anthem effects here are commutative `ModifyPt`, order does not affect the result. (When non-commutative effects arrive in a later plan, timestamps must come from a real source rather than iteration order — flagged in out-of-scope.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd magicbot-engine && cargo test --lib state`
Expected: PASS — all three anthem-scoping tests plus prior pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/state.rs
git commit -m "feat(engine): active_static_effects_for gathers + scopes anthem effects"
```

---

### Task 4: `GameState::characteristics` — compose base + effects + counters via compute()

**Files:**
- Modify: `magicbot-engine/src/state.rs`
- Create: `magicbot-engine/tests/characteristics.rs`
- Test: in-file unit test + new integration file

**Interfaces:**
- Consumes: `base_chars_of` (Task 2), `active_static_effects_for` (Task 3), `Object::layer_counters` (Task 1), `layers::{compute, Status, Chars}`.
- Produces: `impl GameState { pub fn characteristics(&self, id: ObjectId, db: &CardDb) -> Option<Chars> }` — assembles base + effects + counters + status and returns `compute(...)`. `None` if base unresolvable. The end-to-end payoff: an anthem on the battlefield makes another creature's `characteristics()` report the buffed P/T, and the object stores NO P/T (it's all computed).

- [ ] **Step 1: Write the failing unit test**

Add to the `characteristics_tests` module in `magicbot-engine/src/state.rs`:

```rust
    #[test]
    fn characteristics_applies_anthem_and_counters() {
        use crate::layers::Pt;
        use crate::object::CounterKind;

        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        let bears = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        let anthem = s.create_object(PlayerId(0), Zone::Battlefield, 3);
        s.get_mut(bears).unwrap().set_controller(Some(PlayerId(0)));
        s.get_mut(anthem).unwrap().set_controller(Some(PlayerId(0)));
        // a +1/+1 counter on the bears as well
        s.get_mut(bears).unwrap().add_counter(CounterKind::PlusOnePlusOne, 1);

        // base 2/2, +1/+1 anthem (7c), +1/+1 counter (7c) -> 4/4
        let chars = s.characteristics(bears, &db).expect("chars");
        assert_eq!(chars.pt, Some(Pt { power: 4, toughness: 4 }));
    }

    #[test]
    fn characteristics_of_unknown_is_none() {
        let db = sample_pool();
        let s = GameState::new(2, 0);
        assert!(s.characteristics(crate::ids::ObjectId(123), &db).is_none());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd magicbot-engine && cargo test --lib state::characteristics_tests::characteristics`
Expected: FAIL — `characteristics` not defined.

- [ ] **Step 3: Write the implementation**

Add to `impl GameState` in `magicbot-engine/src/state.rs`:

```rust
    /// Current characteristics of an object, computed fresh (never stored;
    /// CR 613.5). Composes the printed base, the static effects that apply to
    /// it, and its counters, through the CR 613 layer engine.
    pub fn characteristics(
        &self,
        id: ObjectId,
        db: &crate::carddb::CardDb,
    ) -> Option<crate::layers::Chars> {
        let base = self.base_chars_of(id, db)?;
        let effects = self.active_static_effects_for(id, db);
        let obj = self.objects.get(&id)?;
        let counters = obj.layer_counters();
        let status = crate::layers::Status { tapped: obj.is_tapped() };
        Some(crate::layers::compute(&base, &effects, &counters, status))
    }
```

- [ ] **Step 4: Run the unit tests to verify they pass**

Run: `cd magicbot-engine && cargo test --lib state`
Expected: PASS — both new unit tests plus all prior state tests pass.

- [ ] **Step 5: Write the integration test (the composed payoff)**

Create `magicbot-engine/tests/characteristics.rs`:

```rust
// End-to-end: a live GameState object's CURRENT characteristics, computed by
// composing state + CardDb + the CR 613 layer engine. Nothing is stored on the
// object; characteristics() recomputes every call (CR 613.5).
use magicbot_engine::carddb::sample_pool;
use magicbot_engine::ids::PlayerId;
use magicbot_engine::layers::Pt;
use magicbot_engine::object::{CounterKind, Zone};
use magicbot_engine::state::GameState;

#[test]
fn anthem_buffs_a_creature_live_in_gamestate() {
    let db = sample_pool();
    let mut s = GameState::new(2, 0);
    let bears = s.create_object(PlayerId(0), Zone::Battlefield, 1); // 2/2
    let anthem = s.create_object(PlayerId(0), Zone::Battlefield, 3); // +1/+1 anthem
    s.get_mut(bears).unwrap().set_controller(Some(PlayerId(0)));
    s.get_mut(anthem).unwrap().set_controller(Some(PlayerId(0)));

    // With the anthem out, the bears are 3/3.
    let chars = s.characteristics(bears, &db).unwrap();
    assert_eq!(chars.pt, Some(Pt { power: 3, toughness: 3 }));
}

#[test]
fn characteristics_are_not_stored_recompute_reflects_change() {
    let db = sample_pool();
    let mut s = GameState::new(2, 0);
    let bears = s.create_object(PlayerId(0), Zone::Battlefield, 1);
    s.get_mut(bears).unwrap().set_controller(Some(PlayerId(0)));

    // No anthem yet: 2/2.
    assert_eq!(
        s.characteristics(bears, &db).unwrap().pt,
        Some(Pt { power: 2, toughness: 2 })
    );

    // Add an anthem; recompute reflects it immediately (nothing cached).
    let anthem = s.create_object(PlayerId(0), Zone::Battlefield, 3);
    s.get_mut(anthem).unwrap().set_controller(Some(PlayerId(0)));
    assert_eq!(
        s.characteristics(bears, &db).unwrap().pt,
        Some(Pt { power: 3, toughness: 3 })
    );

    // Add a +1/+1 counter; recompute -> 4/4.
    s.get_mut(bears)
        .unwrap()
        .add_counter(CounterKind::PlusOnePlusOne, 1);
    assert_eq!(
        s.characteristics(bears, &db).unwrap().pt,
        Some(Pt { power: 4, toughness: 4 })
    );
}

#[test]
fn fork_isolates_characteristics() {
    // Forking and adding an anthem to the child must not change the parent's
    // computed characteristics — ties the CoW guarantee to the new query.
    let db = sample_pool();
    let mut parent = GameState::new(2, 0);
    let bears = parent.create_object(PlayerId(0), Zone::Battlefield, 1);
    parent.get_mut(bears).unwrap().set_controller(Some(PlayerId(0)));

    let mut child = parent.fork();
    let anthem = child.create_object(PlayerId(0), Zone::Battlefield, 3);
    child.get_mut(anthem).unwrap().set_controller(Some(PlayerId(0)));

    assert_eq!(
        child.characteristics(bears, &db).unwrap().pt,
        Some(Pt { power: 3, toughness: 3 }),
        "child sees the anthem"
    );
    assert_eq!(
        parent.characteristics(bears, &db).unwrap().pt,
        Some(Pt { power: 2, toughness: 2 }),
        "parent does not"
    );
}
```

- [ ] **Step 6: Run the integration test**

Run: `cd magicbot-engine && cargo test --test characteristics`
Expected: PASS — all three integration tests pass (anthem buffs live; recompute reflects changes; fork isolates).

- [ ] **Step 7: Verify whole crate green + lint-clean**

Run: `cd magicbot-engine && cargo test && cargo clippy --all-targets -- -D warnings`
Expected: all tests pass; clippy no warnings.

- [ ] **Step 8: Commit**

```bash
git add magicbot-engine/src/state.rs magicbot-engine/tests/characteristics.rs
git commit -m "feat(engine): GameState::characteristics composes base+effects+counters via compute()"
```

---

## Out of scope for this plan (deferred to later plans)

1. **Non-commutative / ordered static effects.** Timestamps here come from iteration order, which is safe ONLY because anthem `ModifyPt`s commute. Real timestamps (from a source's battlefield-entry order) are needed before any non-commutative static (e.g. a set-P/T effect) is added.
2. **Other static-ability kinds** (animate, set-P/T, type/color granting, "all creatures" non-controller-scoped, conditional anthems). Only `AnthemOtherCreatures` is scoped here.
3. **Layers 1-3, dependency ordering (613.8), CDAs from game state.** Inherited from the layer engine's existing deferrals.
4. **`controller` defaulting.** This slice requires callers to `set_controller` explicitly on battlefield objects. Auto-setting controller = owner on entering the battlefield belongs to the move pipeline (a later plan).
5. **The move pipeline, zone-change events, SBA/priority/stack.** Unrelated subsystems.
6. **Caching/memoization of `characteristics()`.** Pure recompute every call; caching is a later optimization.

## Self-review notes

- **Spec coverage:** REVISED §3 inputs to the layer function (copiable base, continuous effects, counters, status) → Task 2 (base), Task 3 (effects), Task 1 (counters) + Task 4 status, all composed in Task 4. REVISED §0 inv. 3 (never stored) → Task 4 computes fresh, integration test `characteristics_are_not_stored_recompute_reflects_change` proves it. The "behavior static, not forked" invariant → `&CardDb` parameter everywhere, asserted in constraints, `fork_isolates_characteristics` test ties it to the CoW guarantee.
- **Type consistency:** `base_chars_of(ObjectId, &CardDb) -> Option<BaseChars>` (Task 2) consumed by `characteristics` (Task 4). `active_static_effects_for(ObjectId, &CardDb) -> Vec<ContinuousEffect>` (Task 3) consumed by Task 4. `layer_counters() -> Counters` (Task 1) consumed by Task 4. `is_creature()`/`static_abilities()` (Task 1) consumed by Task 3. `DefId(obj.def_id())` — `def_id()` returns `u64`, `DefId` wraps `u64` — consistent (the width was reconciled in the carddb plan).
- **No stored characteristics, no CardDb field on GameState:** verified by construction — `characteristics` returns a fresh `compute()` result; `CardDb` is only ever a parameter.
- **Reuse:** `characteristics` calls `base_chars()` + `compute()`; no layer logic is duplicated in `state.rs`.
