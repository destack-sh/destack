use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use super::super::value::{ManagedReference, Value};

/// Reference-scanning metadata for one managed allocation payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceMap {
    /// Allocation contains no managed references.
    None,
    /// Allocation stores a packed array of VM values.
    ValueArray {
        /// The number of encoded VM values in this allocation.
        count: u32,
    },
    /// Allocation stores direct managed-reference words at fixed byte offsets.
    ReferenceOffsets {
        /// Byte offsets of encoded managed references.
        offsets: Vec<u32>,
    },
    /// Allocation stores repeated elements with managed-reference words at fixed element offsets.
    RepeatedReferenceOffsets {
        /// The number of elements in the allocation.
        count: u32,
        /// The element byte stride.
        element_size: u32,
        /// Managed-reference byte offsets within each element.
        offsets: Vec<u32>,
    },
}

impl ReferenceMap {
    /// Return an empty reference map.
    pub const fn empty() -> Self {
        Self::None
    }

    /// Return one reference map for a packed array of VM values.
    pub const fn value_array(count: usize) -> Self {
        Self::ValueArray {
            count: count as u32,
        }
    }

    /// Report whether this reference map can reach managed references.
    pub fn has_managed_edges(&self) -> bool {
        match self {
            Self::None => false,
            Self::ValueArray { count } => *count > 0,
            Self::ReferenceOffsets { offsets } => !offsets.is_empty(),
            Self::RepeatedReferenceOffsets { count, offsets, .. } => {
                *count > 0 && !offsets.is_empty()
            }
        }
    }

    /// Visit each managed reference stored in the given payload bytes.
    pub fn for_each_reference(&self, bytes: &[u8], mut visit: impl FnMut(ManagedReference)) {
        match self {
            Self::None => {}
            Self::ValueArray { count } => {
                let value_size = Value::BYTE_LEN;

                for index in 0..(*count as usize) {
                    let start = index.saturating_mul(value_size);
                    let end = start.saturating_add(value_size);
                    let Some(window) = bytes.get(start..end) else {
                        break;
                    };

                    let Some(value) = Value::from_byte_slice(window) else {
                        break;
                    };

                    if let Some(handle) = value.as_managed_reference() {
                        visit(handle);
                    }
                }
            }
            Self::ReferenceOffsets { offsets } => {
                for &offset in offsets {
                    let start = offset as usize;
                    let end = start.saturating_add(std::mem::size_of::<u64>());
                    let Some(window) = bytes.get(start..end) else {
                        continue;
                    };

                    let mut raw = [0u8; 8];
                    raw.copy_from_slice(window);
                    visit(ManagedReference::from_bits(u64::from_le_bytes(raw)));
                }
            }
            Self::RepeatedReferenceOffsets {
                count,
                element_size,
                offsets,
            } => {
                let element_size = *element_size as usize;

                for index in 0..(*count as usize) {
                    let base = index.saturating_mul(element_size);

                    for &offset in offsets {
                        let start = base.saturating_add(offset as usize);
                        let end = start.saturating_add(std::mem::size_of::<u64>());
                        let Some(window) = bytes.get(start..end) else {
                            continue;
                        };

                        let mut raw = [0u8; 8];
                        raw.copy_from_slice(window);
                        visit(ManagedReference::from_bits(u64::from_le_bytes(raw)));
                    }
                }
            }
        }
    }

    /// Return the retained bytes owned by this reference map.
    pub(crate) fn retained_bytes(&self) -> usize {
        match self {
            Self::None | Self::ValueArray { .. } => 0,
            Self::ReferenceOffsets { offsets } => offsets.capacity() * std::mem::size_of::<u32>(),
            Self::RepeatedReferenceOffsets { offsets, .. } => {
                offsets.capacity() * std::mem::size_of::<u32>()
            }
        }
    }
}

/// One interned managed reference-map identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReferenceMapId(u32);

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

    /// Return one borrowed reference map for the given identifier.
    pub(crate) fn get(&self, map_id: ReferenceMapId) -> Option<&ReferenceMap> {
        self.maps.get(map_id.index())
    }

    /// Intern one reference map and return its stable identifier.
    pub(crate) fn intern(&mut self, map: ReferenceMap) -> ReferenceMapId {
        if self.map_slots.is_empty() {
            self.rebuild_map_slots();
        }

        if let Ok(map_id) = self.lookup(&map) {
            return map_id;
        }

        if self.needs_rehash() {
            self.rebuild_map_slots_for_count(self.maps.len().saturating_add(1));
        }

        let map_id = ReferenceMapId::new(self.maps.len());
        self.maps.push(map);
        let Err(slot_index) = self.lookup(
            self.maps
                .last()
                .expect("newly interned reference map must stay addressable"),
        ) else {
            unreachable!("newly interned reference map must not already exist");
        };
        self.map_slots[slot_index] = map_id.index() as u32;

        map_id
    }

    /// Return the flattened reference maps.
    pub(crate) fn snapshot(&self) -> Vec<ReferenceMap> {
        self.maps.clone()
    }

    /// Return the retained bytes owned by this table.
    pub(crate) fn retained_bytes(&self) -> usize {
        let mut retained_bytes = self.maps.capacity() * std::mem::size_of::<ReferenceMap>();

        for map in &self.maps {
            retained_bytes += map.retained_bytes();
        }

        retained_bytes += self.map_slots.capacity() * std::mem::size_of::<u32>();

        retained_bytes
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
