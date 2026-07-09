use destack_program as program;
use serde::{Deserialize, Serialize};

use crate::runtime::{RuntimeId, WorkerId};

/// Runtime watchpoint definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watchpoint {
    /// Stable watchpoint identifier.
    pub id: program::WatchpointId,
    /// Runtime that owns the executing program.
    pub runtime_id: Option<RuntimeId>,
    /// Worker that executes the access.
    pub worker_id: Option<WorkerId>,
    /// Memory access operation selected by the watchpoint.
    pub access: program::MemoryAccess,
    /// Memory target selected by the watchpoint.
    pub target: program::MemoryTarget,
    /// Whether this watchpoint can currently match.
    pub is_enabled: bool,
}

impl Watchpoint {
    /// Create one enabled watchpoint.
    pub const fn new(
        id: program::WatchpointId,
        runtime_id: Option<RuntimeId>,
        worker_id: Option<WorkerId>,
        access: program::MemoryAccess,
        target: program::MemoryTarget,
    ) -> Self {
        Self {
            id,
            runtime_id,
            worker_id,
            access,
            target,
            is_enabled: true,
        }
    }

    /// Return whether this watchpoint selects one worker execution.
    pub fn selects(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> bool {
        let runtime_matches = self.runtime_id.is_none_or(|target| target == runtime_id);
        let worker_matches = self.worker_id.is_none_or(|target| target == worker_id);

        runtime_matches && worker_matches
    }

    /// Return this watchpoint as an executable memory stop.
    pub const fn memory_stop(&self) -> program::MemoryStop {
        program::MemoryStop::new(self.id, self.access, self.target)
    }
}
