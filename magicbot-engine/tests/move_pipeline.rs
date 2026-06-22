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
