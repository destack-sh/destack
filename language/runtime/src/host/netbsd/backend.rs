use crate::host::{HostBackend, Platform};

/// NetBSD host implementation.
#[derive(Debug, Default)]
pub(crate) struct NetBsdHost;

impl NetBsdHost {
    /// Create one NetBSD host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for NetBsdHost {
    fn platform(&self) -> Platform {
        Platform::NetBsd
    }
}
