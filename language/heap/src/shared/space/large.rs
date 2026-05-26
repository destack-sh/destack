use serde::{Deserialize, Serialize};

use destack_mir::TraceMap;

use crate::allocator::PageRun;
use crate::{HeapError, HeapResult};

/// One live shared heap large allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedLargeAllocation {
    /// Whether this large-allocation slot is live.
    pub(crate) is_live: bool,
    /// The first byte offset inside shared heap space.
    pub(crate) first_offset: usize,
    /// The logical byte length of this allocation.
    pub(crate) len: usize,
    /// The allocator pages for this allocation.
    pub(crate) pages: PageRun,
    /// The trace map for this allocation.
    pub(crate) trace_map: TraceMap,
    /// Whether this allocation is marked in the active cycle.
    pub(crate) is_marked: bool,
}

impl SharedLargeAllocation {
    /// Retire this shared heap large-allocation slot.
    pub(crate) fn retire(&mut self) {
        self.is_live = false;
        self.first_offset = 0;
        self.len = 0;
        self.pages = PageRun::empty();
        self.is_marked = false;
    }
}

/// One stable shared heap large-allocation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SharedLargeAllocationId(u64);

impl SharedLargeAllocationId {
    /// Create one shared heap large-allocation identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the shared heap large-allocation identifier value.
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

/// One frozen shared heap large-allocation image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapLargeAllocationImage {
    /// Whether this allocation slot is live.
    pub is_live: bool,
    /// The first byte offset inside shared heap space.
    pub first_offset: usize,
    /// The logical byte length of this allocation.
    pub len: usize,
    /// The allocator pages for this allocation.
    pub pages: PageRun,
    /// The trace map for this allocation.
    pub trace_map: TraceMap,
}
