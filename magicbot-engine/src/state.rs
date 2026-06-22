use crate::ids::{IdMinter, ObjectId, PlayerId};
use crate::object::{Object, Zone};
use crate::zones::Zones;

/// The whole game. Cloning this IS the fork — cheap by construction because every
/// field is either Copy or an Arc-backed persistent container. CR 400.7's "new
/// object per zone change" is served by `minter`.
#[derive(Clone, Debug)]
pub struct GameState {
    objects: im::HashMap<ObjectId, Object>,
    zones: Zones,
    #[allow(dead_code)]
    stack: im::Vector<ObjectId>,
    minter: IdMinter,
    #[allow(dead_code)]
    n_players: usize,
    #[allow(dead_code)]
    rng_seed: u64,
}

impl GameState {
    pub fn new(n_players: usize, rng_seed: u64) -> Self {
        GameState {
            objects: im::HashMap::new(),
            zones: Zones::new(n_players),
            stack: im::Vector::new(),
            minter: IdMinter::new(),
            n_players,
            rng_seed,
        }
    }

    /// The constant-time copy-on-write fork. Clones a handful of Arc roots.
    pub fn fork(&self) -> GameState {
        self.clone()
    }

    pub fn create_object(&mut self, owner: PlayerId, zone: Zone, def_id: u64) -> ObjectId {
        let id = self.minter.mint();
        let obj = Object::new(id, owner, zone, def_id);
        self.objects.insert(id, obj);
        self.zones.insert(zone, owner, id);
        id
    }

    pub fn get(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(&id)
    }

    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        self.objects.get_mut(&id)
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn battlefield_len(&self) -> usize {
        self.zones.battlefield_len()
    }

    /// The layer-1 printed base for an object, resolved through the CardDb.
    /// `None` if the object or its definition is unknown. CardDb is passed in —
    /// it is static shared data, never stored on GameState.
    pub fn base_chars_of(
        &self,
        id: ObjectId,
        db: &crate::carddb::CardDb,
    ) -> Option<crate::layers::BaseChars> {
        let obj = self.objects.get(&id)?;
        let def = db.get(crate::carddb::DefId(obj.def_id()))?;
        Some(def.base_chars())
    }
}

#[cfg(test)]
mod characteristics_tests {
    use super::*;
    use crate::carddb::sample_pool;
    use crate::ids::PlayerId;
    use crate::layers::Pt;
    use crate::object::Zone;

    #[test]
    fn base_chars_of_resolves_printed_pt() {
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        // Grizzly Bears is DefId(1), a 2/2.
        let id = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        let base = s.base_chars_of(id, &db).expect("base chars");
        assert_eq!(base.pt, Some(Pt { power: 2, toughness: 2 }));
        assert!(base.types.is_creature());
    }

    #[test]
    fn base_chars_of_unknown_object_is_none() {
        let db = sample_pool();
        let s = GameState::new(2, 0);
        assert!(s.base_chars_of(crate::ids::ObjectId(999), &db).is_none());
    }

    #[test]
    fn base_chars_of_unknown_def_is_none() {
        let db = sample_pool();
        let mut s = GameState::new(2, 0);
        // def_id 777 is not in sample_pool.
        let id = s.create_object(PlayerId(0), Zone::Battlefield, 777);
        assert!(s.base_chars_of(id, &db).is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::object::Zone;

    #[test]
    fn create_object_lands_in_table_and_zone() {
        let mut s = GameState::new(2, 0);
        let id = s.create_object(PlayerId(0), Zone::Battlefield, 1);
        assert_eq!(s.object_count(), 1);
        assert_eq!(s.battlefield_len(), 1);
        assert_eq!(s.get(id).unwrap().zone(), Zone::Battlefield);
    }

    #[test]
    fn fork_is_independent_parent_unchanged_after_child_mutation() {
        let mut parent = GameState::new(2, 0);
        let id = parent.create_object(PlayerId(0), Zone::Battlefield, 1);

        let mut child = parent.fork();
        child.get_mut(id).unwrap().tap();
        child.create_object(PlayerId(1), Zone::Battlefield, 2);

        // child changed
        assert!(child.get(id).unwrap().is_tapped());
        assert_eq!(child.object_count(), 2);

        // parent did NOT change
        assert!(!parent.get(id).unwrap().is_tapped());
        assert_eq!(parent.object_count(), 1);
    }
}
