use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{StaticAddress, StaticAllocator, StaticId, StaticRegion};

/// Static memory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticSpace {
    /// The static bytes.
    bytes: Box<[u8]>,
    /// Static regions.
    regions: Box<[StaticRegion]>,
    /// Region index by id.
    region_by_id: HashMap<StaticId, usize>,
}

impl StaticSpace {
    /// Create empty static memory.
    pub fn empty() -> Self {
        Self {
            bytes: Box::default(),
            regions: Box::default(),
            region_by_id: HashMap::new(),
        }
    }

    /// Create a static allocator.
    pub fn allocator() -> StaticAllocator {
        StaticAllocator::new()
    }

    /// Create static memory.
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
        let end = region.offset + region.byte_len;

        self.bytes.get(region.offset..end)
    }

    /// Borrow one static byte range mutably.
    pub fn bytes_mut(&mut self, id: StaticId) -> Option<&mut [u8]> {
        let region = self.region(id)?;

        // immutable regions have no mutable projection
        if !region.is_mutable {
            return None;
        }

        let end = region.offset + region.byte_len;

        self.bytes.get_mut(region.offset..end)
    }

    /// Return a stable address to one static region.
    pub fn address(&self, id: StaticId) -> Option<StaticAddress> {
        self.region(id)?;

        Some(StaticAddress::new(id, 0))
    }

    /// Return a native address for one static byte range.
    pub fn native_address(&self, address: StaticAddress, byte_len: usize) -> Option<usize> {
        let region = self.region(address.id())?;
        let start = address.byte_offset();
        let end = start.checked_add(byte_len)?;
        if end > region.byte_len {
            return None;
        }

        Some(self.bytes.as_ptr() as usize + region.offset + start)
    }

    /// Return a mutable native address for one static byte range.
    pub fn native_address_mut(&mut self, address: StaticAddress, byte_len: usize) -> Option<usize> {
        let region = self.region(address.id())?;
        let region_offset = region.offset;
        let region_byte_len = region.byte_len;
        if !region.is_mutable {
            return None;
        }

        let start = address.byte_offset();
        let end = start.checked_add(byte_len)?;
        if end > region_byte_len {
            return None;
        }

        Some(self.bytes.as_mut_ptr() as usize + region_offset + start)
    }

    /// Return whether static memory owns one byte range.
    pub fn owns_address_range(&self, address: StaticAddress, byte_len: usize) -> bool {
        self.native_address(address, byte_len).is_some()
    }

    /// Return an iterator over static regions.
    pub fn iter_regions(&self) -> impl Iterator<Item = (StaticId, &StaticRegion, &[u8])> + '_ {
        self.regions.iter().map(|region| {
            let end = region.offset + region.byte_len;
            let bytes = &self.bytes[region.offset..end];

            (region.id, region, bytes)
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
