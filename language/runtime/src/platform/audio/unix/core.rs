use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core::constants::AudioBackendOpenFlags;
use crate::platform::audio::core::model::{AudioStreamHostState, HostDeviceDescriptor};
use crate::platform::audio::core::monitor::AudioMonitorHandle;
use crate::platform::audio::{AudioBackend, AudioShareMode, AudioStreamConfig};

use super::backend;

#[cfg(target_os = "macos")]
const UNIX_BACKEND_PRIORITY: &[AudioBackend] = &[AudioBackend::CoreAudio];
#[cfg(target_os = "android")]
const UNIX_BACKEND_PRIORITY: &[AudioBackend] = &[AudioBackend::AAudio, AudioBackend::OpenSLES];
#[cfg(all(unix, not(any(target_os = "macos", target_os = "android"))))]
const UNIX_BACKEND_PRIORITY: &[AudioBackend] = &[
    AudioBackend::PipeWire,
    AudioBackend::PulseAudio,
    AudioBackend::Alsa,
    AudioBackend::Jack,
];

/// Return unix backend priority order for auto-selection.
pub(crate) fn preferred_host_backends() -> &'static [AudioBackend] {
    UNIX_BACKEND_PRIORITY
}

/// Return whether one unix backend is available for this build.
pub(crate) fn backend_supported(backend: AudioBackend) -> bool {
    backend::backend_supported(backend)
}

/// Return whether one unix backend supports stream creation.
pub(crate) fn backend_stream_supported(backend: AudioBackend) -> bool {
    backend::backend_stream_supported(backend)
}

/// Return whether one unix backend supports a native device monitor.
pub(crate) fn backend_supports_native_device_monitor(backend: AudioBackend) -> bool {
    backend::backend_supports_native_device_monitor(backend)
}

/// Enumerate host devices for one selected unix backend.
pub(crate) fn enumerate_host_devices(
    backend: AudioBackend,
) -> RuntimeResult<Vec<HostDeviceDescriptor>> {
    backend::enumerate_host_devices(backend)
}

/// Open one host stream for one unix backend device.
pub(crate) fn open_host_stream(
    device_info: &HostDeviceDescriptor,
    config: AudioStreamConfig,
    share_mode: AudioShareMode,
    backend_flags: AudioBackendOpenFlags,
) -> RuntimeResult<std::sync::Arc<AudioStreamHostState>> {
    backend::open_host_stream(device_info, config, share_mode, backend_flags)
}

/// Trigger one unix backend device rescan.
pub(crate) fn rescan_host_backend(backend: AudioBackend) -> RuntimeResult<()> {
    backend::rescan_host_backend(backend)
}

/// Resolve one unix host device by stable identifier.
pub(crate) fn resolve_host_device_by_id(
    backend: AudioBackend,
    id: &str,
) -> RuntimeResult<HostDeviceDescriptor> {
    backend::resolve_host_device_by_id(backend, id)
}

/// Start one unix backend native device-event monitor.
pub(crate) fn start_backend_native_device_events_impl(
    backend: AudioBackend,
) -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    backend::start_backend_native_device_events(backend)
}
