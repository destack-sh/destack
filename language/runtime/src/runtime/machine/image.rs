use destack_program as program;
use destack_vm as vm;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::MachineId;

/// Immutable image for one machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Image {
    /// Immutable VM machine image.
    Vm {
        /// Machine that owns this image.
        machine: MachineId,
        /// Immutable VM machine image.
        image: Arc<vm::MachineImage>,
    },
    /// Immutable native execution image.
    Native {
        /// Machine that owns this image.
        machine: MachineId,
        /// Immutable VM fallback machine image.
        vm: Arc<vm::MachineImage>,
    },
}

impl Image {
    /// Return the machine that owns this image.
    pub const fn machine(&self) -> MachineId {
        match self {
            Self::Vm { machine, .. } | Self::Native { machine, .. } => *machine,
        }
    }

    /// Return active machine frames captured in this image.
    pub fn frames(&self) -> &[program::FrameImage] {
        match self {
            Self::Vm { image, .. } | Self::Native { vm: image, .. } => image.frames.as_slice(),
        }
    }
}
