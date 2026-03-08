use crate::host::{HostBackend, Platform};

/// Solaris host implementation.
#[derive(Debug, Default)]
pub(crate) struct SolarisHost;

impl SolarisHost {
    /// Create one Solaris host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for SolarisHost {
    fn platform(&self) -> Platform {
        Platform::Solaris
    }
}
