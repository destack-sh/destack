use crate::host::{Host, Platform};

/// Windows host integration.
#[derive(Debug, Default)]
pub(crate) struct WindowsHost;

impl WindowsHost {
    /// Create one Windows host integration.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Host for WindowsHost {
    fn platform(&self) -> Platform {
        Platform::Windows
    }
}
