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
