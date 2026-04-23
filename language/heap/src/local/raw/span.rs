use serde::{Deserialize, Serialize};

use crate::allocator::{Bitmap, PageView};

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
    /// The full byte payload for this span.
    pub bytes: Box<[u8]>,
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
    /// The allocator pages for this span.
    pub(crate) pages: PageView,
}
