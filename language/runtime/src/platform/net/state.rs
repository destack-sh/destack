#[cfg(any(target_os = "macos", windows))]
use std::sync::{Arc, OnceLock};

#[cfg(target_os = "macos")]
use super::host::MacosRouteRuntimeState;
#[cfg(windows)]
use super::host::WindowsUdsRuntimeState;

/// Runtime-owned network module state.
#[derive(Default)]
pub(crate) struct PlatformNetState {
    /// Runtime-owned windows UDS helper state.
    #[cfg(windows)]
    windows_uds_runtime_state: OnceLock<Arc<WindowsUdsRuntimeState>>,
    /// Runtime-owned macOS route state.
    #[cfg(target_os = "macos")]
    macos_route_runtime_state: OnceLock<Arc<MacosRouteRuntimeState>>,
}

impl std::fmt::Debug for PlatformNetState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformNetState")
            .finish_non_exhaustive()
    }
}

impl PlatformNetState {
    /// Return whether any runtime-owned network state was initialized.
    pub(crate) fn is_initialized(&self) -> bool {
        #[cfg(windows)]
        if self.windows_uds_runtime_state.get().is_some() {
            return true;
        }

        #[cfg(target_os = "macos")]
        if self.macos_route_runtime_state.get().is_some() {
            return true;
        }

        false
    }

    /// Return runtime-owned windows UDS helper state.
    #[cfg(windows)]
    pub(crate) fn windows_uds_runtime_state(
        &self,
        initialize: impl FnOnce() -> WindowsUdsRuntimeState,
    ) -> Arc<WindowsUdsRuntimeState> {
        Arc::clone(
            self.windows_uds_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return runtime-owned macOS route state.
    #[cfg(target_os = "macos")]
    pub(crate) fn macos_route_runtime_state(
        &self,
        initialize: impl FnOnce() -> MacosRouteRuntimeState,
    ) -> Arc<MacosRouteRuntimeState> {
        Arc::clone(
            self.macos_route_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }
}
