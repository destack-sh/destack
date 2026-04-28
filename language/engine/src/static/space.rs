use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{StaticAllocator, StaticId, StaticPointer, StaticRegion};

/// Static byte memory with a fixed region table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticSpace {
    /// The static bytes.
    bytes: Box<[u8]>,
    /// The fixed static region table.
    regions: Box<[StaticRegion]>,
    /// Static region table index by static id.
    region_by_id: HashMap<StaticId, usize>,
}

impl StaticSpace {
    /// Create empty static bytes.
    pub fn empty() -> Self {
        Self {
            bytes: Box::default(),
            regions: Box::default(),
            region_by_id: HashMap::new(),
        }
    }

    /// Allocate static bytes incrementally.
    pub fn allocator() -> StaticAllocator {
        StaticAllocator::new()
    }

    /// Create static memory from allocated bytes and regions.
    pub(super) fn new(
        bytes: Box<[u8]>,
        regions: Box<[StaticRegion]>,
        region_by_id: HashMap<StaticId, usize>,
    ) -> Self {
        Self {
            bytes,
            regions,
            region_by_id,
        }
    }

    /// Borrow one static region.
    pub fn region(&self, id: StaticId) -> Option<&StaticRegion> {
        let index = self.region_by_id.get(&id)?;

        self.regions.get(*index)
    }

    /// Borrow one static byte range.
    pub fn bytes(&self, id: StaticId) -> Option<&[u8]> {
        let region = self.region(id)?;
        let end = region.offset.checked_add(region.byte_len)?;

        self.bytes.get(region.offset..end)
    }

    /// Borrow one static byte range mutably.
    pub fn bytes_mut(&mut self, id: StaticId) -> Option<&mut [u8]> {
        let region = self.region(id)?;

        // immutable regions have no mutable projection
        if !region.is_mutable {
            return None;
        }

        let end = region.offset.checked_add(region.byte_len)?;

        self.bytes.get_mut(region.offset..end)
    }

    /// Return a stable pointer to one static region.
    pub fn ptr(&self, id: StaticId) -> Option<StaticPointer> {
        let region = self.region(id)?;
        let address = self.bytes.as_ptr() as usize;
        let address = address.checked_add(region.offset)?;

        Some(StaticPointer::from_address(address))
    }

    /// Return the static region containing one static pointer.
    pub fn region_for_pointer(&self, pointer: StaticPointer) -> Option<&StaticRegion> {
        let address = pointer.address();
        self.regions.iter().find(|region| {
            let start = self.bytes.as_ptr() as usize + region.offset;
            let Some(end) = start.checked_add(region.byte_len) else {
                return false;
            };

            start <= address && address < end
        })
    }

    /// Return whether static memory owns one byte range.
    pub fn owns_pointer_range(&self, pointer: StaticPointer, byte_len: usize) -> bool {
        let address = pointer.address();
        let Some(end) = address.checked_add(byte_len) else {
            return false;
        };

        self.regions.iter().any(|region| {
            let start = self.bytes.as_ptr() as usize + region.offset;
            let region_end = start.saturating_add(region.byte_len);

            start <= address && end <= region_end
        })
    }

    /// Return whether static memory owns one mutable byte range.
    pub fn owns_mutable_pointer_range(&self, pointer: StaticPointer, byte_len: usize) -> bool {
        let address = pointer.address();
        let Some(end) = address.checked_add(byte_len) else {
            return false;
        };

        // mutable region ownership
        self.regions.iter().any(|region| {
            if !region.is_mutable {
                return false;
            }

            let start = self.bytes.as_ptr() as usize + region.offset;
            let region_end = start.saturating_add(region.byte_len);

            start <= address && end <= region_end
        })
    }

    /// Return an iterator over static regions.
    pub fn iter_regions(&self) -> impl Iterator<Item = (StaticId, &StaticRegion, &[u8])> + '_ {
        self.regions.iter().filter_map(|region| {
            let end = region.offset.checked_add(region.byte_len)?;
            let bytes = self.bytes.get(region.offset..end)?;

            Some((region.id, region, bytes))
        })
    }

    /// Return all static ids in region order.
    pub fn ids(&self) -> impl Iterator<Item = StaticId> + '_ {
        self.regions.iter().map(|region| region.id)
    }

    /// Return the number of static regions.
    pub fn len(&self) -> usize {
        self.region_by_id.len()
    }

    /// Return whether no static regions exist.
    pub fn is_empty(&self) -> bool {
        self.region_by_id.is_empty()
    }

    /// Return the static byte count.
    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }
}
