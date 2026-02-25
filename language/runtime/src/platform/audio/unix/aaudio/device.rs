#[cfg(not(target_os = "android"))]
use super::super::backend::backend_not_supported;
#[cfg(target_os = "android")]
use super::core::is_backend_supported as host_backend_supported;
#[cfg(target_os = "android")]
use super::descriptor::enumerate_devices;
use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

/// Return whether AAudio backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    #[cfg(target_os = "android")]
    {
        host_backend_supported()
    }

    #[cfg(not(target_os = "android"))]
    {
        false
    }
}

/// Enumerate AAudio devices.
pub(crate) fn enumerate_host_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    #[cfg(target_os = "android")]
    {
        enumerate_devices()
    }

    #[cfg(not(target_os = "android"))]
    {
        Err(backend_not_supported("destack.audio.device.list", "aaudio"))
    }
}
