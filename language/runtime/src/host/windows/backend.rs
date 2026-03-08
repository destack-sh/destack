use super::message as windows_message;
use crate::diagnostic::RuntimeResult;
use crate::host::{HostBackend, Platform};

/// Windows host implementation.
#[derive(Debug, Default)]
pub(crate) struct WindowsHost;

impl WindowsHost {
    /// Create one Windows host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for WindowsHost {
    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn process_native_ingress(&self) -> RuntimeResult<bool> {
        // service one ready slice of the Win32 message queue
        Ok(windows_message::process_ingress_ready(true))
    }
}
