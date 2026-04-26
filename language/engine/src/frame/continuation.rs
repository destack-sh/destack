use crate::{FrameImage, ResumePointId};

/// Unique identity for one live engine instance.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct EngineId(pub u64);

impl EngineId {
    /// Create one engine identity.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw engine identity.
    pub const fn get(self) -> u64 {
        self.0
    }
}

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

/// One durable continuation image.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContinuationImage {
    /// The engine identity used to validate resumption.
    pub engine_id: EngineId,
    /// The captured frames from outermost to innermost.
    pub frames: Vec<FrameImage>,
}
