use crate::host::{HostBackend, Platform};

/// Linux host implementation.
#[derive(Debug, Default)]
pub(crate) struct LinuxHost;

impl LinuxHost {
    /// Create one Linux host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for LinuxHost {
    fn platform(&self) -> Platform {
        Platform::Linux
    }
}
