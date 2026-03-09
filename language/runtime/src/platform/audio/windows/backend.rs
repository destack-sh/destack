use std::sync::Arc;

#[cfg(feature = "audio-asio")]
use super::asio;
#[cfg(feature = "audio-wasapi")]
use super::wasapi;
use crate::diagnostic::RuntimeResult;
use crate::platform::audio::backend::backend_name;
use crate::platform::audio::core::constants::AudioBackendOpenFlags;
use crate::platform::audio::core::model::{AudioStreamHostState, HostDeviceDescriptor};
use crate::platform::audio::core::monitor::AudioMonitorHandle;
use crate::platform::audio::{AudioBackend, AudioShareMode, AudioStreamConfig};
use crate::platform::core::{self as core_platform, BackendSupport, backend_support_error};

/// Return whether one windows backend can exist on this target family.
fn backend_supports_target(backend: AudioBackend) -> bool {
    matches!(backend, AudioBackend::Wasapi | AudioBackend::Asio)
}

/// Return whether one windows backend is enabled by compile-time feature selection.
fn backend_feature_enabled(backend: AudioBackend) -> bool {
    // selectors are not real host implementations
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

/// Return whether one enabled windows backend is reachable on the current host.
fn backend_host_available(backend: AudioBackend) -> bool {
    match backend {
        #[cfg(feature = "audio-wasapi")]
        AudioBackend::Wasapi => wasapi::is_backend_supported(),
        #[cfg(feature = "audio-asio")]
        AudioBackend::Asio => asio::is_backend_supported(),
        _ => false,
    }
}

/// Return host backend support for one windows backend.
pub(crate) fn backend_support(backend: AudioBackend) -> BackendSupport {
    // reject backend families that do not exist for this target
    if !backend_supports_target(backend) {
        return BackendSupport::UnsupportedTarget;
    }

    // report feature-gated backends separately from target support
    if !backend_feature_enabled(backend) {
        return BackendSupport::DisabledByBuild;
    }

    // report whether the host integration is reachable right now
    if backend_host_available(backend) {
        BackendSupport::Available
    } else {
        BackendSupport::HostUnavailable
    }
}

/// Return whether one windows backend supports stream creation.
pub(crate) fn backend_stream_supported(backend: AudioBackend) -> bool {
    // stream support cannot exist when the backend is compiled out
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
    // native monitor support is backend-family specific
    if !backend_feature_enabled(backend) {
        return false;
    }

    matches!(backend, AudioBackend::Wasapi | AudioBackend::Asio)
}

/// Enumerate host devices for one requested windows backend.
pub(crate) fn enumerate_host_devices(
    backend: AudioBackend,
) -> RuntimeResult<Vec<HostDeviceDescriptor>> {
    // fail early when the requested backend integration is unavailable
    let support = backend_support(backend);
    if !support.is_available() {
        return Err(backend_support_error(
            "destack.audio.device.list",
            backend_name(backend),
            support,
        ));
    }

    match backend {
        #[cfg(feature = "audio-wasapi")]
        AudioBackend::Wasapi => wasapi::enumerate_host_devices(),
        #[cfg(feature = "audio-asio")]
        AudioBackend::Asio => asio::enumerate_host_devices(),
        _ => Err(backend_support_error(
            "destack.audio.device.list",
            "windows",
            BackendSupport::UnsupportedTarget,
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
    // fail before dispatching to one backend-specific stream opener
    if !backend_stream_supported(device_info.backend) {
        return Err(backend_support_error(
            "destack.audio.stream.open",
            backend_name(device_info.backend),
            backend_support(device_info.backend),
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
        _ => Err(backend_support_error(
            "destack.audio.stream.open",
            "windows",
            BackendSupport::UnsupportedTarget,
        )),
    }
}

/// Trigger one windows backend device rescan.
pub(crate) fn rescan_host_backend(backend: AudioBackend) -> RuntimeResult<()> {
    let support = backend_support(backend);

    // report success when the backend integration exists for this host
    if support.is_available() {
        Ok(())
    } else {
        Err(backend_support_error(
            "destack.audio.device.rescan",
            backend_name(backend),
            support,
        ))
    }
}

/// Resolve one windows host device by stable identifier.
pub(crate) fn resolve_host_device_by_id(
    backend: AudioBackend,
    id: &str,
) -> RuntimeResult<HostDeviceDescriptor> {
    // search the refreshed device snapshot for one stable host id
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
    // dispatch to the backend-specific native monitor implementation
    #[cfg(feature = "audio-wasapi")]
    if backend == AudioBackend::Wasapi {
        return wasapi::start_native_device_event_monitor();
    }

    #[cfg(feature = "audio-asio")]
    if backend == AudioBackend::Asio {
        return asio::start_native_device_event_monitor();
    }

    Err(backend_support_error(
        "destack.audio.event.open",
        backend_name(backend),
        backend_support(backend),
    ))
}
