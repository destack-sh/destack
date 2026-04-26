use serde::{Deserialize, Serialize};

use super::Heap;

/// Exact heap-space usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeapSpaceUsage {
    /// The number of live heap allocations.
    pub allocation_count: usize,
    /// The logical live heap payload bytes.
    pub allocated_bytes: u64,
    /// The exact active heap allocator bytes.
    pub active_bytes: u64,
    /// The exact mapped heap allocator page bytes.
    pub mapped_bytes: u64,
    /// The exact borrowed heap image bytes.
    pub borrowed_bytes: u64,
}

/// Exact raw-space usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RawSpaceUsage {
    /// The number of live raw allocations.
    pub allocation_count: usize,
    /// The logical live raw payload bytes.
    pub allocated_bytes: u64,
    /// The exact active raw allocator bytes.
    pub active_bytes: u64,
    /// The exact mapped raw allocator page bytes.
    pub mapped_bytes: u64,
    /// The exact borrowed raw image bytes.
    pub borrowed_bytes: u64,
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

    /// Return the exact total active heap bytes.
    pub fn active_bytes(&self) -> u64 {
        self.heap.active_bytes + self.raw.active_bytes
    }

    /// Return the exact total mapped heap bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.heap.mapped_bytes + self.raw.mapped_bytes
    }

    /// Return the exact total borrowed image bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        self.heap.borrowed_bytes + self.raw.borrowed_bytes
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

    /// Return the number of live heap allocations.
    pub fn heap_allocation_count(&self) -> usize {
        self.heap.allocation_count()
    }

    /// Return the number of live raw allocations.
    pub fn raw_allocation_count(&self) -> usize {
        self.raw.allocation_count()
    }
}
