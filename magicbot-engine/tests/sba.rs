// State-based actions composed with the full engine: SBA must read CURRENT
// (layer-computed) toughness, use the move pipeline, and honor CoW fork.
use magicbot_engine::carddb::sample_pool;
use magicbot_engine::ids::PlayerId;
use magicbot_engine::object::Zone;
use magicbot_engine::state::GameState;

#[test]
fn anthem_saves_a_creature_from_lethal_then_removal_kills_it() {
    // Bears 2/2 with 2 marked damage would die, but a +1/+1 anthem makes it 3/3
    // (3 toughness > 2 damage) -> survives. SBA reads COMPUTED toughness.
    let db = sample_pool();
    let mut s = GameState::new(2, 0);
    let bears = s.create_object(PlayerId(0), Zone::Battlefield, 1);
    let anthem = s.create_object(PlayerId(0), Zone::Battlefield, 3);
    s.get_mut(bears).unwrap().set_controller(Some(PlayerId(0)));
    s.get_mut(anthem).unwrap().set_controller(Some(PlayerId(0)));
    s.get_mut(bears).unwrap().mark_damage(2);

    // With the anthem, the bears are 3/3 and survive 2 damage.
    let events = s.check_state_based_actions(&db);
    assert!(events.is_empty(), "anthem keeps the bears alive");
    assert_eq!(s.battlefield_len(), 2);

    // Remove the anthem; now the bears are 2/2 with 2 damage -> dies on next check.
    // (Use the move pipeline to send the anthem to the graveyard.)
    let _ = s.move_object(anthem, Zone::Graveyard);
    let events2 = s.check_state_based_actions(&db);
    assert_eq!(events2.len(), 1, "bears now lethal without the anthem");
    // Only the (new) graveyard objects remain off the battlefield.
    assert_eq!(s.battlefield_len(), 0);
}

#[test]
fn sba_on_a_fork_does_not_disturb_parent() {
    let db = sample_pool();
    let mut parent = GameState::new(2, 0);
    let bears = parent.create_object(PlayerId(0), Zone::Battlefield, 1);
    parent.get_mut(bears).unwrap().set_controller(Some(PlayerId(0)));
    parent.get_mut(bears).unwrap().mark_damage(2);

    // In a fork, run SBA -> the bears die there.
    let mut child = parent.fork();
    let events = child.check_state_based_actions(&db);
    assert_eq!(events.len(), 1);
    assert_eq!(child.battlefield_len(), 0);

    // Parent is untouched: the bears are still on the battlefield (with their damage).
    assert_eq!(parent.battlefield_len(), 1);
    assert!(parent.get(bears).is_some());
}
