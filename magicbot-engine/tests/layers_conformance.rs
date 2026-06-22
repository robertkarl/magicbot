// CR 613 conformance, black-box through compute(). Cases from
// prior-art/conformance-cases.md section F.
use magicbot_engine::ids::Timestamp;
use magicbot_engine::layers::{
    compute, Ability, BaseChars, Chars, Color, ColorSet, ContinuousEffect, Counters, Effect,
    Pt, Status, TypeSet,
};

fn creature(power: i32, toughness: i32) -> BaseChars {
    BaseChars {
        pt: Some(Pt { power, toughness }),
        types: TypeSet::none().with_creature(),
        colors: ColorSet::colorless(),
        abilities: {
            let mut a = magicbot_engine::layers::AbilitySet::new();
            a.add(Ability::Flying);
            a
        },
    }
}

fn untapped() -> Status {
    Status { tapped: false }
}

#[test]
fn f1_no_effects_is_identity() {
    let base = creature(2, 2);
    let out = compute(&base, &[], &Counters::new(), untapped());
    assert_eq!(out.pt, Some(Pt { power: 2, toughness: 2 }));
    assert!(out.abilities.has(Ability::Flying));
}

#[test]
fn f5_humility_plus_anthem_set_vs_modify() {
    // Humility: base P/T 1/1 (7b set) + remove all abilities (layer 6).
    // Glorious Anthem: +1/+1 (7c modify). Result: 2/2 with NO abilities.
    let base = creature(3, 3); // base has Flying
    let effects = vec![
        ContinuousEffect {
            effect: Effect::RemoveAllAbilities,
            timestamp: Timestamp(1),
            is_cda: false,
        },
        ContinuousEffect {
            effect: Effect::SetBasePt(Pt { power: 1, toughness: 1 }),
            timestamp: Timestamp(1),
            is_cda: false,
        },
        ContinuousEffect {
            effect: Effect::ModifyPt { power: 1, toughness: 1 },
            timestamp: Timestamp(2),
            is_cda: false,
        },
    ];
    let out: Chars = compute(&base, &effects, &Counters::new(), untapped());
    assert_eq!(out.pt, Some(Pt { power: 2, toughness: 2 }), "set 1/1 then +1/+1 = 2/2");
    assert!(!out.abilities.has(Ability::Flying), "Humility stripped abilities");
}

#[test]
fn f2_cross_layer_order_type_before_color_independent_of_timestamp() {
    // Type change (layer 4) at a LATER timestamp than a color change (layer 5)
    // still applies first because cross-layer order is fixed (CR 613.1).
    let base = BaseChars {
        pt: None,
        types: TypeSet::none().with_land(),
        colors: ColorSet::colorless(),
        abilities: magicbot_engine::layers::AbilitySet::new(),
    };
    let effects = vec![
        ContinuousEffect {
            effect: Effect::SetColors(ColorSet::colorless().with(Color::Blue)),
            timestamp: Timestamp(1),
            is_cda: false,
        },
        ContinuousEffect {
            effect: Effect::AddType(TypeSet::none().with_creature()),
            timestamp: Timestamp(9),
            is_cda: false,
        },
    ];
    let out = compute(&base, &effects, &Counters::new(), untapped());
    assert!(out.types.is_creature());
    assert!(out.colors.has(Color::Blue));
}
