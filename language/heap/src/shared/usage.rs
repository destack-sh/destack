use serde::{Deserialize, Serialize};

/// Exact shared heap-space usage for one live shared heap space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedHeapSpaceUsage {
    /// The number of live shared heap allocations.
    pub allocation_count: usize,
    /// The logical live shared heap payload bytes.
    pub allocated_bytes: u64,
    /// The exact retained shared heap allocator-page bytes.
    pub retained_bytes: u64,
}

/// Exact shared raw-space usage for one live shared raw-space store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedRawSpaceUsage {
    /// The number of live shared raw-space allocations.
    pub allocation_count: usize,
    /// The logical live shared raw-space payload bytes.
    pub allocated_bytes: u64,
    /// The exact retained shared raw allocator-page bytes.
    pub retained_bytes: u64,
}

/// Exact shared-heap usage for one live shared heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedHeapUsage {
    /// The exact shared heap-space usage.
    pub heap: SharedHeapSpaceUsage,
    /// The exact shared raw-space usage.
    pub raw: SharedRawSpaceUsage,
}

impl SharedHeapUsage {
    /// Return the exact total live allocated bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.heap.allocated_bytes + self.raw.allocated_bytes
    }

    /// Return the exact total retained allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.heap.retained_bytes + self.raw.retained_bytes
    }
}
