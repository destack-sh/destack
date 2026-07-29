use destack_memory::MemoryImage;
use destack_program::{Continuation, ContinuationId, ContinuationTable, FrameStateId, Program};
use serde::{Deserialize, Serialize};

use super::native;
use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Immutable state of one retained runtime machine.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineImage {
    /// Captured first-class continuations.
    continuations: ContinuationTable,
    /// Captured engine state.
    state: MachineStateImage,
}

/// Captured state for one concrete machine implementation.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MachineStateImage {
    /// Retained bytecode execution state.
    Vm(destack_vm::MachineImage),
    /// Retained native execution state.
    Native(native::MachineImage),
}

impl Clone for MachineImage {
    /// Share retained machine storage into one immutable image copy.
    fn clone(&self) -> Self {
        self.fork()
    }
}

impl Clone for MachineStateImage {
    /// Share retained engine storage into one immutable image copy.
    fn clone(&self) -> Self {
        match self {
            Self::Vm(machine) => Self::Vm(machine.fork()),
            Self::Native(machine) => Self::Native(*machine),
        }
    }
}

impl MachineImage {
    /// Create one captured machine image.
    pub(crate) fn new(continuations: ContinuationTable, state: MachineStateImage) -> Self {
        Self {
            continuations,
            state,
        }
    }

    /// Fork this machine image for one forked World.
    pub fn fork(&self) -> Self {
        Self {
            continuations: self.continuations.fork(),
            state: self.state.clone(),
        }
    }

    /// Return the captured concrete machine state.
    pub const fn state(&self) -> &MachineStateImage {
        &self.state
    }

    /// Return whether this image contains no active execution.
    pub fn is_empty(&self) -> bool {
        match &self.state {
            MachineStateImage::Vm(machine) => machine.is_empty(),
            MachineStateImage::Native(machine) => machine.is_empty(),
        }
    }

    /// Return the captured physical frame count.
    pub fn frame_count(&self) -> usize {
        match &self.state {
            MachineStateImage::Vm(machine) => machine.frame_count(),
            MachineStateImage::Native(machine) => machine.frame_count(),
        }
    }

    /// Return one captured physical frame state.
    pub fn frame_state(&self, index: usize) -> Option<FrameStateId> {
        match &self.state {
            MachineStateImage::Vm(machine) => machine.frame_state(index),
            MachineStateImage::Native(machine) => machine.frame_state(index),
        }
    }

    /// Project one captured physical frame into its canonical live value layout.
    pub fn frame_bytes(
        &self,
        memory: &MemoryImage,
        program: &Program,
        index: usize,
    ) -> RuntimeResult<Vec<u8>> {
        match &self.state {
            MachineStateImage::Vm(machine) => machine
                .frame_bytes(memory, program, index)
                .map_err(Box::<RuntimeError>::from),
            MachineStateImage::Native(machine) => machine.frame_bytes(index),
        }
    }

    /// Iterate over every live first-class continuation.
    pub(crate) fn continuations(&self) -> impl Iterator<Item = (ContinuationId, &Continuation)> {
        self.continuations.iter()
    }

    /// Return the captured continuation table.
    pub(crate) fn continuation_table(&self) -> &ContinuationTable {
        &self.continuations
    }
}
