use std::sync::atomic::AtomicU32;
use std::{ptr, slice};

use super::{allocate_page_segment_bytes, free_page_segment_bytes};
use crate::HeapResult;

/// One contiguous allocator memory segment of fixed-width pages.
#[derive(Debug)]
pub(crate) struct Segment {
    /// The allocated bytes for this segment.
    pub(crate) data: *mut u8,
    /// The byte length for this segment.
    pub(crate) byte_len: usize,
    /// The page alignment for this segment allocation.
    pub(crate) page_bytes: usize,

    /// The next never-allocated page inside this segment.
    pub(crate) next_unused_page: AtomicU32,
    /// The run refcounts keyed by run-start page index inside this segment.
    pub(crate) run_refcounts: Box<[AtomicU32]>,
}

impl Segment {
    /// Allocate one zeroed allocator segment.
    pub(crate) fn zeroed(
        byte_len: usize,
        pages_per_segment: usize,
        page_bytes: usize,
    ) -> HeapResult<Self> {
        Ok(Self {
            data: allocate_page_segment_bytes(byte_len, page_bytes)?,
            byte_len,
            page_bytes,
            next_unused_page: AtomicU32::new(0),
            run_refcounts: std::iter::repeat_with(|| AtomicU32::new(0))
                .take(pages_per_segment)
                .collect(),
        })
    }

    /// Return one immutable page slice.
    pub(crate) fn page(&self, page_index: usize, page_bytes: usize) -> &[u8] {
        let start = page_index * page_bytes;
        let data = unsafe { self.data.add(start) };

        unsafe { slice::from_raw_parts(data, page_bytes) }
    }

    /// Return one mutable page slice pointer.
    pub(crate) fn page_mut_ptr(&self, page_index: usize, page_bytes: usize) -> *mut [u8] {
        let start = page_index * page_bytes;
        let data = unsafe { self.data.add(start) };

        ptr::slice_from_raw_parts_mut(data, page_bytes)
    }
}

impl Drop for Segment {
    fn drop(&mut self) {
        free_page_segment_bytes(self.data, self.byte_len, self.page_bytes);
    }
}

// segment memory is only accessed through allocator page ownership
unsafe impl Send for Segment {}

// segment memory is only accessed through allocator page ownership
unsafe impl Sync for Segment {}
