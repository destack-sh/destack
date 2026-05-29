use serde::{Deserialize, Serialize};

use super::Heap;

/// Exact heap usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeapUsage {
    /// The number of live heap blocks.
    pub allocation_count: usize,
    /// The logical live heap payload bytes.
    pub allocated_bytes: u64,
    /// The exact retained heap allocator-page bytes.
    pub retained_bytes: u64,
}

impl HeapUsage {
    /// Return the exact total live allocated bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the exact total retained allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }
}

impl Heap {
    /// Return the number of allocated heap bytes.
    pub fn heap_allocated_bytes(&self) -> u64 {
        self.storage.allocated_bytes()
    }

    /// Return the exact live usage for this heap.
    pub fn usage(&self) -> HeapUsage {
        self.storage.usage()
    }

    /// Return the number of live heap blocks.
    pub fn heap_allocation_count(&self) -> usize {
        self.storage.allocation_count()
    }
}
