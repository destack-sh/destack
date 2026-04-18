#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
use std::sync::Arc;

use destack_core::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;
#[cfg(any(windows, target_os = "linux", target_os = "macos"))]
use crate::runtime::process::service::ServiceHandle;

#[cfg(target_os = "macos")]
use super::unix::{AppKitDisplayService, AppKitRuntimeState};
#[cfg(target_os = "linux")]
use super::unix::{WaylandDisplayService, WaylandRuntimeState, X11DisplayService, X11RuntimeState};
#[cfg(windows)]
use super::windows::{Win32DisplayService, Win32RuntimeState};

/// Worker-owned display module state.
#[derive(Default)]
pub(crate) struct PlatformDisplayState {
    /// Shared Win32 display service handle for this worker.
    #[cfg(windows)]
    win32_service: ServiceHandle<Win32DisplayService>,
    /// Worker-owned Win32 display runtime state.
    #[cfg(windows)]
    win32_runtime_state: std::sync::OnceLock<Arc<Win32RuntimeState>>,
    /// Worker-owned linux x11 runtime state.
    #[cfg(target_os = "linux")]
    x11_runtime_state: std::sync::OnceLock<Arc<X11RuntimeState>>,
    /// Shared linux x11 display service handle for this worker.
    #[cfg(target_os = "linux")]
    x11_service: ServiceHandle<X11DisplayService>,
    /// Worker-owned linux wayland runtime state.
    #[cfg(target_os = "linux")]
    wayland_runtime_state: std::sync::OnceLock<Arc<WaylandRuntimeState>>,
    /// Shared linux wayland display service handle for this worker.
    #[cfg(target_os = "linux")]
    wayland_service: ServiceHandle<WaylandDisplayService>,
    /// Shared AppKit display service handle for this worker.
    #[cfg(target_os = "macos")]
    appkit_service: ServiceHandle<AppKitDisplayService>,
    /// Worker-owned AppKit runtime state.
    #[cfg(target_os = "macos")]
    appkit_runtime_state: std::sync::OnceLock<Arc<AppKitRuntimeState>>,
}

impl std::fmt::Debug for PlatformDisplayState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformDisplayState")
            .finish_non_exhaustive()
    }
}

impl PlatformDisplayState {
    /// Return one shared Win32 display service handle for this worker.
    #[cfg(windows)]
    pub(crate) fn win32_service(&self) -> Arc<Win32DisplayService> {
        self.win32_service
            .get_or_init(super::windows::win32_display_service)
    }

    /// Return whether any worker-owned display runtime state is active.
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
    fn image(&self, mode: CaptureMode) -> Result<PlatformDisplayImage, Box<RuntimeError>> {
        if !self.has_runtime_state() {
            return Ok(PlatformDisplayImage);
        }

        Err(RuntimeError::CaptureBarrier {
            component: "platform.display".to_string(),
            mode: format!("{mode:?}"),
            detail: "runtime state is active".to_string(),
        }
        .boxed())
    }

    /// Return worker-owned Win32 display runtime state.
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

    /// Return worker-owned linux x11 display runtime state.
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

    /// Return one shared x11 display service handle for this worker.
    #[cfg(target_os = "linux")]
    pub(crate) fn x11_service(&self) -> Arc<X11DisplayService> {
        self.x11_service
            .get_or_init(super::unix::x11_display_service)
    }

    /// Return worker-owned linux wayland display runtime state.
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

    /// Return one shared wayland display service handle for this worker.
    #[cfg(target_os = "linux")]
    pub(crate) fn wayland_service(&self) -> Arc<WaylandDisplayService> {
        self.wayland_service
            .get_or_init(super::unix::wayland_display_service)
    }

    /// Return one shared AppKit display service handle for this worker.
    #[cfg(target_os = "macos")]
    pub(crate) fn appkit_service(&self) -> Arc<AppKitDisplayService> {
        self.appkit_service
            .get_or_init(super::unix::appkit_display_service)
    }

    /// Return worker-owned AppKit display runtime state.
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
    type Error = Box<RuntimeError>;
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
