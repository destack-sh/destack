use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

use super::core::is_backend_supported as host_backend_supported;
use super::descriptor::enumerate_devices;

/// Return whether WASAPI backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    host_backend_supported()
}

/// Enumerate WASAPI host devices.
pub(crate) fn enumerate_host_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    enumerate_devices()
}
