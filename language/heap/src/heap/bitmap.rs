use serde::{Deserialize, Serialize};

use super::page::HEAP_PAGE_CAPACITY;

const PAGE_BITMAP_WORD_BITS: usize = u64::BITS as usize;
const PAGE_BITMAP_WORDS: usize = HEAP_PAGE_CAPACITY / PAGE_BITMAP_WORD_BITS;
const _: () = assert!(HEAP_PAGE_CAPACITY % PAGE_BITMAP_WORD_BITS == 0);

/// The occupancy or mark bitmap for one heap page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(transparent)]
pub struct Bitmap([u64; PAGE_BITMAP_WORDS]);

impl Bitmap {
    /// Create one empty bitmap.
    pub const fn new() -> Self {
        Self([0; PAGE_BITMAP_WORDS])
    }

    /// Report whether one slot offset is set.
    #[inline]
    pub fn contains(self, offset: usize) -> bool {
        let word_index = offset / PAGE_BITMAP_WORD_BITS;
        let bit_offset = offset % PAGE_BITMAP_WORD_BITS;
        let mask = 1_u64 << bit_offset;

        self.0[word_index] & mask != 0
    }

    /// Set one slot offset.
    #[inline]
    pub fn set(&mut self, offset: usize) {
        let word_index = offset / PAGE_BITMAP_WORD_BITS;
        let bit_offset = offset % PAGE_BITMAP_WORD_BITS;
        let mask = 1_u64 << bit_offset;

        self.0[word_index] |= mask;
    }

    /// Clear one slot offset.
    #[inline]
    pub fn clear(&mut self, offset: usize) {
        let word_index = offset / PAGE_BITMAP_WORD_BITS;
        let bit_offset = offset % PAGE_BITMAP_WORD_BITS;
        let mask = !(1_u64 << bit_offset);

        self.0[word_index] &= mask;
    }

    /// Clear all bits.
    #[inline]
    pub fn clear_all(&mut self) {
        self.0.fill(0);
    }

    /// Count the number of set bits.
    #[inline]
    pub fn count_ones(self) -> usize {
        self.0.iter().map(|word| word.count_ones() as usize).sum()
    }
}
