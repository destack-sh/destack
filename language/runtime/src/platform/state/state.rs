use crate::platform::audio::PlatformAudioState;
use crate::platform::crypto::PlatformCryptoState;
use crate::platform::display::PlatformDisplayState;
use crate::platform::fs::PlatformFsState;
use crate::platform::input::PlatformInputState;
use crate::platform::io::PlatformIoState;
use crate::platform::net::PlatformNetState;
use crate::platform::os::PlatformOsState;

/// Runtime-owned platform module state slots.
#[derive(Debug, Default)]
pub struct PlatformState {
    /// Audio module state.
    pub audio: PlatformAudioState,
    /// Crypto module state.
    pub crypto: PlatformCryptoState,
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
