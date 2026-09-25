use serde::{Deserialize, Serialize};
use tspp_vm as vm;

/// Immutable state of one retained runtime machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MachineImage {
    /// Captured stopped fiber when one was retained.
    stopped: Option<vm::FiberImage>,
}

impl MachineImage {
    /// Create one captured machine image.
    pub(crate) fn new(stopped: Option<vm::FiberImage>) -> Self {
        Self { stopped }
    }

    /// Fork this machine image for one forked World.
    pub fn inherit(&self) -> Self {
        self.clone()
    }

    /// Return the captured stopped fiber when present.
    pub(crate) fn stopped(&self) -> Option<&vm::FiberImage> {
        self.stopped.as_ref()
    }

    /// Return whether this image contains no active execution.
    pub fn is_empty(&self) -> bool {
        self.stopped.is_none()
    }
}
