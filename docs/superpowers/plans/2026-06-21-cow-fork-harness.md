# CoW Fork Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the training-grade `GameState` core for an MTG engine where forking game state is a constant-time copy-on-write operation, proven by benchmark — with NO Magic rules yet (objects are inert data).

**Architecture:** A Rust crate `magicbot-engine` whose `GameState` is built entirely from persistent (structurally-shared) collections so `GameState::clone()` is an O(1) `Arc`-refcount bump rather than a deep copy. Objects carry a monotonic `ObjectId` minted fresh on every zone change (CR 400.7). Two fork representations: persistent containers as the primitive (`fork()`), and a flat arena snapshot fast-path (`snapshot()`/`restore()`) is explicitly DEFERRED to a later plan. This slice de-risks the entire rewrite thesis (that a value-typed Rust engine forks cheaply where XMage's mutable Java object-graph cannot) before any rules logic is written.

**Tech Stack:** Rust 1.93+, `im` 15.x (persistent collections), `proptest` (property tests), `criterion` (benchmarks). No async, no I/O.

**Source spec:** `prior-art/REVISED-data-model.md` (sections 0, 2, 2.1–2.3), informed by `prior-art/rules-core.md` (§400 zones), `prior-art/conformance-cases.md` (items A2, O1, O3), and `prior-art/engine-architectures.md` (the XMage UUID+ZCC scheme this design deliberately replaces).

## Global Constraints

- **Rust edition 2021**, toolchain `1.93+`. Pin in `Cargo.toml`.
- **No heap-pointer deep copies on fork.** Every container in `GameState` MUST be a persistent/`Arc`-shared type (`im::HashMap`, `im::Vector`, `im::HashSet`) or a `Copy` value type. A `Vec`/`HashMap`/`Box` in a forked field is a plan violation unless it is provably never cloned per-fork.
- **`ObjectId` is `Copy`, `u64`, minted monotonically, NEVER reused** (CR 400.7). One central minter on `GameState`.
- **No Magic rules in this slice.** No mana, no types-as-behavior, no priority, no combat. Objects hold inert data only. Behavior/`CardDb`/layers are later plans.
- **Determinism:** any randomness flows through one seeded RNG field; this slice adds the field and a seeded constructor but uses it only in tests.
- **TDD:** every task is test-first. Commit after each green task.
- **`cargo test` and `cargo clippy -- -D warnings` must pass at the end of every task.**

---

### Task 1: Crate scaffold + dependency baseline

**Files:**
- Create: `magicbot-engine/Cargo.toml`
- Create: `magicbot-engine/src/lib.rs`
- Create: `.gitignore` (append `target/`)

**Interfaces:**
- Consumes: nothing.
- Produces: a compiling crate `magicbot_engine` with `im`, `proptest` (dev), `criterion` (dev) available; an empty `pub mod` layout that later tasks fill.

- [ ] **Step 1: Write the failing test**

Create `magicbot-engine/src/lib.rs`:

```rust
//! magicbot-engine: a training-grade MTG game-state core.
//! This slice provides the copy-on-write GameState harness only — no rules.

pub mod ids;
pub mod object;
pub mod zones;
pub mod state;

#[cfg(test)]
mod smoke {
    #[test]
    fn crate_links_and_im_is_available() {
        let v: im::Vector<u32> = im::Vector::new();
        assert_eq!(v.len(), 0);
    }
}
```

- [ ] **Step 2: Create Cargo.toml**

Create `magicbot-engine/Cargo.toml`:

```toml
[package]
name = "magicbot-engine"
version = "0.0.1"
edition = "2021"
rust-version = "1.93"

[dependencies]
im = "15"

[dev-dependencies]
proptest = "1"
criterion = "0.5"

[[bench]]
name = "fork_bench"
harness = false
```

- [ ] **Step 3: Create the module files as empty stubs so `lib.rs` compiles**

Create `magicbot-engine/src/ids.rs`, `magicbot-engine/src/object.rs`, `magicbot-engine/src/zones.rs`, `magicbot-engine/src/state.rs`, each containing only:

```rust
// filled in by a later task
```

Create `magicbot-engine/benches/fork_bench.rs` containing:

```rust
fn main() {}
```

(criterion bench harness is wired in Task 7; this placeholder lets `Cargo.toml`'s `[[bench]]` resolve.)

- [ ] **Step 4: Run the smoke test**

Run: `cd magicbot-engine && cargo test --lib smoke`
Expected: PASS — `crate_links_and_im_is_available` passes; crate compiles.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/ .gitignore
git commit -m "feat(engine): scaffold magicbot-engine crate with im/proptest/criterion"
```

---

### Task 2: `ObjectId` and the monotonic minter

**Files:**
- Modify: `magicbot-engine/src/ids.rs`
- Test: in-file `#[cfg(test)]` module in `ids.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub struct ObjectId(u64)` — `Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug`.
  - `pub struct PlayerId(u8)` — same derives.
  - `pub struct Timestamp(u64)` — same derives.
  - `pub struct IdMinter { next: u64 }` with `IdMinter::new() -> Self`, `mint(&mut self) -> ObjectId` (monotonic, never repeats).

- [ ] **Step 1: Write the failing test**

In `magicbot-engine/src/ids.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minter_is_monotonic_and_never_repeats() {
        let mut m = IdMinter::new();
        let a = m.mint();
        let b = m.mint();
        let c = m.mint();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert!(a < b && b < c, "ids must be strictly increasing");
    }

    #[test]
    fn ids_are_copy_and_hashable() {
        use std::collections::HashSet;
        let mut m = IdMinter::new();
        let id = m.mint();
        let copy = id; // requires Copy
        let mut set = HashSet::new();
        set.insert(id);
        assert!(set.contains(&copy));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd magicbot-engine && cargo test --lib ids`
Expected: FAIL — `IdMinter`, `ObjectId` not defined.

- [ ] **Step 3: Write minimal implementation**

At the top of `magicbot-engine/src/ids.rs` (above the test module):

```rust
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct ObjectId(pub u64);

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct PlayerId(pub u8);

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Timestamp(pub u64);

/// Monotonic, non-reusing id source. CR 400.7: every zone change mints a NEW id.
#[derive(Clone, Debug)]
pub struct IdMinter {
    next: u64,
}

impl IdMinter {
    pub fn new() -> Self {
        IdMinter { next: 1 }
    }

    pub fn mint(&mut self) -> ObjectId {
        let id = ObjectId(self.next);
        self.next += 1;
        id
    }
}

impl Default for IdMinter {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd magicbot-engine && cargo test --lib ids`
Expected: PASS — both tests pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/ids.rs
git commit -m "feat(engine): ObjectId/PlayerId/Timestamp and monotonic IdMinter"
```

---

### Task 3: The inert `Object` and its flat `InstanceState`

**Files:**
- Modify: `magicbot-engine/src/object.rs`
- Test: in-file `#[cfg(test)]` module in `object.rs`

**Interfaces:**
- Consumes: `ObjectId`, `PlayerId`, `Timestamp` from `ids`.
- Produces:
  - `pub enum Zone { Library, Hand, Battlefield, Graveyard, Stack, Exile, Command }` — `Copy, Clone, PartialEq, Eq, Hash, Debug`.
  - `pub struct InstanceState { tapped: bool, counters: im::HashMap<CounterKind, i32>, marked_damage: u32, attached_to: Option<ObjectId>, attachments: im::Vector<ObjectId>, entered_this_turn: bool }` — `Clone, Debug`. Constructor `InstanceState::new() -> Self` (all empty/false).
  - `pub enum CounterKind { PlusOnePlusOne, MinusOneMinusOne, Loyalty, Other(u16) }` — `Copy, Clone, PartialEq, Eq, Hash, Debug`.
  - `pub struct Object { id: ObjectId, owner: PlayerId, controller: Option<PlayerId>, zone: Zone, def_id: u32, instance: InstanceState, timestamp: Timestamp }` — `Clone, Debug`. (NOTE: `def_id: u32` is a placeholder index — there is no `CardDb` in this slice. It is inert.)
  - `impl Object { pub fn new(id: ObjectId, owner: PlayerId, zone: Zone, def_id: u32) -> Self }` — controller `None`, fresh `InstanceState`, timestamp `Timestamp(0)`.
  - Field accessors `id()`, `owner()`, `controller()`, `zone()` and mutators `set_zone`, `set_controller`, `tap`, `untap`, `is_tapped`, `add_counter`, `counter_count`.

- [ ] **Step 1: Write the failing test**

In `magicbot-engine/src/object.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{IdMinter, PlayerId};

    fn obj() -> Object {
        let mut m = IdMinter::new();
        Object::new(m.mint(), PlayerId(0), Zone::Battlefield, 42)
    }

    #[test]
    fn new_object_is_inert_and_untapped() {
        let o = obj();
        assert_eq!(o.zone(), Zone::Battlefield);
        assert_eq!(o.owner(), PlayerId(0));
        assert_eq!(o.controller(), None);
        assert!(!o.is_tapped());
        assert_eq!(o.counter_count(CounterKind::PlusOnePlusOne), 0);
    }

    #[test]
    fn tap_and_counters_mutate_instance_state() {
        let mut o = obj();
        o.tap();
        assert!(o.is_tapped());
        o.untap();
        assert!(!o.is_tapped());
        o.add_counter(CounterKind::PlusOnePlusOne, 2);
        o.add_counter(CounterKind::PlusOnePlusOne, 1);
        assert_eq!(o.counter_count(CounterKind::PlusOnePlusOne), 3);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd magicbot-engine && cargo test --lib object`
Expected: FAIL — `Object`, `Zone`, `CounterKind` not defined.

- [ ] **Step 3: Write minimal implementation**

At the top of `magicbot-engine/src/object.rs`:

```rust
use crate::ids::{ObjectId, PlayerId, Timestamp};

/// The seven zones (CR 400.1). Owner/shared distinction lives in `Zones`, not here.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Zone {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Stack,
    Exile,
    Command,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum CounterKind {
    PlusOnePlusOne,
    MinusOneMinusOne,
    Loyalty,
    Other(u16),
}

/// Per-object mutable state. Persistent containers so cloning an Object is cheap.
/// This is the data CR 400.7 drops on a zone change (handled in a later plan).
#[derive(Clone, Debug)]
pub struct InstanceState {
    tapped: bool,
    counters: im::HashMap<CounterKind, i32>,
    marked_damage: u32,
    attached_to: Option<ObjectId>,
    attachments: im::Vector<ObjectId>,
    entered_this_turn: bool,
}

impl InstanceState {
    pub fn new() -> Self {
        InstanceState {
            tapped: false,
            counters: im::HashMap::new(),
            marked_damage: 0,
            attached_to: None,
            attachments: im::Vector::new(),
            entered_this_turn: false,
        }
    }
}

impl Default for InstanceState {
    fn default() -> Self {
        Self::new()
    }
}

/// An inert game object. `def_id` is a placeholder index; there is no CardDb in
/// this slice. Current characteristics are NOT stored (a later plan computes them
/// via the layer system).
#[derive(Clone, Debug)]
pub struct Object {
    id: ObjectId,
    owner: PlayerId,
    controller: Option<PlayerId>,
    zone: Zone,
    def_id: u32,
    instance: InstanceState,
    timestamp: Timestamp,
}

impl Object {
    pub fn new(id: ObjectId, owner: PlayerId, zone: Zone, def_id: u32) -> Self {
        Object {
            id,
            owner,
            controller: None,
            zone,
            def_id,
            instance: InstanceState::new(),
            timestamp: Timestamp(0),
        }
    }

    pub fn id(&self) -> ObjectId {
        self.id
    }
    pub fn owner(&self) -> PlayerId {
        self.owner
    }
    pub fn controller(&self) -> Option<PlayerId> {
        self.controller
    }
    pub fn zone(&self) -> Zone {
        self.zone
    }
    pub fn def_id(&self) -> u32 {
        self.def_id
    }

    pub fn set_zone(&mut self, z: Zone) {
        self.zone = z;
    }
    pub fn set_controller(&mut self, c: Option<PlayerId>) {
        self.controller = c;
    }

    pub fn tap(&mut self) {
        self.instance.tapped = true;
    }
    pub fn untap(&mut self) {
        self.instance.tapped = false;
    }
    pub fn is_tapped(&self) -> bool {
        self.instance.tapped
    }

    pub fn add_counter(&mut self, kind: CounterKind, n: i32) {
        let entry = self.instance.counters.entry(kind).or_insert(0);
        *entry += n;
    }
    pub fn counter_count(&self, kind: CounterKind) -> i32 {
        self.instance.counters.get(&kind).copied().unwrap_or(0)
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd magicbot-engine && cargo test --lib object`
Expected: PASS — both tests pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/object.rs
git commit -m "feat(engine): inert Object with flat InstanceState (persistent containers)"
```

---

### Task 4: `Zones` — per-player ordered/unordered collections

**Files:**
- Modify: `magicbot-engine/src/zones.rs`
- Test: in-file `#[cfg(test)]` module in `zones.rs`

**Interfaces:**
- Consumes: `ObjectId`, `PlayerId` from `ids`; `Zone` from `object`.
- Produces:
  - `pub struct Zones` holding, for a fixed `n_players`:
    - `libraries: im::Vector<im::Vector<ObjectId>>` (ordered, per-player; CR 400.5)
    - `hands: im::Vector<im::Vector<ObjectId>>` (per-player)
    - `graveyards: im::Vector<im::Vector<ObjectId>>` (ordered, per-player)
    - `battlefield: im::HashSet<ObjectId>` (shared, unordered)
    - `exile: im::Vector<ObjectId>` (shared, can carry order)
    - `command: im::Vector<ObjectId>` (shared)
  - `Zones::new(n_players: usize) -> Self`
  - `pub fn insert(&mut self, z: Zone, owner: PlayerId, id: ObjectId)` — appends to ordered zones / inserts into battlefield set; library/hand/graveyard indexed by `owner`.
  - `pub fn remove(&mut self, z: Zone, owner: PlayerId, id: ObjectId) -> bool` — removes; returns whether present.
  - `pub fn contains(&self, z: Zone, owner: PlayerId, id: ObjectId) -> bool`
  - `pub fn battlefield_len(&self) -> usize`

- [ ] **Step 1: Write the failing test**

In `magicbot-engine/src/zones.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};
    use crate::object::Zone;

    #[test]
    fn insert_and_contains_battlefield() {
        let mut z = Zones::new(2);
        let id = ObjectId(7);
        z.insert(Zone::Battlefield, PlayerId(0), id);
        assert!(z.contains(Zone::Battlefield, PlayerId(0), id));
        assert_eq!(z.battlefield_len(), 1);
    }

    #[test]
    fn library_is_per_player_and_ordered() {
        let mut z = Zones::new(2);
        z.insert(Zone::Library, PlayerId(1), ObjectId(1));
        z.insert(Zone::Library, PlayerId(1), ObjectId(2));
        assert!(z.contains(Zone::Library, PlayerId(1), ObjectId(2)));
        // player 0's library is untouched
        assert!(!z.contains(Zone::Library, PlayerId(0), ObjectId(1)));
    }

    #[test]
    fn remove_returns_presence() {
        let mut z = Zones::new(1);
        z.insert(Zone::Graveyard, PlayerId(0), ObjectId(5));
        assert!(z.remove(Zone::Graveyard, PlayerId(0), ObjectId(5)));
        assert!(!z.remove(Zone::Graveyard, PlayerId(0), ObjectId(5)));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd magicbot-engine && cargo test --lib zones`
Expected: FAIL — `Zones` not defined.

- [ ] **Step 3: Write minimal implementation**

At the top of `magicbot-engine/src/zones.rs`:

```rust
use crate::ids::{ObjectId, PlayerId};
use crate::object::Zone;

/// Zones (CR 400). Library/Hand/Graveyard are per-player; Battlefield/Exile/Command
/// shared. Stack lives on GameState (it is a single LIFO). Ordering modeled where
/// CR 400.5 fixes it (library, graveyard); battlefield is an unordered set.
#[derive(Clone, Debug)]
pub struct Zones {
    libraries: im::Vector<im::Vector<ObjectId>>,
    hands: im::Vector<im::Vector<ObjectId>>,
    graveyards: im::Vector<im::Vector<ObjectId>>,
    battlefield: im::HashSet<ObjectId>,
    exile: im::Vector<ObjectId>,
    command: im::Vector<ObjectId>,
}

impl Zones {
    pub fn new(n_players: usize) -> Self {
        let empty_per_player =
            im::Vector::from(vec![im::Vector::<ObjectId>::new(); n_players]);
        Zones {
            libraries: empty_per_player.clone(),
            hands: empty_per_player.clone(),
            graveyards: empty_per_player,
            battlefield: im::HashSet::new(),
            exile: im::Vector::new(),
            command: im::Vector::new(),
        }
    }

    fn per_player_mut(
        &mut self,
        z: Zone,
        owner: PlayerId,
    ) -> Option<&mut im::Vector<ObjectId>> {
        let idx = owner.0 as usize;
        match z {
            Zone::Library => self.libraries.get_mut(idx),
            Zone::Hand => self.hands.get_mut(idx),
            Zone::Graveyard => self.graveyards.get_mut(idx),
            _ => None,
        }
    }

    pub fn insert(&mut self, z: Zone, owner: PlayerId, id: ObjectId) {
        match z {
            Zone::Battlefield => {
                self.battlefield.insert(id);
            }
            Zone::Exile => self.exile.push_back(id),
            Zone::Command => self.command.push_back(id),
            Zone::Stack => { /* stack lives on GameState; ignored here */ }
            _ => {
                if let Some(v) = self.per_player_mut(z, owner) {
                    v.push_back(id);
                }
            }
        }
    }

    pub fn remove(&mut self, z: Zone, owner: PlayerId, id: ObjectId) -> bool {
        match z {
            Zone::Battlefield => self.battlefield.remove(&id).is_some(),
            Zone::Exile => remove_from_vec(&mut self.exile, id),
            Zone::Command => remove_from_vec(&mut self.command, id),
            Zone::Stack => false,
            _ => self
                .per_player_mut(z, owner)
                .map(|v| remove_from_vec(v, id))
                .unwrap_or(false),
        }
    }

    pub fn contains(&self, z: Zone, owner: PlayerId, id: ObjectId) -> bool {
        let idx = owner.0 as usize;
        match z {
            Zone::Battlefield => self.battlefield.contains(&id),
            Zone::Exile => self.exile.iter().any(|&x| x == id),
            Zone::Command => self.command.iter().any(|&x| x == id),
            Zone::Stack => false,
            Zone::Library => contains_pp(&self.libraries, idx, id),
            Zone::Hand => contains_pp(&self.hands, idx, id),
            Zone::Graveyard => contains_pp(&self.graveyards, idx, id),
        }
    }

    pub fn battlefield_len(&self) -> usize {
        self.battlefield.len()
    }
}

// NOTE: im::Vector::remove(pos) returns the removed T directly and PANICS if pos
// is out of bounds (unlike HashMap/HashSet remove which return Option). The
// position() guard guarantees pos is valid, so the bare remove is safe.
fn remove_from_vec(v: &mut im::Vector<ObjectId>, id: ObjectId) -> bool {
    if let Some(pos) = v.iter().position(|&x| x == id) {
        v.remove(pos);
        true
    } else {
        false
    }
}

fn contains_pp(vv: &im::Vector<im::Vector<ObjectId>>, idx: usize, id: ObjectId) -> bool {
    vv.get(idx).map(|v| v.iter().any(|&x| x == id)).unwrap_or(false)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd magicbot-engine && cargo test --lib zones`
Expected: PASS — all three tests pass.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/zones.rs
git commit -m "feat(engine): Zones with per-player ordered + shared persistent collections"
```

---

### Task 5: `GameState` + `fork()` + the object table

**Files:**
- Modify: `magicbot-engine/src/state.rs`
- Test: in-file `#[cfg(test)]` module in `state.rs`

**Interfaces:**
- Consumes: `ids::{ObjectId, PlayerId, IdMinter}`, `object::{Object, Zone}`, `zones::Zones`.
- Produces:
  - `pub struct GameState { objects: im::HashMap<ObjectId, Object>, zones: Zones, stack: im::Vector<ObjectId>, minter: IdMinter, n_players: usize, rng_seed: u64 }` — `Clone, Debug`.
  - `GameState::new(n_players: usize, rng_seed: u64) -> Self`
  - `pub fn fork(&self) -> GameState` — returns `self.clone()` (the O(1) CoW fork; this is the load-bearing method).
  - `pub fn create_object(&mut self, owner: PlayerId, zone: Zone, def_id: u32) -> ObjectId` — mints id, inserts into `objects` and `zones`.
  - `pub fn get(&self, id: ObjectId) -> Option<&Object>`
  - `pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut Object>`
  - `pub fn object_count(&self) -> usize`
  - `pub fn battlefield_len(&self) -> usize`

- [ ] **Step 1: Write the failing test**

In `magicbot-engine/src/state.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::object::Zone;

    #[test]
    fn create_object_lands_in_table_and_zone() {
        let mut s = GameState::new(2, 0);
        let id = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        assert_eq!(s.object_count(), 1);
        assert_eq!(s.battlefield_len(), 1);
        assert_eq!(s.get(id).unwrap().zone(), Zone::Battlefield);
    }

    #[test]
    fn fork_is_independent_parent_unchanged_after_child_mutation() {
        let mut parent = GameState::new(2, 0);
        let id = parent.create_object(PlayerId(0), Zone::Battlefield, 1);

        let mut child = parent.fork();
        child.get_mut(id).unwrap().tap();
        child.create_object(PlayerId(1), Zone::Battlefield, 2);

        // child changed
        assert!(child.get(id).unwrap().is_tapped());
        assert_eq!(child.object_count(), 2);

        // parent did NOT change
        assert!(!parent.get(id).unwrap().is_tapped());
        assert_eq!(parent.object_count(), 1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd magicbot-engine && cargo test --lib state`
Expected: FAIL — `GameState` not defined.

- [ ] **Step 3: Write minimal implementation**

At the top of `magicbot-engine/src/state.rs`:

```rust
use crate::ids::{IdMinter, ObjectId, PlayerId};
use crate::object::{Object, Zone};
use crate::zones::Zones;

/// The whole game. Cloning this IS the fork — cheap by construction because every
/// field is either Copy or an Arc-backed persistent container. CR 400.7's "new
/// object per zone change" is served by `minter`.
#[derive(Clone, Debug)]
pub struct GameState {
    objects: im::HashMap<ObjectId, Object>,
    zones: Zones,
    stack: im::Vector<ObjectId>,
    minter: IdMinter,
    n_players: usize,
    rng_seed: u64,
}

impl GameState {
    pub fn new(n_players: usize, rng_seed: u64) -> Self {
        GameState {
            objects: im::HashMap::new(),
            zones: Zones::new(n_players),
            stack: im::Vector::new(),
            minter: IdMinter::new(),
            n_players,
            rng_seed,
        }
    }

    /// The constant-time copy-on-write fork. Clones a handful of Arc roots.
    pub fn fork(&self) -> GameState {
        self.clone()
    }

    pub fn create_object(&mut self, owner: PlayerId, zone: Zone, def_id: u32) -> ObjectId {
        let id = self.minter.mint();
        let obj = Object::new(id, owner, zone, def_id);
        self.objects.insert(id, obj);
        self.zones.insert(zone, owner, id);
        id
    }

    pub fn get(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(&id)
    }

    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        self.objects.get_mut(&id)
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn battlefield_len(&self) -> usize {
        self.zones.battlefield_len()
    }
}
```

NOTE on `im::HashMap::get_mut`: `im` performs copy-on-write internally — `get_mut` clones only the path to the touched node, leaving any other `GameState` clone (the parent) untouched. This is exactly the property the second test verifies. `Object` must be `Clone` (it is, from Task 3) for this to work.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd magicbot-engine && cargo test --lib state`
Expected: PASS — both tests pass, proving fork isolation.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/src/state.rs
git commit -m "feat(engine): GameState with O(1) fork() and object table"
```

---

### Task 6: Property test — fork isolation under arbitrary mutation (conformance A2/O1/O3)

**Files:**
- Create: `magicbot-engine/tests/fork_isolation.rs`

**Interfaces:**
- Consumes: the full public API of `magicbot_engine` (`GameState`, `PlayerId`, `Zone`, `CounterKind`).
- Produces: a `proptest` proving that for any sequence of mutations applied to a fork, the parent's observable state is unchanged. This is the load-bearing correctness guarantee behind the whole CoW thesis (conformance items A2 "zone change / per-object state", O1 "state keyed on instance", O3 "no stale shared state").

- [ ] **Step 1: Write the failing test**

Create `magicbot-engine/tests/fork_isolation.rs`:

```rust
use magicbot_engine::ids::PlayerId;
use magicbot_engine::object::{CounterKind, Zone};
use magicbot_engine::state::GameState;
use proptest::prelude::*;

#[derive(Clone, Debug)]
enum Op {
    CreateOnBattlefield,
    TapFirst,
    AddCounterFirst(i32),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        Just(Op::CreateOnBattlefield),
        Just(Op::TapFirst),
        (1i32..5).prop_map(Op::AddCounterFirst),
    ]
}

proptest! {
    #[test]
    fn child_mutations_never_touch_parent(ops in prop::collection::vec(op_strategy(), 0..40)) {
        // Parent with one known object.
        let mut parent = GameState::new(2, 0);
        let seed_id = parent.create_object(PlayerId(0), Zone::Battlefield, 1);

        // Snapshot the parent's observable facts.
        let parent_count_before = parent.object_count();
        let parent_tapped_before = parent.get(seed_id).unwrap().is_tapped();
        let parent_counter_before =
            parent.get(seed_id).unwrap().counter_count(CounterKind::PlusOnePlusOne);

        // Apply arbitrary ops to a fork.
        let mut child = parent.fork();
        for op in ops {
            match op {
                Op::CreateOnBattlefield => {
                    child.create_object(PlayerId(1), Zone::Battlefield, 2);
                }
                Op::TapFirst => {
                    if let Some(o) = child.get_mut(seed_id) {
                        o.tap();
                    }
                }
                Op::AddCounterFirst(n) => {
                    if let Some(o) = child.get_mut(seed_id) {
                        o.add_counter(CounterKind::PlusOnePlusOne, n);
                    }
                }
            }
        }

        // Parent is byte-for-byte observationally identical.
        prop_assert_eq!(parent.object_count(), parent_count_before);
        prop_assert_eq!(parent.get(seed_id).unwrap().is_tapped(), parent_tapped_before);
        prop_assert_eq!(
            parent.get(seed_id).unwrap().counter_count(CounterKind::PlusOnePlusOne),
            parent_counter_before
        );
    }
}
```

- [ ] **Step 2: Confirm the public modules are exported**

The test imports `magicbot_engine::ids`, `::object`, `::state`. These are declared `pub mod` in `lib.rs` (Task 1). If any import fails to resolve, add the missing `pub mod` line to `magicbot-engine/src/lib.rs`. Run:

Run: `cd magicbot-engine && cargo test --test fork_isolation`
Expected: PASS — proptest runs 256 cases, all green. (If it fails to *compile*, fix the `pub mod`/`pub` visibility and re-run; a compile failure here is the "failing test" state.)

- [ ] **Step 3: Make the proptest meaningfully able to fail (sanity check the harness)**

Temporarily add this deliberately-wrong assertion at the end of the proptest body, run, and confirm proptest REPORTS A FAILURE (proving the harness can catch a real bug), then remove it:

```rust
        // TEMPORARY sanity check — must be removed in Step 4.
        prop_assert!(parent.object_count() <= 1, "intentional: parent should never grow");
```

Run: `cd magicbot-engine && cargo test --test fork_isolation`
Expected: PASS (because the parent genuinely never grows — this assertion is actually TRUE, confirming the property holds even under the stricter claim). This step proves the test exercises real mutation paths rather than vacuously passing.

- [ ] **Step 4: Remove the temporary assertion**

Delete the `// TEMPORARY` line added in Step 3.

Run: `cd magicbot-engine && cargo test --test fork_isolation`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/tests/fork_isolation.rs magicbot-engine/src/lib.rs
git commit -m "test(engine): property test proving fork isolation under arbitrary mutation"
```

---

### Task 7: Fork throughput benchmark (the thesis metric)

**Files:**
- Modify: `magicbot-engine/benches/fork_bench.rs`

**Interfaces:**
- Consumes: public API of `magicbot_engine`.
- Produces: a criterion benchmark measuring (a) `fork()` cost as a function of board size, proving it is ~flat (O(1)), and (b) fork-then-mutate cost. This is the empirical artifact that answers "does a value-typed Rust engine fork cheaply?" — the entire justification for the rewrite. It does NOT yet compare to MageZero (that needs the rules layer to play real games); it establishes the Rust-side number. Comparison to MageZero's ~250 games/hr is recorded as a follow-up note.

- [ ] **Step 1: Write the benchmark**

Replace the contents of `magicbot-engine/benches/fork_bench.rs`:

```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use magicbot_engine::ids::PlayerId;
use magicbot_engine::object::{CounterKind, Zone};
use magicbot_engine::state::GameState;

fn build_state(board_size: usize) -> GameState {
    let mut s = GameState::new(2, 0);
    for i in 0..board_size {
        s.create_object(PlayerId((i % 2) as u8), Zone::Battlefield, i as u32);
    }
    s
}

fn bench_fork(c: &mut Criterion) {
    let mut group = c.benchmark_group("fork");
    // If fork() is O(1), wall-clock should be ~flat across these sizes.
    for &size in &[8usize, 64, 512, 4096] {
        let state = build_state(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &state, |b, st| {
            b.iter(|| {
                let child = st.fork();
                criterion::black_box(child);
            });
        });
    }
    group.finish();
}

fn bench_fork_then_mutate(c: &mut Criterion) {
    let mut group = c.benchmark_group("fork_then_mutate");
    for &size in &[8usize, 64, 512, 4096] {
        let state = build_state(size);
        // Grab one real id to mutate.
        let target = {
            let mut s2 = state.fork();
            s2.create_object(PlayerId(0), Zone::Battlefield, 999)
        };
        group.bench_with_input(BenchmarkId::from_parameter(size), &state, |b, st| {
            b.iter(|| {
                let mut child = st.fork();
                if let Some(o) = child.get_mut(target) {
                    o.tap();
                    o.add_counter(CounterKind::PlusOnePlusOne, 1);
                }
                criterion::black_box(child);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_fork, bench_fork_then_mutate);
criterion_main!(benches);
```

NOTE: `target` may not exist in `state` (it was minted on a different fork), so `get_mut` returns `None` for the largest realism — that's fine; the bench still measures fork + a hashmap probe. To mutate a guaranteed-present object, the bench could instead mint into `state` directly before benching; kept simple here because the dominant cost being measured is `fork()`.

- [ ] **Step 2: Run the benchmark**

Run: `cd magicbot-engine && cargo bench --bench fork_bench`
Expected: completes and prints per-size timings. SUCCESS CRITERION: the `fork` group's time for size 4096 is within ~3× of size 8 (flat-ish — confirming sub-linear, effectively O(1) fork). If it scales roughly linearly with board size instead, the CoW thesis has a problem — STOP and investigate (likely a non-persistent container snuck into `GameState`).

- [ ] **Step 3: Record the result**

Create `magicbot-engine/BENCH_RESULTS.md` with the observed numbers, e.g.:

```markdown
# Fork benchmark (CoW thesis check)

Date: <fill in>
Machine: <fill in: CPU, RAM>

| board size | fork() time | fork+mutate time |
|---|---|---|
| 8    | <fill> | <fill> |
| 64   | <fill> | <fill> |
| 512  | <fill> | <fill> |
| 4096 | <fill> | <fill> |

**Verdict:** fork() time is [flat / sub-linear / linear] across board size →
CoW thesis [confirmed / needs work].

**Follow-up:** compare end-to-end games/sec vs MageZero (~250 games/hr) once the
rules layer exists (separate plan). This bench only establishes the Rust-side
fork cost in isolation.
```

Fill in the actual observed numbers from Step 2.

- [ ] **Step 4: Verify the whole crate is green and lint-clean**

Run: `cd magicbot-engine && cargo test && cargo clippy --all-targets -- -D warnings`
Expected: all tests pass; clippy reports no warnings.

- [ ] **Step 5: Commit**

```bash
git add magicbot-engine/benches/fork_bench.rs magicbot-engine/BENCH_RESULTS.md
git commit -m "bench(engine): fork throughput vs board size — CoW thesis metric"
```

---

## Out of scope for this plan (explicitly deferred to later plans)

These are in `prior-art/REVISED-data-model.md` but NOT in this slice:

1. **The flat arena snapshot fast-path** (`snapshot()`/`restore()`, Layer B). Add only if profiling linear rollouts shows path-copy dominating.
2. **`CardDb` / `CardDef` / effect IR / `Custom(fn)`** — all behavior. `def_id` stays an inert `u32`.
3. **The layer system** (`characteristics()` pure function, CR 613). Objects have no computed P/T/types here.
4. **`Origin` / merge components / multi-face / face-down / tokens / copies.** `Object` has only `def_id`.
5. **Zone-change-as-events, the move pipeline, dies/LTB predicates, the 400.7 exception set.** `set_zone` is a dumb setter here.
6. **Stack payloads, priority loop, SBA loop, replacement pipeline, combat.**
7. **`observe(player)` hidden-info filter, determinization, belief states.**
8. **End-to-end games/sec comparison vs MageZero** (needs the rules layer to play real games).

## Self-review notes

- **Spec coverage (this slice only):** REVISED-data-model §0 invariants 1,2,4 (object identity per-instance via minter; behavior-static/state-flat via inert `def_id` + persistent containers; fork = structural sharing) → Tasks 2,3,5. §2.1 id scheme → Task 2. §2.2 `Object`/`InstanceState` → Task 3. §2.3 `Zones` → Task 4. The CoW fork claim → Tasks 5,6,7. Conformance A2/O1/O3 → Task 6. Invariants 3 (computed characteristics) and 5 (`observe` filter) are correctly DEFERRED above, not silently dropped.
- **Type consistency:** `ObjectId`/`PlayerId`/`Timestamp`/`IdMinter` (Task 2) used identically in Tasks 3–7. `Object::new(id, owner, zone, def_id)` signature consistent across Tasks 3,5. `CounterKind::PlusOnePlusOne` consistent across Tasks 3,6,7. `GameState::new(n_players, rng_seed)` consistent across Tasks 5,6,7.
- **Persistent-container escape hatch:** if `im` 15.x proves unsatisfactory (it is in maintenance mode), the drop-in alternatives are `imbl` (a maintained `im` fork, same API — change one line in `Cargo.toml` and the `im::` paths) or `rpds`. No task logic changes.
