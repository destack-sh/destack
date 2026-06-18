use std::sync::Arc;

use destack_native as native;
use destack_vm as vm;
use serde::{Deserialize, Serialize};

use super::ExecutorId;

/// Immutable execution image for one executor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Image {
    /// Immutable VM execution image.
    Vm {
        /// Executor that owns this image.
        executor: ExecutorId,
        /// Immutable VM execution image.
        image: Arc<vm::MachineImage>,
    },
    /// Immutable native execution image.
    Native {
        /// Executor that owns this image.
        executor: ExecutorId,
        /// Immutable native execution image.
        image: native::Image,
    },
}

impl Image {
    /// Return the executor that owns this image.
    pub const fn executor(&self) -> ExecutorId {
        match self {
            Self::Vm { executor, .. } | Self::Native { executor, .. } => *executor,
        }
    }
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        let left = image_bytes(self);
        let right = image_bytes(other);

        left.is_ok() && left == right
    }
}

impl Eq for Image {}

/// Serialize one executor image for exact equality checks.
fn image_bytes(image: &Image) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(image)
}
