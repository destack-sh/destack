use serde::{Deserialize, Serialize};
use tspp_program as program;
use tspp_serde::Reflect;

use crate::runtime::RuntimeId;
use crate::worker::WorkerId;

/// Runtime watchpoint definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Watchpoint {
    /// Stable watchpoint identifier.
    pub id: program::WatchpointId,
    /// Memory access selected by this watchpoint.
    pub filter: MemoryFilter,
    /// Whether this watchpoint can currently match.
    pub is_enabled: bool,
}

/// One filtered memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemoryFilter {
    /// Runtime that owns the executing Program.
    pub runtime_id: Option<RuntimeId>,
    /// Worker that executes the memory access.
    pub worker_id: Option<WorkerId>,
    /// The selected memory access operation.
    pub access: program::MemoryAccess,
    /// The selected memory target.
    pub target: program::MemoryTarget,
}

impl Watchpoint {
    /// Create one enabled watchpoint.
    pub const fn new(id: program::WatchpointId, filter: MemoryFilter) -> Self {
        Self {
            id,
            filter,
            is_enabled: true,
        }
    }

    /// Return this watchpoint as an executable memory stop.
    pub const fn memory_stop(&self) -> program::MemoryStop {
        program::MemoryStop::new(self.id, self.filter.access, self.filter.target)
    }
}

impl MemoryFilter {
    /// Return whether this filter can select accesses from one Worker.
    pub fn selects_worker(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> bool {
        let runtime_matches = self
            .runtime_id
            .is_none_or(|selected| selected == runtime_id);
        let worker_matches = self.worker_id.is_none_or(|selected| selected == worker_id);

        runtime_matches && worker_matches
    }
}
