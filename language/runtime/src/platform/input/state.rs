#[cfg(windows)]
use std::sync::{Arc, OnceLock};

#[cfg(windows)]
use super::host::WindowsRawInputRuntimeState;

/// Runtime-owned input module state.
#[derive(Default)]
pub(crate) struct PlatformInputState {
    /// Runtime-owned windows raw-input state.
    #[cfg(windows)]
    windows_raw_input_runtime_state: OnceLock<Arc<WindowsRawInputRuntimeState>>,
}

impl std::fmt::Debug for PlatformInputState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformInputState")
            .finish_non_exhaustive()
    }
}

impl PlatformInputState {
    /// Return whether any runtime-owned input state was initialized.
    pub(crate) fn is_initialized(&self) -> bool {
        #[cfg(windows)]
        if self.windows_raw_input_runtime_state.get().is_some() {
            return true;
        }

        false
    }

    /// Return runtime-owned windows raw-input state.
    #[cfg(windows)]
    pub(crate) fn windows_raw_input_runtime_state(
        &self,
        initialize: impl FnOnce() -> WindowsRawInputRuntimeState,
    ) -> Arc<WindowsRawInputRuntimeState> {
        Arc::clone(
            self.windows_raw_input_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }
}
