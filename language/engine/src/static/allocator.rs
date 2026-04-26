use std::collections::HashMap;

use crate::{StaticId, StaticRegion, StaticSpace, TypeId};

/// Allocator for one static byte memory image.
#[derive(Debug, Default)]
pub struct StaticAllocator {
    /// The static bytes being built.
    bytes: Vec<u8>,
    /// The static regions being built.
    regions: Vec<StaticRegion>,
    /// Static region table index by static id.
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
    ) -> Option<()> {
        // static ids are unique inside one static space
        if self.region_by_id.contains_key(&id) {
            return None;
        }

        // align the next region start
        let offset = align_static_offset(self.bytes.len(), alignment)?;
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

        Some(())
    }

    /// Return whether one static region is already defined.
    pub fn contains(&self, id: StaticId) -> bool {
        self.region_by_id.contains_key(&id)
    }

    /// Seal the static bytes into stable static memory.
    pub fn finish(self) -> StaticSpace {
        StaticSpace::new(
            self.bytes.into_boxed_slice(),
            self.regions.into_boxed_slice(),
            self.region_by_id,
        )
    }
}

/// Align one static byte offset.
fn align_static_offset(offset: usize, alignment: usize) -> Option<usize> {
    // byte alignment is already satisfied
    if alignment <= 1 {
        return Some(offset);
    }

    // exact alignment does not need padding
    let remainder = offset % alignment;
    if remainder == 0 {
        return Some(offset);
    }

    // pad to the next aligned byte offset
    offset.checked_add(alignment - remainder)
}
