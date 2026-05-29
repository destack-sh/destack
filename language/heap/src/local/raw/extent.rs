use serde::{Deserialize, Serialize};

use super::LargeBlockId;
use crate::RawPointer;
use crate::allocator::Slot;

/// One raw block storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RawStorage {
    /// One small-space block stored in one span slot.
    SmallSlot(Slot),
    /// One block stored in raw large space.
    LargeBlock(LargeBlockId),
}

/// One page map entry in local raw space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RawPageMapEntry {
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
        block_id: LargeBlockId,
        /// The logical page index inside the large block.
        logical_page_index: usize,
    },
}

/// One resolved raw extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RawExtent {
    /// The raw block storage.
    pub(crate) storage: RawStorage,
    /// The base pointer for the owning block.
    pub(crate) base: RawPointer,
    /// The byte offset from the base block.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning block.
    pub(crate) byte_len: usize,
}
