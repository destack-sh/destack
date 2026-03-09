#[cfg(all(target_os = "android", feature = "audio-aaudio"))]
use super::aaudio;
#[cfg(all(target_os = "linux", feature = "audio-alsa"))]
use super::alsa;
#[cfg(any(target_os = "macos", target_os = "ios"))]
#[cfg(feature = "audio-coreaudio")]
use super::coreaudio;
#[cfg(all(target_os = "linux", feature = "audio-jack"))]
use super::jack;
#[cfg(all(target_os = "android", feature = "audio-opensles"))]
use super::opensles;
#[cfg(all(target_os = "linux", feature = "audio-pipewire"))]
use super::pipewire;
#[cfg(all(target_os = "linux", feature = "audio-pulseaudio"))]
use super::pulseaudio;
use crate::diagnostic::RuntimeResult;
use crate::platform::audio::backend::backend_name;
use crate::platform::audio::core::constants::AudioBackendOpenFlags;
use crate::platform::audio::core::model::{AudioStreamHostState, HostDeviceDescriptor};
use crate::platform::audio::core::monitor::AudioMonitorHandle;
use crate::platform::audio::{AudioBackend, AudioShareMode, AudioStreamConfig};
use crate::platform::core::{self as core_platform, BackendSupport, backend_support_error};
use std::sync::Arc;

/// Return whether one unix backend can exist on this target family.
fn backend_supports_target(backend: AudioBackend) -> bool {
    cfg!(target_os = "linux")
        && matches!(
            backend,
            AudioBackend::Alsa
                | AudioBackend::PulseAudio
                | AudioBackend::PipeWire
                | AudioBackend::Jack
        )
        || cfg!(any(target_os = "macos", target_os = "ios")) && backend == AudioBackend::CoreAudio
        || cfg!(target_os = "android")
            && matches!(backend, AudioBackend::AAudio | AudioBackend::OpenSLES)
}

/// Return whether one unix backend is enabled by compile-time feature selection.
fn backend_feature_enabled(backend: AudioBackend) -> bool {
    // selectors are not real host implementations
    if backend == AudioBackend::Auto || backend == AudioBackend::Null {
        return false;
    }

    #[cfg(all(target_os = "linux", feature = "audio-alsa"))]
    if backend == AudioBackend::Alsa {
        return true;
    }

    #[cfg(all(target_os = "linux", feature = "audio-pulseaudio"))]
    if backend == AudioBackend::PulseAudio {
        return true;
    }

    #[cfg(all(target_os = "linux", feature = "audio-pipewire"))]
    if backend == AudioBackend::PipeWire {
        return true;
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[cfg(feature = "audio-coreaudio")]
    if backend == AudioBackend::CoreAudio {
        return true;
    }

    #[cfg(all(target_os = "android", feature = "audio-aaudio"))]
    if backend == AudioBackend::AAudio {
        return true;
    }

    #[cfg(all(target_os = "android", feature = "audio-opensles"))]
    if backend == AudioBackend::OpenSLES {
        return true;
    }

    #[cfg(all(target_os = "linux", feature = "audio-jack"))]
    if backend == AudioBackend::Jack {
        return true;
    }

    false
}

/// Return whether one enabled unix backend is reachable on the current host.
fn backend_host_available(backend: AudioBackend) -> bool {
    match backend {
        #[cfg(all(target_os = "linux", feature = "audio-alsa"))]
        AudioBackend::Alsa => alsa::is_backend_supported(),
        #[cfg(all(target_os = "linux", feature = "audio-pulseaudio"))]
        AudioBackend::PulseAudio => pulseaudio::is_backend_supported(),
        #[cfg(all(target_os = "linux", feature = "audio-pipewire"))]
        AudioBackend::PipeWire => pipewire::is_backend_supported(),
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        #[cfg(feature = "audio-coreaudio")]
        AudioBackend::CoreAudio => coreaudio::is_backend_supported(),
        #[cfg(all(target_os = "android", feature = "audio-aaudio"))]
        AudioBackend::AAudio => aaudio::is_backend_supported(),
        #[cfg(all(target_os = "android", feature = "audio-opensles"))]
        AudioBackend::OpenSLES => opensles::is_backend_supported(),
        #[cfg(all(target_os = "linux", feature = "audio-jack"))]
        AudioBackend::Jack => jack::is_backend_supported(),
        _ => false,
    }
}

/// Return host backend support for one unix backend.
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

/// Return whether one unix backend supports stream creation.
pub(crate) fn backend_stream_supported(backend: AudioBackend) -> bool {
    // stream support cannot exist when the backend is compiled out
    if !backend_feature_enabled(backend) {
        return false;
    }

    match backend {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        #[cfg(feature = "audio-coreaudio")]
        AudioBackend::CoreAudio => coreaudio::is_stream_supported(),
        _ => backend_support(backend).is_available(),
    }
}

/// Return whether one unix backend supports a native device monitor.
pub(crate) fn backend_supports_native_device_monitor(backend: AudioBackend) -> bool {
    // native monitor support is backend-family specific
    if !backend_feature_enabled(backend) {
        return false;
    }

    matches!(
        backend,
        AudioBackend::Alsa
            | AudioBackend::PulseAudio
            | AudioBackend::PipeWire
            | AudioBackend::CoreAudio
            | AudioBackend::Jack
    )
}

/// Enumerate host devices for one requested unix audio backend.
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
        #[cfg(all(target_os = "linux", feature = "audio-alsa"))]
        AudioBackend::Alsa => alsa::enumerate_host_devices(),
        #[cfg(all(target_os = "linux", feature = "audio-pulseaudio"))]
        AudioBackend::PulseAudio => pulseaudio::enumerate_host_devices(),
        #[cfg(all(target_os = "linux", feature = "audio-pipewire"))]
        AudioBackend::PipeWire => pipewire::enumerate_host_devices(),
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        #[cfg(feature = "audio-coreaudio")]
        AudioBackend::CoreAudio => coreaudio::enumerate_host_devices(),
        #[cfg(all(target_os = "android", feature = "audio-aaudio"))]
        AudioBackend::AAudio => aaudio::enumerate_host_devices(),
        #[cfg(all(target_os = "android", feature = "audio-opensles"))]
        AudioBackend::OpenSLES => opensles::enumerate_host_devices(),
        #[cfg(all(target_os = "linux", feature = "audio-jack"))]
        AudioBackend::Jack => jack::enumerate_host_devices(),
        _ => Err(backend_support_error(
            "destack.audio.device.list",
            "unix",
            BackendSupport::UnsupportedTarget,
        )),
    }
}

