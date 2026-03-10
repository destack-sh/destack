use destack_core::{Capture, CaptureMode};
use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeError;
use crate::platform::audio::{PlatformAudioImage, PlatformAudioState};
use crate::platform::display::{PlatformDisplayImage, PlatformDisplayState};
use crate::platform::fs::{PlatformFsImage, PlatformFsState};
use crate::platform::input::{PlatformInputImage, PlatformInputState};
use crate::platform::midi::{PlatformMidiImage, PlatformMidiState};
use crate::platform::net::{PlatformNetImage, PlatformNetState};
use crate::platform::os::{PlatformOsImage, PlatformOsState};

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
    /// MIDI module state.
    pub midi: PlatformMidiState,
    /// Network module state.
    pub net: PlatformNetState,
    /// OS module state.
    pub os: PlatformOsState,
}

/// Materialized platform-state image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlatformStateImage {
    /// Audio platform-state image.
    pub audio: PlatformAudioImage,
    /// Display platform-state image.
    pub display: PlatformDisplayImage,
    /// Filesystem platform-state image.
    pub fs: PlatformFsImage,
    /// Input platform-state image.
    pub input: PlatformInputImage,
    /// MIDI platform-state image.
    pub midi: PlatformMidiImage,
    /// Network platform-state image.
    pub net: PlatformNetImage,
    /// OS platform-state image.
    pub os: PlatformOsImage,
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
            display: self.display.capture_image(mode, ())?,
            fs: self.fs.capture_image(mode, ())?,
            input: self.input.capture_image(mode, ())?,
            midi: self.midi.capture_image(mode, ())?,
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
        self.display.restore_image(&image.display, ())?;
        self.fs.restore_image(&image.fs, ())?;
        self.input.restore_image(&image.input, ())?;
        self.midi.restore_image(&image.midi, ())?;
        self.net.restore_image(&image.net, ())?;
        self.os.restore_image(&image.os, ())?;

        Ok(())
    }
}
