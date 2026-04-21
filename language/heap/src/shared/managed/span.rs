use serde::{Deserialize, Serialize};

use crate::ShapeId;
use crate::arena::{Bitmap, PageView};

/// One frozen shared managed small-span root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedManagedSmallSpanImage {
    /// The size class for this span in bytes.
    pub size_class: usize,
    /// The number of slots in this span.
    pub slot_count: usize,
    /// The occupied slots in this span.
    pub occupied: Bitmap,
    /// The per-slot stable reference ids for this span.
    pub reference_ids: Box<[Option<u32>]>,
    /// The per-slot shape ids for this span.
    pub shape_ids: Box<[Option<u32>]>,
    /// The arena pages for this span.
    pub pages: PageView,
}

/// One live shared managed span.
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
    /// The per-slot stable reference ids for this span.
    pub(crate) reference_ids: Box<[Option<u32>]>,
    /// The per-slot shape ids for this span.
    pub(crate) shape_ids: Box<[Option<ShapeId>]>,
    /// The arena pages for this span.
    pub(crate) pages: PageView,
}

impl SharedSmallSpan {
    /// Set the reference id for one slot in this span.
    pub(crate) fn set_reference_id(&mut self, slot_index: usize, reference_id: Option<u32>) {
        self.reference_ids[slot_index] = reference_id;
    }

    /// Set the shape id for one slot in this span.
    pub(crate) fn set_shape_id(&mut self, slot_index: usize, shape_id: Option<ShapeId>) {
        self.shape_ids[slot_index] = shape_id;
    }
}
