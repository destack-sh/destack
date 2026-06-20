use destack_native as native;
use destack_vm as vm;
use serde::{Deserialize, Serialize};

use super::MachineId;

/// Live runnable continuation owned by one machine.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: revisit large runtime continuation payloads
pub enum Continuation {
    /// VM continuation that resumes MIR execution.
    Vm {
        /// Machine that owns the continuation.
        machine: MachineId,
        /// VM continuation that resumes MIR execution.
        continuation: vm::Continuation,
    },
    /// Native continuation that resumes native execution.
    Native {
        /// Machine that owns the continuation.
        machine: MachineId,
        /// Native continuation that resumes native execution.
        continuation: native::Continuation,
    },
}

/// Durable continuation image owned by one machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContinuationImage {
    /// VM continuation image.
    Vm {
        /// Machine that owns the continuation image.
        machine: MachineId,
        /// VM continuation image.
        image: vm::ContinuationImage,
    },
    /// Native continuation image.
    Native {
        /// Machine that owns the continuation image.
        machine: MachineId,
        /// Native continuation image.
        image: native::Continuation,
    },
}

impl Continuation {
    /// Return the machine that owns this continuation.
    pub const fn machine(&self) -> MachineId {
        match self {
            Self::Vm { machine, .. } | Self::Native { machine, .. } => *machine,
        }
    }
}

impl ContinuationImage {
    /// Return the machine that owns this continuation image.
    pub const fn machine(&self) -> MachineId {
        match self {
            Self::Vm { machine, .. } | Self::Native { machine, .. } => *machine,
        }
    }
}
