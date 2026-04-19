use crate::{FrameImage, ResumePointId, RunStats};

/// One pending call transfer captured on a suspended frame.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CallTransfer {
    /// Branch to one normal or unwind continuation after the call completes.
    Branch {
        /// The resume point to enter when the callee returns normally.
        normal_resume_point: ResumePointId,
        /// The resume point to enter when the callee throws.
        unwind_resume_point: ResumePointId,
    },
}

/// One pending control transfer captured on a suspended frame.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ControlTransfer {
    /// One pending call continuation.
    Call(CallTransfer),
}

/// One durable suspended execution image.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Continuation {
    /// The isolate identity used to validate resumption.
    pub isolate_id: u64,
    /// The captured frames from outermost to innermost.
    pub frames: Vec<FrameImage>,
    /// The run statistics captured at suspension.
    pub stats: RunStats,
}
