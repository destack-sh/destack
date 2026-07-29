use destack_program::FrameStateId;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Immutable retained state of one native machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MachineImage;

impl MachineImage {
    /// Return whether this image contains no active execution.
    pub const fn is_empty(&self) -> bool {
        true
    }

    /// Return the captured physical frame count.
    pub const fn frame_count(&self) -> usize {
        0
    }

    /// Return one captured physical frame state.
    pub const fn frame_state(&self, _index: usize) -> Option<FrameStateId> {
        None
    }

    /// Reject frame projection because native retained frames are not implemented.
    pub fn frame_bytes(&self, _index: usize) -> RuntimeResult<Vec<u8>> {
        Err(RuntimeError::Internal {
            message: "native machine image contains no materialized frames".to_string(),
        }
        .boxed())
    }
}
