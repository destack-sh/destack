use crate::diagnostic::RuntimeResult;
use crate::host::macos::event;
use crate::host::{Host, Platform};

/// macOS host integration.
#[derive(Debug, Default)]
pub(crate) struct MacosHost;

impl MacosHost {
    /// Create one macOS host integration.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Host for MacosHost {
    fn platform(&self) -> Platform {
        Platform::MacOS
    }

    fn is_process_main_context(&self) -> bool {
        event::is_process_main_context()
    }

    fn advance_events(&self) -> RuntimeResult<()> {
        event::drain_ready_events();

        Ok(())
    }
}
