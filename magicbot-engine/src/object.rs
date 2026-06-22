use crate::ids::{ObjectId, PlayerId, Timestamp};

/// The seven zones (CR 400.1). Owner/shared distinction lives in `Zones`, not here.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Zone {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Stack,
    Exile,
    Command,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum CounterKind {
    PlusOnePlusOne,
    MinusOneMinusOne,
    Loyalty,
    Other(u16),
}

/// Per-object mutable state. Persistent containers so cloning an Object is cheap.
/// This is the data CR 400.7 drops on a zone change (handled in a later plan).
#[derive(Clone, Debug)]
pub struct InstanceState {
    tapped: bool,
    counters: im::HashMap<CounterKind, i32>,
    #[allow(dead_code)]
    marked_damage: u32,
    #[allow(dead_code)]
    attached_to: Option<ObjectId>,
    #[allow(dead_code)]
    attachments: im::Vector<ObjectId>,
    #[allow(dead_code)]
    entered_this_turn: bool,
}

impl InstanceState {
    pub fn new() -> Self {
        InstanceState {
            tapped: false,
            counters: im::HashMap::new(),
            marked_damage: 0,
            attached_to: None,
            attachments: im::Vector::new(),
            entered_this_turn: false,
        }
    }
}

impl Default for InstanceState {
    fn default() -> Self {
        Self::new()
    }
}

/// An inert game object. `def_id` is a placeholder index; there is no CardDb in
/// this slice. Current characteristics are NOT stored (a later plan computes them
/// via the layer system).
#[derive(Clone, Debug)]
pub struct Object {
    id: ObjectId,
    owner: PlayerId,
    controller: Option<PlayerId>,
    zone: Zone,
    def_id: u64,
    instance: InstanceState,
    #[allow(dead_code)]
    timestamp: Timestamp,
}

impl Object {
    pub fn new(id: ObjectId, owner: PlayerId, zone: Zone, def_id: u64) -> Self {
        Object {
            id,
            owner,
            controller: None,
            zone,
            def_id,
            instance: InstanceState::new(),
            timestamp: Timestamp(0),
        }
    }

    pub fn id(&self) -> ObjectId {
        self.id
    }
    pub fn owner(&self) -> PlayerId {
        self.owner
    }
    pub fn controller(&self) -> Option<PlayerId> {
        self.controller
    }
    pub fn zone(&self) -> Zone {
        self.zone
    }
    pub fn def_id(&self) -> u64 {
        self.def_id
    }

    pub fn set_zone(&mut self, z: Zone) {
        self.zone = z;
    }
    pub fn set_controller(&mut self, c: Option<PlayerId>) {
        self.controller = c;
    }

    pub fn tap(&mut self) {
        self.instance.tapped = true;
    }
    pub fn untap(&mut self) {
        self.instance.tapped = false;
    }
    pub fn is_tapped(&self) -> bool {
        self.instance.tapped
    }

    pub fn add_counter(&mut self, kind: CounterKind, n: i32) {
        let entry = self.instance.counters.entry(kind).or_insert(0);
        *entry += n;
    }
    pub fn counter_count(&self, kind: CounterKind) -> i32 {
        self.instance.counters.get(&kind).copied().unwrap_or(0)
    }

    /// Build the layer engine's Counters view from this object's +1/+1 and
    /// -1/-1 counters (the counters that affect P/T in layer 7c).
    pub fn layer_counters(&self) -> crate::layers::Counters {
        crate::layers::Counters::new()
            .with_plus1(self.counter_count(CounterKind::PlusOnePlusOne))
            .with_minus1(self.counter_count(CounterKind::MinusOneMinusOne))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{IdMinter, PlayerId};

    fn obj() -> Object {
        let mut m = IdMinter::new();
        Object::new(m.mint(), PlayerId(0), Zone::Battlefield, 42)
    }

    #[test]
    fn new_object_is_inert_and_untapped() {
        let o = obj();
        assert_eq!(o.zone(), Zone::Battlefield);
        assert_eq!(o.owner(), PlayerId(0));
        assert_eq!(o.controller(), None);
        assert!(!o.is_tapped());
        assert_eq!(o.counter_count(CounterKind::PlusOnePlusOne), 0);
    }

    #[test]
    fn tap_and_counters_mutate_instance_state() {
        let mut o = obj();
        o.tap();
        assert!(o.is_tapped());
        o.untap();
        assert!(!o.is_tapped());
        o.add_counter(CounterKind::PlusOnePlusOne, 2);
        o.add_counter(CounterKind::PlusOnePlusOne, 1);
        assert_eq!(o.counter_count(CounterKind::PlusOnePlusOne), 3);
    }

    #[test]
    fn layer_counters_reflects_plus_and_minus() {
        use crate::ids::{IdMinter, PlayerId};
        let mut m = IdMinter::new();
        let mut o = Object::new(m.mint(), PlayerId(0), Zone::Battlefield, 1);
        o.add_counter(CounterKind::PlusOnePlusOne, 3);
        o.add_counter(CounterKind::MinusOneMinusOne, 1);
        let c = o.layer_counters();
        // net +2/+2
        assert_eq!(c.net_pt(), (2, 2));
    }
}
