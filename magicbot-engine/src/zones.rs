use crate::ids::{ObjectId, PlayerId};
use crate::object::Zone;

/// Zones (CR 400). Library/Hand/Graveyard are per-player; Battlefield/Exile/Command
/// shared. Stack lives on GameState (it is a single LIFO). Ordering modeled where
/// CR 400.5 fixes it (library, graveyard); battlefield is an unordered set.
#[derive(Clone, Debug)]
pub struct Zones {
    libraries: im::Vector<im::Vector<ObjectId>>,
    hands: im::Vector<im::Vector<ObjectId>>,
    graveyards: im::Vector<im::Vector<ObjectId>>,
    battlefield: im::HashSet<ObjectId>,
    exile: im::Vector<ObjectId>,
    command: im::Vector<ObjectId>,
}

impl Zones {
    pub fn new(n_players: usize) -> Self {
        let empty_per_player =
            im::Vector::from(vec![im::Vector::<ObjectId>::new(); n_players]);
        Zones {
            libraries: empty_per_player.clone(),
            hands: empty_per_player.clone(),
            graveyards: empty_per_player,
            battlefield: im::HashSet::new(),
            exile: im::Vector::new(),
            command: im::Vector::new(),
        }
    }

    fn per_player_mut(
        &mut self,
        z: Zone,
        owner: PlayerId,
    ) -> Option<&mut im::Vector<ObjectId>> {
        let idx = owner.0 as usize;
        match z {
            Zone::Library => self.libraries.get_mut(idx),
            Zone::Hand => self.hands.get_mut(idx),
            Zone::Graveyard => self.graveyards.get_mut(idx),
            _ => None,
        }
    }

    pub fn insert(&mut self, z: Zone, owner: PlayerId, id: ObjectId) {
        match z {
            Zone::Battlefield => {
                self.battlefield.insert(id);
            }
            Zone::Exile => self.exile.push_back(id),
            Zone::Command => self.command.push_back(id),
            Zone::Stack => { /* stack lives on GameState; ignored here */ }
            _ => {
                if let Some(v) = self.per_player_mut(z, owner) {
                    v.push_back(id);
                }
            }
        }
    }

    pub fn remove(&mut self, z: Zone, owner: PlayerId, id: ObjectId) -> bool {
        match z {
            Zone::Battlefield => self.battlefield.remove(&id).is_some(),
            Zone::Exile => remove_from_vec(&mut self.exile, id),
            Zone::Command => remove_from_vec(&mut self.command, id),
            Zone::Stack => false,
            _ => self
                .per_player_mut(z, owner)
                .map(|v| remove_from_vec(v, id))
                .unwrap_or(false),
        }
    }

    pub fn contains(&self, z: Zone, owner: PlayerId, id: ObjectId) -> bool {
        let idx = owner.0 as usize;
        match z {
            Zone::Battlefield => self.battlefield.contains(&id),
            Zone::Exile => self.exile.iter().any(|&x| x == id),
            Zone::Command => self.command.iter().any(|&x| x == id),
            Zone::Stack => false,
            Zone::Library => contains_pp(&self.libraries, idx, id),
            Zone::Hand => contains_pp(&self.hands, idx, id),
            Zone::Graveyard => contains_pp(&self.graveyards, idx, id),
        }
    }

    pub fn battlefield_len(&self) -> usize {
        self.battlefield.len()
    }
}

// NOTE: im::Vector::remove(pos) returns the removed T directly and PANICS if pos
// is out of bounds (unlike HashMap/HashSet remove which return Option). The
// position() guard guarantees pos is valid, so the bare remove is safe.
fn remove_from_vec(v: &mut im::Vector<ObjectId>, id: ObjectId) -> bool {
    if let Some(pos) = v.iter().position(|&x| x == id) {
        v.remove(pos);
        true
    } else {
        false
    }
}

fn contains_pp(vv: &im::Vector<im::Vector<ObjectId>>, idx: usize, id: ObjectId) -> bool {
    vv.get(idx).map(|v| v.iter().any(|&x| x == id)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};
    use crate::object::Zone;

    #[test]
    fn insert_and_contains_battlefield() {
        let mut z = Zones::new(2);
        let id = ObjectId(7);
        z.insert(Zone::Battlefield, PlayerId(0), id);
        assert!(z.contains(Zone::Battlefield, PlayerId(0), id));
        assert_eq!(z.battlefield_len(), 1);
    }

    #[test]
    fn library_is_per_player_and_ordered() {
        let mut z = Zones::new(2);
        z.insert(Zone::Library, PlayerId(1), ObjectId(1));
        z.insert(Zone::Library, PlayerId(1), ObjectId(2));
        assert!(z.contains(Zone::Library, PlayerId(1), ObjectId(2)));
        // player 0's library is untouched
        assert!(!z.contains(Zone::Library, PlayerId(0), ObjectId(1)));
    }

    #[test]
    fn remove_returns_presence() {
        let mut z = Zones::new(1);
        z.insert(Zone::Graveyard, PlayerId(0), ObjectId(5));
        assert!(z.remove(Zone::Graveyard, PlayerId(0), ObjectId(5)));
        assert!(!z.remove(Zone::Graveyard, PlayerId(0), ObjectId(5)));
    }
}
