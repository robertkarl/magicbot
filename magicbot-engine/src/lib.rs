//! magicbot-engine: a training-grade MTG game-state core.
//! This slice provides the copy-on-write GameState harness only — no rules.

pub mod ids;
pub mod object;
pub mod zones;
pub mod state;
pub mod layers;
pub mod carddb;
pub mod events;
pub mod sba;

#[cfg(test)]
mod smoke {
    #[test]
    fn crate_links_and_im_is_available() {
        let v: im::Vector<u32> = im::Vector::new();
        assert_eq!(v.len(), 0);
    }
}
