#[cfg(windows)]
use std::sync::{Arc, OnceLock};

use destack_core::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;

#[cfg(windows)]
use super::host::WindowsRawInputRuntimeState;

/// Runtime-owned input module state.
#[derive(Default)]
pub(crate) struct PlatformInputState {
    /// Runtime-owned windows raw-input state.
    #[cfg(windows)]
    windows_raw_input_runtime_state: OnceLock<Arc<WindowsRawInputRuntimeState>>,
}

impl std::fmt::Debug for PlatformInputState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PlatformInputState")
            .finish_non_exhaustive()
    }
}

impl PlatformInputState {
    /// Return whether any runtime-owned input state is active.
    fn has_runtime_state(&self) -> bool {
        #[cfg(windows)]
        if self.windows_raw_input_runtime_state.get().is_some() {
            return true;
        }

        false
    }

    /// Capture one input-state image.
    fn image(&self, mode: CaptureMode) -> Result<PlatformInputImage, Box<RuntimeError>> {
        if !self.has_runtime_state() {
            return Ok(PlatformInputImage);
        }

        Err(RuntimeError::CaptureBarrier {
            component: "platform.input".to_string(),
            mode: format!("{mode:?}"),
            detail: "runtime state is active".to_string(),
        }
        .boxed())
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
}

/// Materialized input platform-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformInputImage;

impl Capture for PlatformInputState {
    type Image = PlatformInputImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one input platform-state image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image(mode)
    }

    /// Restore one input platform-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        *self = Self::default();

        Ok(())
    }
}
