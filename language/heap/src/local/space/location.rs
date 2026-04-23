use super::{HeapYoungId, LargeEntryId};
use crate::HeapReference;
use crate::allocator::SpanSlot;

/// One resolved heap entry location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum HeapStorage {
    /// One young-space entry.
    Young(HeapYoungId),
    /// One small-space entry stored in one span slot.
    Small(SpanSlot),
    /// One entry stored in heap large space.
    Large(LargeEntryId),
}

/// One physical page owner in local heap space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum HeapPageOwner {
    /// One young-space page and its logical page index.
    Young {
        /// The logical page index inside young space.
        logical_page_index: usize,
    },
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

/// One resolved heap location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeapLocation {
    /// The owning heap location.
    pub(crate) storage: HeapStorage,
    /// The base reference for the owning allocation.
    pub(crate) base: HeapReference,
    /// The byte offset from the base allocation.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning allocation.
    pub(crate) byte_len: usize,
}
