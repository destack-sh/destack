use serde::{Deserialize, Serialize};

use super::Heap;

/// Exact managed-space usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ManagedSpaceUsage {
    /// The number of live managed allocations.
    pub allocation_count: usize,
    /// The logical live managed payload bytes.
    pub allocated_bytes: u64,
    /// The exact active managed allocator bytes.
    pub active_bytes: u64,
    /// The exact mapped managed page-arena bytes.
    pub mapped_bytes: u64,
    /// The exact borrowed managed image bytes.
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
    /// The exact mapped raw page-arena bytes.
    pub mapped_bytes: u64,
    /// The exact borrowed raw image bytes.
    pub borrowed_bytes: u64,
}

/// Exact shared-memory usage for one live shared-memory space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedSpaceUsage {
    /// The number of live shared-memory regions.
    pub allocation_count: usize,
    /// The logical live shared-memory payload bytes.
    pub allocated_bytes: u64,
    /// The exact active shared allocator bytes.
    pub active_bytes: u64,
    /// The exact mapped shared page-arena bytes.
    pub mapped_bytes: u64,
    /// The exact borrowed shared image bytes.
    pub borrowed_bytes: u64,
}

/// Exact heap usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeapUsage {
    /// The exact managed-space usage.
    pub managed: ManagedSpaceUsage,
    /// The exact raw-space usage.
    pub raw: RawSpaceUsage,
}

impl HeapUsage {
    /// Return the exact total live allocated bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.managed
            .allocated_bytes
            .saturating_add(self.raw.allocated_bytes)
    }

    /// Return the exact total active heap bytes.
    pub fn active_bytes(&self) -> u64 {
        self.managed
            .active_bytes
            .saturating_add(self.raw.active_bytes)
    }

    /// Return the exact total mapped heap bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.managed
            .mapped_bytes
            .saturating_add(self.raw.mapped_bytes)
    }

    /// Return the exact total borrowed image bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        self.managed
            .borrowed_bytes
            .saturating_add(self.raw.borrowed_bytes)
    }
}

/// Exact memory-context usage across local and shared memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// The local heap usage.
    pub heap: HeapUsage,
    /// The world-shared memory usage.
    pub shared: SharedSpaceUsage,
}

impl MemoryUsage {
    /// Return the exact total live allocated bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.heap
            .allocated_bytes()
            .saturating_add(self.shared.allocated_bytes)
    }

    /// Return the exact total active bytes.
    pub fn active_bytes(&self) -> u64 {
        self.heap
            .active_bytes()
            .saturating_add(self.shared.active_bytes)
    }

    /// Return the exact total mapped bytes.
    pub fn mapped_bytes(&self) -> u64 {
        self.heap
            .mapped_bytes()
            .saturating_add(self.shared.mapped_bytes)
    }

    /// Return the exact total borrowed image bytes.
    pub fn borrowed_bytes(&self) -> u64 {
        self.heap
            .borrowed_bytes()
            .saturating_add(self.shared.borrowed_bytes)
    }
}

impl Heap {
    /// Return the number of allocated managed bytes.
    pub fn managed_allocated_bytes(&self) -> u64 {
        self.managed.allocated_bytes()
    }

    /// Return the exact live usage for this heap.
    pub fn usage(&self) -> HeapUsage {
        HeapUsage {
            managed: self.managed.usage(),
            raw: self.raw.usage(),
        }
    }

    /// Return the number of live managed allocations.
    pub fn managed_allocation_count(&self) -> usize {
        self.managed.allocation_count()
    }

    /// Return the number of live raw allocations.
    pub fn raw_allocation_count(&self) -> usize {
        self.raw.allocation_count()
    }
}
