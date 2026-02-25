#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::{AudioBackend, core as audio_core};

pub(crate) use crate::platform::audio::simulation::native::*;

/// Return host backend priority order for unsupported targets.
pub(crate) fn preferred_host_backends() -> &'static [AudioBackend] {
    &[]
}

/// Return whether one host backend is available for unsupported targets.
pub(crate) fn backend_supported(_backend: AudioBackend) -> bool {
    false
}

/// Return whether one host backend supports stream creation on unsupported targets.
pub(crate) fn backend_stream_supported(_backend: AudioBackend) -> bool {
    false
}

/// Return whether one host backend exposes native device-event subscriptions on unsupported targets.
pub(crate) fn backend_native_device_events_supported_impl(_backend: AudioBackend) -> bool {
    false
}

/// Start one host backend native device-event monitor on unsupported targets.
pub(crate) fn start_backend_native_device_events_impl(_backend: AudioBackend) -> RuntimeResult<()> {
    Ok(())
}

/// Stop one host backend native device-event monitor on unsupported targets.
pub(crate) fn stop_backend_native_device_events_impl(_backend: AudioBackend) {}

/// Enumerate host devices for unsupported targets.
pub(crate) fn enumerate_host_devices(
    backend: AudioBackend,
) -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    let _ = backend;
    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.device.list")).boxed())
}
