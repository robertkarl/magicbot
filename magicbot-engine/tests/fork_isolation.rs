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
