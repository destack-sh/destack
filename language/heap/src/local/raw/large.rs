use serde::{Deserialize, Serialize};

use crate::allocator::PageRun;
use crate::{HeapError, HeapResult};

/// One live raw large allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeAllocation {
    /// Whether this large-allocation slot is live.
    pub(crate) is_live: bool,
    /// The first byte offset inside raw space.
    pub(crate) first_offset: usize,
    /// The logical byte length of this allocation.
    pub(crate) byte_len: usize,
    /// The allocator pages for this allocation.
    pub(crate) pages: PageRun,
}

impl LargeAllocation {
    /// Retire this raw large-allocation slot.
    pub(crate) fn retire(&mut self) {
        self.is_live = false;
        self.first_offset = 0;
        self.byte_len = 0;
        self.pages = PageRun::empty();
    }
}

/// One stable raw large-allocation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LargeAllocationId(u64);

impl LargeAllocationId {
    /// Create one raw large-allocation identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the raw large-allocation identifier value.
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

/// One frozen raw large-allocation image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LargeAllocationImage {
    /// Whether this allocation slot is live.
    pub is_live: bool,
    /// The first byte offset inside raw space.
    pub first_offset: usize,
    /// The logical byte length of this allocation.
    pub byte_len: usize,
    /// The full byte payload for this allocation.
    pub bytes: Box<[u8]>,
}
