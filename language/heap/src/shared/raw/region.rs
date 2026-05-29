use serde::{Deserialize, Serialize};

use crate::SharedRawPointer;

/// One page map entry in shared raw space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SharedRawPageMapEntry {
    /// The owning allocation index.
    pub(crate) allocation_index: usize,
    /// The logical page index inside the allocation.
    pub(crate) logical_page_index: usize,
}

/// One resolved shared raw region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SharedRawRegion {
    /// The owning allocation index.
    pub(crate) allocation_index: usize,
    /// The base pointer for the owning allocation.
    pub(crate) base: SharedRawPointer,
    /// The byte offset from the base allocation.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning allocation.
    pub(crate) byte_len: usize,
}
