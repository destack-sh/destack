use crate::host::{Host, Platform};

/// Linux host integration.
#[derive(Debug, Default)]
pub(crate) struct LinuxHost;

impl LinuxHost {
    /// Create one Linux host integration.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Host for LinuxHost {
    fn platform(&self) -> Platform {
        Platform::Linux
    }
}
