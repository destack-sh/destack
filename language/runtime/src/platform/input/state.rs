use std::sync::{Arc, OnceLock};

#[cfg(target_os = "macos")]
use super::host::MacosTapRuntimeState;
#[cfg(windows)]
use super::host::{
    WindowsInputCoreRuntimeState, WindowsInputEventRuntimeState, WindowsRawInputRuntimeState,
};

/// Runtime-owned input module state.
#[derive(Default)]
pub struct PlatformInputState {
    /// Runtime-owned windows input core state.
    #[cfg(windows)]
    windows_input_core_runtime_state: OnceLock<Arc<WindowsInputCoreRuntimeState>>,
    /// Runtime-owned windows input event state.
    #[cfg(windows)]
    windows_input_event_runtime_state: OnceLock<Arc<WindowsInputEventRuntimeState>>,
    /// Runtime-owned windows raw-input state.
    #[cfg(windows)]
    windows_raw_input_runtime_state: OnceLock<Arc<WindowsRawInputRuntimeState>>,
    /// Runtime-owned macOS event-tap state.
    #[cfg(target_os = "macos")]
    macos_tap_runtime_state: OnceLock<Arc<MacosTapRuntimeState>>,
}

impl std::fmt::Debug for PlatformInputState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformInputState")
            .finish_non_exhaustive()
    }
}

impl PlatformInputState {
    /// Return runtime-owned windows input core state.
    #[cfg(windows)]
    pub(crate) fn windows_input_core_runtime_state(
        &self,
        initialize: impl FnOnce() -> WindowsInputCoreRuntimeState,
    ) -> Arc<WindowsInputCoreRuntimeState> {
        Arc::clone(
            self.windows_input_core_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return runtime-owned windows input event state.
    #[cfg(windows)]
    pub(crate) fn windows_input_event_runtime_state(
        &self,
        initialize: impl FnOnce() -> WindowsInputEventRuntimeState,
    ) -> Arc<WindowsInputEventRuntimeState> {
        Arc::clone(
            self.windows_input_event_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
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

    /// Return runtime-owned macOS event-tap state.
    #[cfg(target_os = "macos")]
    pub(crate) fn macos_tap_runtime_state(
        &self,
        initialize: impl FnOnce() -> MacosTapRuntimeState,
    ) -> Arc<MacosTapRuntimeState> {
        Arc::clone(
            self.macos_tap_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }
}
