use serde::{Deserialize, Serialize};

use super::LargeAllocationId;
use crate::allocator::SpanSlot;

/// One raw allocation place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RawPlace {
    /// One small-space allocation stored in one span slot.
    Small(SpanSlot),
    /// One allocation stored in raw large space.
    Large(LargeAllocationId),
}

/// One page map entry in local raw space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RawPageMapEntry {
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
        allocation_id: LargeAllocationId,
        /// The logical page index inside the large allocation.
        logical_page_index: usize,
    },
}

/// One resolved raw location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RawLocation {
    /// The raw allocation place.
    pub(crate) place: RawPlace,
    /// The base pointer for the owning allocation.
    pub(crate) base: crate::RawPointer,
    /// The byte offset from the base allocation.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning allocation.
    pub(crate) byte_len: usize,
}
