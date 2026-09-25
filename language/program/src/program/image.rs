use std::ops::Range;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_memory::{MemoryMap, MemoryRange};
use tspp_serde::Reflect;

use crate::Result;

use super::{Context, FrameStateId, ProgramPoint};

/// One retained call chain independent of its execution engine.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ActivationImage {
    /// Completion selected when the root frame returns.
    completion: Completion,
    /// Retained frames in caller to active order.
    frames: Arc<[FrameImage]>,
    /// Packed live frame bytes inside the owning MemoryMap.
    memory: MemoryRange,
    /// Dynamically scoped context captured with the call chain.
    context: Context,
}

/// Root completion mode preserved by one retained activation.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Completion {
    /// Publish the function's returned value.
    Return,
    /// Discard the terminal value after cancellation cleanup.
    Cancel,
}

/// The tag bit that separates encoded frame addresses from world offsets and nullish words.
const FRAME_ADDRESS_TAG: u64 = 1 << 63;

/// One contiguous segment of frame bytes and the address it moves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameSegment {
    /// The addresses before the move.
    pub source: Range<usize>,
    /// The address of the first byte after the move.
    pub target: usize,
}

impl FrameSegment {
    /// Move one address inside this segment.
    pub fn relocate(&self, address: usize) -> Option<usize> {
        self.source
            .contains(&address)
            .then(|| self.target + address - self.source.start)
    }
}

impl ActivationImage {
    /// Encode one canonical frame offset as an image frame address.
    pub const fn encode_frame_address(offset: usize) -> u64 {
        offset as u64 | FRAME_ADDRESS_TAG
    }

    /// Decode one image frame address into its canonical frame offset.
    pub const fn decode_frame_address(address: u64) -> Option<usize> {
        if address & FRAME_ADDRESS_TAG == 0 {
            return None;
        }

        Some((address & !FRAME_ADDRESS_TAG) as usize)
    }

    /// Create one retained activation image.
    pub fn new(
        completion: Completion,
        frames: impl Into<Arc<[FrameImage]>>,
        memory: MemoryRange,
        context: Context,
    ) -> Self {
        Self {
            completion,
            frames: frames.into(),
            memory,
            context,
        }
    }

    /// Return the root completion mode.
    pub const fn completion(&self) -> Completion {
        self.completion
    }

    /// Return whether this image contains no active frames.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Return retained frames in caller to active order.
    pub fn frames(&self) -> &[FrameImage] {
        &self.frames
    }

    /// Return packed frame bytes inside the owning MemoryMap.
    pub const fn memory(&self) -> MemoryRange {
        self.memory
    }

    /// Return the dynamically scoped context captured with this call chain.
    pub const fn context(&self) -> Context {
        self.context
    }

    /// Borrow the mutable context root retained by this call chain.
    pub(crate) fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Inherit this activation into an already-forked memory map.
    pub fn inherit(&self) -> Self {
        Self {
            completion: self.completion,
            frames: self.frames.clone(),
            memory: self.memory,
            context: self.context,
        }
    }

    /// Release this activation's frame bytes.
    pub fn release(self, memory: &MemoryMap) -> Result<()> {
        memory.release(self.memory)?;

        Ok(())
    }
}

/// One frame inside a retained activation image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameImage {
    /// Canonical frame state defining the packed values.
    state: FrameStateId,
    /// Retained program operation cursor.
    point: ProgramPoint,
    /// Return behavior selected when this frame completes.
    return_to: FrameReturn,
}

impl FrameImage {
    /// Create one retained frame image.
    pub const fn new(state: FrameStateId, point: ProgramPoint, return_to: FrameReturn) -> Self {
        Self {
            state,
            point,
            return_to,
        }
    }

    /// Return the Program frame state.
    pub const fn state(self) -> FrameStateId {
        self.state
    }

    /// Return the retained program operation cursor.
    pub const fn point(self) -> ProgramPoint {
        self.point
    }

    /// Return the behavior selected when this frame completes.
    pub const fn return_to(self) -> FrameReturn {
        self.return_to
    }
}

/// Return behavior selected when one retained frame completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum FrameReturn {
    /// Exit the root call chain.
    Root,
    /// Return from one ordinary call.
    Call,
    /// Return from one destructor invocation.
    Drop {
        /// Retained continuation frames released after this destructor returns.
        frame_count: u16,
    },
}
