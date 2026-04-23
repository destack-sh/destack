use serde::{Deserialize, Serialize};

use crate::SharedRawPointer;

/// One physical page owner in shared raw space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SharedRawPageOwner {
    /// The owning entry index.
    pub(crate) entry_index: usize,
    /// The logical page index inside the entry.
    pub(crate) logical_page_index: usize,
}

/// One resolved shared raw location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SharedRawLocation {
    /// The owning entry index.
    pub(crate) entry_index: usize,
    /// The base pointer for the owning allocation.
    pub(crate) base: SharedRawPointer,
    /// The byte offset from the base allocation.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning allocation.
    pub(crate) byte_len: usize,
}
