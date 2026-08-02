use crate::{GlobalAddress, StaticBytes};

/// Construction-time allocator for static global bytes.
#[derive(Debug)]
pub struct GlobalAllocator {
    /// Static bytes.
    bytes: Vec<u8>,
    /// Maximum global alignment.
    alignment: usize,
}

impl Default for GlobalAllocator {
    fn default() -> Self {
        Self {
            bytes: vec![0; GlobalAddress::FIRST_OFFSET],
            alignment: 1,
        }
    }
}

impl GlobalAllocator {
    /// Create an empty global allocator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate one static global byte range.
    pub fn allocate(&mut self, alignment: usize, bytes: &[u8]) -> (usize, usize) {
        assert!(
            alignment.is_power_of_two(),
            "global alignment must be a power of two"
        );

        // align the next global start
        let offset = align_static_offset(self.bytes.len(), alignment);
        self.bytes.resize(offset, 0);
        self.alignment = self.alignment.max(alignment);

        // append global bytes
        self.bytes.extend_from_slice(bytes);

        (offset, bytes.len())
    }

    /// Build the initialized and aligned static bytes.
    pub fn build(self) -> StaticBytes {
        StaticBytes::new(self.bytes, self.alignment)
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
