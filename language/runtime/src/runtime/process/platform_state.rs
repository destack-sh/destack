use destack_core::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;
use crate::platform::audio::{PlatformAudioImage, PlatformAudioState};
use crate::platform::device::{PlatformDeviceImage, PlatformDeviceState};
use crate::platform::display::{PlatformDisplayImage, PlatformDisplayState};
use crate::platform::fs::{PlatformFsImage, PlatformFsState};
use crate::platform::input::{PlatformInputImage, PlatformInputState};
use crate::platform::io::{PlatformIoImage, PlatformIoState};
use crate::platform::net::{PlatformNetImage, PlatformNetState};
use crate::platform::os::PlatformOsState;

/// Runtime-owned platform module state slots.
#[derive(Debug, Default)]
pub(crate) struct PlatformState {
    /// Audio module state.
    pub audio: PlatformAudioState,
    /// Device module state.
    pub device: PlatformDeviceState,
    /// Display module state.
    pub display: PlatformDisplayState,
    /// Filesystem module state.
    pub fs: PlatformFsState,
    /// Input module state.
    pub input: PlatformInputState,
    /// I/O module state.
    pub io: PlatformIoState,
    /// Network module state.
    pub net: PlatformNetState,
    /// OS module state.
    pub os: PlatformOsState,
}

/// Materialized platform-state image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformStateImage {
    /// Audio platform-state image.
    pub audio: PlatformAudioImage,
    /// Device platform-state image.
    pub device: PlatformDeviceImage,
    /// Display platform-state image.
    pub display: PlatformDisplayImage,
    /// Filesystem platform-state image.
    pub fs: PlatformFsImage,
    /// Input platform-state image.
    pub input: PlatformInputImage,
    /// I/O platform-state image.
    pub io: PlatformIoImage,
    /// Network platform-state image.
    pub net: PlatformNetImage,
    /// OS platform-state image.
    pub os: (),
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
        Ok(PlatformStateImage {
            audio: self.audio.capture_image(mode, ())?,
            device: self.device.capture_image(mode, ())?,
            display: self.display.capture_image(mode, ())?,
            fs: self.fs.capture_image(mode, ())?,
            input: self.input.capture_image(mode, ())?,
            io: self.io.capture_image(mode, ())?,
            net: self.net.capture_image(mode, ())?,
            os: self.os.capture_image(mode, ())?,
        })
    }

    /// Restore one platform-state image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        self.audio.restore_image(&image.audio, ())?;
        self.device.restore_image(&image.device, ())?;
        self.display.restore_image(&image.display, ())?;
        self.fs.restore_image(&image.fs, ())?;
        self.input.restore_image(&image.input, ())?;
        self.io.restore_image(&image.io, ())?;
        self.net.restore_image(&image.net, ())?;
        self.os.restore_image(&image.os, ())?;

        Ok(())
    }
}

impl PlatformState {
    /// Fork this platform state for one child worker.
    pub(crate) fn fork(&mut self) -> Result<Self, Box<RuntimeError>> {
        // capture the current platform state first
        let image = self.capture_image(CaptureMode::Fork, ())?;

        // rebuild one fresh platform-state container
        let mut forked = Self::default();
        forked.restore_image(&image, ())?;

        Ok(forked)
    }
}
