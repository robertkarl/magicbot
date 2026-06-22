//! Zone-change events and the predicates the rules define over them.
//! CR 400.6/400.7: a zone change is a typed event; "dies"/"leaves the
//! battlefield"/etc. are PREDICATES over the transition, not object kinds.

use crate::ids::{ObjectId, PlayerId};
use crate::object::Zone;

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
