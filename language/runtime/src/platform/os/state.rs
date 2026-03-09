#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::sync::{Arc, OnceLock};

#[cfg(any(target_os = "linux", target_os = "windows"))]
use super::credentials::NoReplaceWriteRuntimeState;

/// Runtime-owned OS module state.
#[derive(Default)]
pub(crate) struct PlatformOsState {
    /// Runtime-owned no-replace write guard state.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    no_replace_write_runtime_state: OnceLock<Arc<NoReplaceWriteRuntimeState>>,
}

impl std::fmt::Debug for PlatformOsState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformOsState")
            .finish_non_exhaustive()
    }
}

impl PlatformOsState {
    /// Return whether any runtime-owned OS state was initialized.
    pub(crate) fn is_initialized(&self) -> bool {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        if self.no_replace_write_runtime_state.get().is_some() {
            return true;
        }

        false
    }

    /// Return runtime-owned no-replace write guard state.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub(crate) fn no_replace_write_runtime_state(
        &self,
        initialize: impl FnOnce() -> NoReplaceWriteRuntimeState,
    ) -> Arc<NoReplaceWriteRuntimeState> {
        Arc::clone(
            self.no_replace_write_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }
}
