use std::sync::Arc;

use destack_memory::{MemoryMap, MemoryRange};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{Completion, FrameStateId, ProgramPoint};

/// One retained call chain independent of its execution engine.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ActivationImage {
    /// Completion selected when the root frame returns.
    completion: Completion,
    /// Retained frames in caller to active order.
    frames: Arc<[FrameImage]>,
    /// Packed live frame bytes inside the owning MemoryMap.
    memory: MemoryRange,
}

impl ActivationImage {
    /// Create one retained activation image.
    pub fn new(
        completion: Completion,
        frames: impl Into<Arc<[FrameImage]>>,
        memory: MemoryRange,
    ) -> Self {
        Self {
            completion,
            frames: frames.into(),
            memory,
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

    /// Inherit this activation into an already-forked memory map.
    pub fn inherit(&self) -> Self {
        Self {
            completion: self.completion,
            frames: self.frames.clone(),
            memory: self.memory,
        }
    }

    /// Release this activation's frame bytes.
    pub fn release(self, memory: &MemoryMap) -> crate::Result<()> {
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
    /// Transition from this frame to its caller.
    link: FrameLink,
}

impl FrameImage {
    /// Create one retained frame image.
    pub const fn new(state: FrameStateId, point: ProgramPoint, link: FrameLink) -> Self {
        Self { state, point, link }
    }

    /// Return the Program frame state.
    pub const fn state(self) -> FrameStateId {
        self.state
    }

    /// Return the retained program operation cursor.
    pub const fn point(self) -> ProgramPoint {
        self.point
    }

    /// Return the transition from this frame to its caller.
    pub const fn link(self) -> FrameLink {
        self.link
    }
}

/// Return behavior from one frame to its caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum FrameLink {
    /// Exit the root call chain.
    Root,
    /// Return from one ordinary call.
    Call,
    /// Return from one continuation transfer.
    Continuation,
    /// Return from one eager task execution.
    Task,
    /// Return from one destructor invocation.
    Drop {
        /// Retained continuation frames released after this destructor returns.
        frame_count: u16,
    },
}
