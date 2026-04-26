use serde::{Deserialize, Serialize};

use crate::SmallSpanClass;
use crate::allocator::{Bitmap, PageView};

/// One frozen shared heap small-span root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapSmallSpanImage {
    /// The homogeneous payload class for this span.
    pub class: SmallSpanClass,
    /// The number of slots in this span.
    pub slot_count: usize,
    /// The occupied slots in this span.
    pub occupied: Bitmap,
    /// The exact local-reference bits for each occupied slot.
    pub local_reference_bits: Bitmap,
    /// The exact shared-reference bits for each occupied slot.
    pub shared_reference_bits: Bitmap,
    /// The allocator pages for this span.
    pub pages: PageView,
}

/// One live shared heap span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedSmallSpan {
    /// The homogeneous payload class for this span.
    pub(crate) class: SmallSpanClass,
    /// The number of slots in this span.
    pub(crate) slot_count: usize,
    /// The number of occupied slots in this span.
    pub(crate) occupied_count: usize,
    /// The next likely free-slot search cursor.
    pub(crate) free_cursor: usize,
    /// The occupied slots in this span.
    pub(crate) occupied: Bitmap,
    /// The exact local-reference bits for each occupied slot.
    pub(crate) local_reference_bits: Bitmap,
    /// The exact shared-reference bits for each occupied slot.
    pub(crate) shared_reference_bits: Bitmap,
    /// The marked slots in this span.
    pub(crate) marked: Bitmap,
    /// The marked slots whose payloads have already been scanned.
    pub(crate) scanned: Bitmap,
    /// Whether this span already has one queued scan work item.
    pub(crate) is_queued_for_scan: bool,
    /// The allocation list this span belongs to.
    pub(crate) list: SpanList,
    /// The allocator pages for this span.
    pub(crate) pages: PageView,
}

/// One shared small-span allocation list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpanList {
    /// The central partial list.
    Central,
    /// One shared allocation front end.
    Cached,
    /// No allocation list because the span has no free slots.
    Full,
    /// No allocation list because the span has no mapped pages.
    Released,
}

impl SharedSmallSpan {
    /// Clear every mark and scan bit in this span.
    pub(crate) fn clear_marks(&mut self) {
        self.marked.clear_all();
        self.scanned.clear_all();
        self.is_queued_for_scan = false;
    }
}
