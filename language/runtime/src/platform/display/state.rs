use std::sync::{Arc, OnceLock};

use destack_base::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

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
    /// Return whether any runtime-owned display state is active.
    fn has_runtime_state(&self) -> bool {
        #[cfg(windows)]
        if self.win32_runtime_state.get().is_some() {
            return true;
        }

        #[cfg(target_os = "linux")]
        if self.x11_runtime_state.get().is_some() || self.wayland_runtime_state.get().is_some() {
            return true;
        }

        #[cfg(target_os = "macos")]
        if self.appkit_runtime_state.get().is_some() {
            return true;
        }

        false
    }

    /// Capture one display-state image.
    fn image(
        &self,
        mode: CaptureMode,
    ) -> Result<PlatformDisplayImage, Box<crate::diagnostic::RuntimeError>> {
        if !self.has_runtime_state() {
            return Ok(PlatformDisplayImage);
        }

        Err(crate::diagnostic::RuntimeError::CaptureBarrier {
            component: "platform.display".to_string(),
            mode: format!("{mode:?}"),
            detail: "runtime state is active".to_string(),
        }
        .boxed())
    }

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

/// Materialized display platform-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformDisplayImage;

impl Capture for PlatformDisplayState {
    type Image = PlatformDisplayImage;
    type Error = Box<crate::diagnostic::RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one display platform-state image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image(mode)
    }

    /// Restore one display platform-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        *self = Self::default();

        Ok(())
    }
}
