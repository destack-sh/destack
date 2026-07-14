use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::FrameStateId;

/// Durable suspended computation captured at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Continuation {
    /// The captured frames from outermost to innermost.
    pub frames: Vec<ContinuationFrame>,
    /// Captured frame bytes in durable continuation encoding.
    pub bytes: Vec<u8>,
}

impl Continuation {
    /// Create one empty continuation.
    pub fn empty() -> Self {
        Self {
            frames: Vec::new(),
            bytes: Vec::new(),
        }
    }

    /// Fork this continuation for multi-shot resumption.
    pub fn fork(&self) -> Self {
        self.clone()
    }

    /// Append one frame byte range and return its continuation offset.
    pub fn push_frame_bytes(&mut self, bytes: &[u8]) -> usize {
        let byte_offset = self.bytes.len();
        self.bytes.extend_from_slice(bytes);

        byte_offset
    }

    /// Return one captured frame byte range.
    pub fn frame_bytes(&self, frame: &ContinuationFrame) -> Option<&[u8]> {
        let end = frame.byte_offset.checked_add(frame.byte_len)?;

        self.bytes.get(frame.byte_offset..end)
    }

    /// Return one captured frame byte range mutably.
    pub fn frame_bytes_mut(&mut self, frame_index: usize) -> Option<&mut [u8]> {
        let frame = self.frames.get(frame_index)?;
        let end = frame.byte_offset.checked_add(frame.byte_len)?;

        self.bytes.get_mut(frame.byte_offset..end)
    }
}

/// Durable frame captured inside one continuation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ContinuationFrame {
    /// The captured frame state.
    pub frame_state: FrameStateId,
    /// The caller normal frame state.
    pub normal_state: Option<FrameStateId>,
    /// The caller unwind frame state.
    pub unwind_state: Option<FrameStateId>,
    /// The byte offset inside the continuation byte store.
    pub byte_offset: usize,
    /// The byte offset inside the worker stack range.
    pub stack_offset: usize,
    /// The captured frame byte width.
    pub byte_len: usize,
}

impl ContinuationFrame {
    /// Return the captured frame byte width.
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }
}
