use serde::{Deserialize, Serialize};
use tspp_program as program;
use tspp_serde::Reflect;

use crate::runtime::RuntimeId;
use crate::worker::WorkerId;

/// Runtime breakpoint definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Breakpoint {
    /// Stable breakpoint identifier.
    pub id: program::BreakpointId,
    /// Executable Program point selected by this breakpoint.
    pub filter: PointFilter,
    /// Whether this breakpoint can currently match.
    pub is_enabled: bool,
}

/// One filtered Program point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct PointFilter {
    /// Runtime that owns the executing Program.
    pub runtime_id: Option<RuntimeId>,
    /// Worker that executes the Program point.
    pub worker_id: Option<WorkerId>,
    /// The selected Program point.
    pub point: program::ProgramPoint,
}

impl Breakpoint {
    /// Create one enabled breakpoint.
    pub const fn new(id: program::BreakpointId, filter: PointFilter) -> Self {
        Self {
            id,
            filter,
            is_enabled: true,
        }
    }
}

impl PointFilter {
    /// Return whether this filter can select points from one Worker.
    pub fn selects_worker(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> bool {
        let runtime_matches = self
            .runtime_id
            .is_none_or(|selected| selected == runtime_id);
        let worker_matches = self.worker_id.is_none_or(|selected| selected == worker_id);

        runtime_matches && worker_matches
    }
}
