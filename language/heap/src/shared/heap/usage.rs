use serde::{Deserialize, Serialize};

/// Exact shared-heap usage for one live shared heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedHeapUsage {
    /// The number of live shared heap blocks.
    pub allocation_count: usize,
    /// The logical live shared heap payload bytes.
    pub allocated_bytes: u64,
    /// The exact retained shared heap allocator-page bytes.
    pub retained_bytes: u64,
}

impl SharedHeapUsage {
    /// Return the exact total live allocated bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the exact total retained allocator-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }
}
