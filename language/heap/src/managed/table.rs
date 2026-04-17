use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use super::ReferenceMap;
use crate::{HeapError, HeapResult};

/// The sentinel for one empty reverse-lookup slot.
const EMPTY_REFERENCE_MAP_SLOT: u32 = u32::MAX;

/// One interned managed reference-map identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct MapId(u32);

impl MapId {
    /// The empty reference-map identifier.
    pub(crate) const EMPTY: Self = Self(0);

    /// Create one managed reference-map identifier.
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the empty reference-map identifier.
    pub(crate) const fn empty() -> Self {
        Self::EMPTY
    }

    /// Create one managed reference-map identifier from a table index.
    pub(crate) fn from_index(index: usize) -> HeapResult<Self> {
        if index >= EMPTY_REFERENCE_MAP_SLOT as usize {
            return Err(HeapError::InvalidMapId { index });
        }

        Ok(Self(index as u32))
    }

    /// Return the reference-map table index for this identifier.
    pub(crate) const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Interned managed reference maps for one heap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MapTable {
    /// Interned reference maps.
    maps: Vec<ReferenceMap>,
    /// Open-addressed reverse lookup keyed by reference-map id.
    map_slots: Vec<u32>,
}

impl MapTable {
    /// Create one managed reference-map table with the empty map pre-interned.
    pub(crate) fn new() -> HeapResult<Self> {
        Self::from_maps(vec![ReferenceMap::empty()])
    }

    /// Restore one managed reference-map table from one flattened map list.
    pub(crate) fn from_maps(maps: Vec<ReferenceMap>) -> HeapResult<Self> {
        let maps = if maps.is_empty() {
            vec![ReferenceMap::empty()]
        } else {
            maps
        };

        if maps[MapId::empty().index()] != ReferenceMap::empty() {
            return Err(HeapError::InvalidMapSentinel {
                index: MapId::empty().index(),
            });
        }

        let mut table = Self {
            maps,
            map_slots: Vec::new(),
        };

        table.rebuild_map_slots()?;

        Ok(table)
    }

    /// Intern one reference map and return its stable identifier.
    pub(crate) fn intern(&mut self, map: ReferenceMap) -> HeapResult<(MapId, i64)> {
        if self.map_slots.is_empty() {
            self.rebuild_map_slots()?;
        }

        if let Ok(map_id) = self.lookup(&map) {
            return Ok((map_id, 0));
        }

        if self.needs_rehash() {
            self.rebuild_map_slots_for_count(self.maps.len().saturating_add(1))?;
        }

        let slot_index = match self.lookup(&map) {
            Ok(map_id) => return Ok((map_id, 0)),
            Err(slot_index) => slot_index,
        };
        let map_id = MapId::from_index(self.maps.len())?;
        self.maps.push(map);
        self.map_slots[slot_index] = map_id.index() as u32;

        Ok((map_id, 0))
    }

    /// Return the flattened reference maps.
    pub(crate) fn snapshot(&self) -> Vec<ReferenceMap> {
        self.maps.clone()
    }

    /// Return one interned reference map by id.
    pub(crate) fn map(&self, map_id: MapId) -> Option<&ReferenceMap> {
        self.maps.get(map_id.index())
    }

    /// Return whether the reverse lookup should grow before one more insert.
    fn needs_rehash(&self) -> bool {
        self.map_slots.is_empty()
            || self.maps.len().saturating_add(1) * 4 > self.map_slots.len() * 3
    }

    /// Rebuild the reverse lookup for the current trace set.
    fn rebuild_map_slots(&mut self) -> HeapResult<()> {
        self.rebuild_map_slots_for_count(self.maps.len())
    }

    /// Rebuild the reverse lookup for the given target trace count.
    fn rebuild_map_slots_for_count(&mut self, count: usize) -> HeapResult<()> {
        let slot_count = count
            .checked_mul(2)
            .and_then(|count| count.max(8).checked_next_power_of_two())
            .ok_or(HeapError::InvalidMapTableLen { len: count })?;
        self.map_slots = vec![EMPTY_REFERENCE_MAP_SLOT; slot_count];

        for (index, map) in self.maps.iter().enumerate() {
            let map_id = MapId::from_index(index)?;
            let slot_index = match self.lookup_in_slots(map, &self.map_slots) {
                Ok(_) => return Err(HeapError::DuplicateMap { index }),
                Err(slot_index) => slot_index,
            };
            self.map_slots[slot_index] = map_id.0;
        }

        Ok(())
    }

    /// Return one interned identifier or one insertion slot for this reference map.
    fn lookup(&self, map: &ReferenceMap) -> Result<MapId, usize> {
        self.lookup_in_slots(map, &self.map_slots).map(MapId::new)
    }

    /// Probe one reverse-lookup table for this reference map.
    fn lookup_in_slots(&self, map: &ReferenceMap, slots: &[u32]) -> Result<u32, usize> {
        if slots.is_empty() {
            return Err(0);
        }

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
