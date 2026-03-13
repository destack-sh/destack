use serde::{Deserialize, Serialize};

/// Exact managed heap usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ManagedHeapUsage {
    /// The number of live managed allocations.
    pub allocation_count: usize,
    /// The logical live managed allocation bytes.
    pub allocation_bytes: u64,
    /// The exact retained managed heap bytes.
    pub retained_bytes: u64,
}

/// Exact raw heap usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RawHeapUsage {
    /// The number of live raw allocations.
    pub allocation_count: usize,
    /// The logical live raw allocation bytes.
    pub allocation_bytes: u64,
    /// The exact retained raw heap bytes.
    pub retained_bytes: u64,
}

/// Exact heap usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HeapUsage {
    /// The exact managed heap usage.
    pub managed: ManagedHeapUsage,
    /// The exact raw heap usage.
    pub raw: RawHeapUsage,
}

impl HeapUsage {
    /// Return the exact total live allocation bytes.
    pub fn allocation_bytes(&self) -> u64 {
        self.managed
            .allocation_bytes
            .saturating_add(self.raw.allocation_bytes)
    }

    /// Return the exact total retained heap bytes.
    pub fn retained_bytes(&self) -> u64 {
        self.managed
            .retained_bytes
            .saturating_add(self.raw.retained_bytes)
    }
}
