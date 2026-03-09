#[cfg(target_os = "linux")]
use super::descriptor::enumerate_devices;
use crate::diagnostic::RuntimeResult;
#[cfg(not(target_os = "linux"))]
use crate::platform::audio::backend::backend_not_supported;
use crate::platform::audio::core as audio_core;

/// Enumerate JACK devices.
pub(crate) fn enumerate_host_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    #[cfg(target_os = "linux")]
    {
        enumerate_devices()
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(backend_not_supported("destack.audio.device.list", "jack"))
    }
}
