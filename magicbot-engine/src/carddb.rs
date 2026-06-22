//! Static CardDb: printed card definitions as data. Produces the layer-1
//! BaseChars and static continuous effects that layers::compute() consumes.
//! This is shared `&'static`-style data — it is NOT part of the forked GameState.

use crate::layers::{Ability, Color, ColorSet, Pt, TypeSet};
use std::collections::HashMap;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct DefId(pub u64);

/// A single data-driven static continuous ability. Expressed as the layer
/// effect it produces; the layer engine (layers::compute) applies it.
#[derive(Clone, Debug)]
pub enum StaticAbility {
    /// "Other creatures you control get +power/+toughness." (Glorious Anthem.)
    AnthemOtherCreatures { power: i32, toughness: i32 },
}

/// Printed characteristics of a card, as data. Keyed on `def_id` (oracle_id).
#[derive(Clone, Debug)]
pub struct CardDef {
    def_id: DefId,
    name: String,
    mana_value: u8,
    types: TypeSet,
    colors: ColorSet,
    printed_pt: Option<Pt>,
    keywords: Vec<Ability>,
    statics: Vec<StaticAbility>,
}

impl CardDef {
    pub fn new(def_id: u64, name: &str, mana_value: u8) -> CardDef {
        CardDef {
            def_id: DefId(def_id),
            name: name.to_string(),
            mana_value,
            types: TypeSet::none(),
            colors: ColorSet::colorless(),
            printed_pt: None,
            keywords: Vec::new(),
            statics: Vec::new(),
        }
    }

    pub fn with_types(mut self, t: TypeSet) -> Self {
        self.types = t;
        self
    }
    pub fn with_colors(mut self, c: ColorSet) -> Self {
        self.colors = c;
        self
    }
    pub fn with_pt(mut self, power: i32, toughness: i32) -> Self {
        self.printed_pt = Some(Pt { power, toughness });
        self
    }
    pub fn with_keyword(mut self, a: Ability) -> Self {
        self.keywords.push(a);
        self
    }
    pub fn with_static(mut self, s: StaticAbility) -> Self {
        self.statics.push(s);
        self
    }

    /// The layer-1 copiable base (CR 613.2c) for this card: printed
    /// characteristics folded into the form layers::compute() consumes.
    pub fn base_chars(&self) -> crate::layers::BaseChars {
        let mut abilities = crate::layers::AbilitySet::new();
        for &kw in &self.keywords {
            abilities.add(kw);
        }
        crate::layers::BaseChars {
            pt: self.printed_pt,
            types: self.types,
            colors: self.colors,
            abilities,
        }
    }

    /// The layer-system effect bodies produced by this card's static abilities.
    /// The affected-set scoping ("other creatures you control") is the layer
    /// engine's responsibility once wired into GameState (a later plan); this
    /// produces the effect body only.
    pub fn static_effects(&self) -> Vec<crate::layers::Effect> {
        self.statics
            .iter()
            .map(|s| match s {
                StaticAbility::AnthemOtherCreatures { power, toughness } => {
                    crate::layers::Effect::ModifyPt { power: *power, toughness: *toughness }
                }
            })
            .collect()
    }

    pub fn def_id(&self) -> DefId {
        self.def_id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn mana_value(&self) -> u8 {
        self.mana_value
    }
}

/// The static card database. Shared, not forked — a plain HashMap is correct
/// because this is never on the GameState copy-on-write path.
#[derive(Clone, Debug)]
pub struct CardDb {
    defs: HashMap<DefId, CardDef>,
}

impl CardDb {
    pub fn new() -> Self {
        CardDb { defs: HashMap::new() }
    }
    pub fn insert(&mut self, def: CardDef) {
        self.defs.insert(def.def_id(), def);
    }
    pub fn get(&self, id: DefId) -> Option<&CardDef> {
        self.defs.get(&id)
    }
    pub fn len(&self) -> usize {
        self.defs.len()
    }
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }
}

impl Default for CardDb {
    fn default() -> Self {
        Self::new()
    }
}

