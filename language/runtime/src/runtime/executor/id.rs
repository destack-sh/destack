use serde::{Deserialize, Serialize};

/// Stable identifier for one worker-owned executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExecutorId(pub u64);

impl ExecutorId {
    /// Create one executor identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw executor identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}
