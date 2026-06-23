# Priority + Stack Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The CR 117 priority loop and CR 405 stack core: players pass priority in turn order; when all pass with a non-empty stack the top object resolves; when all pass with an empty stack the step ends. State-based actions run before priority is granted (CR 117.5). No real spell casting yet — a placeholder stack object exercises the machinery.

**Architecture:** A new module `magicbot-engine/src/priority.rs` holds the `StackObject` (a placeholder resolvable, real spells deferred) and a pure `PriorityState` value type (active player, who holds priority, consecutive-pass count). `GameState` gains a `priority: PriorityState` field plus methods: `push_stack(obj)` (LIFO, resets the pass count and gives the active player priority), `pass_priority(&CardDb) -> PriorityOutcome` (the CR 117.5 step: run SBAs to fixpoint, then either advance priority, resolve the top of stack, or end the step). The `GameState.stack` field — previously dead — becomes the single source of truth for the stack (resolving the move-pipeline's documented Stack split-brain: stack objects are pushed/popped here, never via `move_object`). This composes the SBA loop (built last slice) into its correct home: SBAs fire as part of granting priority.

**Tech Stack:** Rust 1.93+. No new deps.

**Source spec:** `prior-art/rules-core.md` §117 (priority), §405 (the stack, LIFO, APNAP), §117.5 (the core loop: SBAs to fixpoint then triggers then priority). `prior-art/conformance-cases.md` section C (C1 priority loop, C2 LIFO/in-response, C3 empty-stack-ends-step). Bridges `state.rs`, `ids.rs`, `events.rs`, `sba` (via `check_state_based_actions`).

## Global Constraints

- **Rust edition 2021**, toolchain `1.93+`.
- **APNAP priority order** (CR 405.3 / 117.3d): the active player gets priority first; passing moves it to the next player in turn order (here: `(player + 1) % n_players`). When everyone has passed in succession, the top of the stack resolves (or the step ends if empty).
- **The stack is LIFO** (CR 405.2): `push_stack` adds to the top; resolution pops the top. "In response" → last-in resolves first (conformance C2).
- **SBAs run before priority is granted, to fixpoint** (CR 117.5): every time the game would grant priority (after a resolution or at the start of a priority round), `check_state_based_actions` runs first. Triggers are DEFERRED (the loop does SBAs then grants priority, skipping the "put triggers on the stack" step).
- **`GameState.stack` is the single source of truth for the stack.** The `Zones` Stack arm stays a no-op; the move pipeline's `debug_assert` against Stack moves stays. Pushing/popping the stack happens ONLY through `push_stack`/resolution here.
- **Passing priority resets correctly:** any action that puts an object on the stack resets the consecutive-pass count and returns priority to the active player (CR 117.3c-style — after something happens, the active player gets priority again). `push_stack` does this.
- **Real spell casting, costs, targeting, modes, mana, phases/steps, the turn cycle** are DEFERRED — this is the priority-and-resolution skeleton with a placeholder stack object.
- **TDD**; `cargo test` + `cargo clippy --all-targets -- -D warnings` clean after every task.

---

### Task 1: `StackObject` + `PriorityState` value types

**Files:**
- Create: `magicbot-engine/src/priority.rs`
- Modify: `magicbot-engine/src/lib.rs` (add `pub mod priority;`)
- Test: in-file `#[cfg(test)]` in `priority.rs`

**Interfaces:**
- Consumes: `ids::{ObjectId, PlayerId}`.
- Produces:
  - `pub struct StackObject { pub id: ObjectId, pub controller: PlayerId }` — a placeholder resolvable. `Copy, Clone, PartialEq, Eq, Debug`. (Real spell/ability payloads deferred; an `id`+`controller` is enough to exercise LIFO + APNAP.)
  - `pub struct PriorityState { active_player: PlayerId, priority_holder: PlayerId, passes_in_a_row: u8, n_players: u8 }` — `Copy, Clone, PartialEq, Eq, Debug`.
  - `PriorityState::new(active_player: PlayerId, n_players: u8) -> Self` — priority starts with the active player, zero passes.
  - `impl PriorityState`:
    - `pub fn holder(&self) -> PlayerId`
    - `pub fn active_player(&self) -> PlayerId`
    - `pub fn give_to_active(&mut self)` — set holder = active, reset passes to 0 (used when something is put on the stack).
    - `pub fn record_pass(&mut self) -> bool` — advance holder to `(holder + 1) % n_players`, increment `passes_in_a_row`; returns `true` iff all players have now passed in succession (`passes_in_a_row == n_players`).
    - `pub fn reset_passes(&mut self)` — set passes to 0, holder to active (used after a resolution).

- [ ] **Step 1: Write the failing test**

Create `magicbot-engine/src/priority.rs`:

```rust
//! CR 117 priority + CR 405 stack core. This slice: the priority-passing and
//! stack-resolution machinery with a placeholder StackObject (real spells later).

use crate::ids::{ObjectId, PlayerId};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_starts_with_active_player() {
        let p = PriorityState::new(PlayerId(0), 2);
        assert_eq!(p.holder(), PlayerId(0));
        assert_eq!(p.active_player(), PlayerId(0));
    }

    #[test]
    fn passing_cycles_and_detects_full_round() {
        let mut p = PriorityState::new(PlayerId(0), 2);
        // P0 passes -> holder P1, not yet all-passed
        assert!(!p.record_pass());
        assert_eq!(p.holder(), PlayerId(1));
        // P1 passes -> all 2 have passed in succession
        assert!(p.record_pass());
    }

    #[test]
    fn give_to_active_resets() {
        let mut p = PriorityState::new(PlayerId(0), 2);
        p.record_pass(); // holder now P1, passes=1
        p.give_to_active();
        assert_eq!(p.holder(), PlayerId(0));
        // a fresh pass should not immediately report all-passed
        assert!(!p.record_pass());
    }

    #[test]
    fn stack_object_is_a_value() {
        let s = StackObject { id: ObjectId(7), controller: PlayerId(1) };
        let s2 = s; // Copy
        assert_eq!(s2.id, ObjectId(7));
        assert_eq!(s2.controller, PlayerId(1));
    }
}
```

- [ ] **Step 2: Add the module declaration**

In `magicbot-engine/src/lib.rs`, add after the other `pub mod` lines:

```rust
pub mod priority;
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd magicbot-engine && cargo test --lib priority`
Expected: FAIL — `StackObject`, `PriorityState` not defined.

- [ ] **Step 4: Write minimal implementation**

In `magicbot-engine/src/priority.rs` (above the test module):

```rust
/// A placeholder resolvable on the stack. Real spell/ability payloads (targets,
/// modes, X, source) are deferred; id + controller exercises LIFO + APNAP.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct StackObject {
    pub id: ObjectId,
    pub controller: PlayerId,
}

/// Whose turn it is, who holds priority, and how many players have passed in a
/// row (CR 117). When passes_in_a_row reaches n_players, the round is complete.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct PriorityState {
    active_player: PlayerId,
    priority_holder: PlayerId,
    passes_in_a_row: u8,
    n_players: u8,
}

impl PriorityState {
    pub fn new(active_player: PlayerId, n_players: u8) -> Self {
        PriorityState {
            active_player,
            priority_holder: active_player,
            passes_in_a_row: 0,
            n_players,
        }
    }

    pub fn holder(&self) -> PlayerId {
        self.priority_holder
    }
    pub fn active_player(&self) -> PlayerId {
        self.active_player
    }

    /// Something was put on the stack: the active player receives priority and
    /// the pass count resets (CR 117.3c — the round restarts).
    pub fn give_to_active(&mut self) {
        self.priority_holder = self.active_player;
        self.passes_in_a_row = 0;
    }

    /// The current holder passes. Advances priority to the next player in turn
    /// order; returns true iff all players have now passed in succession.
    pub fn record_pass(&mut self) -> bool {
        self.priority_holder = PlayerId((self.priority_holder.0 + 1) % self.n_players);
        self.passes_in_a_row += 1;
        self.passes_in_a_row >= self.n_players
    }

    /// After a resolution, the active player gets priority and passes reset.
    pub fn reset_passes(&mut self) {
        self.priority_holder = self.active_player;
        self.passes_in_a_row = 0;
    }
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd magicbot-engine && cargo test --lib priority`
Expected: PASS — all four tests pass.

- [ ] **Step 6: Commit**

```bash
git add magicbot-engine/src/priority.rs magicbot-engine/src/lib.rs
git commit -m "feat(priority): StackObject + PriorityState (APNAP passing, full-round detection)"
```

---

### Task 2: Wire `priority` + an active stack into `GameState`

**Files:**
- Modify: `magicbot-engine/src/state.rs`
- Test: in-file `#[cfg(test)]` in `state.rs`

**Interfaces:**
- Consumes: `priority::{PriorityState, StackObject}`.
- Produces: `GameState` gains a `priority: PriorityState` field and changes `stack` from `im::Vector<ObjectId>` to `im::Vector<StackObject>` (the single source of truth). New methods:
  - `GameState::new` initializes `priority: PriorityState::new(PlayerId(0), n_players as u8)` and an empty stack. The `#[allow(dead_code)]` on `stack` and `n_players` is removed (both now used).
  - `pub fn priority_holder(&self) -> PlayerId`
  - `pub fn stack_len(&self) -> usize`
  - `pub fn push_stack(&mut self, obj: StackObject)` — push to the top of `stack` and call `priority.give_to_active()`.
  - `pub fn peek_stack(&self) -> Option<StackObject>` — the top (last) object, or None.

- [ ] **Step 1: Write the failing test**

Add a test module to `magicbot-engine/src/state.rs`:

```rust
#[cfg(test)]
mod priority_tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::priority::StackObject;

    #[test]
    fn new_game_priority_is_active_player_empty_stack() {
        let s = GameState::new(2, 0);
        assert_eq!(s.priority_holder(), PlayerId(0));
        assert_eq!(s.stack_len(), 0);
        assert!(s.peek_stack().is_none());
    }

    #[test]
    fn push_stack_is_lifo_and_gives_active_priority() {
        let mut s = GameState::new(2, 0);
        // pretend P1 had priority (simulate a pass) then P1 puts something on stack
        s.push_stack(StackObject { id: crate::ids::ObjectId(1), controller: PlayerId(0) });
        s.push_stack(StackObject { id: crate::ids::ObjectId(2), controller: PlayerId(1) });
        assert_eq!(s.stack_len(), 2);
        // top is the last pushed (LIFO)
        assert_eq!(s.peek_stack().unwrap().id, crate::ids::ObjectId(2));
        // active player (P0) holds priority after a push
        assert_eq!(s.priority_holder(), PlayerId(0));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd magicbot-engine && cargo test --lib state::priority_tests`
Expected: FAIL — `priority_holder`, `push_stack`, etc. not defined (and `stack` is the wrong element type).

- [ ] **Step 3: Write the implementation**

In `magicbot-engine/src/state.rs`:

1. Update imports to include `use crate::priority::{PriorityState, StackObject};` (add to the existing `use` lines).
2. In the `GameState` struct: remove `#[allow(dead_code)]` from `stack` and `n_players`; change `stack: im::Vector<ObjectId>` to `stack: im::Vector<StackObject>`; add a field `priority: PriorityState`.
3. In `GameState::new`, initialize the new field and keep the empty stack:

```rust
            stack: im::Vector::new(),
            priority: PriorityState::new(PlayerId(0), n_players as u8),
```

4. Add these methods to `impl GameState`:

```rust
    pub fn priority_holder(&self) -> PlayerId {
        self.priority.holder()
    }

    pub fn stack_len(&self) -> usize {
        self.stack.len()
    }

    pub fn peek_stack(&self) -> Option<StackObject> {
        self.stack.last().copied()
    }

    /// Put an object on top of the stack (CR 405.2 LIFO). The active player then
    /// receives priority and the pass round restarts (CR 117.3c).
    pub fn push_stack(&mut self, obj: StackObject) {
        self.stack.push_back(obj);
        self.priority.give_to_active();
    }
```

NOTE: changing `stack`'s element type from `ObjectId` to `StackObject` — check that nothing else in `state.rs` referenced `stack` as `ObjectId`s. The field was `#[allow(dead_code)]` (unused), so there should be no other references; if the compiler finds one, it is in dead code and can be updated to the new type.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd magicbot-engine && cargo test --lib state`
Expected: PASS — both priority tests plus all prior state tests pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/state.rs
git commit -m "feat(engine): GameState holds PriorityState + a StackObject stack (push/peek/len)"
```

---

### Task 3: `pass_priority` — the CR 117.5 loop (SBAs → resolve / advance / end step)

**Files:**
- Modify: `magicbot-engine/src/state.rs`
- Modify: `magicbot-engine/src/priority.rs` (add the `PriorityOutcome` enum)
- Test: in-file `#[cfg(test)]` in `state.rs`

**Interfaces:**
- Consumes: `PriorityState`, `check_state_based_actions`, the stack.
- Produces:
  - In `priority.rs`: `pub enum PriorityOutcome { Passed(PlayerId), Resolved(StackObject), StepEnded }` — `Clone, PartialEq, Debug`. (`Passed` carries the new holder; `Resolved` the object that just resolved; `StepEnded` when all pass with an empty stack.)
  - In `state.rs`: `pub fn pass_priority(&mut self, db: &CardDb) -> PriorityOutcome` — the current holder passes:
    1. record the pass; if NOT a full round → return `Passed(new_holder)`.
    2. full round: run `check_state_based_actions(db)` (CR 117.5 — SBAs before anything proceeds).
    3. if the stack is non-empty → pop the top, return `Resolved(obj)`, and give the active player priority again (`reset_passes`) so a fresh priority round follows the resolution.
    4. if the stack is empty → return `StepEnded`.

- [ ] **Step 1: Write the failing test**

Add the `PriorityOutcome` enum and a doc note to `priority.rs` first is not required for the test, but the test references it. Add to the `priority_tests` module in `magicbot-engine/src/state.rs`:

```rust
    #[test]
    fn all_pass_empty_stack_ends_step() {
        use crate::priority::PriorityOutcome;
        let db = crate::carddb::sample_pool();
        let mut s = GameState::new(2, 0);
        // P0 passes (holder -> P1)
        assert_eq!(s.pass_priority(&db), PriorityOutcome::Passed(PlayerId(1)));
        // P1 passes -> full round, empty stack -> step ends
        assert_eq!(s.pass_priority(&db), PriorityOutcome::StepEnded);
    }

    #[test]
    fn all_pass_nonempty_stack_resolves_top() {
        use crate::priority::PriorityOutcome;
        let db = crate::carddb::sample_pool();
        let mut s = GameState::new(2, 0);
        let top = StackObject { id: crate::ids::ObjectId(42), controller: PlayerId(1) };
        s.push_stack(StackObject { id: crate::ids::ObjectId(41), controller: PlayerId(0) });
        s.push_stack(top); // top of stack
        // push gave priority to active (P0). P0 passes -> P1. P1 passes -> full round.
        assert_eq!(s.pass_priority(&db), PriorityOutcome::Passed(PlayerId(1)));
        let outcome = s.pass_priority(&db);
        assert_eq!(outcome, PriorityOutcome::Resolved(top), "LIFO: last pushed resolves first");
        assert_eq!(s.stack_len(), 1, "one object left on the stack");
        // after a resolution, the active player gets priority again
        assert_eq!(s.priority_holder(), PlayerId(0));
    }

    #[test]
    fn resolution_runs_sbas_first() {
        use crate::priority::PriorityOutcome;
        // A creature with lethal damage on the battlefield should be cleaned up by
        // the SBA check that runs when all players pass (CR 117.5), even with an
        // empty stack ending the step.
        let db = crate::carddb::sample_pool();
        let mut s = GameState::new(2, 0);
        let bears = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        s.get_mut(bears).unwrap().set_controller(Some(PlayerId(0)));
        s.get_mut(bears).unwrap().mark_damage(2); // lethal
        // pass the full round; the SBA check fires before the step ends
        let _ = s.pass_priority(&db); // P0 -> P1
        let outcome = s.pass_priority(&db); // full round: SBAs run, then empty stack -> StepEnded
        assert_eq!(outcome, PriorityOutcome::StepEnded);
        assert_eq!(s.battlefield_len(), 0, "lethal creature cleaned up by SBA in the priority loop");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd magicbot-engine && cargo test --lib state::priority_tests`
Expected: FAIL — `pass_priority`, `PriorityOutcome` not defined.

- [ ] **Step 3: Write the implementations**

In `magicbot-engine/src/priority.rs`, add (above the test module):

```rust
/// The result of a player passing priority.
#[derive(Clone, PartialEq, Debug)]
pub enum PriorityOutcome {
    /// Priority advanced to this player; the round is not yet complete.
    Passed(PlayerId),
    /// All players passed with a non-empty stack: this object resolved.
    Resolved(StackObject),
    /// All players passed with an empty stack: the step/phase ends.
    StepEnded,
}
```

In `magicbot-engine/src/state.rs`, add to `impl GameState`:

```rust
    /// The current priority holder passes (CR 117). On a full round of passes,
    /// runs state-based actions (CR 117.5), then either resolves the top of the
    /// stack (CR 608.1) or ends the step if the stack is empty.
    pub fn pass_priority(
        &mut self,
        db: &crate::carddb::CardDb,
    ) -> crate::priority::PriorityOutcome {
        use crate::priority::PriorityOutcome;

        let full_round = self.priority.record_pass();
        if !full_round {
            return PriorityOutcome::Passed(self.priority.holder());
        }

        // CR 117.5: state-based actions are performed before the game proceeds.
        let _sba_events = self.check_state_based_actions(db);

        // CR 608.1 / 405.5: resolve the top of the stack, or end the step.
        if let Some(top) = self.stack.pop_back() {
            // A fresh priority round follows the resolution.
            self.priority.reset_passes();
            PriorityOutcome::Resolved(top)
        } else {
            PriorityOutcome::StepEnded
        }
    }
```

NOTE: `im::Vector::pop_back` returns `Option<StackObject>` and removes the last (top) element — the LIFO pop. Resolving gives the active player priority again (`reset_passes`) so the next `pass_priority` begins a new round, matching CR 117.3b (after a spell/ability resolves, the active player gets priority).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd magicbot-engine && cargo test --lib state`
Expected: PASS — all three new priority tests plus all prior state tests pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/state.rs magicbot-engine/src/priority.rs
git commit -m "feat(engine): pass_priority runs CR 117.5 loop (SBAs -> resolve/advance/end step)"
```

---

### Task 4: Integration test — a full priority round with resolution + SBA + fork

**Files:**
- Create: `magicbot-engine/tests/priority.rs`

**Interfaces:**
- Consumes: the public API — `GameState`, `priority::{StackObject, PriorityOutcome}`, `carddb::sample_pool`.
- Produces: integration tests proving (a) the full conformance C-sequence: push two objects, pass the round, top resolves (LIFO), pass again, second resolves, pass again on empty stack → step ends; (b) SBAs fire within the loop (a lethal creature is cleaned up when the round completes); (c) the priority loop on a fork does not disturb the parent.

- [ ] **Step 1: Write the failing test**

Create `magicbot-engine/tests/priority.rs`:

```rust
// The CR 117 priority loop + CR 405 stack, end to end.
use magicbot_engine::carddb::sample_pool;
use magicbot_engine::ids::{ObjectId, PlayerId};
use magicbot_engine::object::Zone;
use magicbot_engine::priority::{PriorityOutcome, StackObject};
use magicbot_engine::state::GameState;

#[test]
fn c2_lifo_two_objects_resolve_in_reverse_order() {
    let db = sample_pool();
    let mut s = GameState::new(2, 0);
    let first = StackObject { id: ObjectId(1), controller: PlayerId(0) };
    let second = StackObject { id: ObjectId(2), controller: PlayerId(1) };
    s.push_stack(first);
    s.push_stack(second); // second is on top

    // Round 1: P0 passes, P1 passes -> second (top) resolves.
    assert_eq!(s.pass_priority(&db), PriorityOutcome::Passed(PlayerId(1)));
    assert_eq!(s.pass_priority(&db), PriorityOutcome::Resolved(second));

    // Round 2: P0 passes, P1 passes -> first resolves.
    assert_eq!(s.pass_priority(&db), PriorityOutcome::Passed(PlayerId(1)));
    assert_eq!(s.pass_priority(&db), PriorityOutcome::Resolved(first));

    // Round 3: empty stack -> step ends.
    assert_eq!(s.pass_priority(&db), PriorityOutcome::Passed(PlayerId(1)));
    assert_eq!(s.pass_priority(&db), PriorityOutcome::StepEnded);
}

#[test]
fn sba_fires_during_the_priority_loop() {
    let db = sample_pool();
    let mut s = GameState::new(2, 0);
    let bears = s.create_object(PlayerId(0), Zone::Battlefield, 1);
    s.get_mut(bears).unwrap().set_controller(Some(PlayerId(0)));
    s.get_mut(bears).unwrap().mark_damage(2);

    let _ = s.pass_priority(&db); // P0 -> P1
    assert_eq!(s.pass_priority(&db), PriorityOutcome::StepEnded);
    assert_eq!(s.battlefield_len(), 0, "lethal creature cleaned up");
}

#[test]
fn priority_loop_on_fork_does_not_disturb_parent() {
    let db = sample_pool();
    let mut parent = GameState::new(2, 0);
    parent.push_stack(StackObject { id: ObjectId(1), controller: PlayerId(0) });

    let mut child = parent.fork();
    let _ = child.pass_priority(&db); // P0 -> P1
    let _ = child.pass_priority(&db); // resolves the object in the child

    assert_eq!(child.stack_len(), 0, "child resolved its stack");
    assert_eq!(parent.stack_len(), 1, "parent's stack is untouched");
}
```

- [ ] **Step 2: Run the integration test**

Run: `cd magicbot-engine && cargo test --test priority`
Expected: PASS — all three integration tests pass.

- [ ] **Step 3: Verify whole crate green + lint-clean**

Run: `cd magicbot-engine && cargo test && cargo clippy --all-targets -- -D warnings`
Expected: all tests pass; clippy no warnings.

- [ ] **Step 4: Commit**

```bash
git add magicbot-engine/tests/priority.rs
git commit -m "test(engine): full priority round with LIFO resolution + SBA + fork isolation"
```

---

## Out of scope for this plan (deferred to later plans)

1. **Real spell/ability casting** — `StackObject` is a placeholder (`id` + `controller`). Putting an actual spell on the stack (with source, targets, modes, X, cost payment) and having resolution DO something (deal damage, create a permanent) is the casting plan.
2. **Triggers in the 117.5 loop** — CR 117.5 puts triggered abilities on the stack between SBAs and granting priority. This slice does SBAs then proceeds; trigger collection/dispatch off the `MoveEvent`s is deferred. (`check_state_based_actions` already returns them, ready to consume.)
3. **Phases and steps / the turn cycle** — `StepEnded` is returned but nothing advances to the next step, untaps, draws, etc. The turn-structure plan consumes `StepEnded`.
4. **Mana abilities not using the stack** (CR 405.6c) — no mana system yet.
5. **APNAP for simultaneous stack additions** (CR 405.3) — only single `push_stack` here; simultaneous triggers ordering is part of the trigger plan.
6. **Active-player rotation** — `active_player` is fixed at P0 (set in `new`); rotating it each turn is the turn-structure plan.
7. **The resolved `StackObject`'s effect** — resolution pops it and returns it but applies no effect (placeholder).

## Self-review notes

- **Spec coverage:** rules-core §117 priority + §117.3d APNAP advance → Task 1 `record_pass`, Task 3 loop. §405.2 LIFO → Task 2 `push_stack`/`peek_stack` + Task 3 `pop_back`, conformance C2 integration. §117.5 SBAs-before-priority → Task 3 `pass_priority` calls `check_state_based_actions` on a full round, `resolution_runs_sbas_first` + `sba_fires_during_the_priority_loop` tests. §405.5 empty-stack-ends-step → `PriorityOutcome::StepEnded`, conformance C3. §608.1 resolve top → `Resolved`.
- **Split-brain resolved:** `GameState.stack` is now the single source of truth (typed `StackObject`, actively pushed/popped); the `Zones` Stack arm stays a no-op and the move-pipeline Stack guard stays — consistent, no dual bookkeeping.
- **SBA composition:** `pass_priority` runs `check_state_based_actions` exactly where CR 117.5 mandates, composing the prior slice into its real home.
- **CoW:** Task 4 `priority_loop_on_fork_does_not_disturb_parent` ties the new mutable stack to fork isolation.
- **Termination:** `record_pass` increments a bounded `passes_in_a_row`; a full round either resolves one stack object (strictly shrinking the stack) or ends the step — no unbounded loop.
- **Type consistency:** `PriorityState` methods (Task 1) used by `pass_priority` (Task 3). `StackObject` (Task 1) is the stack element type (Task 2) and the `Resolved` payload (Task 3). `PriorityOutcome` (Task 3) returned by `pass_priority`, matched in Tasks 3,4. `n_players as u8` cast consistent (Task 2 `new`).
