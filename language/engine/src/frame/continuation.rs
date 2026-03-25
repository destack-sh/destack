use crate::{ExecutionStats, FrameImage};

/// One durable suspended execution image.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Continuation {
    /// The isolate identity used to validate resumption.
    pub isolate_id: u64,
    /// The captured frames from outermost to innermost.
    pub frames: Vec<FrameImage>,
    /// The execution statistics captured at suspension.
    pub stats: ExecutionStats,
}
