// End-to-end: CardDb-derived base + static effects flow through the CR 613
// layer engine. Proves the producer (carddb) feeds the consumer (layers) correctly.
use magicbot_engine::carddb::{sample_pool, DefId};
use magicbot_engine::ids::Timestamp;
use magicbot_engine::layers::{compute, Ability, ContinuousEffect, Counters, Pt, Status};

#[test]
fn anthem_buffs_a_creature_through_the_layer_engine() {
    let db = sample_pool();

    // Grizzly Bears (def 1) is a 2/2.
    let bears = db.get(DefId(1)).expect("bears");
    let base = bears.base_chars();
    assert_eq!(base.pt, Some(Pt { power: 2, toughness: 2 }));

    // Glorious Anthem (def 3) produces a +1/+1 ModifyPt static effect.
    let anthem = db.get(DefId(3)).expect("anthem");
    let effects: Vec<ContinuousEffect> = anthem
        .static_effects()
        .into_iter()
        .map(|e| ContinuousEffect { effect: e, timestamp: Timestamp(1), is_cda: false })
        .collect();

    // Feed both to the layer engine: the bears should be 3/3.
    let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
    assert_eq!(out.pt, Some(Pt { power: 3, toughness: 3 }), "anthem buffs bears to 3/3");
}

#[test]
fn serra_keywords_survive_the_layer_engine() {
    let db = sample_pool();
    let serra = db.get(DefId(2)).expect("serra");
    let out = compute(&serra.base_chars(), &[], &Counters::new(), Status { tapped: false });
    assert!(out.abilities.has(Ability::Flying));
    assert!(out.abilities.has(Ability::Vigilance));
}
