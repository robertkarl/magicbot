//! State-based actions (CR 704). This slice: the creature-death SBAs
//! (704.5f zero toughness, 704.5g lethal marked damage). The loop that performs
//! them lives on GameState; this module holds the pure decision predicate.

use crate::layers;

/// Whether a creature should be put into its owner's graveyard by a state-based
/// action this slice models: CR 704.5f (toughness <= 0) or CR 704.5g (marked
/// damage >= toughness, for toughness > 0). Noncreatures never match.
pub fn creature_dies_to_sba(current: &layers::Chars, marked_damage: u32) -> bool {
    if !current.types.is_creature() {
        return false;
    }
    let pt = match current.pt {
        Some(pt) => pt,
        None => return false,
    };
    if pt.toughness <= 0 {
        // CR 704.5f
        return true;
    }
    // CR 704.5g: toughness > 0 here; lethal if marked damage meets/exceeds it.
    marked_damage >= pt.toughness as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::{Ability, AbilitySet, ColorSet, Pt, TypeSet};

    fn creature_chars(power: i32, toughness: i32) -> layers::Chars {
        layers::Chars {
            pt: Some(Pt { power, toughness }),
            types: TypeSet::none().with_creature(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        }
    }

    fn noncreature_chars() -> layers::Chars {
        layers::Chars {
            pt: None,
            types: TypeSet::none().with_enchantment(),
            colors: ColorSet::colorless(),
            abilities: AbilitySet::new(),
        }
    }

    #[test]
    fn zero_toughness_dies_704_5f() {
        assert!(creature_dies_to_sba(&creature_chars(2, 0), 0));
        assert!(creature_dies_to_sba(&creature_chars(2, -1), 0));
    }

    #[test]
    fn lethal_marked_damage_dies_704_5g() {
        // 2 toughness, 2 marked damage -> dies.
        assert!(creature_dies_to_sba(&creature_chars(2, 2), 2));
        // 3 toughness, 2 marked damage -> survives.
        assert!(!creature_dies_to_sba(&creature_chars(2, 3), 2));
    }

    #[test]
    fn healthy_creature_survives() {
        assert!(!creature_dies_to_sba(&creature_chars(2, 2), 0));
        assert!(!creature_dies_to_sba(&creature_chars(2, 2), 1));
    }

    #[test]
    fn noncreature_never_dies_to_these_sbas() {
        assert!(!creature_dies_to_sba(&noncreature_chars(), 0));
        assert!(!creature_dies_to_sba(&noncreature_chars(), 99));
    }

    #[test]
    fn keyword_does_not_save_from_lethal_in_this_slice() {
        // (no deathtouch/indestructible modeled yet; a plain creature with lethal dies)
        let mut c = creature_chars(1, 1);
        c.abilities.add(Ability::Vigilance);
        assert!(creature_dies_to_sba(&c, 1));
    }
}
