use serde::{Deserialize, Serialize};

use crate::alloc::PageMap;

/// One frozen raw allocation root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AllocationImage {
    /// Whether this allocation slot is currently allocated.
    pub is_allocated: bool,
    /// The logical byte length of this allocation.
    pub len: usize,
    /// The arena pages for this allocation.
    pub pages: PageMap,
}

/// One stable raw allocation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationId(u64);

impl AllocationId {
    /// Create one raw allocation identifier.
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the raw allocation identifier value.
    pub const fn id(self) -> u64 {
        self.0
    }

    /// Return the zero-based allocation slot index.
    pub const fn index(self) -> usize {
        self.0.saturating_sub(1) as usize
    }
}

/// One live raw allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Allocation {
    /// Whether this allocation slot is currently allocated.
    pub(crate) is_allocated: bool,
    /// The logical byte length of this allocation.
    pub(crate) len: usize,
    /// The arena pages for this allocation.
    pub(crate) pages: PageMap,
}
