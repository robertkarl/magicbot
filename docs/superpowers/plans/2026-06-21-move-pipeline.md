# Move Pipeline + Zone-Change Events Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `GameState::move_object(id, to_zone) -> MoveEvent` — the single choke point through which an object changes zones, minting a NEW `ObjectId` in the destination (CR 400.7), dropping per-object state, and emitting a typed `MoveEvent` over which `dies` / `leaves_battlefield` / `bounced` are PREDICATES (not stored kinds).

**Architecture:** A new module `magicbot-engine/src/events.rs` holds `MoveEvent` and the zone-transition predicates. `GameState::move_object` (in `state.rs`) implements the core CR 400.7 mechanic: snapshot the leaving object's identity into the event, remove it from its source zone and the object table, mint a fresh `ObjectId`, construct a NEW `Object` in the destination with default (empty) `InstanceState` — counters/tapped/attachments dropped — insert it into the destination zone, and return the `MoveEvent { object_before, from, object_after, to, owner, def_id }`. The dies-vs-LTB-vs-bounce distinction lives entirely in free predicate functions over the event, exactly as the CR defines "dies" = Battlefield→Graveyard (700.4) and LTB = Battlefield→anywhere (603.6c). Replacement effects (614/616), the 400.7a-m carry-across exceptions, redirect-to-owner (400.3), and trigger collection are all DEFERRED — this is the bare mechanic + the event vocabulary that those later subsystems hang off.

**Tech Stack:** Rust 1.93+. No new deps.

**Source spec:** `prior-art/rules-core.md` §400.7 (new object / no memory), §700.4 (dies), §603.6c (LTB), and the "Cross-cutting takeaways" #1 and #5. `prior-art/conformance-cases.md` section A (A1 dies, A2 new-object-drops-state). Bridges `state.rs`, `object.rs`, `zones.rs`, `ids.rs`.

## Global Constraints

