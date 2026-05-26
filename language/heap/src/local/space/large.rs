use serde::{Deserialize, Serialize};

use destack_mir::TraceMap;

use super::CardSet;
use crate::allocator::PageRun;
use crate::{HeapError, HeapResult};

/// One frozen heap large-allocation image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LargeAllocationImage {
    /// Whether this allocation slot is live.
    pub is_live: bool,
    /// The first byte offset inside heap space.
    pub first_offset: usize,
    /// The logical byte length of this allocation.
    pub len: usize,
    /// The allocator pages for this allocation.
    pub pages: PageRun,
    /// The trace map for this allocation.
    pub trace_map: TraceMap,
}

/// One heap large-allocation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LargeAllocationId(u64);

impl LargeAllocationId {
    /// Create one heap large-allocation identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the heap large-allocation identifier value.
    pub(crate) const fn id(self) -> u64 {
        self.0
    }

    /// Return the zero-based large-allocation slot index.
    pub(crate) fn index(self) -> HeapResult<usize> {
        let Some(index) = self.0.checked_sub(1) else {
            return Err(HeapError::InvalidLargeAllocationId { id: self.0 });
        };

        Ok(index as usize)
    }
}

/// One live heap large allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeAllocation {
    /// Whether this large-allocation slot is live.
    pub(crate) is_live: bool,
    /// The first byte offset inside heap space.
    pub(crate) first_offset: usize,
    /// The logical byte length of this allocation.
    pub(crate) len: usize,
    /// The allocator pages for this allocation.
    pub(crate) pages: PageRun,
    /// The trace map for this allocation.
    pub(crate) trace_map: TraceMap,
    /// Whether this allocation is marked in the active cycle.
    pub(crate) is_marked: bool,
    /// The dirty cards remembered for young tracing.
    pub(crate) dirty_cards: CardSet,
    /// Whether this allocation is already queued for dirty-card scanning.
    pub(crate) is_dirty_queued: bool,
}

impl LargeAllocation {
    /// Retire this heap large-allocation slot.
    pub(crate) fn retire(&mut self) {
        self.is_live = false;
        self.first_offset = 0;
        self.len = 0;
        self.pages = PageRun::empty();
        self.is_marked = false;
        self.dirty_cards.clear();
        self.is_dirty_queued = false;
    }
}
