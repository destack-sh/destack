use std::sync::Arc;

use super::super::backend_name as host_backend_name;
#[cfg(feature = "audio-aaudio")]
use super::aaudio;
#[cfg(feature = "audio-alsa")]
use super::alsa;
#[cfg(feature = "audio-coreaudio")]
use super::coreaudio;
#[cfg(feature = "audio-jack")]
use super::jack;
#[cfg(feature = "audio-opensles")]
use super::opensles;
#[cfg(feature = "audio-pipewire")]
use super::pipewire;
#[cfg(feature = "audio-pulseaudio")]
use super::pulseaudio;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

/// Return whether one unix backend is enabled by compile-time feature selection.
fn backend_feature_enabled(backend: audio_core::AudioBackend) -> bool {
    // backend families that are never host implemented
    if backend == audio_core::AudioBackend::Auto || backend == audio_core::AudioBackend::Null {
        return false;
    }

    #[cfg(feature = "audio-alsa")]
    if backend == audio_core::AudioBackend::Alsa {
        return true;
    }

    #[cfg(feature = "audio-pulseaudio")]
    if backend == audio_core::AudioBackend::PulseAudio {
        return true;
    }

    #[cfg(feature = "audio-pipewire")]
    if backend == audio_core::AudioBackend::PipeWire {
        return true;
    }

    #[cfg(feature = "audio-coreaudio")]
    if backend == audio_core::AudioBackend::CoreAudio {
        return true;
    }

    #[cfg(feature = "audio-aaudio")]
    if backend == audio_core::AudioBackend::AAudio {
        return true;
    }

    #[cfg(feature = "audio-opensles")]
    if backend == audio_core::AudioBackend::OpenSLES {
        return true;
    }

    #[cfg(feature = "audio-jack")]
    if backend == audio_core::AudioBackend::Jack {
        return true;
    }

    false
}

/// Build one not-supported error for one unix audio backend operation.
pub(super) fn backend_not_supported(
    operation: &'static str,
    backend_name: &'static str,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: backend {backend_name} is not implemented",
    )))
    .boxed()
}

/// Return whether one unix backend is implemented for this build.
pub(crate) fn backend_supported(backend: audio_core::AudioBackend) -> bool {
    if !backend_feature_enabled(backend) {
        return false;
    }

    match backend {
        #[cfg(feature = "audio-alsa")]
        audio_core::AudioBackend::Alsa => alsa::is_backend_supported(),
        #[cfg(feature = "audio-pulseaudio")]
        audio_core::AudioBackend::PulseAudio => pulseaudio::is_backend_supported(),
        #[cfg(feature = "audio-pipewire")]
        audio_core::AudioBackend::PipeWire => pipewire::is_backend_supported(),
        #[cfg(feature = "audio-coreaudio")]
        audio_core::AudioBackend::CoreAudio => coreaudio::is_backend_supported(),
        #[cfg(feature = "audio-aaudio")]
        audio_core::AudioBackend::AAudio => aaudio::is_backend_supported(),
        #[cfg(feature = "audio-opensles")]
        audio_core::AudioBackend::OpenSLES => opensles::is_backend_supported(),
        #[cfg(feature = "audio-jack")]
        audio_core::AudioBackend::Jack => jack::is_backend_supported(),
        _ => false,
    }
}

/// Return whether one unix backend supports stream creation.
pub(crate) fn backend_stream_supported(backend: audio_core::AudioBackend) -> bool {
    if !backend_feature_enabled(backend) {
        return false;
    }

    match backend {
        #[cfg(feature = "audio-coreaudio")]
        audio_core::AudioBackend::CoreAudio => coreaudio::is_stream_supported(),
        _ => backend_supported(backend),
    }
}

/// Enumerate host devices for one requested unix audio backend.
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
        #[cfg(feature = "audio-alsa")]
        audio_core::AudioBackend::Alsa => alsa::enumerate_host_devices(),
        #[cfg(feature = "audio-pulseaudio")]
        audio_core::AudioBackend::PulseAudio => pulseaudio::enumerate_host_devices(),
        #[cfg(feature = "audio-pipewire")]
        audio_core::AudioBackend::PipeWire => pipewire::enumerate_host_devices(),
        #[cfg(feature = "audio-coreaudio")]
        audio_core::AudioBackend::CoreAudio => coreaudio::enumerate_host_devices(),
        #[cfg(feature = "audio-aaudio")]
        audio_core::AudioBackend::AAudio => aaudio::enumerate_host_devices(),
        #[cfg(feature = "audio-opensles")]
        audio_core::AudioBackend::OpenSLES => opensles::enumerate_host_devices(),
        #[cfg(feature = "audio-jack")]
        audio_core::AudioBackend::Jack => jack::enumerate_host_devices(),
        _ => Err(backend_not_supported("destack.audio.device.list", "unix")),
    }
}

