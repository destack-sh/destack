use serde::{Deserialize, Serialize};

use super::SharedLargeAllocationId;
use crate::allocator::SpanSlot;

/// One shared heap allocation place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SharedHeapPlace {
    /// One small-space allocation stored in one span slot.
    Small(SpanSlot),
    /// One allocation stored in shared heap large space.
    Large(SharedLargeAllocationId),
}

/// One page map entry in shared heap space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SharedHeapPageMapEntry {
    /// One small-span page and its logical page index.
    Small {
        /// The owning span index.
        span_index: usize,
        /// The logical page index inside the span.
        logical_page_index: usize,
    },
    /// One large-allocation page and its logical page index.
    Large {
        /// The owning large-allocation id.
        allocation_id: SharedLargeAllocationId,
        /// The logical page index inside the large allocation.
        logical_page_index: usize,
    },
}

/// One resolved shared heap location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SharedHeapLocation {
    /// The shared heap allocation place.
    pub(crate) place: SharedHeapPlace,
    /// The base reference for the owning allocation.
    pub(crate) base: crate::SharedHeapReference,
    /// The byte offset from the base allocation.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning allocation.
    pub(crate) byte_len: usize,
}
