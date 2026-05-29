use serde::{Deserialize, Serialize};

use super::Heap;

/// Exact heap-space usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeapSpaceUsage {
    /// The number of live heap blocks.
    pub allocation_count: usize,
    /// The logical live heap payload bytes.
    pub allocated_bytes: u64,
    /// The exact retained heap allocator-page bytes.
    pub retained_bytes: u64,
}

/// Exact raw-space usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RawSpaceUsage {
    /// The number of live raw blocks.
    pub allocation_count: usize,
    /// The logical live raw payload bytes.
    pub allocated_bytes: u64,
    /// The exact retained raw allocator-page bytes.
    pub retained_bytes: u64,
}

/// Exact heap usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeapUsage {
    /// The exact heap-space usage.
    pub heap: HeapSpaceUsage,
    /// The exact raw-space usage.
    pub raw: RawSpaceUsage,
}

impl HeapUsage {
    /// Return the exact total live allocated bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.heap.allocated_bytes + self.raw.allocated_bytes
    }

    /// Return the exact total retained allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.heap.retained_bytes + self.raw.retained_bytes
    }
}

impl Heap {
    /// Return the number of allocated heap bytes.
    pub fn heap_allocated_bytes(&self) -> u64 {
        self.heap.allocated_bytes()
    }

    /// Return the exact live usage for this heap.
    pub fn usage(&self) -> HeapUsage {
        HeapUsage {
            heap: self.heap.usage(),
            raw: self.raw.usage(),
        }
    }

    /// Return the number of live heap blocks.
    pub fn heap_allocation_count(&self) -> usize {
        self.heap.allocation_count()
    }

    /// Return the number of live raw blocks.
    pub fn raw_allocation_count(&self) -> usize {
        self.raw.allocation_count()
    }
}
