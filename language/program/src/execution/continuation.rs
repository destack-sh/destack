use crate::MaterializedFrame;
use serde::{Deserialize, Serialize};

/// Materialized continuation captured at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedContinuation {
    /// The frames from outermost to innermost.
    pub frames: Vec<MaterializedFrame>,
}
