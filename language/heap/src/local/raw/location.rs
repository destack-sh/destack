use serde::{Deserialize, Serialize};

use super::LargeEntryId;
use crate::allocator::SpanSlot;

/// One stable raw storage partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RawStorage {
    /// One small-space entry stored in one span slot.
    Small(SpanSlot),
    /// One entry stored in raw large space.
    Large(LargeEntryId),
}

/// One physical page owner in local raw space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RawPageOwner {
    /// One small-span page and its logical page index.
    Small {
        /// The owning span index.
        span_index: usize,
        /// The logical page index inside the span.
        logical_page_index: usize,
    },
    /// One large-entry page and its logical page index.
    Large {
        /// The owning large-entry id.
        entry_id: LargeEntryId,
        /// The logical page index inside the large entry.
        logical_page_index: usize,
    },
}

/// One resolved raw location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RawLocation {
    /// The owning raw storage.
    pub(crate) storage: RawStorage,
    /// The base pointer for the owning allocation.
    pub(crate) base: crate::RawPointer,
    /// The byte offset from the base allocation.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning allocation.
    pub(crate) byte_len: usize,
}
