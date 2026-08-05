use crate::{GlobalAddress, StaticBytes};

/// Construction-time allocator for static global bytes.
#[derive(Debug)]
pub struct GlobalAllocator {
    /// Static bytes.
    bytes: Vec<u8>,
    /// Image-relative word offsets holding image-relative target offsets.
    relocations: Vec<u64>,
    /// Maximum global alignment.
    alignment: usize,
}

impl Default for GlobalAllocator {
    fn default() -> Self {
        Self {
            bytes: vec![0; GlobalAddress::FIRST_OFFSET],
            relocations: Vec::new(),
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
        let offset = self.reserve(alignment, bytes.len());
        self.write(offset, bytes);

        (offset, bytes.len())
    }

    /// Reserve one zeroed static global byte range.
    pub fn reserve(&mut self, alignment: usize, byte_len: usize) -> usize {
        assert!(
            alignment.is_power_of_two(),
            "global alignment must be a power of two"
        );

        // align the next global start
        let offset = align_static_offset(self.bytes.len(), alignment);
        self.alignment = self.alignment.max(alignment);

        // extend zeroed storage over the reserved range
        self.bytes.resize(offset + byte_len, 0);

        offset
    }

    /// Write bytes into one reserved range.
    pub fn write(&mut self, offset: usize, bytes: &[u8]) {
        self.bytes[offset..offset + bytes.len()].copy_from_slice(bytes);
    }

    /// Record one address word to rebase at materialization.
    pub fn relocate(&mut self, word_offset: usize) {
        self.relocations.push(word_offset as u64);
    }

    /// Build the initialized and aligned static bytes.
    pub fn build(self) -> StaticBytes {
        StaticBytes::relocated(self.bytes, self.relocations, self.alignment)
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
