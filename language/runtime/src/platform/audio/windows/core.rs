use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

const WINDOWS_BACKEND_PRIORITY: &[audio_core::AudioBackend] = &[
    audio_core::AudioBackend::Wasapi,
    audio_core::AudioBackend::Asio,
];

/// Return windows backend priority order for auto-selection.
pub(crate) fn preferred_host_backends() -> &'static [audio_core::AudioBackend] {
    WINDOWS_BACKEND_PRIORITY
}

/// Return whether one windows backend is available for this build.
pub(crate) fn backend_supported(backend: audio_core::AudioBackend) -> bool {
    super::backend::backend_supported(backend)
}

/// Return whether one windows backend supports stream creation.
pub(crate) fn backend_stream_supported(backend: audio_core::AudioBackend) -> bool {
    super::backend::backend_stream_supported(backend)
}

/// Enumerate host devices for one selected windows backend.
pub(crate) fn enumerate_host_devices(
    backend: audio_core::AudioBackend,
) -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    super::backend::enumerate_host_devices(backend)
}

/// Open one host stream for one windows backend device.
pub(crate) fn open_host_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
) -> RuntimeResult<Arc<audio_core::AudioStreamBinding>> {
    super::backend::open_host_stream(device_info, config, share_mode)
}

/// Trigger one windows backend device rescan.
pub(crate) fn rescan_host_backend(backend: audio_core::AudioBackend) -> RuntimeResult<()> {
    super::backend::rescan_host_backend(backend)
}

/// Resolve one windows host device by stable identifier.
pub(crate) fn resolve_host_device_by_id(
    backend: audio_core::AudioBackend,
    id: &str,
) -> RuntimeResult<audio_core::HostDeviceDescriptor> {
    super::backend::resolve_host_device_by_id(backend, id)
}
