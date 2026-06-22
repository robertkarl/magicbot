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

fn effects_in_layer(
    effects: &[ContinuousEffect],
    layer: Layer,
) -> Vec<&ContinuousEffect> {
    let mut v: Vec<&ContinuousEffect> =
        effects.iter().filter(|e| layer_of(&e.effect) == layer).collect();
    // CR 613.7: ascending timestamp order within a layer.
    // CR 613.8 dependency ordering is DEFERRED (stub: timestamp only).
    v.sort_by_key(|e| e.timestamp.0);
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
    let abilities = base.abilities.clone();
    let pt = base.pt;

    // Layer 4: type-changing (OR new types in).
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

    // Layers 6 and 7 added in later tasks.
    let _ = counters;

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
