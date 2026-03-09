use destack_base::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::PlatformAudioState;
use crate::platform::display::PlatformDisplayState;
use crate::platform::net::PlatformNetState;

#[cfg(windows)]
use crate::platform::fs::PlatformFsState;

#[cfg(not(windows))]
#[derive(Debug, Default)]
pub(crate) struct PlatformFsState;

#[cfg(not(windows))]
impl PlatformFsState {
    /// Return whether any runtime-owned filesystem state was initialized.
    pub(crate) const fn is_initialized(&self) -> bool {
        false
    }
}

#[cfg(windows)]
use crate::platform::input::PlatformInputState;

#[cfg(not(windows))]
#[derive(Debug, Default)]
pub(crate) struct PlatformInputState;

#[cfg(not(windows))]
impl PlatformInputState {
    /// Return whether any runtime-owned input state was initialized.
    pub(crate) const fn is_initialized(&self) -> bool {
        false
    }
}

#[cfg(any(target_os = "linux", windows))]
use crate::platform::os::PlatformOsState;

#[cfg(not(any(target_os = "linux", windows)))]
#[derive(Debug, Default)]
pub(crate) struct PlatformOsState;

#[cfg(not(any(target_os = "linux", windows)))]
impl PlatformOsState {
    /// Return whether any runtime-owned OS state was initialized.
    pub(crate) const fn is_initialized(&self) -> bool {
        false
    }
}

/// Runtime-owned platform module state slots.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub(crate) struct PlatformState {
    /// Audio module state.
    pub audio: PlatformAudioState,
    /// Display module state.
    pub display: PlatformDisplayState,
    /// Filesystem module state.
    pub fs: PlatformFsState,
    /// Input module state.
    pub input: PlatformInputState,
    /// Network module state.
    pub net: PlatformNetState,
    /// OS module state.
    pub os: PlatformOsState,
}

/// Materialized platform-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformStateImage;

impl PlatformState {
    // active modules
    fn active_module_names(&self) -> Vec<&'static str> {
        let mut modules = Vec::new();

        if self.audio.is_initialized() {
            modules.push("audio");
        }

        if self.display.is_initialized() {
            modules.push("display");
        }

        if self.fs.is_initialized() {
            modules.push("fs");
        }

        if self.input.is_initialized() {
            modules.push("input");
        }

        if self.net.is_initialized() {
            modules.push("net");
        }

        if self.os.is_initialized() {
            modules.push("os");
        }

        modules
    }

    // capture barrier
    fn capture_barrier(&self, mode: CaptureMode) -> RuntimeResult<()> {
        let modules = self.active_module_names();
        if modules.is_empty() {
            return Ok(());
        }

        Err(RuntimeError::Internal {
            message: format!(
                "platform state cannot capture for {mode:?} while module state is active: {}",
                modules.join(", ")
            ),
        }
        .boxed())
    }

    // restore default
    fn restore_default(&mut self) {
        *self = Self::default();
    }
}

impl Capture for PlatformState {
    type Image = PlatformStateImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one platform-state image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.capture_barrier(mode)?;

        Ok(PlatformStateImage)
    }

    /// Restore one platform-state image.
    fn restore_image(
        &mut self,
        _image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        self.restore_default();

        Ok(())
    }
}
