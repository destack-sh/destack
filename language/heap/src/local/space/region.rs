use serde::{Deserialize, Serialize};

use super::LargeAllocationId;
use crate::HeapReference;
use crate::allocator::SpanSlot;

/// One heap allocation place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum HeapPlace {
    /// One allocation stored in young space.
    Young(YoungPlace),
    /// One small-space allocation stored in one span slot.
    Small(SpanSlot),
    /// One allocation stored in heap large space.
    Large(LargeAllocationId),
}

/// One young-space allocation place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum YoungPlace {
    /// One young-space allocation at a base offset.
    Range {
        /// The allocation base byte offset inside young space.
        first_offset: usize,
    },
    /// One fixed-size young allocation stored in one run slot.
    Slot(SpanSlot),
}

/// One page map entry in heap space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum HeapPageMapEntry {
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
    /// One large-allocation page and its logical page index.
    Large {
        /// The owning large-allocation id.
        allocation_id: LargeAllocationId,
        /// The logical page index inside the large allocation.
        logical_page_index: usize,
    },
}

/// One resolved heap region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeapRegion {
    /// The heap allocation place.
    pub(crate) place: HeapPlace,
    /// The base reference for the owning allocation.
    pub(crate) base: HeapReference,
    /// The byte offset from the base allocation.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning allocation.
    pub(crate) byte_len: usize,
}