/// Open one host stream for one unix backend device.
pub(crate) fn open_host_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
    backend_flags: audio_core::AudioBackendOpenFlags,
) -> RuntimeResult<Arc<audio_core::AudioStreamBinding>> {
    let _ = (&config, &share_mode);

    if !backend_stream_supported(device_info.backend) {
        return Err(backend_not_supported(
            "destack.audio.stream.open",
            host_backend_name(device_info.backend),
        ));
    }

    match device_info.backend {
        #[cfg(feature = "audio-alsa")]
        audio_core::AudioBackend::Alsa => {
            alsa::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(feature = "audio-pulseaudio")]
        audio_core::AudioBackend::PulseAudio => {
            pulseaudio::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(feature = "audio-pipewire")]
        audio_core::AudioBackend::PipeWire => {
            pipewire::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(feature = "audio-coreaudio")]
        audio_core::AudioBackend::CoreAudio => {
            coreaudio::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(feature = "audio-aaudio")]
        audio_core::AudioBackend::AAudio => {
            aaudio::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(feature = "audio-opensles")]
        audio_core::AudioBackend::OpenSLES => {
            opensles::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        #[cfg(feature = "audio-jack")]
        audio_core::AudioBackend::Jack => {
            jack::open_host_stream(device_info, config, share_mode, backend_flags)
        }
        _ => Err(backend_not_supported("destack.audio.stream.open", "unix")),
    }
}

/// Trigger one unix backend device rescan.
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

/// Resolve one unix host device by stable identifier.
pub(crate) fn resolve_host_device_by_id(
    backend: audio_core::AudioBackend,
    id: &str,
) -> RuntimeResult<audio_core::HostDeviceDescriptor> {
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

/// Return whether one unix backend exposes native device-event subscriptions.
pub(crate) fn backend_native_device_events_supported(backend: audio_core::AudioBackend) -> bool {
    #[cfg(feature = "audio-alsa")]
    if backend == audio_core::AudioBackend::Alsa {
        return alsa::native_device_events_supported();
    }

    #[cfg(feature = "audio-pulseaudio")]
    if backend == audio_core::AudioBackend::PulseAudio {
        return pulseaudio::native_device_events_supported();
    }

    #[cfg(feature = "audio-pipewire")]
    if backend == audio_core::AudioBackend::PipeWire {
        return pipewire::native_device_events_supported();
    }

    #[cfg(feature = "audio-coreaudio")]
    if backend == audio_core::AudioBackend::CoreAudio {
        return coreaudio::native_device_events_supported();
    }

    #[cfg(feature = "audio-jack")]
    if backend == audio_core::AudioBackend::Jack {
        return jack::native_device_events_supported();
    }

    false
}

/// Start one unix backend native device-event monitor.
pub(crate) fn start_backend_native_device_events(
    context: &BindingCallContext,
    backend: audio_core::AudioBackend,
) -> RuntimeResult<()> {
    #[cfg(feature = "audio-alsa")]
    if backend == audio_core::AudioBackend::Alsa {
        return alsa::start_native_device_event_monitor(context);
    }

    #[cfg(feature = "audio-pulseaudio")]
    if backend == audio_core::AudioBackend::PulseAudio {
        return pulseaudio::start_native_device_event_monitor(context);
    }

    #[cfg(feature = "audio-pipewire")]
    if backend == audio_core::AudioBackend::PipeWire {
        return pipewire::start_native_device_event_monitor(context);
    }

    #[cfg(feature = "audio-coreaudio")]
    if backend == audio_core::AudioBackend::CoreAudio {
        return coreaudio::start_native_device_event_monitor(context);
    }

    #[cfg(feature = "audio-jack")]
    if backend == audio_core::AudioBackend::Jack {
        return jack::start_native_device_event_monitor(context);
    }

    let _ = backend;
    Ok(())
}

/// Stop one unix backend native device-event monitor.
pub(crate) fn stop_backend_native_device_events(
    context: &BindingCallContext,
    backend: audio_core::AudioBackend,
) {
    #[cfg(feature = "audio-alsa")]
    if backend == audio_core::AudioBackend::Alsa {
        alsa::stop_native_device_event_monitor(context);
        return;
    }

    #[cfg(feature = "audio-pulseaudio")]
    if backend == audio_core::AudioBackend::PulseAudio {
        pulseaudio::stop_native_device_event_monitor(context);
        return;
    }

    #[cfg(feature = "audio-pipewire")]
    if backend == audio_core::AudioBackend::PipeWire {
        pipewire::stop_native_device_event_monitor(context);
        return;
    }

    #[cfg(feature = "audio-coreaudio")]
    if backend == audio_core::AudioBackend::CoreAudio {
        coreaudio::stop_native_device_event_monitor(context);
        return;
    }

    #[cfg(feature = "audio-jack")]
    if backend == audio_core::AudioBackend::Jack {
        jack::stop_native_device_event_monitor(context);
        return;
    }

    let _ = backend;
}
