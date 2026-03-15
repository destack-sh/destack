use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::{AudioBackend, core as audio_core};
use crate::platform::core::BackendSupport;

pub(crate) use crate::platform::audio::simulation::native::*;

/// Return host backend priority order for unsupported targets.
pub(crate) fn preferred_host_backends() -> &'static [AudioBackend] {
    &[]
}

/// Return host backend support for unsupported targets.
pub(crate) fn backend_support(backend: AudioBackend) -> BackendSupport {
    if backend == AudioBackend::Null {
        return BackendSupport::Available;
    }

    BackendSupport::UnsupportedTarget
}

/// Return whether one host backend supports stream creation on unsupported targets.
pub(crate) fn backend_stream_supported(_backend: AudioBackend) -> bool {
    false
}

/// Return whether one host backend supports a native device monitor.
pub(crate) fn backend_supports_native_device_monitor(_backend: AudioBackend) -> bool {
    false
}

/// Start one host backend native device-event monitor on unsupported targets.
pub(crate) fn start_backend_native_device_events_impl(
    _backend: AudioBackend,
) -> RuntimeResult<Box<dyn audio_core::AudioMonitorHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.event.open")).boxed())
}

/// Enumerate host devices for unsupported targets.
pub(crate) fn enumerate_host_devices(
    backend: AudioBackend,
) -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    let _ = backend;
    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.device.list")).boxed())
}
