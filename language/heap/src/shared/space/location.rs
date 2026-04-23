use serde::{Deserialize, Serialize};

use super::SharedLargeEntryId;
use crate::allocator::SpanSlot;

/// One stable shared heap storage partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SharedHeapStorage {
    /// One small-space entry stored in one span slot.
    Small(SpanSlot),
    /// One entry stored in shared heap large space.
    Large(SharedLargeEntryId),
}

/// One physical page owner in shared heap space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SharedHeapPageOwner {
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
        entry_id: SharedLargeEntryId,
        /// The logical page index inside the large entry.
        logical_page_index: usize,
    },
}

/// One resolved shared heap location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SharedHeapLocation {
    /// The owning shared heap storage.
    pub(crate) storage: SharedHeapStorage,
    /// The base reference for the owning allocation.
    pub(crate) base: crate::SharedHeapReference,
    /// The byte offset from the base allocation.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning allocation.
    pub(crate) byte_len: usize,
}
