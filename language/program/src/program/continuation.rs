use std::ops::Range;
use std::sync::Arc;

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::FrameStateId;

/// One suspended computation captured at managed safepoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Continuation {
    /// The captured frames from outermost to innermost.
    pub frames: Arc<[ContinuationFrame]>,
    /// The canonical frame bytes shared by continuation forks.
    pub bytes: Arc<[u8]>,
}

impl Continuation {
    /// Fork this continuation for multi-shot resumption.
    pub fn fork(&self) -> Self {
        self.clone()
    }

    /// Return the innermost captured frame.
    pub fn innermost(&self) -> Option<&ContinuationFrame> {
        self.frames.last()
    }

    /// Return one captured frame byte range.
    pub fn frame_bytes(&self, frame: &ContinuationFrame) -> Option<&[u8]> {
        self.bytes.get(frame.byte_range()?)
    }

    /// Return one captured frame byte range mutably.
    pub fn frame_bytes_mut(&mut self, frame_index: usize) -> Option<&mut [u8]> {
        let range = self.frames.get(frame_index)?.byte_range()?;
        let storage = Arc::make_mut(&mut self.bytes);

        storage.get_mut(range)
    }
}

/// One continuation under construction.
#[derive(Debug, Default)]
pub struct ContinuationBuilder {
    /// The captured frames from outermost to innermost.
    frames: Vec<ContinuationFrame>,
    /// The canonical frame bytes under construction.
    bytes: Vec<u8>,
}

impl ContinuationBuilder {
    /// Create one empty continuation builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one captured frame.
    pub fn push(
        &mut self,
        frame_state: FrameStateId,
        normal_state: Option<FrameStateId>,
        unwind_state: Option<FrameStateId>,
        bytes: &[u8],
    ) {
        let byte_offset = self.bytes.len() as u64;
        let byte_len = bytes.len() as u64;
        self.bytes.extend_from_slice(bytes);

        // retain the frame state and its canonical byte range
        self.frames.push(ContinuationFrame {
            frame_state,
            normal_state,
            unwind_state,
            byte_offset,
            byte_len,
        });
    }

    /// Build one immutable forkable continuation.
    pub fn build(self) -> Continuation {
        Continuation {
            frames: self.frames.into(),
            bytes: self.bytes.into(),
        }
    }
}

/// One frame captured inside a continuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ContinuationFrame {
    /// The captured frame state.
    pub frame_state: FrameStateId,
    /// The caller normal frame state.
    pub normal_state: Option<FrameStateId>,
    /// The caller unwind frame state.
    pub unwind_state: Option<FrameStateId>,
    /// The byte offset in the continuation byte storage.
    pub byte_offset: u64,
    /// The captured frame byte width.
    pub byte_len: u64,
}

impl ContinuationFrame {
    /// Return the captured frame byte range for this host.
    pub fn byte_range(self) -> Option<Range<usize>> {
        let start = usize::try_from(self.byte_offset).ok()?;
        let byte_len = usize::try_from(self.byte_len).ok()?;
        let end = start.checked_add(byte_len)?;

        Some(start..end)
    }

    /// Return the captured frame byte width.
    pub const fn byte_len(self) -> u64 {
        self.byte_len
    }
}
