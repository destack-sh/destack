#[cfg(windows)]
use std::sync::Arc;
use std::sync::OnceLock;

use destack_core::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;

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
    /// Return whether any runtime-owned filesystem state is active.
    fn has_runtime_state(&self) -> bool {
        #[cfg(windows)]
        if self.windows_mmap_runtime_state.get().is_some() {
            return true;
        }

        false
    }

    /// Capture one filesystem-state image.
    fn image(&self, mode: CaptureMode) -> Result<PlatformFsImage, Box<RuntimeError>> {
        if !self.has_runtime_state() {
            return Ok(PlatformFsImage);
        }

        Err(RuntimeError::CaptureBarrier {
            component: "platform.fs".to_string(),
            mode: format!("{mode:?}"),
            detail: "runtime state is active".to_string(),
        }
        .boxed())
    }

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

/// Materialized filesystem platform-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformFsImage;

impl Capture for PlatformFsState {
    type Image = PlatformFsImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one filesystem platform-state image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image(mode)
    }

    /// Restore one filesystem platform-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        *self = Self::default();

        Ok(())
    }
}
