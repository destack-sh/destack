#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::sync::{Arc, OnceLock};

use destack_core::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

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
    /// Return whether any runtime-owned OS state is active.
    fn has_runtime_state(&self) -> bool {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        if self.no_replace_write_runtime_state.get().is_some() {
            return true;
        }

        false
    }

    /// Capture one OS-state image.
    fn image(
        &self,
        mode: CaptureMode,
    ) -> Result<PlatformOsImage, Box<crate::diagnostic::RuntimeError>> {
        if !self.has_runtime_state() {
            return Ok(PlatformOsImage);
        }

        Err(crate::diagnostic::RuntimeError::CaptureBarrier {
            component: "platform.os".to_string(),
            mode: format!("{mode:?}"),
            detail: "runtime state is active".to_string(),
        }
        .boxed())
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

/// Materialized OS platform-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformOsImage;

impl Capture for PlatformOsState {
    type Image = PlatformOsImage;
    type Error = Box<crate::diagnostic::RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one OS platform-state image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image(mode)
    }

    /// Restore one OS platform-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        *self = Self::default();

        Ok(())
    }
}
