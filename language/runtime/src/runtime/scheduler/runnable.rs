use destack_program as program;
use serde::{Deserialize, Serialize};

use crate::runtime::machine::Continuation;

/// Runnable continuation queued by the event loop.
#[derive(Debug)]
pub struct Runnable {
    /// Runnable identifier used for ordering and logging.
    pub id: RunnableId,
    /// Continuation to resume.
    pub continuation: Continuation,
    /// Resume payload passed back into the machine.
    pub resume_value: program::Value,
}

/// Opaque runnable identifier used by the event loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RunnableId(u64);

impl RunnableId {
    /// Create a new runnable identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw runnable identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}
