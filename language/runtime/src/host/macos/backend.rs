use crate::diagnostic::RuntimeResult;
use crate::host::apple::message as apple_message;
use crate::host::{HostBackend, Platform};

/// macOS host implementation.
#[derive(Debug, Default)]
pub(crate) struct MacosHost;

impl MacosHost {
    /// Create one macOS host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for MacosHost {
    fn platform(&self) -> Platform {
        Platform::MacOS
    }

    fn is_process_main_context(&self) -> bool {
        apple_message::is_process_main_context()
    }

    fn process_native_ingress(&self) -> RuntimeResult<bool> {
        // service one ready slice of the Apple run loop
        Ok(apple_message::process_ingress_ready(true))
    }
}
