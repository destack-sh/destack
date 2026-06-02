use crate::{EngineId, MaterializedFrame};
use serde::{Deserialize, Serialize};

/// Materialized continuation captured at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedContinuation {
    /// The engine id.
    pub engine_id: EngineId,
    /// The frames from outermost to innermost.
    pub frames: Vec<MaterializedFrame>,
}
