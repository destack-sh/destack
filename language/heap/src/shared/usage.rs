use serde::{Deserialize, Serialize};

use crate::heap::sum_bytes;
use crate::{HeapResult, HeapUsage};

/// Exact shared-space usage for one live shared-space store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedSpaceUsage {
    /// The number of live shared-space entries.
    pub allocation_count: usize,
    /// The logical live shared-space payload bytes.
    pub allocated_bytes: u64,
    /// The exact active shared allocator bytes.
    pub active_bytes: u64,
    /// The exact mapped shared page-arena bytes.
    pub mapped_bytes: u64,
    /// The exact borrowed shared image bytes.
    pub borrowed_bytes: u64,
}

/// Exact memory-context usage across local and shared space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MemoryContextUsage {
    /// The local heap usage.
    pub heap: HeapUsage,
    /// The world-shared space usage.
    pub shared: SharedSpaceUsage,
}

impl MemoryContextUsage {
    /// Return the exact total live allocated bytes.
    pub fn allocated_bytes(&self) -> HeapResult<u64> {
        sum_bytes(self.heap.allocated_bytes()?, self.shared.allocated_bytes)
    }

    /// Return the exact total active bytes.
    pub fn active_bytes(&self) -> HeapResult<u64> {
        sum_bytes(self.heap.active_bytes()?, self.shared.active_bytes)
    }

    /// Return the exact total mapped bytes.
    pub fn mapped_bytes(&self) -> HeapResult<u64> {
        sum_bytes(self.heap.mapped_bytes()?, self.shared.mapped_bytes)
    }

    /// Return the exact total borrowed image bytes.
    pub fn borrowed_bytes(&self) -> HeapResult<u64> {
        sum_bytes(self.heap.borrowed_bytes()?, self.shared.borrowed_bytes)
    }
}