- **Rust edition 2021**, toolchain `1.93+`.
- **Every zone change mints a NEW `ObjectId`** (CR 400.7). The destination object is a fresh object with a fresh id; the old id is removed from the table. "Same object across a zone change" must be impossible by construction.
- **Per-object state is DROPPED on the move** (CR 400.7): the destination object gets a default `InstanceState` (no counters, untapped, no attachments, `entered_this_turn` reset). The carry-across exceptions (400.7a-m) are DEFERRED — this slice drops unconditionally.
- **dies / leaves_battlefield / bounced are PREDICATES over `MoveEvent`**, never stored fields on the object. `dies` = `from == Battlefield && to == Graveyard` (CR 700.4). `leaves_battlefield` = `from == Battlefield` (any destination). `bounced` = `from == Battlefield && to == Hand`.
- **The move preserves `owner` and `def_id`** (these are intrinsic to the card, not per-instance state) but resets `controller` to `None` (controller is set by entering a zone / the move pipeline's later control logic, deferred).
- **`move_object` is the ONLY way to change an object's zone going forward.** (The existing `Object::set_zone` stays for now but the move pipeline does not use it — it builds a fresh object instead.)
- **No replacement effects, no triggers, no redirect-to-owner, no SBAs** in this slice — just the mechanic + event.
- **TDD**; `cargo test` + `cargo clippy --all-targets -- -D warnings` clean after every task.

---

### Task 1: `MoveEvent` + zone-transition predicates

**Files:**
- Create: `magicbot-engine/src/events.rs`
- Modify: `magicbot-engine/src/lib.rs` (add `pub mod events;`)
- Test: in-file `#[cfg(test)]` in `events.rs`

**Interfaces:**
- Consumes: `ids::{ObjectId, PlayerId}`, `object::Zone`.
- Produces:
  - `pub struct MoveEvent { pub object_before: ObjectId, pub object_after: ObjectId, pub from: Zone, pub to: Zone, pub owner: PlayerId, pub def_id: u64 }` — `Copy, Clone, PartialEq, Eq, Debug`.
  - `pub fn dies(e: &MoveEvent) -> bool` — CR 700.4: `from == Battlefield && to == Graveyard`.
  - `pub fn leaves_battlefield(e: &MoveEvent) -> bool` — CR 603.6c: `from == Battlefield` (any destination).
  - `pub fn bounced(e: &MoveEvent) -> bool` — `from == Battlefield && to == Hand`.
  - `pub fn entered_battlefield(e: &MoveEvent) -> bool` — `to == Battlefield` (ETB; CR 603.6a).

- [ ] **Step 1: Write the failing test**

Create `magicbot-engine/src/events.rs`:

```rust
//! Zone-change events and the predicates the rules define over them.
//! CR 400.6/400.7: a zone change is a typed event; "dies"/"leaves the
//! battlefield"/etc. are PREDICATES over the transition, not object kinds.

use crate::ids::{ObjectId, PlayerId};
use crate::object::Zone;

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(from: Zone, to: Zone) -> MoveEvent {
        MoveEvent {
            object_before: ObjectId(1),
            object_after: ObjectId(2),
            from,
            to,
            owner: PlayerId(0),
            def_id: 1,
        }
    }

    #[test]
    fn dies_is_exactly_battlefield_to_graveyard() {
        // CR 700.4
        assert!(dies(&ev(Zone::Battlefield, Zone::Graveyard)));
        assert!(!dies(&ev(Zone::Battlefield, Zone::Exile))); // LTB but not dies
        assert!(!dies(&ev(Zone::Hand, Zone::Graveyard))); // discard, not dies
    }

    #[test]
    fn leaves_battlefield_is_any_departure_from_battlefield() {
        // CR 603.6c
        assert!(leaves_battlefield(&ev(Zone::Battlefield, Zone::Graveyard)));
        assert!(leaves_battlefield(&ev(Zone::Battlefield, Zone::Exile)));
        assert!(leaves_battlefield(&ev(Zone::Battlefield, Zone::Hand)));
        assert!(!leaves_battlefield(&ev(Zone::Hand, Zone::Battlefield)));
    }

    #[test]
    fn bounced_and_entered_predicates() {
        assert!(bounced(&ev(Zone::Battlefield, Zone::Hand)));
        assert!(!bounced(&ev(Zone::Battlefield, Zone::Graveyard)));
        assert!(entered_battlefield(&ev(Zone::Hand, Zone::Battlefield)));
        assert!(!entered_battlefield(&ev(Zone::Battlefield, Zone::Graveyard)));
    }
}
```

- [ ] **Step 2: Add the module declaration**

In `magicbot-engine/src/lib.rs`, add after the other `pub mod` lines:

```rust
pub mod events;
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd magicbot-engine && cargo test --lib events`
Expected: FAIL — `MoveEvent`, `dies`, etc. not defined.

- [ ] **Step 4: Write minimal implementation**

In `magicbot-engine/src/events.rs` (above the test module):

```rust
/// A zone-change event. Carries the old and new object identities (CR 400.7
/// mints a new id), the transition, and the card's intrinsic owner/def.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct MoveEvent {
    pub object_before: ObjectId,
    pub object_after: ObjectId,
    pub from: Zone,
    pub to: Zone,
    pub owner: PlayerId,
    pub def_id: u64,
}

/// CR 700.4: "dies" means put into a graveyard from the battlefield.
pub fn dies(e: &MoveEvent) -> bool {
    e.from == Zone::Battlefield && e.to == Zone::Graveyard
}

/// CR 603.6c: leaves the battlefield = from the battlefield to anywhere.
pub fn leaves_battlefield(e: &MoveEvent) -> bool {
    e.from == Zone::Battlefield
}

/// Battlefield -> hand.
pub fn bounced(e: &MoveEvent) -> bool {
    e.from == Zone::Battlefield && e.to == Zone::Hand
}

/// CR 603.6a: enters the battlefield.
pub fn entered_battlefield(e: &MoveEvent) -> bool {
    e.to == Zone::Battlefield
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd magicbot-engine && cargo test --lib events`
Expected: PASS — all three tests pass.

- [ ] **Step 6: Commit**

```bash
git add magicbot-engine/src/events.rs magicbot-engine/src/lib.rs
git commit -m "feat(events): MoveEvent + dies/leaves_battlefield/bounced/entered predicates"
```

---

### Task 2: `GameState::move_object` — the CR 400.7 mechanic

**Files:**
- Modify: `magicbot-engine/src/state.rs`
- Modify: `magicbot-engine/src/object.rs` (add an `owner`-preserving rebuild path — see below)
- Test: in-file `#[cfg(test)]` in `state.rs`

**Interfaces:**
- Consumes: `GameState`, `Object`, `Zones`, `IdMinter`, `events::MoveEvent`.
- Produces:
  - `impl GameState { pub fn move_object(&mut self, id: ObjectId, to: Zone) -> Option<MoveEvent> }` — if `id` is unknown, returns `None`. Otherwise:
    1. read the leaving object's `owner`, `zone` (from), `def_id`;
    2. remove it from its source zone (`zones.remove(from, owner, id)`) and from the object table;
    3. mint a fresh `ObjectId`;
    4. construct a NEW `Object::new(new_id, owner, to, def_id)` (default InstanceState — state dropped per CR 400.7), insert into `objects` and `zones.insert(to, owner, new_id)`;
    5. return `Some(MoveEvent { object_before: id, object_after: new_id, from, to, owner, def_id })`.

- [ ] **Step 1: Write the failing test**

Add a test module to `magicbot-engine/src/state.rs`:

```rust
#[cfg(test)]
mod move_tests {
    use super::*;
    use crate::events::{dies, leaves_battlefield};
    use crate::ids::PlayerId;
    use crate::object::{CounterKind, Zone};

    #[test]
    fn move_mints_new_id_and_drops_state() {
        // CR 400.7: a zone change creates a new object with no memory.
        let mut s = GameState::new(2, 0);
        let id = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        s.get_mut(id).unwrap().tap();
        s.get_mut(id).unwrap().add_counter(CounterKind::PlusOnePlusOne, 2);

        let ev = s.move_object(id, Zone::Graveyard).expect("moved");

        // new id, old id gone
        assert_ne!(ev.object_before, ev.object_after);
        assert!(s.get(id).is_none(), "old object removed from table");
        let new_obj = s.get(ev.object_after).expect("new object present");

        // state dropped
        assert!(!new_obj.is_tapped(), "tapped dropped");
        assert_eq!(new_obj.counter_count(CounterKind::PlusOnePlusOne), 0, "counters dropped");
        assert_eq!(new_obj.zone(), Zone::Graveyard);
        assert_eq!(new_obj.owner(), PlayerId(0), "owner preserved");
        assert_eq!(new_obj.def_id(), 1, "def_id preserved");
    }

    #[test]
    fn move_battlefield_to_graveyard_is_dies() {
        let mut s = GameState::new(2, 0);
        let id = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        let ev = s.move_object(id, Zone::Graveyard).unwrap();
        assert!(dies(&ev));
        assert!(leaves_battlefield(&ev));
    }

    #[test]
    fn move_exile_from_battlefield_is_ltb_not_dies() {
        let mut s = GameState::new(2, 0);
        let id = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        let ev = s.move_object(id, Zone::Exile).unwrap();
        assert!(!dies(&ev));
        assert!(leaves_battlefield(&ev));
    }

    #[test]
    fn move_unknown_object_is_none() {
        let mut s = GameState::new(2, 0);
        assert!(s.move_object(crate::ids::ObjectId(999), Zone::Graveyard).is_none());
    }

    #[test]
    fn moved_object_is_in_destination_zone_table() {
        let mut s = GameState::new(2, 0);
        let id = s.create_object(PlayerId(0), Zone::Hand, 1);
        let ev = s.move_object(id, Zone::Battlefield).unwrap();
        assert_eq!(s.battlefield_len(), 1);
        assert_eq!(s.get(ev.object_after).unwrap().zone(), Zone::Battlefield);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd magicbot-engine && cargo test --lib state::move_tests`
Expected: FAIL — `move_object` not defined.

- [ ] **Step 3: Write the implementation**

Add to `impl GameState` in `magicbot-engine/src/state.rs`:

```rust
    /// Move an object to another zone. CR 400.7: this mints a NEW object id and
    /// drops all per-object state (counters, tapped, attachments). Returns the
    /// MoveEvent, or None if `id` is unknown. Replacement effects, the 400.7a-m
    /// carry-across exceptions, redirect-to-owner, and triggers are deferred.
    pub fn move_object(
        &mut self,
        id: ObjectId,
        to: Zone,
    ) -> Option<crate::events::MoveEvent> {
        let (owner, from, def_id) = {
            let obj = self.objects.get(&id)?;
            (obj.owner(), obj.zone(), obj.def_id())
        };

        // Remove from source zone and the object table.
        self.zones.remove(from, owner, id);
        self.objects.remove(&id);

        // Mint a fresh object in the destination (state dropped).
        let new_id = self.minter.mint();
        let new_obj = Object::new(new_id, owner, to, def_id);
        self.objects.insert(new_id, new_obj);
        self.zones.insert(to, owner, new_id);

        Some(crate::events::MoveEvent {
            object_before: id,
            object_after: new_id,
            from,
            to,
            owner,
            def_id,
        })
    }
```

NOTE: no change to `object.rs` is actually required — `Object::new` already builds a fresh object with default `InstanceState`. (The Files list mentioned object.rs defensively; if no change is needed, do not modify it, and drop it from the commit.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd magicbot-engine && cargo test --lib state`
Expected: PASS — all five move tests plus prior state tests pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/state.rs
git commit -m "feat(engine): GameState::move_object implements CR 400.7 zone change (new id, drop state)"
```

---

### Task 3: Integration test — move pipeline composes with characteristics + fork (conformance A1/A2)

**Files:**
- Create: `magicbot-engine/tests/move_pipeline.rs`

**Interfaces:**
- Consumes: the public API — `GameState`, `events::{dies, leaves_battlefield, entered_battlefield}`, `carddb::sample_pool`, `layers`.
- Produces: integration tests proving (a) the new-object rule drops state end-to-end and the predicates classify transitions correctly (conformance A1/A2), (b) a moved object's `characteristics()` reflects its NEW zone/def (composes with the prior slice), and (c) `move_object` on a fork does not disturb the parent (ties the move pipeline to the CoW guarantee).

- [ ] **Step 1: Write the failing test**

Create `magicbot-engine/tests/move_pipeline.rs`:

```rust
// The move pipeline (CR 400.7) composed with the rest of the engine.
use magicbot_engine::carddb::sample_pool;
use magicbot_engine::events::{dies, entered_battlefield, leaves_battlefield};
use magicbot_engine::ids::PlayerId;
use magicbot_engine::layers::Pt;
use magicbot_engine::object::{CounterKind, Zone};
use magicbot_engine::state::GameState;

#[test]
fn a2_zone_change_drops_state_and_mints_new_object() {
    // Conformance A2: counters + tapped dropped, new identity, owner/def kept.
    let mut s = GameState::new(2, 0);
    let id = s.create_object(PlayerId(0), Zone::Battlefield, 1);
    s.get_mut(id).unwrap().tap();
    s.get_mut(id).unwrap().add_counter(CounterKind::PlusOnePlusOne, 3);

    let ev = s.move_object(id, Zone::Hand).unwrap();
    assert_ne!(ev.object_before, ev.object_after);
    let moved = s.get(ev.object_after).unwrap();
    assert!(!moved.is_tapped());
    assert_eq!(moved.counter_count(CounterKind::PlusOnePlusOne), 0);
    assert_eq!(moved.owner(), PlayerId(0));
    assert_eq!(moved.def_id(), 1);
}

#[test]
fn a1_dies_classification_and_characteristics_after_move() {
    // A1: Battlefield->Graveyard is dies; the new graveyard object still resolves
    // its printed characteristics via the CardDb (Grizzly Bears 2/2).
    let db = sample_pool();
    let mut s = GameState::new(2, 0);
    let id = s.create_object(PlayerId(0), Zone::Battlefield, 1);
    s.get_mut(id).unwrap().set_controller(Some(PlayerId(0)));

    let ev = s.move_object(id, Zone::Graveyard).unwrap();
    assert!(dies(&ev));
    assert!(leaves_battlefield(&ev));
    assert!(!entered_battlefield(&ev));

    // The new graveyard object still has printed 2/2 base (no anthem in graveyard).
    let chars = s.characteristics(ev.object_after, &db).unwrap();
    assert_eq!(chars.pt, Some(Pt { power: 2, toughness: 2 }));
}

#[test]
fn move_on_fork_does_not_disturb_parent() {
    // Ties CR 400.7 move to the CoW guarantee.
    let mut parent = GameState::new(2, 0);
    let id = parent.create_object(PlayerId(0), Zone::Battlefield, 1);

    let mut child = parent.fork();
    let ev = child.move_object(id, Zone::Graveyard).unwrap();

    // child: old gone, new in graveyard
    assert!(child.get(id).is_none());
    assert!(child.get(ev.object_after).is_some());

    // parent: original object still on the battlefield, untouched
    assert!(parent.get(id).is_some());
    assert_eq!(parent.get(id).unwrap().zone(), Zone::Battlefield);
    assert_eq!(parent.battlefield_len(), 1);
}
```

- [ ] **Step 2: Run the integration test**

Run: `cd magicbot-engine && cargo test --test move_pipeline`
Expected: PASS — all three integration tests pass.

- [ ] **Step 3: Verify whole crate green + lint-clean**

Run: `cd magicbot-engine && cargo test && cargo clippy --all-targets -- -D warnings`
Expected: all tests pass; clippy no warnings.

- [ ] **Step 4: Commit**

```bash
git add magicbot-engine/tests/move_pipeline.rs
git commit -m "test(engine): move pipeline composes with characteristics + fork (A1/A2)"
```

---

## Out of scope for this plan (deferred to later plans)

1. **Replacement effects (CR 614/616)** — the move pipeline does not yet consult replacement effects before the move. The `move_object` choke point is where they'll plug in.
2. **The 400.7a-m carry-across exceptions** — this slice drops ALL per-object state unconditionally. The exceptions (permanent-spell continuity, zone-change-trigger finds-new-object, stickers, etc.) are deferred; the `MoveEvent` carries the old+new ids that those will need.
3. **Trigger collection** — `move_object` returns a `MoveEvent` but nothing gathers/dispatches triggers off it yet. The SBA/priority/stack loop (a later plan) will consume these events.
4. **Redirect-to-owner (CR 400.3)** — moving a controlled-but-not-owned object to a hidden zone should redirect to the owner's zone. This slice moves within the object's own `owner`'s zones; redirect needs the full controller/owner move semantics.
5. **LKI snapshots (CR 608.2h / 603.10 / 704.8)** — the `MoveEvent` carries ids but not a frozen characteristics snapshot of the leaving object. LKI is a later plan.
6. **Token cease-to-exist SBA (CR 111.7)** — a token moved off the battlefield should cease to exist via SBA after triggers look. No SBA loop yet.
7. **Controller assignment on ETB** — the new object's `controller` is `None`; auto-setting it belongs to the move pipeline's later control logic.
8. **Simultaneous/batch moves** — one object at a time here.

## Self-review notes

- **Spec coverage:** rules-core §400.7 (new object, drop state) → Task 2 `move_object` + tests `move_mints_new_id_and_drops_state`, conformance A2 integration. §700.4 dies / §603.6c LTB / §603.6a ETB → Task 1 predicates + Task 2/3 classification tests. "Event pipeline is the choke point" → `move_object` is the single entry. Conformance A1 (dies = BF→GY) → Task 1 + Task 3. A2 (drop state, new id) → Task 2 + Task 3.
- **Invariant — new id always:** `move_object` always mints via `self.minter` and removes the old id; "same object across zones" is impossible by construction (test `move_mints_new_id_and_drops_state` asserts `object_before != object_after` and old id gone).
- **Invariant — predicates not stored:** `dies`/`leaves_battlefield`/etc. are free functions over `&MoveEvent`; no kind field on `Object`.
- **Composition:** Task 3 proves the moved object still answers `characteristics()` (prior slice) and that a fork's move doesn't disturb the parent (CoW slice) — the move pipeline composes cleanly with both existing subsystems.
- **Type consistency:** `move_object(ObjectId, Zone) -> Option<MoveEvent>` (Task 2) consumed in Task 3. `MoveEvent` fields (Task 1) read by predicates (Task 1) and tests (Tasks 2,3). `owner: PlayerId`, `def_id: u64` match `Object::owner()`/`def_id()` return types.