/// A small hand-authored frozen pool for engine bring-up. Real Scryfall/MTGJSON
/// ingest is a later plan.
pub fn sample_pool() -> CardDb {
    let mut db = CardDb::new();
    db.insert(
        CardDef::new(1, "Grizzly Bears", 2)
            .with_types(TypeSet::none().with_creature())
            .with_colors(ColorSet::colorless().with(Color::Green))
            .with_pt(2, 2),
    );
    db.insert(
        CardDef::new(2, "Serra Angel", 5)
            .with_types(TypeSet::none().with_creature())
            .with_colors(ColorSet::colorless().with(Color::White))
            .with_pt(4, 4)
            .with_keyword(Ability::Flying)
            .with_keyword(Ability::Vigilance),
    );
    db.insert(
        CardDef::new(3, "Glorious Anthem", 3)
            .with_types(TypeSet::none().with_enchantment())
            .with_colors(ColorSet::colorless().with(Color::White))
            .with_static(StaticAbility::AnthemOtherCreatures { power: 1, toughness: 1 }),
    );
    db.insert(
        CardDef::new(4, "Forest", 0).with_types(TypeSet::none().with_land()),
    );
    db.insert(
        CardDef::new(5, "Plains", 0).with_types(TypeSet::none().with_land()),
    );
    db
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carddb_insert_and_get() {
        let mut db = CardDb::new();
        db.insert(CardDef::new(1, "Grizzly Bears", 2).with_pt(2, 2));
        assert_eq!(db.len(), 1);
        assert_eq!(db.get(DefId(1)).unwrap().name(), "Grizzly Bears");
        assert!(db.get(DefId(999)).is_none());
    }

    #[test]
    fn sample_pool_has_the_expected_cards() {
        let db = sample_pool();
        // 5 cards: bears, serra, anthem, forest, plains
        assert_eq!(db.len(), 5);
        // Serra Angel is a 4/4 flyer.
        let serra = db
            .defs
            .values()
            .find(|d| d.name() == "Serra Angel")
            .expect("serra present");
        let base = serra.base_chars();
        assert_eq!(base.pt, Some(Pt { power: 4, toughness: 4 }));
        assert!(base.abilities.has(Ability::Flying));
    }

    #[test]
    fn builds_a_creature_def() {
        let d = CardDef::new(1, "Grizzly Bears", 2)
            .with_types(TypeSet::none().with_creature())
            .with_colors(ColorSet::colorless().with(Color::Green))
            .with_pt(2, 2);
        assert_eq!(d.name(), "Grizzly Bears");
        assert_eq!(d.mana_value(), 2);
        assert_eq!(d.def_id(), DefId(1));
    }

    #[test]
    fn base_chars_reflects_printed_values() {
        use crate::layers::{compute, Counters, Status};

        let serra = CardDef::new(2, "Serra Angel", 5)
            .with_types(TypeSet::none().with_creature())
            .with_colors(ColorSet::colorless().with(Color::White))
            .with_pt(4, 4)
            .with_keyword(Ability::Flying)
            .with_keyword(Ability::Vigilance);

        let base = serra.base_chars();
        assert_eq!(base.pt, Some(Pt { power: 4, toughness: 4 }));
        assert!(base.types.is_creature());
        assert!(base.colors.has(Color::White));
        assert!(base.abilities.has(Ability::Flying));
        assert!(base.abilities.has(Ability::Vigilance));

        // And it flows through the layer engine unchanged with no effects.
        let out = compute(&base, &[], &Counters::new(), Status { tapped: false });
        assert_eq!(out.pt, Some(Pt { power: 4, toughness: 4 }));
        assert!(out.abilities.has(Ability::Flying));
    }

    #[test]
    fn anthem_static_effect_translates_to_modify_pt() {
        use crate::layers::Effect;

        let anthem = CardDef::new(3, "Glorious Anthem", 3)
            .with_static(StaticAbility::AnthemOtherCreatures { power: 1, toughness: 1 });
        let effects = anthem.static_effects();
        assert_eq!(effects.len(), 1);
        match &effects[0] {
            Effect::ModifyPt { power, toughness } => {
                assert_eq!(*power, 1);
                assert_eq!(*toughness, 1);
            }
            other => panic!("expected ModifyPt, got {other:?}"),
        }
    }

    #[test]
    fn builds_a_def_with_keyword_and_static() {
        let d = CardDef::new(2, "Serra Angel", 5)
            .with_types(TypeSet::none().with_creature())
            .with_pt(4, 4)
            .with_keyword(Ability::Flying)
            .with_keyword(Ability::Vigilance);
        assert_eq!(d.name(), "Serra Angel");

        let anthem = CardDef::new(3, "Glorious Anthem", 3)
            .with_types(TypeSet::none().with_enchantment())
            .with_static(StaticAbility::AnthemOtherCreatures { power: 1, toughness: 1 });
        assert_eq!(anthem.mana_value(), 3);
    }
}
