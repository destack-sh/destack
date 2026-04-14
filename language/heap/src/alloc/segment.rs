use std::collections::BTreeMap;
use std::slice;
use std::sync::Mutex;
use std::sync::atomic::AtomicU32;

use super::{PageRun, allocate_page_segment_bytes, free_page_segment_bytes};

/// One contiguous arena segment of fixed-width pages.
#[derive(Debug)]
pub(crate) struct Segment {
    /// The allocated bytes for this segment.
    pub(crate) data: *mut u8,
    /// The byte length for this segment.
    pub(crate) byte_len: usize,

    /// The next never-allocated page inside this segment.
    pub(crate) next_unused_page: AtomicU32,
    /// The run refcounts keyed by run-start page index inside this segment.
    pub(crate) run_refcounts: Box<[AtomicU32]>,
    /// The reusable runs owned by this segment, bucketed by page count.
    pub(crate) free_runs: Mutex<BTreeMap<u32, Vec<PageRun>>>,
}

impl Segment {
    /// Allocate one zeroed arena segment.
    pub(crate) fn zeroed(byte_len: usize, pages_per_segment: usize) -> Self {
        Self {
            data: allocate_page_segment_bytes(byte_len),
            byte_len,
            next_unused_page: AtomicU32::new(0),
            run_refcounts: std::iter::repeat_with(|| AtomicU32::new(0))
                .take(pages_per_segment)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            free_runs: Mutex::new(BTreeMap::new()),
        }
    }

    /// Return one immutable page slice.
    pub(crate) fn page(&self, page_index: usize, page_bytes: usize) -> &[u8] {
        let start = page_index.saturating_mul(page_bytes);
        let data = unsafe { self.data.add(start) };

        unsafe { slice::from_raw_parts(data, page_bytes) }
    }

    /// Return one mutable page slice.
    pub(crate) fn page_mut(&self, page_index: usize, page_bytes: usize) -> &mut [u8] {
        let start = page_index.saturating_mul(page_bytes);
        let data = unsafe { self.data.add(start) };

        unsafe { slice::from_raw_parts_mut(data, page_bytes) }
    }
}

impl Drop for Segment {
    fn drop(&mut self) {
        free_page_segment_bytes(self.data, self.byte_len);
    }
}

// segment memory is only accessed through arena page ownership
unsafe impl Send for Segment {}

// segment memory is only accessed through arena page ownership
unsafe impl Sync for Segment {}
