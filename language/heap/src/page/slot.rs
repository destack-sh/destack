use serde::{Deserialize, Serialize};

/// The bit shift used to pack one page index into a page slot.
///
/// 32 bits leaves ample room for the page-local slot index while keeping the
/// packed page slot in one compact `u64`.
const PAGE_INDEX_SHIFT: u64 = 32;
/// The bit mask used to recover one slot index from a packed page slot.
const SLOT_INDEX_MASK: u64 = 0xFFFF_FFFF;

/// One stable page slot within page-backed storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSlot(u64);

impl PageSlot {
    /// The zero page slot.
    pub const ZERO: Self = Self::new(0, 0);

    /// Create one page slot.
    #[inline]
    pub const fn new(page_index: usize, slot_index: usize) -> Self {
        Self(((page_index as u64) << PAGE_INDEX_SHIFT) | (slot_index as u64))
    }

    /// Return the page index.
    #[inline]
    pub const fn page_index(self) -> usize {
        (self.0 >> PAGE_INDEX_SHIFT) as usize
    }

    /// Return the slot index within the page.
    #[inline]
    pub const fn slot_index(self) -> usize {
        (self.0 & SLOT_INDEX_MASK) as usize
    }

    /// Return the page and slot position.
    #[inline]
    pub const fn position(self) -> (usize, usize) {
        (self.page_index(), self.slot_index())
    }
}
