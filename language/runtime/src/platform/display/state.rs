#[cfg(windows)]
use std::sync::{Arc, OnceLock};
#[cfg(target_os = "linux")]
use std::sync::{Arc, OnceLock};

#[cfg(target_os = "linux")]
use super::host::X11RuntimeState;
#[cfg(windows)]
use super::host::{DisplayEventRuntimeState, WindowRuntimeState};

/// Runtime-owned display module state.
#[derive(Default)]
pub(crate) struct PlatformDisplayState {
    /// Runtime-owned windows display-event state.
    #[cfg(windows)]
    display_event_runtime_state: OnceLock<Arc<DisplayEventRuntimeState>>,
    /// Runtime-owned windows window state.
    #[cfg(windows)]
    window_runtime_state: OnceLock<Arc<WindowRuntimeState>>,
    /// Runtime-owned linux x11 state.
    #[cfg(target_os = "linux")]
    x11_runtime_state: OnceLock<Arc<X11RuntimeState>>,
}

impl std::fmt::Debug for PlatformDisplayState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformDisplayState")
            .finish_non_exhaustive()
    }
}

impl PlatformDisplayState {
    /// Return runtime-owned windows display-event state.
    #[cfg(windows)]
    pub(crate) fn display_event_runtime_state(
        &self,
        initialize: impl FnOnce() -> DisplayEventRuntimeState,
    ) -> Arc<DisplayEventRuntimeState> {
        Arc::clone(
            self.display_event_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return runtime-owned windows window state.
    #[cfg(windows)]
    pub(crate) fn window_runtime_state(
        &self,
        initialize: impl FnOnce() -> WindowRuntimeState,
    ) -> Arc<WindowRuntimeState> {
        Arc::clone(
            self.window_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return runtime-owned linux x11 display state.
    #[cfg(target_os = "linux")]
    pub(crate) fn x11_runtime_state(
        &self,
        initialize: impl FnOnce() -> X11RuntimeState,
    ) -> Arc<X11RuntimeState> {
        Arc::clone(
            self.x11_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }
}
