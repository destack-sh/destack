use serde::{Deserialize, Serialize};

use super::super::SizeClassTable;

/// The default run width for local managed and raw spaces.
pub const DEFAULT_RUN_BYTES: usize = 16 * 1024;

/// The default chunk width for chunk-backed extents and shared regions.
pub const DEFAULT_CHUNK_BYTES: usize = 4 * 1024;

/// Internal layout configuration for one heap instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapLayoutOptions {
    /// The configured small-allocation class table.
    pub size_classes: SizeClassTable,
    /// The byte width for managed runs.
    pub managed_run_bytes: usize,
    /// The byte width for raw runs.
    pub raw_run_bytes: usize,
    /// The byte width for chunk-backed extent and shared leaves.
    pub chunk_bytes: usize,
}

impl Default for HeapLayoutOptions {
    fn default() -> Self {
        Self {
            size_classes: SizeClassTable::default(),
            managed_run_bytes: DEFAULT_RUN_BYTES,
            raw_run_bytes: DEFAULT_RUN_BYTES,
            chunk_bytes: DEFAULT_CHUNK_BYTES,
        }
    }
}
