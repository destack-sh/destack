use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use super::ReferenceMap;

/// One interned managed reference-map identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ReferenceMapId(u32);

impl ReferenceMapId {
    /// Create one managed reference-map identifier.
    pub(crate) const fn new(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the reference-map table index for this identifier.
    pub(crate) const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Interned managed reference maps for one heap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReferenceMapTable {
    /// Interned reference maps.
    maps: Vec<ReferenceMap>,
    /// Open-addressed reverse lookup keyed by reference-map id.
    map_slots: Vec<u32>,
}

/// The sentinel for one empty reverse-lookup slot.
const EMPTY_REFERENCE_MAP_SLOT: u32 = u32::MAX;

impl Default for ReferenceMapTable {
    fn default() -> Self {
        Self::new()
    }
}

impl ReferenceMapTable {
    /// Create one managed reference-map table with the empty map pre-interned.
    pub(crate) fn new() -> Self {
        Self::from_maps(vec![ReferenceMap::empty()])
    }

    /// Restore one managed reference-map table from one flattened map list.
    pub(crate) fn from_maps(maps: Vec<ReferenceMap>) -> Self {
        let maps = if maps.is_empty() {
            vec![ReferenceMap::empty()]
        } else {
            maps
        };
        let mut table = Self {
            maps,
            map_slots: Vec::new(),
        };

        table.rebuild_map_slots();

        table
    }

    /// Intern one reference map and return its stable identifier.
    pub(crate) fn intern(&mut self, map: ReferenceMap) -> (ReferenceMapId, i64) {
        if self.map_slots.is_empty() {
            self.rebuild_map_slots();
        }

        if let Ok(map_id) = self.lookup(&map) {
            return (map_id, 0);
        }

        if self.needs_rehash() {
            self.rebuild_map_slots_for_count(self.maps.len().saturating_add(1));
        }

        let map_id = ReferenceMapId::new(self.maps.len());
        self.maps.push(map);
        let map = &self.maps[map_id.index()];
        let Err(slot_index) = self.lookup(map) else {
            unreachable!("newly interned reference map must not already exist");
        };
        self.map_slots[slot_index] = map_id.index() as u32;

        (map_id, 0)
    }

    /// Return the flattened reference maps.
    pub(crate) fn snapshot(&self) -> Vec<ReferenceMap> {
        self.maps.clone()
    }

    /// Return one interned reference map by id.
    pub(crate) fn map(&self, map_id: ReferenceMapId) -> Option<&ReferenceMap> {
        self.maps.get(map_id.index())
    }

    /// Return whether the reverse lookup should grow before one more insert.
    fn needs_rehash(&self) -> bool {
        self.map_slots.is_empty()
            || self.maps.len().saturating_add(1) * 4 > self.map_slots.len() * 3
    }

    /// Rebuild the reverse lookup for the current trace set.
    fn rebuild_map_slots(&mut self) {
        self.rebuild_map_slots_for_count(self.maps.len());
    }

    /// Rebuild the reverse lookup for the given target trace count.
    fn rebuild_map_slots_for_count(&mut self, count: usize) {
        let slot_count = count.saturating_mul(2).max(8).next_power_of_two();
        self.map_slots = vec![EMPTY_REFERENCE_MAP_SLOT; slot_count];

        for (index, map) in self.maps.iter().enumerate() {
            let Err(slot_index) = self.lookup_in_slots(map, &self.map_slots) else {
                unreachable!("reference-map table rebuild must not duplicate entries");
            };
            self.map_slots[slot_index] = index as u32;
        }
    }

    /// Return one interned identifier or one insertion slot for this reference map.
    fn lookup(&self, map: &ReferenceMap) -> Result<ReferenceMapId, usize> {
        self.lookup_in_slots(map, &self.map_slots)
            .map(|map_id| ReferenceMapId::new(map_id as usize))
    }

    /// Probe one reverse-lookup table for this reference map.
    fn lookup_in_slots(&self, map: &ReferenceMap, slots: &[u32]) -> Result<u32, usize> {
        let mask = slots.len() - 1;
        let mut slot_index = (Self::hash_reference_map(map) as usize) & mask;

        loop {
            let map_index = slots[slot_index];
            if map_index == EMPTY_REFERENCE_MAP_SLOT {
                return Err(slot_index);
            }

            if self.maps[map_index as usize] == *map {
                return Ok(map_index);
            }

            slot_index = (slot_index + 1) & mask;
        }
    }

    /// Hash one managed reference map for the reverse lookup.
    fn hash_reference_map(map: &ReferenceMap) -> u64 {
        let mut hasher = DefaultHasher::new();
        map.hash(&mut hasher);
        hasher.finish()
    }
}
