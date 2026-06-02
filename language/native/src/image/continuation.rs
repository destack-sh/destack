use destack_engine::MaterializedContinuation;
use serde::{Deserialize, Serialize};

/// Native continuation materialized at a managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Continuation {
    /// The continuation reconstructed from native state.
    pub continuation: MaterializedContinuation,
}

impl Continuation {
    /// Create one continuation.
    pub const fn new(continuation: MaterializedContinuation) -> Self {
        Self { continuation }
    }
}
