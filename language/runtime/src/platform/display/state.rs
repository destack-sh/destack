use std::sync::{Arc, OnceLock};

#[cfg(target_os = "macos")]
use super::unix::AppKitRuntimeState;
#[cfg(target_os = "linux")]
use super::unix::{WaylandRuntimeState, X11RuntimeState};
#[cfg(windows)]
use super::windows::Win32RuntimeState;

/// Runtime-owned display module state.
#[derive(Default)]
pub(crate) struct PlatformDisplayState {
    /// Runtime-owned windows display-event state.
    #[cfg(windows)]
    win32_runtime_state: OnceLock<Arc<Win32RuntimeState>>,
    /// Runtime-owned linux x11 state.
    #[cfg(target_os = "linux")]
    x11_runtime_state: OnceLock<Arc<X11RuntimeState>>,
    /// Runtime-owned linux wayland state.
    #[cfg(target_os = "linux")]
    wayland_runtime_state: OnceLock<Arc<WaylandRuntimeState>>,
    /// Runtime-owned macOS appkit state.
    #[cfg(target_os = "macos")]
    appkit_runtime_state: OnceLock<Arc<AppKitRuntimeState>>,
}

impl std::fmt::Debug for PlatformDisplayState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformDisplayState")
            .finish_non_exhaustive()
    }
}

impl PlatformDisplayState {
    /// Return runtime-owned Win32 display state.
    #[cfg(windows)]
    pub(crate) fn win32_runtime_state(
        &self,
        initialize: impl FnOnce() -> Win32RuntimeState,
    ) -> Arc<Win32RuntimeState> {
        Arc::clone(
            self.win32_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return existing Win32 display state when it was initialized already.
    #[cfg(windows)]
    pub(crate) fn existing_win32_runtime_state(&self) -> Option<Arc<Win32RuntimeState>> {
        self.win32_runtime_state.get().map(Arc::clone)
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

    /// Return existing linux x11 display state when it was initialized already.
    #[cfg(target_os = "linux")]
    pub(crate) fn existing_x11_runtime_state(&self) -> Option<Arc<X11RuntimeState>> {
        self.x11_runtime_state.get().map(Arc::clone)
    }

    /// Return runtime-owned linux wayland display state.
    #[cfg(target_os = "linux")]
    pub(crate) fn wayland_runtime_state(
        &self,
        initialize: impl FnOnce() -> WaylandRuntimeState,
    ) -> Arc<WaylandRuntimeState> {
        Arc::clone(
            self.wayland_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }

    /// Return existing linux wayland display state when it was initialized already.
    #[cfg(target_os = "linux")]
    pub(crate) fn existing_wayland_runtime_state(&self) -> Option<Arc<WaylandRuntimeState>> {
        self.wayland_runtime_state.get().map(Arc::clone)
    }

    /// Return runtime-owned macOS appkit display state.
    #[cfg(target_os = "macos")]
    pub(crate) fn appkit_runtime_state(
        &self,
        initialize: impl FnOnce() -> AppKitRuntimeState,
    ) -> Arc<AppKitRuntimeState> {
        Arc::clone(
            self.appkit_runtime_state
                .get_or_init(|| Arc::new(initialize())),
        )
    }
}
