use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use super::EdgeMap;
use crate::{HeapError, HeapResult};

/// The sentinel for one empty reverse-lookup slot.
const EMPTY_EDGE_SLOT: u32 = u32::MAX;

/// One interned managed edge-map identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct EdgeId(u32);

impl EdgeId {
    /// The empty edge-map identifier.
    pub(crate) const EMPTY: Self = Self(0);

    /// Create one managed edge-map identifier.
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the empty edge-map identifier.
    pub(crate) const fn empty() -> Self {
        Self::EMPTY
    }

    /// Create one managed edge-map identifier from a table index.
    pub(crate) fn from_index(index: usize) -> HeapResult<Self> {
        if index >= EMPTY_EDGE_SLOT as usize {
            return Err(HeapError::InvalidEdgeId { index });
        }

        Ok(Self(index as u32))
    }

    /// Return the edge-map table index for this identifier.
    pub(crate) const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Interned managed edge maps for one heap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EdgeTable {
    /// Interned edge maps.
    edge_maps: Vec<EdgeMap>,
    /// Open-addressed reverse lookup keyed by edge-map id.
    edge_slots: Vec<u32>,
}

impl EdgeTable {
    /// Create one managed edge-map table with the empty map pre-interned.
    pub(crate) fn new() -> HeapResult<Self> {
        Self::from_edge_maps(vec![EdgeMap::empty()])
    }

    /// Restore one managed edge-map table from one flattened edge-map list.
    pub(crate) fn from_edge_maps(edge_maps: Vec<EdgeMap>) -> HeapResult<Self> {
        let edge_maps = if edge_maps.is_empty() {
            vec![EdgeMap::empty()]
        } else {
            edge_maps
        };

        if edge_maps[EdgeId::empty().index()] != EdgeMap::empty() {
            return Err(HeapError::InvalidEdgeSentinel {
                index: EdgeId::empty().index(),
            });
        }

        let mut table = Self {
            edge_maps,
            edge_slots: Vec::new(),
        };

        table.rebuild_edge_slots()?;

        Ok(table)
    }

    /// Intern one edge map and return its stable identifier.
    pub(crate) fn intern(&mut self, map: EdgeMap) -> HeapResult<(EdgeId, i64)> {
        if self.edge_slots.is_empty() {
            self.rebuild_edge_slots()?;
        }

        if let Ok(edge_id) = self.lookup(&map) {
            return Ok((edge_id, 0));
        }

        if self.needs_rehash() {
            self.rebuild_edge_slots_for_count(self.edge_maps.len().saturating_add(1))?;
        }

        let slot_index = match self.lookup(&map) {
            Ok(edge_id) => return Ok((edge_id, 0)),
            Err(slot_index) => slot_index,
        };
        let edge_id = EdgeId::from_index(self.edge_maps.len())?;
        self.edge_maps.push(map);
        self.edge_slots[slot_index] = edge_id.index() as u32;

        Ok((edge_id, 0))
    }

    /// Return the flattened edge maps.
    pub(crate) fn edge_maps(&self) -> Vec<EdgeMap> {
        self.edge_maps.clone()
    }

    /// Return one interned edge map by id.
    pub(crate) fn edge_map(&self, edge_id: EdgeId) -> Option<&EdgeMap> {
        self.edge_maps.get(edge_id.index())
    }

    /// Return whether the reverse lookup should grow before one more insert.
    fn needs_rehash(&self) -> bool {
        self.edge_slots.is_empty()
            || self.edge_maps.len().saturating_add(1) * 4 > self.edge_slots.len() * 3
    }

    /// Rebuild the reverse lookup for the current edge-map set.
    fn rebuild_edge_slots(&mut self) -> HeapResult<()> {
        self.rebuild_edge_slots_for_count(self.edge_maps.len())
    }

    /// Rebuild the reverse lookup for the given target edge-map count.
    fn rebuild_edge_slots_for_count(&mut self, count: usize) -> HeapResult<()> {
        let slot_count = count
            .checked_mul(2)
            .and_then(|count| count.max(8).checked_next_power_of_two())
            .ok_or(HeapError::InvalidEdgeTableLen { len: count })?;
        self.edge_slots = vec![EMPTY_EDGE_SLOT; slot_count];

        for (index, edge_map) in self.edge_maps.iter().enumerate() {
            let edge_id = EdgeId::from_index(index)?;
            let slot_index = match self.lookup_in_slots(edge_map, &self.edge_slots) {
                Ok(_) => return Err(HeapError::DuplicateEdgeMap { index }),
                Err(slot_index) => slot_index,
            };
            self.edge_slots[slot_index] = edge_id.0;
        }

        Ok(())
    }

    /// Return one interned identifier or one insertion slot for this edge map.
    fn lookup(&self, edge_map: &EdgeMap) -> Result<EdgeId, usize> {
        self.lookup_in_slots(edge_map, &self.edge_slots)
            .map(EdgeId::new)
    }

    /// Probe one reverse-lookup table for this edge map.
    fn lookup_in_slots(&self, edge_map: &EdgeMap, slots: &[u32]) -> Result<u32, usize> {
        if slots.is_empty() {
            return Err(0);
        }

        let mask = slots.len() - 1;
        let mut slot_index = (Self::hash_edge_map(edge_map) as usize) & mask;

        loop {
            let edge_index = slots[slot_index];
            if edge_index == EMPTY_EDGE_SLOT {
                return Err(slot_index);
            }

            if self.edge_maps[edge_index as usize] == *edge_map {
                return Ok(edge_index);
            }

            slot_index = (slot_index + 1) & mask;
        }
    }

    /// Hash one managed edge map for the reverse lookup.
    fn hash_edge_map(map: &EdgeMap) -> u64 {
        let mut hasher = DefaultHasher::new();
        map.hash(&mut hasher);
        hasher.finish()
    }
}
