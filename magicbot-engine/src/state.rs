use crate::ids::{IdMinter, ObjectId, PlayerId};
use crate::object::{Object, Zone};
use crate::priority::{PriorityState, StackObject};
use crate::zones::Zones;

/// The whole game. Cloning this IS the fork — cheap by construction because every
/// field is either Copy or an Arc-backed persistent container. CR 400.7's "new
/// object per zone change" is served by `minter`.
#[derive(Clone, Debug)]
pub struct GameState {
    objects: im::HashMap<ObjectId, Object>,
    zones: Zones,
    stack: im::Vector<StackObject>,
    minter: IdMinter,
    #[allow(dead_code)]
    n_players: usize,
    #[allow(dead_code)]
    rng_seed: u64,
    priority: PriorityState,
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
            priority: PriorityState::new(PlayerId(0), n_players as u8),
        }
    }

    /// The constant-time copy-on-write fork. Clones a handful of Arc roots.
    pub fn fork(&self) -> GameState {
        self.clone()
    }

    pub fn create_object(&mut self, owner: PlayerId, zone: Zone, def_id: u64) -> ObjectId {
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

        debug_assert!(
            to != crate::object::Zone::Stack && from != crate::object::Zone::Stack,
            "Stack moves must go through the (future) cast/resolve path, not move_object"
        );

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

    #[cfg(test)]
    pub(crate) fn zone_contains(&self, z: crate::object::Zone, owner: crate::ids::PlayerId, id: ObjectId) -> bool {
        self.zones.contains(z, owner, id)
    }

    /// CR 704.3 state-based-action loop (this slice: creature death only).
    /// Performs all currently-applicable SBAs simultaneously as a batch, then
    /// repeats until a pass finds none. Returns every MoveEvent produced.
    /// Triggers/priority interleave and the rest of the 704.5 list are deferred.
    pub fn check_state_based_actions(
        &mut self,
        db: &crate::carddb::CardDb,
    ) -> Vec<crate::events::MoveEvent> {
        let mut events = Vec::new();
        loop {
            // Gather one simultaneous batch of doomed creatures.
            let mut doomed: Vec<ObjectId> = Vec::new();
            for (id, obj) in self.objects.iter() {
                if obj.zone() != crate::object::Zone::Battlefield {
                    continue;
                }
                let marked = obj.marked_damage();
                if let Some(chars) = self.characteristics(*id, db) {
                    if crate::sba::creature_dies_to_sba(&chars, marked) {
                        doomed.push(*id);
                    }
                }
            }
            if doomed.is_empty() {
                break;
            }
            // Perform them (move to graveyard). move_object mints new ids; the
            // doomed ids were gathered before any move, so no id is stale within
            // this batch (each id still refers to its battlefield object until
            // its own move).
            for id in doomed {
                if let Some(ev) = self.move_object(id, crate::object::Zone::Graveyard) {
                    events.push(ev);
                }
            }
        }
        events
    }

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
                    StaticAbility::SpellDealsDamage { .. } => {
                        // Spell marker; not a continuous ability — no layer effect.
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod sba_tests {
    use super::*;
    use crate::carddb::sample_pool;
    use crate::events::dies;
    use crate::ids::PlayerId;
    use crate::object::{CounterKind, Zone};

    #[test]
    fn lethal_damage_kills_creature() {
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        // Grizzly Bears (def 1) is 2/2.
        let bears = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        s.get_mut(bears).unwrap().set_controller(Some(PlayerId(0)));
        s.get_mut(bears).unwrap().mark_damage(2);

        let events = s.check_state_based_actions(&db);
        assert_eq!(events.len(), 1, "the bears died");
        assert!(dies(&events[0]));
        assert_eq!(s.battlefield_len(), 0, "battlefield is empty");
    }

    #[test]
    fn minus_counters_to_zero_toughness_kills() {
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        let bears = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        s.get_mut(bears).unwrap().set_controller(Some(PlayerId(0)));
        // 2/2 with two -1/-1 counters -> 0/0 -> dies (704.5f).
        s.get_mut(bears).unwrap().add_counter(CounterKind::MinusOneMinusOne, 2);

        let events = s.check_state_based_actions(&db);
        assert_eq!(events.len(), 1);
        assert_eq!(s.battlefield_len(), 0);
    }

    #[test]
    fn healthy_creatures_survive_and_loop_terminates() {
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        let a = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        let b = s.create_object(PlayerId(0), Zone::Battlefield, 2); // Serra 4/4
        s.get_mut(a).unwrap().set_controller(Some(PlayerId(0)));
        s.get_mut(b).unwrap().set_controller(Some(PlayerId(0)));
        // only 1 damage on the bears (2 toughness) -> survives
        s.get_mut(a).unwrap().mark_damage(1);

        let events = s.check_state_based_actions(&db);
        assert!(events.is_empty(), "no deaths");
        assert_eq!(s.battlefield_len(), 2);
    }

    #[test]
    fn batch_kills_multiple_in_one_pass() {
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        let a = s.create_object(PlayerId(0), Zone::Battlefield, 1); // 2/2
        let b = s.create_object(PlayerId(0), Zone::Battlefield, 1); // 2/2
        s.get_mut(a).unwrap().set_controller(Some(PlayerId(0)));
        s.get_mut(b).unwrap().set_controller(Some(PlayerId(0)));
        s.get_mut(a).unwrap().mark_damage(2);
        s.get_mut(b).unwrap().mark_damage(5);

        let events = s.check_state_based_actions(&db);
        assert_eq!(events.len(), 2, "both died in the batch");
        assert_eq!(s.battlefield_len(), 0);
    }
}

#[cfg(test)]
mod characteristics_tests {
    use super::*;
    use crate::carddb::sample_pool;
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

    #[test]
    fn anthem_does_not_apply_when_controllers_are_none() {
        // create_object leaves controller None. Two uncontrolled objects must
        // NOT satisfy "you control" via None == None — the is_some() guard.
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        let bears = s.create_object(PlayerId(0), Zone::Battlefield, 1); // creature, controller None
        let _anthem = s.create_object(PlayerId(0), Zone::Battlefield, 3); // anthem, controller None
        // deliberately do NOT set_controller on either
        let effects = s.active_static_effects_for(bears, &db);
        assert_eq!(effects.len(), 0, "None controllers must not match via None == None");
    }

    #[test]
    fn two_anthems_stack_additively() {
        // Glorious Anthem is DefId(3) (+1/+1). Two of them on a 2/2 creature -> 4/4.
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        let bears = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        let a1 = s.create_object(PlayerId(0), Zone::Battlefield, 3);
        let a2 = s.create_object(PlayerId(0), Zone::Battlefield, 3);
        s.get_mut(bears).unwrap().set_controller(Some(PlayerId(0)));
        s.get_mut(a1).unwrap().set_controller(Some(PlayerId(0)));
        s.get_mut(a2).unwrap().set_controller(Some(PlayerId(0)));

        let effects = s.active_static_effects_for(bears, &db);
        assert_eq!(effects.len(), 2, "both anthems contribute an effect");

        let chars = s.characteristics(bears, &db).unwrap();
        assert_eq!(chars.pt, Some(Pt { power: 4, toughness: 4 }), "2/2 + two +1/+1 anthems = 4/4");
    }
}

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
        s.get_mut(id).unwrap().set_controller(Some(PlayerId(0)));

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
        assert_eq!(new_obj.controller(), None, "controller reset on move");
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

    #[test]
    fn move_removes_old_id_from_source_zone_table() {
        let mut s = GameState::new(2, 0);
        let id = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        assert!(s.zone_contains(Zone::Battlefield, PlayerId(0), id));
        let _ev = s.move_object(id, Zone::Graveyard).unwrap();
        assert!(!s.zone_contains(Zone::Battlefield, PlayerId(0), id), "old id gone from source zone");
    }
}

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
