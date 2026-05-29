use serde::{Deserialize, Serialize};

use super::LargeBlockId;
use crate::HeapReference;
use crate::allocator::Slot;

/// One heap allocation place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum HeapPlace {
    /// One young space block at a base offset.
    YoungRange {
        /// The block base byte offset inside young space.
        first_offset: usize,
    },
    /// One fixed-size young block stored in one span slot.
    YoungSlot(Slot),
    /// One mature small-space block stored in one span slot.
    MatureSlot(Slot),
    /// One block stored in heap large space.
    LargeBlock(LargeBlockId),
}

/// One page map entry in heap storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum HeapPageMapEntry {
    /// One young space page and its logical page index.
    Young {
        /// The logical page index inside young space.
        logical_page_index: usize,
    },
    /// One small-span page and its logical page index.
    MatureSpan {
        /// The owning span index.
        span_index: usize,
        /// The logical page index inside the span.
        logical_page_index: usize,
    },
    /// One large-block page and its logical page index.
    LargeBlock {
        /// The owning large-block id.
        block_id: LargeBlockId,
        /// The logical page index inside the large block.
        logical_page_index: usize,
    },
}

/// One resolved heap extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeapExtent {
    /// The heap allocation place.
    pub(crate) storage: HeapPlace,
    /// The base reference for the owning block.
    pub(crate) base: HeapReference,
    /// The byte offset from the base block.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning block.
    pub(crate) byte_len: usize,
}
