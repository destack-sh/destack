use serde::{Deserialize, Serialize};

use super::SharedLargeBlockId;
use crate::SharedHeapReference;
use crate::allocator::Slot;

/// One shared heap block storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SharedHeapStorage {
    /// One small-space block stored in one span slot.
    SmallSlot(Slot),
    /// One block stored in shared heap large space.
    LargeBlock(SharedLargeBlockId),
}

/// One page map entry in shared heap space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SharedHeapPageMapEntry {
    /// One small-span page and its logical page index.
    SmallSpan {
        /// The owning span index.
        span_index: usize,
        /// The logical page index inside the span.
        logical_page_index: usize,
    },
    /// One large-block page and its logical page index.
    LargeBlock {
        /// The owning large-block id.
        block_id: SharedLargeBlockId,
        /// The logical page index inside the large block.
        logical_page_index: usize,
    },
}

/// One resolved shared heap extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SharedHeapExtent {
    /// The shared heap block storage.
    pub(crate) storage: SharedHeapStorage,
    /// The base reference for the owning block.
    pub(crate) base: SharedHeapReference,
    /// The byte offset from the base block.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning block.
    pub(crate) byte_len: usize,
}
