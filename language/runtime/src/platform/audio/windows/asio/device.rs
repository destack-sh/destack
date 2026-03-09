use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

use super::core::is_backend_supported as host_backend_supported;
use super::descriptor::enumerate_devices;
use crate::platform::audio as audio_types;

/// Return whether ASIO backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    host_backend_supported()
}

/// Enumerate ASIO host devices.
pub(crate) fn enumerate_host_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    enumerate_devices()
}
