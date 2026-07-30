use destack_memory::MemoryImage;
use destack_program::{Continuation, ContinuationId, ContinuationTable, FrameStateId, Program};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Immutable state of one retained runtime machine.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineImage {
    /// Captured first-class continuations.
    continuations: ContinuationTable,
    /// Captured bytecode execution state when bytecode is available.
    vm: Option<destack_vm::MachineImage>,
}

impl Clone for MachineImage {
    /// Share retained machine storage into one immutable image copy.
    fn clone(&self) -> Self {
        self.fork()
    }
}

impl MachineImage {
    /// Create one captured machine image.
    pub(crate) fn new(
        continuations: ContinuationTable,
        vm: Option<destack_vm::MachineImage>,
    ) -> Self {
        Self { continuations, vm }
    }

    /// Fork this machine image for one forked World.
    pub fn fork(&self) -> Self {
        Self {
            continuations: self.continuations.fork(),
            vm: self.vm.as_ref().map(destack_vm::MachineImage::fork),
        }
    }

    /// Return captured bytecode execution state when present.
    pub(crate) const fn vm(&self) -> Option<&destack_vm::MachineImage> {
        self.vm.as_ref()
    }

    /// Return whether this image contains no active execution.
    pub fn is_empty(&self) -> bool {
        self.vm
            .as_ref()
            .is_none_or(destack_vm::MachineImage::is_empty)
    }

    /// Return the captured physical frame count.
    pub fn frame_count(&self) -> usize {
        self.vm
            .as_ref()
            .map_or(0, destack_vm::MachineImage::frame_count)
    }

    /// Return one captured physical frame state.
    pub fn frame_state(&self, index: usize) -> Option<FrameStateId> {
        self.vm
            .as_ref()
            .and_then(|machine| machine.frame_state(index))
    }

    /// Project one captured physical frame into its canonical live value layout.
    pub fn frame_bytes(
        &self,
        memory: &MemoryImage,
        program: &Program,
        index: usize,
    ) -> RuntimeResult<Vec<u8>> {
        match &self.vm {
            Some(machine) => machine
                .frame_bytes(memory, program, index)
                .map_err(Box::<RuntimeError>::from),
            None => Err(RuntimeError::Internal {
                message: "machine image has no retained bytecode frame".to_string(),
            }
            .boxed()),
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
