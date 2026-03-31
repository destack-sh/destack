use serde::{Deserialize, Serialize};

use super::super::SizeClassTable;

/// The default span width for local small-allocation spaces.
pub const DEFAULT_SMALL_BYTES: usize = 16 * 1024;

/// The default page width shared by spans and chunk-backed payloads.
pub const DEFAULT_PAGE_BYTES: usize = 4 * 1024;

/// The default byte width for managed young space.
pub const DEFAULT_YOUNG_BYTES: usize = 64 * 1024;

/// Validate one encoded managed-reference byte width.
pub fn validate_managed_reference_bytes(managed_reference_bytes: u8) -> u8 {
    match managed_reference_bytes {
        4 | 8 => managed_reference_bytes,
        _ => {
            panic!("unsupported managed reference width for heap layout: {managed_reference_bytes}")
        }
    }
}

/// Internal layout configuration for one heap instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapLayoutOptions {
    /// The configured small-allocation class table.
    pub size_classes: SizeClassTable,
    /// The encoded byte width for managed references inside traced payloads.
    pub managed_reference_bytes: u8,
    /// The byte width for managed young space.
    pub managed_young_bytes: usize,
    /// The byte width for managed small-allocation spans.
    pub managed_small_bytes: usize,
    /// The byte width for raw small-allocation spans.
    pub raw_small_bytes: usize,
    /// The byte width for local heap pages and page-sized chunks.
    pub page_bytes: usize,
}

impl Default for HeapLayoutOptions {
    fn default() -> Self {
        Self {
            size_classes: SizeClassTable::default(),
            managed_reference_bytes: 8,
            managed_young_bytes: DEFAULT_YOUNG_BYTES,
            managed_small_bytes: DEFAULT_SMALL_BYTES,
            raw_small_bytes: DEFAULT_SMALL_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
        }
    }
}

impl HeapLayoutOptions {
    /// Validate this heap layout configuration.
    pub fn validate(&self) {
        validate_managed_reference_bytes(self.managed_reference_bytes);
    }
}
