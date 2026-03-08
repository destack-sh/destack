use crate::host::{HostBackend, Platform};

/// DragonFly BSD host implementation.
#[derive(Debug, Default)]
pub(crate) struct DragonflyHost;

impl DragonflyHost {
    /// Create one DragonFly BSD host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for DragonflyHost {
    fn platform(&self) -> Platform {
        Platform::DragonFly
    }
}
