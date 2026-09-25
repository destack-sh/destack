use serde::{Deserialize, Serialize};
use tspp_memory::MemoryRange;
use tspp_serde::Reflect;

use crate::{Bitmap, SmallSpanClass};

/// One live heap span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallSpan {
    /// The first byte offset inside heap storage.
    pub(crate) first_offset: usize,
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
    /// The slots managed storage referenced, whose release waits for the collector.
    pub(crate) retained: Bitmap,
    /// The released slots whose values moved out, freed by the collector without their drop plan.
    pub(crate) empty: Bitmap,
    /// The mark epoch represented by this span's mark bitmap.
    pub(crate) mark_epoch: u64,
    /// The memory pages for this span.
    pub(crate) pages: MemoryRange,
}

impl SmallSpan {
    /// Ensure the mark bitmap represents one mark epoch.
    pub(crate) fn ensure_mark_epoch(&mut self, mark_epoch: u64) {
        if self.mark_epoch == mark_epoch {
            return;
        }

        self.marked.clear_all();
        self.mark_epoch = mark_epoch;
    }

    /// Return whether one slot is free to reserve.
    pub(crate) fn has_free_slot(&self) -> bool {
        self.free_cursor < self.slot_count
    }

    /// Return the base byte offset of one slot inside heap storage.
    pub(crate) fn slot_offset(&self, slot_index: usize) -> usize {
        self.first_offset + slot_index * self.class.size_class()
    }
}

/// One frozen heap span image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct SmallSpanImage {
    /// The first byte offset inside heap storage.
    pub first_offset: usize,
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
    /// The slots managed storage referenced.
    pub retained: Bitmap,
    /// The released slots whose values moved out.
    pub empty: Bitmap,
}
