use destack_core::SectionBuilder;

use crate::StaticImage;

/// Construction-time allocator for static global bytes.
#[derive(Debug, Default)]
pub struct GlobalAllocator {
    /// Static bytes.
    bytes: Vec<u8>,
}

impl GlobalAllocator {
    /// Create an empty global allocator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate one static global byte range.
    pub fn allocate(&mut self, alignment: usize, bytes: &[u8]) -> (usize, usize) {
        // align the next global start
        let offset = align_static_offset(self.bytes.len(), alignment);
        self.bytes.resize(offset, 0);

        // append global bytes
        self.bytes.extend_from_slice(bytes);

        (offset, bytes.len())
    }

    /// Finish static memory.
    pub fn finish(self, sections: &mut SectionBuilder) -> StaticImage {
        StaticImage::pack(sections, self.bytes)
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
