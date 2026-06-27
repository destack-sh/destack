use std::collections::HashMap;

use crate::{GlobalId, GlobalRegion, StaticSpace, TypeId};

/// Construction-time allocator for global regions in one static space.
#[derive(Debug, Default)]
pub struct GlobalAllocator {
    /// Static bytes.
    bytes: Vec<u8>,
    /// Global regions.
    regions: Vec<GlobalRegion>,
    /// Region index by global id.
    region_by_global: HashMap<GlobalId, usize>,
}

impl GlobalAllocator {
    /// Create an empty global allocator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define one global region.
    pub fn define(
        &mut self,
        global: GlobalId,
        ty: TypeId,
        alignment: usize,
        is_mutable: bool,
        bytes: &[u8],
    ) -> bool {
        // global ids are unique inside one static space
        if self.region_by_global.contains_key(&global) {
            return false;
        }

        // align the next region start
        let offset = align_static_offset(self.bytes.len(), alignment);
        self.bytes.resize(offset, 0);

        // append region bytes and metadata together
        let index = self.regions.len();
        self.bytes.extend_from_slice(bytes);
        self.regions.push(GlobalRegion {
            global,
            offset,
            byte_len: bytes.len(),
            ty,
            is_mutable,
        });
        self.region_by_global.insert(global, index);

        true
    }

    /// Finish static memory.
    pub fn finish(self) -> StaticSpace {
        StaticSpace::new(
            self.bytes.into_boxed_slice(),
            self.regions.into_boxed_slice(),
            self.region_by_global,
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
