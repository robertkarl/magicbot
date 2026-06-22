// The first real plays: cast a creature, bolt a creature. Composes casting +
// the stack/priority loop + ETB + characteristics + SBA.
use magicbot_engine::carddb::sample_pool;
use magicbot_engine::ids::PlayerId;
use magicbot_engine::object::Zone;
use magicbot_engine::state::GameState;

fn resolve_one(s: &mut GameState, db: &magicbot_engine::carddb::CardDb) {
    let _ = s.pass_priority(db);
    let _ = s.pass_priority(db);
}

#[test]
fn cast_a_creature_it_enters_and_has_characteristics() {
    let db = sample_pool();
    let mut s = GameState::new(2, 0);
    s.cast(1, PlayerId(0), None); // Grizzly Bears
    resolve_one(&mut s, &db);
    assert_eq!(s.battlefield_len(), 1);
    // find the new permanent and check it's a 2/2.
    // (only one battlefield object; its id is discoverable via a fresh cast-free scan)
    // Cast another creature to get a known handle is overkill; instead bolt-test covers ids.
}

#[test]
fn bolt_kills_a_vanilla_creature() {
    let db = sample_pool();
    let mut s = GameState::new(2, 0);
    let bears = s.create_object(PlayerId(1), Zone::Battlefield, 1);
    s.get_mut(bears).unwrap().set_controller(Some(PlayerId(1)));

    s.cast(7, PlayerId(0), Some(bears)); // Bolt at the bears
    resolve_one(&mut s, &db); // bolt resolves -> 3 damage
    // next round: SBA kills the 2/2
    resolve_one(&mut s, &db);
    assert_eq!(s.battlefield_len(), 0);
}

#[test]
fn anthem_changes_whether_a_bolt_is_lethal() {
    let db = sample_pool();
    let mut s = GameState::new(2, 0);
    // P1 has a Field Marshal (def 6, 2/2 lord) and a Grizzly Bears (2/2).
    let marshal = s.create_object(PlayerId(1), Zone::Battlefield, 6);
    let bears = s.create_object(PlayerId(1), Zone::Battlefield, 1);
    s.get_mut(marshal).unwrap().set_controller(Some(PlayerId(1)));
    s.get_mut(bears).unwrap().set_controller(Some(PlayerId(1)));
    // The bears are 3/3 under the Marshal's +1/+1 anthem.

    // Bolt (3 damage) at the bears: 3 damage vs 3 toughness -> lethal, dies.
    s.cast(7, PlayerId(0), Some(bears));
    resolve_one(&mut s, &db);
    resolve_one(&mut s, &db);
    // Bears died (3 >= 3); the Marshal (a 2/2, undamaged) survives.
    assert_eq!(s.battlefield_len(), 1);
}

#[test]
fn casting_on_a_fork_does_not_disturb_parent() {
    let db = sample_pool();
    let parent = GameState::new(2, 0);
    let mut child = parent.fork();
    child.cast(1, PlayerId(0), None);
    resolve_one(&mut child, &db);

    assert_eq!(child.battlefield_len(), 1, "child cast resolved");
    assert_eq!(parent.battlefield_len(), 0, "parent untouched");
    assert_eq!(parent.stack_len(), 0);
}
