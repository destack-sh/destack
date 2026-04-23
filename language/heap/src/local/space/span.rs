use serde::{Deserialize, Serialize};

use super::CardSet;
use crate::ShapeId;
use crate::allocator::{Bitmap, PageView};

/// One frozen heap span root.
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
    /// The per-slot shape ids for this span.
    pub shape_ids: Box<[Option<ShapeId>]>,
    /// The allocator pages for this span.
    pub pages: PageView,
}

/// One live heap span.
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
    /// The marked slots in this span.
    pub(crate) marked: Bitmap,
    /// The per-slot shape ids for this span.
    pub(crate) shape_ids: Box<[Option<ShapeId>]>,
    /// The allocator pages for this span.
    pub(crate) pages: PageView,
    /// The dirty cards remembered for young tracing.
    pub(crate) dirty_cards: CardSet,
    /// Whether this span is already queued for dirty-card scanning.
    pub(crate) is_dirty_queued: bool,
}

impl SmallSpan {
    /// Set the logical byte length for one slot in this span.
    pub(crate) fn set_length(&mut self, slot_index: usize, length: usize) {
        if let Some(entry) = self.lengths.get_mut(slot_index) {
            *entry = length;
        }
    }

    /// Set the shape id for one slot in this span.
    pub(crate) fn set_shape_id(&mut self, slot_index: usize, shape_id: Option<ShapeId>) {
        if let Some(entry) = self.shape_ids.get_mut(slot_index) {
            *entry = shape_id;
        }
    }

    /// Clear every mark bit in this span.
    pub(crate) fn clear_marks(&mut self) {
        self.marked.clear_all();
    }
}
