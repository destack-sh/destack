use destack_program as program;
use serde::{Deserialize, Serialize};

use crate::worker::WorkerId;
use crate::world::RuntimeId;

/// Runtime breakpoint definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Breakpoint {
    /// Stable breakpoint identifier.
    pub id: program::BreakpointId,
    /// Executable program point selected by this breakpoint.
    pub target: BreakpointTarget,
    /// Whether this breakpoint can currently match.
    pub is_enabled: bool,
}

/// Executable program point selected by one breakpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BreakpointTarget {
    /// Runtime that owns the executing program.
    pub runtime_id: Option<RuntimeId>,
    /// Worker that executes the program point.
    pub worker_id: Option<WorkerId>,
    /// Program point selected by the breakpoint.
    pub point: program::ProgramPoint,
}

impl Breakpoint {
    /// Create one enabled breakpoint.
    pub const fn new(id: program::BreakpointId, target: BreakpointTarget) -> Self {
        Self {
            id,
            target,
            is_enabled: true,
        }
    }
}

impl BreakpointTarget {
    /// Return whether this target selects one worker execution.
    pub fn selects(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> bool {
        let runtime_matches = self.runtime_id.is_none_or(|target| target == runtime_id);
        let worker_matches = self.worker_id.is_none_or(|target| target == worker_id);

        runtime_matches && worker_matches
    }
}
