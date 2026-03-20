use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core::constants::AudioBackendOpenFlags;
use crate::platform::audio::core::model::{AudioStreamHostState, HostDeviceDescriptor};
use crate::platform::audio::core::monitor::AudioMonitorHandle;
use crate::platform::audio::{
    AudioBackend, AudioShareMode, AudioStreamConfig, AudioStreamFlags, AudioStreamRequirementFlags,
};
use crate::platform::core::BackendSupport;

use super::backend;

const WINDOWS_BACKEND_PRIORITY: &[AudioBackend] = &[AudioBackend::Wasapi, AudioBackend::Asio];

/// Return windows backend priority order for auto-selection.
pub(crate) fn preferred_host_backends() -> &'static [AudioBackend] {
    WINDOWS_BACKEND_PRIORITY
}

/// Return host backend support for one windows backend.
pub(crate) fn backend_support(backend: AudioBackend) -> BackendSupport {
    backend::backend_support(backend)
}

/// Return whether one windows backend supports stream creation.
pub(crate) fn backend_stream_supported(backend: AudioBackend) -> bool {
    backend::backend_stream_supported(backend)
}

/// Return whether one windows backend supports a native device monitor.
pub(crate) fn backend_supports_native_device_monitor(backend: AudioBackend) -> bool {
    backend::backend_supports_native_device_monitor(backend)
}

/// Enumerate host devices for one selected windows backend.
pub(crate) fn enumerate_host_devices(
    backend: AudioBackend,
) -> RuntimeResult<Vec<HostDeviceDescriptor>> {
    backend::enumerate_host_devices(backend)
}

/// Open one host stream for one windows backend device.
pub(crate) fn open_host_stream(
    device_info: &HostDeviceDescriptor,
    config: AudioStreamConfig,
    share_mode: AudioShareMode,
    backend_flags: AudioBackendOpenFlags,
    requested_flags: AudioStreamFlags,
    requested_requirements: AudioStreamRequirementFlags,
) -> RuntimeResult<std::sync::Arc<AudioStreamHostState>> {
    backend::open_host_stream(
        device_info,
        config,
        share_mode,
        backend_flags,
        requested_flags,
        requested_requirements,
    )
}

/// Trigger one windows backend device rescan.
pub(crate) fn rescan_host_backend(backend: AudioBackend) -> RuntimeResult<()> {
    backend::rescan_host_backend(backend)
}

/// Resolve one windows host device by stable identifier.
pub(crate) fn resolve_host_device_by_id(
    backend: AudioBackend,
    id: &str,
) -> RuntimeResult<HostDeviceDescriptor> {
    backend::resolve_host_device_by_id(backend, id)
}

/// Start one windows backend native device-event monitor.
pub(crate) fn start_backend_native_device_events_impl(
    backend: AudioBackend,
) -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    backend::start_backend_native_device_events(backend)
}
