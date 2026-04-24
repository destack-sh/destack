use destack_mir as mir;
use serde::{Deserialize, Serialize};

/// Pointer to one frame-owned stack allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StackPointer {
    /// The frame depth in the active call stack.
    pub frame_idx: usize,
    /// The stack-allocation slot inside that frame.
    pub slot: usize,
    /// The byte offset inside the stack allocation.
    pub byte_offset: usize,
}

impl StackPointer {
    /// Create a stack pointer for one whole allocation.
    #[inline]
    pub fn new(frame_idx: usize, slot: usize) -> Self {
        Self {
            frame_idx,
            slot,
            byte_offset: 0,
        }
    }

    /// Create a stack pointer with one byte offset.
    #[inline]
    pub fn with_offset(frame_idx: usize, slot: usize, byte_offset: usize) -> Self {
        Self {
            frame_idx,
            slot,
            byte_offset,
        }
    }
}

/// Pointer to one frame slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FramePointer {
    /// The frame depth in the active call stack.
    pub frame_idx: usize,
    /// The slot index inside that frame.
    pub slot: usize,
    /// The byte offset inside the slot value.
    pub byte_offset: usize,
}

impl FramePointer {
    /// Create a frame pointer for one whole slot value.
    #[inline]
    pub fn new(frame_idx: usize, slot: usize) -> Self {
        Self {
            frame_idx,
            slot,
            byte_offset: 0,
        }
    }

    /// Create a frame pointer with one byte offset.
    #[inline]
    pub fn with_offset(frame_idx: usize, slot: usize, byte_offset: usize) -> Self {
        Self {
            frame_idx,
            slot,
            byte_offset,
        }
    }
}

/// Pointer to one static value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StaticPointer {
    /// The static identifier.
    pub id: mir::LocalNodeId<mir::Global>,
    /// The byte offset inside the static value.
    pub byte_offset: usize,
}
