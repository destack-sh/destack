use std::sync::{Arc, OnceLock};

#[cfg(target_os = "macos")]
use super::host::MacosRouteRuntimeState;
#[cfg(windows)]
use super::host::{WindowsPacketRuntimeState, WindowsUdsRuntimeState};

/// Runtime-owned network module state.
#[derive(Default)]
pub struct PlatformNetState {
    /// Runtime-owned windows packet state.
    #[cfg(windows)]
    windows_packet_runtime_state: OnceLock<Arc<WindowsPacketRuntimeState>>,
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
    /// Return runtime-owned windows packet state.
    #[cfg(windows)]
    pub(crate) fn windows_packet_runtime_state(
        &self,
        initialize: impl FnOnce() -> WindowsPacketRuntimeState,
    ) -> Arc<WindowsPacketRuntimeState> {
        Arc::clone(
            self.windows_packet_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
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
