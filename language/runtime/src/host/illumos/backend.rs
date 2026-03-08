use crate::host::{HostBackend, Platform};

/// Illumos host implementation.
#[derive(Debug, Default)]
pub(crate) struct IllumosHost;

impl IllumosHost {
    /// Create one Illumos host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for IllumosHost {
    fn platform(&self) -> Platform {
        Platform::Illumos
    }
}
