use std::num::NonZeroU64;

use destack_program as program;
use serde::{Deserialize, Serialize};

use crate::host::binding::BindingId;
use crate::runtime::{RuntimeId, WorkerId};

use super::{MemoryAccess, MemoryTarget};

/// Runtime probe identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProbeId(u64);

/// Runtime probe definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Probe {
    /// Stable probe identifier.
    pub id: ProbeId,
    /// Runtime event selected by this probe.
    pub target: ProbeTarget,
    /// Action to perform when this probe matches.
    pub action: ProbeAction,
    /// Whether this probe can currently match.
    pub is_enabled: bool,
}

/// Runtime event selected by one probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeTarget {
    /// Executable program point selected by the probe.
    Instruction(InstructionProbe),
    /// Memory access selected by the probe.
    Memory(MemoryProbe),
    /// Allocation event selected by the probe.
    Allocation(AllocationProbe),
    /// Frame event selected by the probe.
    Frame(FrameProbe),
    /// Binding call selected by the probe.
    Binding(BindingProbe),
}

/// Probe action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeAction {
    /// Emit one observation for each matching hit.
    Observe,
    /// Count matching hits.
    Count,
    /// Sample matching hits.
    Sample {
        /// Sampling interval in matching hits.
        interval: NonZeroU64,
    },
}

/// Instruction probe target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstructionProbe {
    /// Runtime that owns the executing program.
    pub runtime_id: Option<RuntimeId>,
    /// Worker that executes the program point.
    pub worker_id: Option<WorkerId>,
    /// Program point selected by the probe.
    pub point: program::ProgramPoint,
}

/// Memory probe target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryProbe {
    /// Runtime that owns the executing program.
    pub runtime_id: Option<RuntimeId>,
    /// Worker that executes the memory access.
    pub worker_id: Option<WorkerId>,
    /// Memory access operation selected by the probe.
    pub access: MemoryAccess,
    /// Memory target selected by the probe.
    pub target: MemoryTarget,
}

/// Allocation probe target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AllocationProbe {
    /// Runtime that owns the executing program.
    pub runtime_id: Option<RuntimeId>,
    /// Worker that performs the allocation.
    pub worker_id: Option<WorkerId>,
    /// Program point selected by this probe.
    pub point: Option<program::ProgramPoint>,
    /// Allocation type selected by this probe.
    pub ty: Option<program::TypeId>,
}

/// Frame probe target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameProbe {
    /// Runtime that owns the executing program.
    pub runtime_id: Option<RuntimeId>,
    /// Worker that owns the frame.
    pub worker_id: Option<WorkerId>,
    /// Frame event selected by this probe.
    pub event: FrameEvent,
    /// Function selected by this probe.
    pub function: Option<program::FunctionId>,
}

/// Binding probe target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BindingProbe {
    /// Runtime that owns the executing program.
    pub runtime_id: Option<RuntimeId>,
    /// Worker that calls the binding.
    pub worker_id: Option<WorkerId>,
    /// Binding selected by this probe.
    pub binding_id: Option<BindingId>,
}

/// Frame event selected by one probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameEvent {
    /// Frame entered execution.
    Enter,
    /// Frame returned normally.
    Return,
    /// Frame suspended into a continuation.
    Suspend,
    /// Frame resumed from a continuation.
    Resume,
}

impl ProbeId {
    /// Create one probe identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw probe identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl Probe {
    /// Create one enabled probe.
    pub const fn new(id: ProbeId, target: ProbeTarget, action: ProbeAction) -> Self {
        Self {
            id,
            target,
            action,
            is_enabled: true,
        }
    }
}
