use crate::{EngineId, Frame};
use serde::{Deserialize, Serialize};

/// Captured continuation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Continuation {
    /// The engine id.
    pub engine_id: EngineId,
    /// The frames from outermost to innermost.
    pub frames: Vec<Frame>,
}
