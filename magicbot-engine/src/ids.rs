#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct ObjectId(pub u64);

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct PlayerId(pub u8);

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Timestamp(pub u64);

/// Monotonic, non-reusing id source. CR 400.7: every zone change mints a NEW id.
#[derive(Clone, Debug)]
pub struct IdMinter {
    next: u64,
}

impl IdMinter {
    pub fn new() -> Self {
        IdMinter { next: 1 }
    }

    pub fn mint(&mut self) -> ObjectId {
        let id = ObjectId(self.next);
        self.next += 1;
        id
    }
}

impl Default for IdMinter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minter_is_monotonic_and_never_repeats() {
        let mut m = IdMinter::new();
        let a = m.mint();
        let b = m.mint();
        let c = m.mint();
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert!(a < b && b < c, "ids must be strictly increasing");
    }

    #[test]
    fn ids_are_copy_and_hashable() {
        use std::collections::HashSet;
        let mut m = IdMinter::new();
        let id = m.mint();
        let copy = id; // requires Copy
        let mut set = HashSet::new();
        set.insert(id);
        assert!(set.contains(&copy));
    }
}
