use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Exact shared-heap usage for one live shared heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Reflect)]
pub struct SharedHeapUsage {
    /// The number of live shared heap blocks.
    pub allocation_count: usize,
    /// The logical live shared heap payload bytes.
    pub allocated_bytes: u64,
    /// The exact retained shared heap memory-page bytes.
    pub retained_bytes: u64,
}

impl SharedHeapUsage {
    /// Return the exact total live allocated bytes.
    pub fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the exact total retained memory-page bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }
}
