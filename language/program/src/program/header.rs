use serde::{Deserialize, Serialize};

use destack_heap::{HeapOptions, SharedHeapOptions};

/// Serialized program compatibility header.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramHeader {
    /// Pointer byte width used by layouts and pointer-sized integer types.
    pub pointer_bytes: u8,
    /// Local heap geometry used by lowered allocation sites.
    pub local_heap: HeapOptions,
    /// Shared heap geometry used by lowered allocation sites.
    pub shared_heap: SharedHeapOptions,
}

impl ProgramHeader {
    /// Create a program header.
    pub fn new(pointer_bytes: u8, local_heap: HeapOptions, shared_heap: SharedHeapOptions) -> Self {
        Self {
            pointer_bytes,
            local_heap,
            shared_heap,
        }
    }
}
