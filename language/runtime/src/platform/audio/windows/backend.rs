use std::sync::Arc;

use super::super::backend_name as host_backend_name;
#[cfg(feature = "audio-asio")]
use super::asio;
#[cfg(feature = "audio-wasapi")]
use super::wasapi;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::core as audio_core;
use crate::runtime::BindingCallContext;

/// Return whether one windows backend is enabled by compile-time feature selection.
fn backend_feature_enabled(backend: audio_core::AudioBackend) -> bool {
    // backend families that are never host implemented
    if backend == audio_core::AudioBackend::Auto || backend == audio_core::AudioBackend::Null {
        return false;
    }

    #[cfg(feature = "audio-wasapi")]
    if backend == audio_core::AudioBackend::Wasapi {
        return true;
    }

    #[cfg(feature = "audio-asio")]
    if backend == audio_core::AudioBackend::Asio {
        return true;
    }

    false
}

/// Build one not-supported error for one windows audio backend operation.
pub(super) fn backend_not_supported(
    operation: &'static str,
    backend_name: &'static str,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: backend {backend_name} is not implemented",
    )))
    .boxed()
}

/// Return whether one windows backend is implemented for this build.
pub(crate) fn backend_supported(backend: audio_core::AudioBackend) -> bool {
    if !backend_feature_enabled(backend) {
        return false;
    }

    match backend {
        #[cfg(feature = "audio-wasapi")]
        audio_core::AudioBackend::Wasapi => wasapi::is_backend_supported(),
        #[cfg(feature = "audio-asio")]
        audio_core::AudioBackend::Asio => asio::is_backend_supported(),
        _ => false,
    }
}

/// Return whether one windows backend supports stream creation.
pub(crate) fn backend_stream_supported(backend: audio_core::AudioBackend) -> bool {
    if !backend_feature_enabled(backend) {
        return false;
    }

    match backend {
        #[cfg(feature = "audio-wasapi")]
        audio_core::AudioBackend::Wasapi => wasapi::is_stream_supported(),
        #[cfg(feature = "audio-asio")]
        audio_core::AudioBackend::Asio => asio::is_stream_supported(),
        _ => false,
    }
}

/// Enumerate host devices for one requested windows backend.
pub(crate) fn enumerate_host_devices(
    backend: audio_core::AudioBackend,
) -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    if !backend_supported(backend) {
        return Err(backend_not_supported(
            "destack.audio.device.list",
            host_backend_name(backend),
        ));
    }

    match backend {
        #[cfg(feature = "audio-wasapi")]
        audio_core::AudioBackend::Wasapi => wasapi::enumerate_host_devices(),
        #[cfg(feature = "audio-asio")]
        audio_core::AudioBackend::Asio => asio::enumerate_host_devices(),
        _ => Err(backend_not_supported(
            "destack.audio.device.list",
            "windows",
        )),
    }
}

/// Open one host stream for one windows backend device.
pub(crate) fn open_host_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
    backend_flags: audio_core::AudioBackendOpenFlags,
) -> RuntimeResult<Arc<audio_core::AudioStreamBinding>> {
    if !backend_stream_supported(device_info.backend) {
        return Err(backend_not_supported(
            "destack.audio.stream.open",
            host_backend_name(device_info.backend),
        ));
    }

    match device_info.backend {
        #[cfg(feature = "audio-wasapi")]
        audio_core::AudioBackend::Wasapi => {
            wasapi::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(feature = "audio-asio")]
        audio_core::AudioBackend::Asio => {
            asio::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        _ => Err(backend_not_supported(
            "destack.audio.stream.open",
            "windows",
        )),
    }
}

/// Trigger one windows backend device rescan.
pub(crate) fn rescan_host_backend(backend: audio_core::AudioBackend) -> RuntimeResult<()> {
    if backend_supported(backend) {
        Ok(())
    } else {
        Err(backend_not_supported(
            "destack.audio.device.rescan",
            host_backend_name(backend),
        ))
    }
}

/// Resolve one windows host device by stable identifier.
pub(crate) fn resolve_host_device_by_id(
    backend: audio_core::AudioBackend,
    id: &str,
) -> RuntimeResult<audio_core::HostDeviceDescriptor> {
    let devices = enumerate_host_devices(backend)?;
    devices
        .into_iter()
        .find(|device| device.id == id)
        .ok_or_else(|| {
            audio_core::audio_not_found(
                "destack.audio.device.open",
                format!("audio device id not found: {id}"),
            )
        })
}

/// Return whether one windows backend exposes native device-event subscriptions.
pub(crate) fn backend_native_device_events_supported(backend: audio_core::AudioBackend) -> bool {
    #[cfg(feature = "audio-wasapi")]
    if backend == audio_core::AudioBackend::Wasapi {
        return wasapi::native_device_events_supported();
    }

    #[cfg(feature = "audio-asio")]
    if backend == audio_core::AudioBackend::Asio {
        return asio::native_device_events_supported();
    }

    let _ = backend;
    false
}

/// Start one windows backend native device-event monitor.
pub(crate) fn start_backend_native_device_events(
    context: &BindingCallContext,
    backend: audio_core::AudioBackend,
) -> RuntimeResult<()> {
    #[cfg(feature = "audio-wasapi")]
    if backend == audio_core::AudioBackend::Wasapi {
        return wasapi::start_native_device_event_monitor(context);
    }

    #[cfg(feature = "audio-asio")]
    if backend == audio_core::AudioBackend::Asio {
        return asio::start_native_device_event_monitor(context);
    }

    let _ = backend;
    Ok(())
}

/// Stop one windows backend native device-event monitor.
pub(crate) fn stop_backend_native_device_events(
    context: &BindingCallContext,
    backend: audio_core::AudioBackend,
) {
    #[cfg(feature = "audio-wasapi")]
    if backend == audio_core::AudioBackend::Wasapi {
        wasapi::stop_native_device_event_monitor(context);
        return;
    }

    #[cfg(feature = "audio-asio")]
    if backend == audio_core::AudioBackend::Asio {
        asio::stop_native_device_event_monitor(context);
        return;
    }

    let _ = backend;
}
