use serde::{Deserialize, Serialize};

use crate::SharedRawPointer;

/// One page map entry in shared raw space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SharedRawPageMapEntry {
    /// The owning block index.
    pub(crate) block_index: usize,
    /// The logical page index inside the block.
    pub(crate) logical_page_index: usize,
}

/// One resolved shared raw extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SharedRawExtent {
    /// The owning block index.
    pub(crate) block_index: usize,
    /// The base pointer for the owning block.
    pub(crate) base: SharedRawPointer,
    /// The byte offset from the base block.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning block.
    pub(crate) byte_len: usize,
}
