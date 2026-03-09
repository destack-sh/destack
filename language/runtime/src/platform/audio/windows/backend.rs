#[cfg(feature = "audio-asio")]
use super::asio;
#[cfg(feature = "audio-wasapi")]
use super::wasapi;
use crate::diagnostic::RuntimeResult;
use crate::platform::audio::backend::{backend_name, backend_not_supported};
use crate::platform::audio::core::{
    AudioBackendOpenFlags, AudioMonitorHandle, AudioStreamHostState, HostDeviceDescriptor,
};
use crate::platform::audio::{AudioBackend, AudioShareMode, AudioStreamConfig};
use crate::platform::core as core_platform;
use crate::runtime::BindingCallContext;

/// Return whether one windows backend is enabled by compile-time feature selection.
fn backend_feature_enabled(backend: AudioBackend) -> bool {
    // backend families that are never host implemented
    if backend == AudioBackend::Auto || backend == AudioBackend::Null {
        return false;
    }

    #[cfg(feature = "audio-wasapi")]
    if backend == AudioBackend::Wasapi {
        return true;
    }

    #[cfg(feature = "audio-asio")]
    if backend == AudioBackend::Asio {
        return true;
    }

    false
}

/// Return whether one windows backend is implemented for this build.
pub(crate) fn backend_supported(backend: AudioBackend) -> bool {
    if !backend_feature_enabled(backend) {
        return false;
    }

    match backend {
        #[cfg(feature = "audio-wasapi")]
        AudioBackend::Wasapi => wasapi::is_backend_supported(),
        #[cfg(feature = "audio-asio")]
        AudioBackend::Asio => asio::is_backend_supported(),
        _ => false,
    }
}

/// Return whether one windows backend supports stream creation.
pub(crate) fn backend_stream_supported(backend: AudioBackend) -> bool {
    if !backend_feature_enabled(backend) {
        return false;
    }

    match backend {
        #[cfg(feature = "audio-wasapi")]
        AudioBackend::Wasapi => wasapi::is_stream_supported(),
        #[cfg(feature = "audio-asio")]
        AudioBackend::Asio => asio::is_stream_supported(),
        _ => false,
    }
}

/// Return whether one windows backend supports a native device monitor.
pub(crate) fn backend_supports_native_device_monitor(backend: AudioBackend) -> bool {
    if !backend_feature_enabled(backend) {
        return false;
    }

    matches!(backend, AudioBackend::Wasapi | AudioBackend::Asio)
}

/// Enumerate host devices for one requested windows backend.
pub(crate) fn enumerate_host_devices(
    backend: AudioBackend,
) -> RuntimeResult<Vec<HostDeviceDescriptor>> {
    if !backend_supported(backend) {
        return Err(backend_not_supported(
            "destack.audio.device.list",
            backend_name(backend),
        ));
    }

    match backend {
        #[cfg(feature = "audio-wasapi")]
        AudioBackend::Wasapi => wasapi::enumerate_host_devices(),
        #[cfg(feature = "audio-asio")]
        AudioBackend::Asio => asio::enumerate_host_devices(),
        _ => Err(backend_not_supported(
            "destack.audio.device.list",
            "windows",
        )),
    }
}

/// Open one host stream for one windows backend device.
pub(crate) fn open_host_stream(
    device_info: &HostDeviceDescriptor,
    config: AudioStreamConfig,
    share_mode: AudioShareMode,
    backend_flags: AudioBackendOpenFlags,
) -> RuntimeResult<Arc<AudioStreamHostState>> {
    if !backend_stream_supported(device_info.backend) {
        return Err(backend_not_supported(
            "destack.audio.stream.open",
            backend_name(device_info.backend),
        ));
    }

    match device_info.backend {
        #[cfg(feature = "audio-wasapi")]
        AudioBackend::Wasapi => {
            wasapi::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(feature = "audio-asio")]
        AudioBackend::Asio => {
            asio::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        _ => Err(backend_not_supported(
            "destack.audio.stream.open",
            "windows",
        )),
    }
}

/// Trigger one windows backend device rescan.
pub(crate) fn rescan_host_backend(backend: AudioBackend) -> RuntimeResult<()> {
    if backend_supported(backend) {
        Ok(())
    } else {
        Err(backend_not_supported(
            "destack.audio.device.rescan",
            backend_name(backend),
        ))
    }
}

/// Resolve one windows host device by stable identifier.
pub(crate) fn resolve_host_device_by_id(
    backend: AudioBackend,
    id: &str,
) -> RuntimeResult<HostDeviceDescriptor> {
    let devices = enumerate_host_devices(backend)?;
    devices
        .into_iter()
        .find(|device| device.id == id)
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.audio.device.open",
                format!("audio device id not found: {id}"),
            )
        })
}

/// Start one windows backend native device-event monitor.
pub(crate) fn start_backend_native_device_events(
    backend: AudioBackend,
) -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    #[cfg(feature = "audio-wasapi")]
    if backend == AudioBackend::Wasapi {
        return wasapi::start_native_device_event_monitor();
    }

    #[cfg(feature = "audio-asio")]
    if backend == AudioBackend::Asio {
        return asio::start_native_device_event_monitor();
    }

    Err(backend_not_supported(
        "destack.audio.event.open",
        backend_name(backend),
    ))
}
