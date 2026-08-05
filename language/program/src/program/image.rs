use std::sync::Arc;

use destack_memory::{MemoryMap, MemoryRange};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

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

impl ActivationImage {
    /// Bias added to canonical frame addresses so live offsets never collide
    /// with the nullish words.
    pub const FRAME_ADDRESS_BIAS: u64 = 2;

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
