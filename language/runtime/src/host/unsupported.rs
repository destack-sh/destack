use crate::host::{HostBackend, Platform};
use crate::runtime::capability::PlatformCapabilitySet;

/// Unsupported host implementation.
#[derive(Debug, Default)]
pub(crate) struct UnsupportedHost;

impl UnsupportedHost {
    /// Create one unsupported host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for UnsupportedHost {
    fn platform(&self) -> Platform {
        Platform::Universal
    }

    fn host_capabilities(&self) -> PlatformCapabilitySet {
        PlatformCapabilitySet::new()
    }
}
