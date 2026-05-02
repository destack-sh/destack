use destack_engine::EngineId;
use serde::{Deserialize, Serialize};

use crate::Continuation;

/// In-memory native engine image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Image {
    /// The engine identity captured by this image.
    pub engine_id: EngineId,
    /// The suspended continuation when execution is currently paused.
    pub continuation: Option<Continuation>,
}

impl Image {
    /// Create an empty native engine image.
    pub const fn empty(engine_id: EngineId) -> Self {
        Self {
            engine_id,
            continuation: None,
        }
    }
}
