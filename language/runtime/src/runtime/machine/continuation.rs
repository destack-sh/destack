use destack_program as program;
use serde::{Deserialize, Serialize};

use super::MachineId;

/// Live runnable continuation owned by one machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Continuation {
    /// Machine that owns the continuation.
    pub machine: MachineId,
    /// Suspended program execution.
    pub program: program::Continuation,
}

impl Continuation {
    /// Return the machine that owns this continuation.
    pub const fn machine(&self) -> MachineId {
        self.machine
    }
}
