use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;
use crate::runtime::BindingCallContext;

#[cfg(target_os = "macos")]
const UNIX_BACKEND_PRIORITY: &[audio_core::AudioBackend] = &[audio_core::AudioBackend::CoreAudio];
#[cfg(target_os = "android")]
const UNIX_BACKEND_PRIORITY: &[audio_core::AudioBackend] = &[
    audio_core::AudioBackend::AAudio,
    audio_core::AudioBackend::OpenSLES,
];
#[cfg(all(unix, not(any(target_os = "macos", target_os = "android"))))]
const UNIX_BACKEND_PRIORITY: &[audio_core::AudioBackend] = &[
    audio_core::AudioBackend::PipeWire,
    audio_core::AudioBackend::PulseAudio,
    audio_core::AudioBackend::Alsa,
    audio_core::AudioBackend::Jack,
];

/// Return unix backend priority order for auto-selection.
pub(crate) fn preferred_host_backends() -> &'static [audio_core::AudioBackend] {
    UNIX_BACKEND_PRIORITY
}

/// Return whether one unix backend is available for this build.
pub(crate) fn backend_supported(backend: audio_core::AudioBackend) -> bool {
    super::backend::backend_supported(backend)
}

/// Return whether one unix backend supports stream creation.
pub(crate) fn backend_stream_supported(backend: audio_core::AudioBackend) -> bool {
    super::backend::backend_stream_supported(backend)
}

/// Enumerate host devices for one selected unix backend.
pub(crate) fn enumerate_host_devices(
    backend: audio_core::AudioBackend,
) -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    super::backend::enumerate_host_devices(backend)
}

/// Open one host stream for one unix backend device.
pub(crate) fn open_host_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
    backend_flags: audio_core::AudioBackendOpenFlags,
) -> RuntimeResult<Arc<audio_core::AudioStreamBinding>> {
    super::backend::open_host_stream(device_info, config, share_mode, backend_flags)
}

/// Trigger one unix backend device rescan.
pub(crate) fn rescan_host_backend(backend: audio_core::AudioBackend) -> RuntimeResult<()> {
    super::backend::rescan_host_backend(backend)
}

/// Resolve one unix host device by stable identifier.
pub(crate) fn resolve_host_device_by_id(
    backend: audio_core::AudioBackend,
    id: &str,
) -> RuntimeResult<audio_core::HostDeviceDescriptor> {
    super::backend::resolve_host_device_by_id(backend, id)
}

/// Return whether one unix backend exposes native device-event subscriptions.
pub(crate) fn backend_native_device_events_supported_impl(
    backend: audio_core::AudioBackend,
) -> bool {
    super::backend::backend_native_device_events_supported(backend)
}

/// Start one unix backend native device-event monitor.
pub(crate) fn start_backend_native_device_events_impl(
    binding: &BindingCallContext,
    backend: audio_core::AudioBackend,
) -> RuntimeResult<()> {
    super::backend::start_backend_native_device_events(binding, backend)
}

/// Stop one unix backend native device-event monitor.
pub(crate) fn stop_backend_native_device_events_impl(
    binding: &BindingCallContext,
    backend: audio_core::AudioBackend,
) {
    super::backend::stop_backend_native_device_events(binding, backend);
}
