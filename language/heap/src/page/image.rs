use std::mem::size_of_val;

use serde::{Deserialize, Serialize};

use super::class::PageBitmap;

/// One value that can report extra retained bytes beyond its inline size.
pub trait RetainedBytes {
    /// Return the retained heap bytes owned by this value outside its inline storage.
    fn retained_bytes(&self) -> usize {
        0
    }
}

/// Immutable page image shared by snapshots and forked heaps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageImage<T, const CAPACITY: usize, const WORDS: usize> {
    /// The entries stored in this page.
    pub entries: Box<[T]>,
    /// The occupancy bitmap for this page.
    pub occupied: PageBitmap<CAPACITY, WORDS>,
}

impl<T, const CAPACITY: usize, const WORDS: usize> PageImage<T, CAPACITY, WORDS> {
    /// Report whether one page offset is occupied.
    #[inline]
    pub fn is_occupied(&self, offset: usize) -> bool {
        self.occupied.contains(offset)
    }

    /// Return one entry by page offset.
    #[inline]
    pub fn get(&self, offset: usize) -> Option<&T> {
        if !self.is_occupied(offset) {
            return None;
        }

        self.entries.get(offset)
    }

    /// Return one entry by page offset without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the page offset is valid and occupied.
    #[inline]
    pub unsafe fn get_unchecked(&self, offset: usize) -> &T {
        debug_assert!(self.is_occupied(offset), "page entry is not occupied");

        unsafe { self.entries.get_unchecked(offset) }
    }
}

impl<T: RetainedBytes, const CAPACITY: usize, const WORDS: usize> PageImage<T, CAPACITY, WORDS> {
    /// Return the retained bytes for this immutable page image.
    pub fn retained_bytes(&self) -> usize {
        let mut retained_bytes = size_of_val(self.entries.as_ref());

        for entry in self.entries.iter() {
            retained_bytes += entry.retained_bytes();
        }

        retained_bytes
    }
}
