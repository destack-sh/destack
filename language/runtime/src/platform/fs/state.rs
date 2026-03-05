#[cfg(windows)]
use std::sync::{Arc, OnceLock};

#[cfg(windows)]
use super::host::WindowsMmapRuntimeState;

/// Runtime-owned filesystem module state.
#[derive(Default)]
pub(crate) struct PlatformFsState {
    /// Runtime-owned windows mmap state.
    #[cfg(windows)]
    windows_mmap_runtime_state: OnceLock<Arc<WindowsMmapRuntimeState>>,
}

impl std::fmt::Debug for PlatformFsState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformFsState")
            .finish_non_exhaustive()
    }
}

impl PlatformFsState {
    /// Return runtime-owned windows mmap state.
    #[cfg(windows)]
    pub(crate) fn windows_mmap_runtime_state(
        &self,
        initialize: impl FnOnce() -> WindowsMmapRuntimeState,
    ) -> Arc<WindowsMmapRuntimeState> {
        Arc::clone(
            self.windows_mmap_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }
}
