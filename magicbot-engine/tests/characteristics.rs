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
