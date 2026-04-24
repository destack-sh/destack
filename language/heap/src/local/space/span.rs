use serde::{Deserialize, Serialize};

use super::CardSet;
use crate::SmallSpanClass;
use crate::allocator::{Bitmap, PageView};

/// One frozen heap span root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SmallSpanImage {
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

/// One live heap span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpan {
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
    /// The allocator pages for this span.
    pub(crate) pages: PageView,
    /// The dirty cards remembered for young tracing.
    pub(crate) dirty_cards: CardSet,
    /// Whether this span is already queued for dirty-card scanning.
    pub(crate) is_dirty_queued: bool,
}

impl SmallSpan {
    /// Clear every mark bit in this span.
    pub(crate) fn clear_marks(&mut self) {
        self.marked.clear_all();
    }
}
