use serde::{Deserialize, Serialize};

use crate::alloc::{Bitmap, PageView};

/// One frozen raw span root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SmallSpanImage {
    /// The size class for this span in bytes.
    pub size_class: usize,
    /// The number of slots in this span.
    pub slot_count: usize,
    /// The logical byte length for each slot.
    pub lengths: Box<[usize]>,
    /// The occupied slots in this span.
    pub occupied: Bitmap,
    /// The arena pages for this span.
    pub pages: PageView,
}

/// One live raw span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpan {
    /// The slot payload size in bytes.
    pub(crate) size_class: usize,
    /// The number of slots in this span.
    pub(crate) slot_count: usize,
    /// The number of occupied slots in this span.
    pub(crate) occupied_count: usize,
    /// The next likely free slot.
    pub(crate) next_free_slot: usize,
    /// The logical byte length for each slot.
    pub(crate) lengths: Box<[usize]>,
    /// The occupied slots in this span.
    pub(crate) occupied: Bitmap,
    /// The arena pages for this span.
    pub(crate) pages: PageView,
}
