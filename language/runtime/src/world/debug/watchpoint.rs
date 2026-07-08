use serde::{Deserialize, Serialize};

use crate::host::binding::BindingId;
use crate::runtime::{RuntimeId, WorkerId};

use super::{MemoryAccess, MemoryTarget};

/// Runtime watchpoint identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WatchpointId(u64);

/// Runtime watchpoint definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watchpoint {
    /// Stable watchpoint identifier.
    pub id: WatchpointId,
    /// Memory or binding target selected by this watchpoint.
    pub target: WatchpointTarget,
    /// Whether this watchpoint can currently match.
    pub is_enabled: bool,
}

/// Runtime event selected by one watchpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WatchpointTarget {
    /// Memory access selected by the watchpoint.
    Memory(MemoryWatchpoint),
    /// Binding call selected by the watchpoint.
    Binding(BindingWatchpoint),
}

/// Memory access selected by one watchpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryWatchpoint {
    /// Runtime that owns the executing program.
    pub runtime_id: Option<RuntimeId>,
    /// Worker that executes the access.
    pub worker_id: Option<WorkerId>,
    /// Memory access operation selected by the watchpoint.
    pub access: MemoryAccess,
    /// Memory target selected by the watchpoint.
    pub target: MemoryTarget,
}

/// Binding call selected by one watchpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BindingWatchpoint {
    /// Runtime that owns the executing program.
    pub runtime_id: Option<RuntimeId>,
    /// Worker that executes the binding.
    pub worker_id: Option<WorkerId>,
    /// Binding selected by this watchpoint.
    pub binding_id: BindingId,
}

impl WatchpointId {
    /// Create one watchpoint identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw watchpoint identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl Watchpoint {
    /// Create one enabled watchpoint.
    pub const fn new(id: WatchpointId, target: WatchpointTarget) -> Self {
        Self {
            id,
            target,
            is_enabled: true,
        }
    }
}
