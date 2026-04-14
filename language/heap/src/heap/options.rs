use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use super::super::SizeClassTable;
use super::{
    DEFAULT_MANAGED_REFERENCE_BYTES, DEFAULT_PAGE_BYTES, DEFAULT_SMALL_BYTES, DEFAULT_YOUNG_BYTES,
};

/// Heap layout validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeapLayoutError {
    /// The configured managed-reference width is unsupported.
    UnsupportedManagedReferenceWidth { bytes: u8 },
}

impl Display for HeapLayoutError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedManagedReferenceWidth { bytes } => {
                write!(
                    formatter,
                    "unsupported managed reference width for heap layout: {bytes}"
                )
            }
        }
    }
}

impl Error for HeapLayoutError {}

/// Check one encoded managed-reference byte width.
pub fn check_managed_reference_bytes(managed_reference_bytes: u8) -> Result<u8, HeapLayoutError> {
    match managed_reference_bytes {
        4 | 8 => Ok(managed_reference_bytes),
        _ => Err(HeapLayoutError::UnsupportedManagedReferenceWidth {
            bytes: managed_reference_bytes,
        }),
    }
}

/// The layout configuration for one heap instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapLayout {
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
    /// The byte width for local heap pages.
    pub page_bytes: usize,
}

impl Default for HeapLayout {
    fn default() -> Self {
        Self {
            size_classes: SizeClassTable::default(),
            managed_reference_bytes: DEFAULT_MANAGED_REFERENCE_BYTES,
            managed_young_bytes: DEFAULT_YOUNG_BYTES,
            managed_small_bytes: DEFAULT_SMALL_BYTES,
            raw_small_bytes: DEFAULT_SMALL_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
        }
    }
}

impl HeapLayout {
    /// Check this heap layout configuration.
    pub fn check(&self) -> Result<(), HeapLayoutError> {
        check_managed_reference_bytes(self.managed_reference_bytes)?;

        Ok(())
    }
}
