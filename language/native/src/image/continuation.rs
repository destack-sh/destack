use destack_engine::Continuation as EngineContinuation;
use serde::{Deserialize, Serialize};

/// Native continuation materialized at a managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Continuation {
    /// The continuation reconstructed from native state.
    pub continuation: EngineContinuation,
}

impl Continuation {
    /// Create one continuation.
    pub const fn new(continuation: EngineContinuation) -> Self {
        Self { continuation }
    }
}
