use serde::{Deserialize, Serialize};

use crate::allocator::{Bitmap, PageView};

/// One homogeneous raw small-span class.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) struct RawSmallSpanClass {
    /// The slot payload size in bytes.
    pub size_class: usize,
    /// The span byte width for this class.
    pub span_bytes: usize,
    /// The logical byte length for every slot.
    pub byte_len: usize,
}

/// One frozen raw span root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SmallSpanImage {
    /// The homogeneous class for this span.
    pub class: RawSmallSpanClass,
    /// The number of slots in this span.
    pub slot_count: usize,
    /// The occupied slots in this span.
    pub occupied: Bitmap,
    /// The full byte payload for this span.
    pub bytes: Box<[u8]>,
}

/// One live raw span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpan {
    /// The homogeneous class for this span.
    pub(crate) class: RawSmallSpanClass,
    /// The number of slots in this span.
    pub(crate) slot_count: usize,
    /// The number of occupied slots in this span.
    pub(crate) occupied_count: usize,
    /// The next likely free-slot search cursor.
    pub(crate) free_cursor: usize,
    /// The occupied slots in this span.
    pub(crate) occupied: Bitmap,
    /// The allocator pages for this span.
    pub(crate) pages: PageView,
}
