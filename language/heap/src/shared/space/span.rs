use serde::{Deserialize, Serialize};

use crate::ShapeId;
use crate::allocator::{Bitmap, PageView};

/// One frozen shared heap small-span root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapSmallSpanImage {
    /// The size class for this span in bytes.
    pub size_class: usize,
    /// The number of slots in this span.
    pub slot_count: usize,
    /// The logical byte length for each slot.
    pub lengths: Box<[usize]>,
    /// The occupied slots in this span.
    pub occupied: Bitmap,
    /// The per-slot shape ids for this span.
    pub shape_ids: Box<[Option<u32>]>,
    /// The allocator pages for this span.
    pub pages: PageView,
}

/// One live shared heap span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedSmallSpan {
    /// The slot payload size in bytes.
    pub(crate) size_class: usize,
    /// The number of slots in this span.
    pub(crate) slot_count: usize,
    /// The number of occupied slots in this span.
    pub(crate) occupied_count: usize,
    /// The next likely free slot.
    pub(crate) next_free_slot: usize,
    /// The occupied slots in this span.
    pub(crate) occupied: Bitmap,
    /// The marked slots in this span.
    pub(crate) marked: Bitmap,
    /// The logical byte length for each slot.
    pub(crate) lengths: Box<[usize]>,
    /// The per-slot shape ids for this span.
    pub(crate) shape_ids: Box<[Option<ShapeId>]>,
    /// The allocator pages for this span.
    pub(crate) pages: PageView,
}

impl SharedSmallSpan {
    /// Set the logical byte length for one slot in this span.
    pub(crate) fn set_length(&mut self, slot_index: usize, length: usize) {
        self.lengths[slot_index] = length;
    }

    /// Set the shape id for one slot in this span.
    pub(crate) fn set_shape_id(&mut self, slot_index: usize, shape_id: Option<ShapeId>) {
        self.shape_ids[slot_index] = shape_id;
    }

    /// Clear every mark bit in this span.
    pub(crate) fn clear_marks(&mut self) {
        self.marked.clear_all();
    }
}
