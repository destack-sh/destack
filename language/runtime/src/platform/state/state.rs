use crate::platform::audio::PlatformAudioState;
#[cfg(any(windows, target_os = "linux"))]
use crate::platform::display::PlatformDisplayState;
#[cfg(target_os = "macos")]
use crate::platform::display::PlatformDisplayState;
#[cfg(windows)]
use crate::platform::fs::PlatformFsState;
#[cfg(windows)]
use crate::platform::input::PlatformInputState;
use crate::platform::net::PlatformNetState;
#[cfg(any(target_os = "linux", windows))]
use crate::platform::os::PlatformOsState;

/// Runtime-owned platform module state slots.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub(crate) struct PlatformState {
    /// Audio module state.
    pub audio: PlatformAudioState,

    /// Display module state.
    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    pub display: PlatformDisplayState,

    /// Filesystem module state.
    #[cfg(windows)]
    pub fs: PlatformFsState,

    /// Input module state.
    #[cfg(windows)]
    pub input: PlatformInputState,

    /// Network module state.
    pub net: PlatformNetState,

    /// OS module state.
    #[cfg(any(target_os = "linux", windows))]
    pub os: PlatformOsState,
}
