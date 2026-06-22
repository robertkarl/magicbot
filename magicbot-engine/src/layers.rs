//! CR 613 layer system: compute current characteristics as a pure function.
//! No CardDb / effect IR yet — inputs are explicit value types (a later plan
//! gathers these from GameState).

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Pt {
    pub power: i32,
    pub toughness: i32,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct TypeSet {
    creature: bool,
    land: bool,
    artifact: bool,
    enchantment: bool,
}

impl TypeSet {
    pub fn none() -> Self {
        TypeSet { creature: false, land: false, artifact: false, enchantment: false }
    }
    pub fn with_creature(mut self) -> Self {
        self.creature = true;
        self
    }
    pub fn with_land(mut self) -> Self {
        self.land = true;
        self
    }
    pub fn with_enchantment(mut self) -> Self {
        self.enchantment = true;
        self
    }
    pub fn with_artifact(mut self) -> Self {
        self.artifact = true;
        self
    }
    pub fn is_creature(&self) -> bool {
        self.creature
    }
    pub fn is_land(&self) -> bool {
        self.land
    }
    pub fn is_artifact(&self) -> bool {
        self.artifact
    }
    pub fn is_enchantment(&self) -> bool {
        self.enchantment
    }
    pub fn set_creature(&mut self, v: bool) {
        self.creature = v;
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ColorSet(u8);

impl ColorSet {
    pub fn colorless() -> Self {
        ColorSet(0)
    }
    fn bit(c: Color) -> u8 {
        match c {
            Color::White => 1,
            Color::Blue => 2,
            Color::Black => 4,
            Color::Red => 8,
            Color::Green => 16,
        }
    }
    pub fn with(mut self, c: Color) -> Self {
        self.0 |= Self::bit(c);
        self
    }
    pub fn has(&self, c: Color) -> bool {
        self.0 & Self::bit(c) != 0
    }
    pub fn set_to(&mut self, other: ColorSet) {
        self.0 = other.0;
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Ability {
    Flying,
    Trample,
    Deathtouch,
    Lifelink,
    FirstStrike,
    Vigilance,
    Haste,
    Defender,
}

#[derive(Clone, PartialEq, Debug)]
pub struct AbilitySet(im::HashSet<Ability>);

impl AbilitySet {
    pub fn new() -> Self {
        AbilitySet(im::HashSet::new())
    }
    pub fn add(&mut self, a: Ability) {
        self.0.insert(a);
    }
    pub fn remove(&mut self, a: Ability) {
        self.0.remove(&a);
    }
    pub fn has(&self, a: Ability) -> bool {
        self.0.contains(&a)
    }
    pub fn clear(&mut self) {
        self.0 = im::HashSet::new();
    }
}

impl Default for AbilitySet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Chars {
    pub pt: Option<Pt>,
    pub types: TypeSet,
    pub colors: ColorSet,
    pub abilities: AbilitySet,
}

use crate::ids::Timestamp;

#[derive(Clone, Debug)]
pub struct BaseChars {
    pub pt: Option<Pt>,
    pub types: TypeSet,
    pub colors: ColorSet,
    pub abilities: AbilitySet,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Layer {
    Type,
    Color,
    Ability,
    Pt,
}

#[derive(Clone, Debug)]
pub enum Effect {
    AddType(TypeSet),
    SetColors(ColorSet),
    AddAbility(Ability),
    RemoveAllAbilities,
    SetBasePt(Pt),
    ModifyPt { power: i32, toughness: i32 },
    SwitchPt,
}

#[derive(Clone, Debug)]
pub struct ContinuousEffect {
    pub effect: Effect,
    pub timestamp: Timestamp,
    pub is_cda: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct Counters {
    plus1: i32,
    minus1: i32,
}

impl Counters {
    pub fn new() -> Self {
        Counters { plus1: 0, minus1: 0 }
    }
    pub fn with_plus1(mut self, n: i32) -> Self {
        self.plus1 += n;
        self
    }
    pub fn with_minus1(mut self, n: i32) -> Self {
        self.minus1 += n;
        self
    }
    /// CR 122 / 704.5q net for P/T purposes (annihilation handled elsewhere as SBA).
    pub fn net_pt(&self) -> (i32, i32) {
        let net = self.plus1 - self.minus1;
        (net, net)
    }
}

impl Default for Counters {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Status {
    pub tapped: bool,
}

pub fn layer_of(e: &Effect) -> Layer {
    match e {
        Effect::AddType(_) => Layer::Type,
        Effect::SetColors(_) => Layer::Color,
        Effect::AddAbility(_) | Effect::RemoveAllAbilities => Layer::Ability,
        Effect::SetBasePt(_) | Effect::ModifyPt { .. } | Effect::SwitchPt => Layer::Pt,
    }
}

/// Returns the effects in `layer`, ordered for application.
///
/// CR 613.7 timestamp ordering (ascending) is implemented. CR 613.8 DEPENDENCY
/// ordering is DEFERRED to a later plan: when one same-layer effect changes what
/// another applies to (e.g. Blood Moon vs. an Urborg-style effect in layer 4),
/// dependency order overrides timestamp, with mutual loops falling back to
/// timestamp (613.8b) and re-evaluation after each application (613.8c). This
/// stub applies pure timestamp order, which is correct for all non-dependent
/// effect sets (the common case and every conformance case in this slice).
fn effects_in_layer(
    effects: &[ContinuousEffect],
    layer: Layer,
) -> Vec<&ContinuousEffect> {
    let mut v: Vec<&ContinuousEffect> =
        effects.iter().filter(|e| layer_of(&e.effect) == layer).collect();
    // CR 613.7: ascending timestamp order within a layer.
    v.sort_by_key(|e| e.timestamp.0);
    v
}

/// Like effects_in_layer but applies CR 613.3: CDA-sourced effects first,
/// then non-CDA, each group ascending by timestamp.
fn effects_in_layer_cda_first(
    effects: &[ContinuousEffect],
    layer: Layer,
) -> Vec<&ContinuousEffect> {
    let mut v = effects_in_layer(effects, layer);
    // stable partition: CDAs (true) before non-CDAs (false), preserving timestamp order.
    v.sort_by_key(|e| !e.is_cda);
    v
}

/// CR 613.5: characteristics are a pure function recomputed on demand.
pub fn compute(
    base: &BaseChars,
    effects: &[ContinuousEffect],
    counters: &Counters,
    _status: Status,
) -> Chars {
    // Start from the layer-1 copiable base (copy/face-down handled in a later task).
    let mut types = base.types;
    let mut colors = base.colors;
    let mut abilities = base.abilities.clone();
    let mut pt = base.pt;

    // Layer 4: type-changing. Only creature-addition (animation) is modeled in
    // this slice; land/artifact/enchantment flags on an AddType payload are a
    // no-op until a later plan widens the type model.
    for ce in effects_in_layer(effects, Layer::Type) {
        if let Effect::AddType(t) = &ce.effect {
            if t.is_creature() {
                types.set_creature(true);
            }
        }
    }

    // Layer 5: color-changing (set; last timestamp wins).
    for ce in effects_in_layer(effects, Layer::Color) {
        if let Effect::SetColors(c) = &ce.effect {
            colors.set_to(*c);
        }
    }

    // Layer 6: ability add / remove-all. CR 613.3: CDAs first, then timestamp.
    for ce in effects_in_layer_cda_first(effects, Layer::Ability) {
        match &ce.effect {
            Effect::AddAbility(a) => abilities.add(*a),
            Effect::RemoveAllAbilities => abilities.clear(),
            _ => {}
        }
    }

    // Layer 7 only applies to objects that have a P/T. P/T effects on a base
    // with `pt: None` (a noncreature) are intentionally dropped: in real MTG an
    // "animate" effect grants creature type AND P/T together, but that coupling
    // is deferred to the CardDb plan. Effect::SetBasePt does NOT create P/T here.
    // Layer 7 (P/T), sublayers in fixed order 7a -> 7b -> 7c -> 7d (CR 613.4).
    if pt.is_some() {
        let pt_effects = effects_in_layer(effects, Layer::Pt);

        // 7a: CDA P/T sets (is_cda && SetBasePt), timestamp order.
        for ce in pt_effects.iter().filter(|e| e.is_cda) {
            if let Effect::SetBasePt(p) = &ce.effect {
                pt = Some(*p);
            }
        }
        // 7b: non-CDA sets, timestamp order.
        for ce in pt_effects.iter().filter(|e| !e.is_cda) {
            if let Effect::SetBasePt(p) = &ce.effect {
                pt = Some(*p);
            }
        }
        // 7c: modify effects + counters (CR 613.4c folds counters here).
        let mut p = pt.unwrap();
        for ce in pt_effects.iter() {
            if let Effect::ModifyPt { power, toughness } = &ce.effect {
                p.power += power;
                p.toughness += toughness;
            }
        }
        let (cp, ct) = counters.net_pt();
        p.power += cp;
        p.toughness += ct;
        pt = Some(p);

        // 7d: switch P/T, timestamp order (each switch flips).
        for ce in pt_effects.iter() {
            if let Effect::SwitchPt = &ce.effect {
                let cur = pt.unwrap();
                pt = Some(Pt { power: cur.toughness, toughness: cur.power });
            }
        }
    }

    Chars { pt, types, colors, abilities }
}

#[cfg(test)]
mod value_tests {
    use super::*;

    #[test]
    fn typeset_creature_flag() {
        let t = TypeSet::none().with_creature();
        assert!(t.is_creature());
        assert!(!t.is_land());
    }

    #[test]
    fn colorset_bits() {
        let c = ColorSet::colorless().with(Color::White).with(Color::Blue);
        assert!(c.has(Color::White));
        assert!(c.has(Color::Blue));
        assert!(!c.has(Color::Black));
    }

    #[test]
    fn ability_set_add_remove_has() {
        let mut a = AbilitySet::new();
        a.add(Ability::Flying);
        assert!(a.has(Ability::Flying));
        a.remove(Ability::Flying);
        assert!(!a.has(Ability::Flying));
    }
}

#[cfg(test)]
mod compute_skeleton_tests {
    use super::*;

    fn base_creature() -> BaseChars {
        BaseChars {
            pt: Some(Pt { power: 2, toughness: 2 }),
            types: TypeSet::none().with_creature(),
            colors: ColorSet::colorless().with(Color::Green),
            abilities: AbilitySet::new(),
        }
    }

    #[test]
    fn compute_with_no_effects_returns_base() {
        let base = base_creature();
        let out = compute(&base, &[], &Counters::new(), Status { tapped: false });
        assert_eq!(out.pt, Some(Pt { power: 2, toughness: 2 }));
        assert!(out.types.is_creature());
        assert!(out.colors.has(Color::Green));
    }

    #[test]
    fn layer_of_maps_effects() {
        assert_eq!(layer_of(&Effect::AddType(TypeSet::none())), Layer::Type);
        assert_eq!(layer_of(&Effect::SetColors(ColorSet::colorless())), Layer::Color);
        assert_eq!(layer_of(&Effect::AddAbility(Ability::Flying)), Layer::Ability);
        assert_eq!(layer_of(&Effect::RemoveAllAbilities), Layer::Ability);
        assert_eq!(layer_of(&Effect::SetBasePt(Pt { power: 1, toughness: 1 })), Layer::Pt);
        assert_eq!(layer_of(&Effect::ModifyPt { power: 1, toughness: 1 }), Layer::Pt);
        assert_eq!(layer_of(&Effect::SwitchPt), Layer::Pt);
    }
}

#[cfg(test)]
mod layer_6_tests {
    use super::*;
    use crate::ids::Timestamp;

    fn vanilla() -> BaseChars {
        BaseChars {
            pt: Some(Pt { power: 2, toughness: 2 }),
            types: TypeSet::none().with_creature(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        }
    }

    #[test]
    fn add_ability_layer_6() {
        let base = vanilla();
        let effects = vec![ContinuousEffect {
            effect: Effect::AddAbility(Ability::Flying),
            timestamp: Timestamp(1),
            is_cda: false,
        }];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert!(out.abilities.has(Ability::Flying));
    }

    #[test]
    fn add_then_remove_all_by_timestamp_f6() {
        // CR 613.1f, 613.3: grant flying (ts1), then remove all abilities (ts2) -> no flying.
        let base = vanilla();
        let effects = vec![
            ContinuousEffect {
                effect: Effect::AddAbility(Ability::Flying),
                timestamp: Timestamp(1),
                is_cda: false,
            },
            ContinuousEffect {
                effect: Effect::RemoveAllAbilities,
                timestamp: Timestamp(2),
                is_cda: false,
            },
        ];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert!(!out.abilities.has(Ability::Flying), "later remove-all wins");
    }

    #[test]
    fn remove_all_then_add_by_timestamp_f6_reverse() {
        // Reverse timestamps: remove all (ts1) then grant flying (ts2) -> flying present.
        let base = vanilla();
        let effects = vec![
            ContinuousEffect {
                effect: Effect::RemoveAllAbilities,
                timestamp: Timestamp(1),
                is_cda: false,
            },
            ContinuousEffect {
                effect: Effect::AddAbility(Ability::Flying),
                timestamp: Timestamp(2),
                is_cda: false,
            },
        ];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert!(out.abilities.has(Ability::Flying), "later add wins");
    }
}

#[cfg(test)]
mod layer_4_5_tests {
    use super::*;
    use crate::ids::Timestamp;

    fn land_base() -> BaseChars {
        BaseChars {
            pt: None,
            types: TypeSet::none().with_land(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        }
    }

    #[test]
    fn animate_land_adds_creature_type_layer_4() {
        // CR 613.1d layer 4: type-changing. "Target land becomes a creature."
        let base = land_base();
        let effects = vec![ContinuousEffect {
            effect: Effect::AddType(TypeSet::none().with_creature()),
            timestamp: Timestamp(1),
            is_cda: false,
        }];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert!(out.types.is_creature(), "creature type added");
        assert!(out.types.is_land(), "still a land");
    }

    #[test]
    fn color_set_last_timestamp_wins_layer_5() {
        // CR 613.1e layer 5, CR 613.7 timestamp order within a layer.
        let base = land_base();
        let effects = vec![
            ContinuousEffect {
                effect: Effect::SetColors(ColorSet::colorless().with(Color::White)),
                timestamp: Timestamp(1),
                is_cda: false,
            },
            ContinuousEffect {
                effect: Effect::SetColors(ColorSet::colorless().with(Color::Blue)),
                timestamp: Timestamp(2),
                is_cda: false,
            },
        ];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert!(out.colors.has(Color::Blue), "later timestamp wins");
        assert!(!out.colors.has(Color::White), "earlier set overwritten");
    }
}

#[cfg(test)]
mod layer_7_tests {
    use super::*;
    use crate::ids::Timestamp;

    fn base_3_3() -> BaseChars {
        BaseChars {
            pt: Some(Pt { power: 3, toughness: 3 }),
            types: TypeSet::none().with_creature(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        }
    }

    #[test]
    fn set_7b_before_modify_7c_f3() {
        // CR 613.4b/c: base 3/3, set to 1/1 (7b), +1/+1 counter (7c) -> 2/2,
        // regardless of timestamp. The set must NOT erase the later counter.
        let base = base_3_3();
        let effects = vec![ContinuousEffect {
            effect: Effect::SetBasePt(Pt { power: 1, toughness: 1 }),
            timestamp: Timestamp(5), // later than the counter conceptually; must not matter
            is_cda: false,
        }];
        let counters = Counters::new().with_plus1(1);
        let out = compute(&base, &effects, &counters, Status { tapped: false });
        assert_eq!(out.pt, Some(Pt { power: 2, toughness: 2 }));
    }

    #[test]
    fn modify_7c_stacks_on_base() {
        // base 3/3, +2/+2 modify, +1/+1 counter -> 6/6.
        let base = base_3_3();
        let effects = vec![ContinuousEffect {
            effect: Effect::ModifyPt { power: 2, toughness: 2 },
            timestamp: Timestamp(1),
            is_cda: false,
        }];
        let counters = Counters::new().with_plus1(1);
        let out = compute(&base, &effects, &counters, Status { tapped: false });
        assert_eq!(out.pt, Some(Pt { power: 6, toughness: 6 }));
    }

    #[test]
    fn switch_7d_is_last_f11() {
        // CR 613.4d: 1/4, +2/+0 (7c) -> 3/4, then switch -> 4/3.
        let base = BaseChars {
            pt: Some(Pt { power: 1, toughness: 4 }),
            types: TypeSet::none().with_creature(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        };
        let effects = vec![
            ContinuousEffect {
                effect: Effect::ModifyPt { power: 2, toughness: 0 },
                timestamp: Timestamp(1),
                is_cda: false,
            },
            ContinuousEffect {
                effect: Effect::SwitchPt,
                timestamp: Timestamp(2),
                is_cda: false,
            },
        ];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert_eq!(out.pt, Some(Pt { power: 4, toughness: 3 }));
    }

    #[test]
    fn negative_toughness_is_not_clamped() {
        // compute() returns raw P/T; 0-toughness death is an SBA handled elsewhere.
        // A -3/-3 modify on a 1/1 yields -2/-2 and must NOT be clamped to 0.
        let base = BaseChars {
            pt: Some(Pt { power: 1, toughness: 1 }),
            types: TypeSet::none().with_creature(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        };
        let effects = vec![ContinuousEffect {
            effect: Effect::ModifyPt { power: -3, toughness: -3 },
            timestamp: crate::ids::Timestamp(1),
            is_cda: false,
        }];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert_eq!(out.pt, Some(Pt { power: -2, toughness: -2 }));
    }

    #[test]
    fn cda_pt_7a_before_set_7b_f4() {
        // CR 613.4a/b: a CDA defining P/T applies in 7a, then a non-CDA set in 7b overrides.
        let base = BaseChars {
            pt: Some(Pt { power: 0, toughness: 0 }),
            types: TypeSet::none().with_creature(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        };
        let effects = vec![
            // CDA sets 5/5 in 7a
            ContinuousEffect {
                effect: Effect::SetBasePt(Pt { power: 5, toughness: 5 }),
                timestamp: Timestamp(1),
                is_cda: true,
            },
            // non-CDA set 1/1 in 7b overrides the CDA
            ContinuousEffect {
                effect: Effect::SetBasePt(Pt { power: 1, toughness: 1 }),
                timestamp: Timestamp(2),
                is_cda: false,
            },
        ];
        let out = compute(&base, &effects, &Counters::new(), Status { tapped: false });
        assert_eq!(out.pt, Some(Pt { power: 1, toughness: 1 }), "7b set overrides 7a CDA");
    }
}
