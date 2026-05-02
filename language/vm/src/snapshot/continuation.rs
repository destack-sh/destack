use serde::{Deserialize, Serialize};

use crate::interpreter::ExceptionalCall;
use destack_engine::{EngineId, FrameLayoutId, FrameStateId};

/// Durable continuation image captured from one yielded VM stack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationImage {
    /// The engine identity used to validate resumption.
    pub engine_id: EngineId,
    /// The captured frames from outermost to innermost.
    pub frames: Vec<ContinuationFrame>,
}

/// Durable frame image captured inside one VM continuation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationFrame {
    /// The frame layout used by this frame.
    pub frame_layout: FrameLayoutId,
    /// The logical frame state captured by this frame.
    pub frame_state: FrameStateId,
    /// The active exceptional call owned by this frame when another frame is active.
    pub exceptional_call: Option<ExceptionalCall>,
    /// The captured frame bytes.
    pub bytes: Vec<u8>,
}
