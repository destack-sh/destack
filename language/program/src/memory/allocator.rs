use std::collections::HashMap;

use crate::{StaticId, StaticRegion, StaticSpace, TypeId};

/// Allocator for static memory.
#[derive(Debug, Default)]
pub struct StaticAllocator {
    /// Static bytes.
    bytes: Vec<u8>,
    /// Static regions.
    regions: Vec<StaticRegion>,
    /// Region index by id.
    region_by_id: HashMap<StaticId, usize>,
}

impl StaticAllocator {
    /// Create an empty static allocator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define one static region.
    pub fn define(
        &mut self,
        id: StaticId,
        ty: TypeId,
        alignment: usize,
        is_mutable: bool,
        bytes: &[u8],
    ) -> bool {
        // static ids are unique inside one static space
        if self.region_by_id.contains_key(&id) {
            return false;
        }

        // align the next region start
        let offset = align_static_offset(self.bytes.len(), alignment);
        self.bytes.resize(offset, 0);

        // append region bytes and metadata together
        let index = self.regions.len();
        self.bytes.extend_from_slice(bytes);
        self.regions.push(StaticRegion {
            id,
            offset,
            byte_len: bytes.len(),
            ty,
            is_mutable,
        });
        self.region_by_id.insert(id, index);

        true
    }

    /// Return whether one static region is already defined.
    pub fn contains(&self, id: StaticId) -> bool {
        self.region_by_id.contains_key(&id)
    }

    /// Finish static memory.
    pub fn finish(self) -> StaticSpace {
        StaticSpace::new(
            self.bytes.into_boxed_slice(),
            self.regions.into_boxed_slice(),
            self.region_by_id,
        )
    }
}

/// Align one static byte offset.
fn align_static_offset(offset: usize, alignment: usize) -> usize {
    // byte alignment is already satisfied
    if alignment <= 1 {
        return offset;
    }

    // exact alignment does not need padding
    let remainder = offset % alignment;
    if remainder == 0 {
        return offset;
    }

    // pad to the next aligned byte offset
    offset + (alignment - remainder)
}
