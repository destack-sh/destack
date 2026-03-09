#[cfg(any(target_os = "macos", windows))]
use std::sync::{Arc, OnceLock};

use destack_base::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

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
    /// Return whether any runtime-owned network state is active.
    fn has_runtime_state(&self) -> bool {
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

    /// Capture one network-state image.
    fn image(
        &self,
        mode: CaptureMode,
    ) -> Result<PlatformNetImage, Box<crate::diagnostic::RuntimeError>> {
        if !self.has_runtime_state() {
            return Ok(PlatformNetImage);
        }

        Err(crate::diagnostic::RuntimeError::CaptureBarrier {
            component: "platform.net".to_string(),
            mode: format!("{mode:?}"),
            detail: "runtime state is active".to_string(),
        }
        .boxed())
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

/// Materialized network platform-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformNetImage;

impl Capture for PlatformNetState {
    type Image = PlatformNetImage;
    type Error = Box<crate::diagnostic::RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one network platform-state image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image(mode)
    }

    /// Restore one network platform-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        *self = Self::default();

        Ok(())
    }
}
