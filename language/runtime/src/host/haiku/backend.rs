use crate::host::{HostBackend, Platform};

/// Haiku host implementation.
#[derive(Debug, Default)]
pub(crate) struct HaikuHost;

impl HaikuHost {
    /// Create one Haiku host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for HaikuHost {
    fn platform(&self) -> Platform {
        Platform::Haiku
    }
}
