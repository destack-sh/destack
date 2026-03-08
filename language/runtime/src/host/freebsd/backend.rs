use crate::host::{HostBackend, Platform};

/// FreeBSD host implementation.
#[derive(Debug, Default)]
pub(crate) struct FreeBsdHost;

impl FreeBsdHost {
    /// Create one FreeBSD host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for FreeBsdHost {
    fn platform(&self) -> Platform {
        Platform::FreeBsd
    }
}
