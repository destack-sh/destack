use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

use super::super::backend::backend_not_supported;

/// Enumerate AAudio devices.
pub(crate) fn enumerate_host_devices() -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    Err(backend_not_supported("destack.audio.device.list", "aaudio"))
}