/// Open one host stream for one unix backend device.
pub(crate) fn open_host_stream(
    device_info: &HostDeviceDescriptor,
    config: AudioStreamConfig,
    share_mode: AudioShareMode,
    backend_flags: AudioBackendOpenFlags,
) -> RuntimeResult<Arc<AudioStreamHostState>> {
    let _ = (&config, &share_mode);

    // fail before dispatching to one backend-specific stream opener
    if !backend_stream_supported(device_info.backend) {
        return Err(backend_support_error(
            "destack.audio.stream.open",
            backend_name(device_info.backend),
            backend_support(device_info.backend),
        ));
    }

    match device_info.backend {
        #[cfg(all(target_os = "linux", feature = "audio-alsa"))]
        AudioBackend::Alsa => {
            alsa::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(all(target_os = "linux", feature = "audio-pulseaudio"))]
        AudioBackend::PulseAudio => {
            pulseaudio::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(all(target_os = "linux", feature = "audio-pipewire"))]
        AudioBackend::PipeWire => {
            pipewire::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        #[cfg(feature = "audio-coreaudio")]
        AudioBackend::CoreAudio => {
            coreaudio::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(all(target_os = "android", feature = "audio-aaudio"))]
        AudioBackend::AAudio => {
            aaudio::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(all(target_os = "android", feature = "audio-opensles"))]
        AudioBackend::OpenSLES => {
            opensles::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(all(target_os = "linux", feature = "audio-jack"))]
        AudioBackend::Jack => {
            jack::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        _ => Err(backend_support_error(
            "destack.audio.stream.open",
            "unix",
            BackendSupport::UnsupportedTarget,
        )),
    }
}

/// Trigger one unix backend device rescan.
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

/// Resolve one unix host device by stable identifier.
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

/// Start one unix backend native device-event monitor.
pub(crate) fn start_backend_native_device_events(
    backend: AudioBackend,
) -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    // dispatch to the backend-specific native monitor implementation
    #[cfg(all(target_os = "linux", feature = "audio-alsa"))]
    if backend == AudioBackend::Alsa {
        return alsa::start_native_device_event_monitor();
    }

    #[cfg(all(target_os = "linux", feature = "audio-pulseaudio"))]
    if backend == AudioBackend::PulseAudio {
        return pulseaudio::start_native_device_event_monitor();
    }

    #[cfg(all(target_os = "linux", feature = "audio-pipewire"))]
    if backend == AudioBackend::PipeWire {
        return pipewire::start_native_device_event_monitor();
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[cfg(feature = "audio-coreaudio")]
    if backend == AudioBackend::CoreAudio {
        return coreaudio::start_native_device_event_monitor();
    }

    #[cfg(all(target_os = "linux", feature = "audio-jack"))]
    if backend == AudioBackend::Jack {
        return jack::start_native_device_event_monitor();
    }

    Err(backend_support_error(
        "destack.audio.event.open",
        backend_name(backend),
        backend_support(backend),
    ))
}
