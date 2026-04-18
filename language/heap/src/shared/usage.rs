use serde::{Deserialize, Serialize};

use crate::HeapResult;
use crate::core::sum_bytes;

/// Exact shared managed-space usage for one live shared managed space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedManagedSpaceUsage {
    /// The number of live shared managed allocations.
    pub allocation_count: usize,
    /// The logical live shared managed payload bytes.
    pub allocated_bytes: u64,
    /// The exact active shared managed allocator bytes.
    pub active_bytes: u64,
    /// The exact mapped shared managed page-arena bytes.
    pub mapped_bytes: u64,
    /// The exact borrowed shared managed image bytes.
    pub borrowed_bytes: u64,
}

/// Exact shared raw-space usage for one live shared raw-space store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedRawSpaceUsage {
    /// The number of live shared raw-space entries.
    pub allocation_count: usize,
    /// The logical live shared raw-space payload bytes.
    pub allocated_bytes: u64,
    /// The exact active shared allocator bytes.
    pub active_bytes: u64,
    /// The exact mapped shared page-arena bytes.
    pub mapped_bytes: u64,
    /// The exact borrowed shared image bytes.
    pub borrowed_bytes: u64,
}

/// Exact shared-heap usage for one live shared heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedHeapUsage {
    /// The exact shared managed-space usage.
    pub managed: SharedManagedSpaceUsage,
    /// The exact shared raw-space usage.
    pub raw: SharedRawSpaceUsage,
}

impl SharedHeapUsage {
    /// Return the exact total live allocated bytes.
    pub fn allocated_bytes(&self) -> HeapResult<u64> {
        sum_bytes(self.managed.allocated_bytes, self.raw.allocated_bytes)
    }

    /// Return the exact total active bytes.
    pub fn active_bytes(&self) -> HeapResult<u64> {
        sum_bytes(self.managed.active_bytes, self.raw.active_bytes)
    }

    /// Return the exact total mapped bytes.
    pub fn mapped_bytes(&self) -> HeapResult<u64> {
        sum_bytes(self.managed.mapped_bytes, self.raw.mapped_bytes)
    }

    /// Return the exact total borrowed image bytes.
    pub fn borrowed_bytes(&self) -> HeapResult<u64> {
        sum_bytes(self.managed.borrowed_bytes, self.raw.borrowed_bytes)
    }
}
