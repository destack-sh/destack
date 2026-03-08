#[cfg(not(target_os = "linux"))]
use super::super::backend::backend_not_supported;
#[cfg(target_os = "linux")]
use super::core::is_backend_supported as host_backend_supported;
#[cfg(target_os = "linux")]
use super::descriptor::enumerate_devices;
use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

/// Return whether ALSA backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        host_backend_supported()
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Enumerate ALSA devices.
pub(crate) fn enumerate_host_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    #[cfg(target_os = "linux")]
    {
        enumerate_devices()
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(backend_not_supported("destack.audio.device.list", "alsa"))
    }
}
