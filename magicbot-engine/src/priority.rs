//! CR 117 priority + CR 405 stack core. This slice: the priority-passing and
//! stack-resolution machinery with a placeholder StackObject (real spells later).

use crate::ids::{ObjectId, PlayerId};

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
