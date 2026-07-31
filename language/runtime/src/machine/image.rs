use destack_memory::{MemoryImage, MemoryRange};
use destack_program::{
    ActivationImage, Continuation, ContinuationId, ContinuationTable, FrameStateId, Program,
};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Immutable state of one retained runtime machine.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineImage {
    /// Captured first-class continuations.
    continuations: ContinuationTable,
    /// Captured engine-neutral execution state when present.
    activation: Option<ActivationImage>,
}

impl Clone for MachineImage {
    /// Share retained machine storage into one immutable image copy.
    fn clone(&self) -> Self {
        self.inherit()
    }
}

impl MachineImage {
    /// Create one captured machine image.
    pub(crate) fn new(
        continuations: ContinuationTable,
        activation: Option<ActivationImage>,
    ) -> Self {
        Self {
            continuations,
            activation,
        }
    }

    /// Fork this machine image for one forked World.
    pub fn inherit(&self) -> Self {
        Self {
            continuations: self.continuations.inherit(),
            activation: self.activation.as_ref().map(ActivationImage::inherit),
        }
    }

    /// Return captured engine-neutral execution state when present.
    pub(crate) const fn activation(&self) -> Option<&ActivationImage> {
        self.activation.as_ref()
    }

    /// Return whether this image contains no active execution.
    pub fn is_empty(&self) -> bool {
        self.activation.is_none()
    }

    /// Return the captured logical frame count.
    pub fn frame_count(&self) -> usize {
        self.activation
            .as_ref()
            .map_or(0, |activation| activation.frames().len())
    }

    /// Return one captured logical frame state.
    pub fn frame_state(&self, index: usize) -> Option<FrameStateId> {
        self.activation
            .as_ref()
            .and_then(|activation| activation.frames().get(index))
            .map(|frame| frame.state())
    }

    /// Project one captured logical frame into its canonical live value layout.
    pub fn frame_bytes(
        &self,
        program: &Program,
        memory: &MemoryImage,
        index: usize,
    ) -> RuntimeResult<Vec<u8>> {
        let range = self.frame_range(program, index)?;

        memory
            .read_bytes(range.offset, range.byte_len)
            .map_err(Box::<RuntimeError>::from)
    }

    /// Return the packed memory range for one retained logical frame.
    fn frame_range(&self, program: &Program, index: usize) -> RuntimeResult<MemoryRange> {
        let Some(activation) = &self.activation else {
            return Err(RuntimeError::Internal {
                message: "machine image has no retained activation".to_string(),
            }
            .boxed());
        };
        let frame = activation.frames().get(index).ok_or_else(|| {
            RuntimeError::Internal {
                message: "machine image frame index is out of bounds".to_string(),
            }
            .boxed()
        })?;
        let state = program.frame_state(frame.state()).ok_or_else(|| {
            RuntimeError::Internal {
                message: "machine image frame state is missing".to_string(),
            }
            .boxed()
        })?;
        let layout = program.frame_layout(state.layout).ok_or_else(|| {
            RuntimeError::Internal {
                message: "machine image frame layout is missing".to_string(),
            }
            .boxed()
        })?;

        // locate this frame after every aligned predecessor
        let mut byte_offset = 0usize;
        for prior in &activation.frames()[..index] {
            let state = program.frame_state(prior.state()).ok_or_else(|| {
                RuntimeError::Internal {
                    message: "machine image frame state is missing".to_string(),
                }
                .boxed()
            })?;
            let layout = program.frame_layout(state.layout).ok_or_else(|| {
                RuntimeError::Internal {
                    message: "machine image frame layout is missing".to_string(),
                }
                .boxed()
            })?;
            byte_offset = byte_offset.next_multiple_of(layout.alignment() as usize);
            byte_offset += layout.byte_len() as usize;
        }

        // project the logical frame into canonical activation storage
        byte_offset = byte_offset.next_multiple_of(layout.alignment() as usize);
        let byte_len = layout.byte_len() as usize;
        let activation_range = activation.memory();
        if byte_offset + byte_len > activation_range.byte_len {
            return Err(RuntimeError::Internal {
                message: "machine image frame bytes are out of bounds".to_string(),
            }
            .boxed());
        }

        Ok(MemoryRange {
            offset: activation_range.offset + byte_offset,
            byte_len,
        })
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
