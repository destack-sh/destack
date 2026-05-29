use serde::{Deserialize, Serialize};

use super::CardSet;
use crate::SmallSpanClass;
use crate::allocator::{Bitmap, PageSpan};

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
    /// The mark epoch represented by this span's mark bitmap.
    pub(crate) mark_epoch: u64,
    /// The allocator pages for this span.
    pub(crate) pages: PageSpan,
    /// The dirty cards remembered for young tracing.
    pub(crate) dirty_cards: CardSet,
    /// Whether this span is already queued for dirty-card scanning.
    pub(crate) is_dirty_queued: bool,
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
}

/// One frozen heap span image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// The captured span bytes.
    pub bytes: Box<[u8]>,
}
