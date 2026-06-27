use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::FrameStateId;

/// Durable continuation image captured at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ContinuationImage {
    /// The captured stack bytes.
    pub stack: StackImage,
    /// The captured frames from outermost to innermost.
    pub frames: Vec<FrameImage>,
}

/// Durable stack image captured at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct StackImage {
    /// The captured stack bytes.
    pub bytes: Vec<u8>,
}

impl StackImage {
    /// Create one empty stack image.
    pub fn empty() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Return the captured byte length.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Return whether this stack image has no captured bytes.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Append one frame byte range and return its stack offset.
    pub fn push_frame(&mut self, bytes: &[u8]) -> usize {
        let stack_offset = self.bytes.len();
        self.bytes.extend_from_slice(bytes);

        stack_offset
    }

    /// Return one captured frame byte range.
    pub fn frame_bytes(&self, stack_offset: usize, byte_len: usize) -> Option<&[u8]> {
        let end = stack_offset.checked_add(byte_len)?;

        self.bytes.get(stack_offset..end)
    }

    /// Return one captured frame byte range mutably.
    pub fn frame_bytes_mut(&mut self, stack_offset: usize, byte_len: usize) -> Option<&mut [u8]> {
        let end = stack_offset.checked_add(byte_len)?;

        self.bytes.get_mut(stack_offset..end)
    }
}

/// Durable frame image captured at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameImage {
    /// The captured frame state.
    pub frame_state: FrameStateId,
    /// The caller return frame state.
    pub return_state: Option<FrameStateId>,
    /// The byte offset inside the captured stack image.
    pub stack_offset: usize,
    /// The captured frame byte width.
    pub byte_len: usize,
}
