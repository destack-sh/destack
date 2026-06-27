use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_heap::{HeapOptions, SharedHeapOptions};
use destack_mir::TargetLayout;

/// Serialized program compatibility header.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ProgramHeader {
    /// Target ABI layout used by program layouts and pointer-sized integer types.
    pub target_layout: TargetLayout,
    /// Local heap geometry used by lowered allocation plans.
    pub local_heap: HeapOptions,
    /// Shared heap geometry used by lowered allocation plans.
    pub shared_heap: SharedHeapOptions,
}

impl ProgramHeader {
    /// Create a program header.
    pub fn new(
        target_layout: TargetLayout,
        local_heap: HeapOptions,
        shared_heap: SharedHeapOptions,
    ) -> Self {
        Self {
            target_layout,
            local_heap,
            shared_heap,
        }
    }
}
