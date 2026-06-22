//! Static CardDb: printed card definitions as data. Produces the layer-1
//! BaseChars and static continuous effects that layers::compute() consumes.
//! This is shared `&'static`-style data — it is NOT part of the forked GameState.

use crate::layers::{Ability, ColorSet, Pt, TypeSet};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::Color;

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
