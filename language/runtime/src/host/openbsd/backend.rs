use crate::host::{HostBackend, Platform};

/// OpenBSD host implementation.
#[derive(Debug, Default)]
pub(crate) struct OpenBsdHost;

impl OpenBsdHost {
    /// Create one OpenBSD host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for OpenBsdHost {
    fn platform(&self) -> Platform {
        Platform::OpenBsd
    }
}
