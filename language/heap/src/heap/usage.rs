use serde::{Deserialize, Serialize};

/// Exact managed-space usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ManagedSpaceUsage {
    /// The number of live managed allocations.
    pub allocation_count: usize,
    /// The logical live managed allocation bytes.
    pub allocation_bytes: u64,
    /// The exact retained managed heap bytes.
    pub retained_bytes: u64,
}

/// Exact raw-space usage for one live heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RawSpaceUsage {
    /// The number of live raw allocations.
    pub allocation_count: usize,
    /// The logical live raw allocation bytes.
    pub allocation_bytes: u64,
    /// The exact retained raw heap bytes.
    pub retained_bytes: u64,
}

/// Exact shared-memory usage for one live shared-memory space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SharedSpaceUsage {
    /// The number of live shared-memory regions.
    pub allocation_count: usize,
    /// The logical live shared-memory bytes.
    pub allocation_bytes: u64,
    /// The exact retained shared-memory bytes.
    pub retained_bytes: u64,
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

/// Exact memory-context usage across local and shared memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// The local heap usage.
    pub heap: HeapUsage,
    /// The world-shared memory usage.
    pub shared: SharedSpaceUsage,
}
