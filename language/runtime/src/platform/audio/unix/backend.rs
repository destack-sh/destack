use std::sync::Arc;

use super::super::backend_name as host_backend_name;
use super::{aaudio, alsa, coreaudio, jack, opensles, pipewire, pulseaudio};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::core as audio_core;

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
    match backend {
        audio_core::AudioBackend::Alsa => alsa::is_backend_supported(),
        audio_core::AudioBackend::PulseAudio => pulseaudio::is_backend_supported(),
        audio_core::AudioBackend::PipeWire => pipewire::is_backend_supported(),
        audio_core::AudioBackend::CoreAudio => coreaudio::is_backend_supported(),
        audio_core::AudioBackend::AAudio => aaudio::is_backend_supported(),
        audio_core::AudioBackend::OpenSLES => opensles::is_backend_supported(),
        audio_core::AudioBackend::Jack => jack::is_backend_supported(),
        _ => false,
    }
}

/// Return whether one unix backend supports stream creation.
pub(crate) fn backend_stream_supported(backend: audio_core::AudioBackend) -> bool {
    match backend {
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
        audio_core::AudioBackend::Alsa => alsa::enumerate_host_devices(),
        audio_core::AudioBackend::PulseAudio => pulseaudio::enumerate_host_devices(),
        audio_core::AudioBackend::PipeWire => pipewire::enumerate_host_devices(),
        audio_core::AudioBackend::CoreAudio => coreaudio::enumerate_host_devices(),
        audio_core::AudioBackend::AAudio => aaudio::enumerate_host_devices(),
        audio_core::AudioBackend::OpenSLES => opensles::enumerate_host_devices(),
        audio_core::AudioBackend::Jack => jack::enumerate_host_devices(),
        _ => Err(backend_not_supported("destack.audio.device.list", "unix")),
    }
}

/// Open one host stream for one unix backend device.
pub(crate) fn open_host_stream(
    device_info: &audio_core::HostDeviceDescriptor,
    config: audio_core::AudioStreamConfig,
    share_mode: audio_core::AudioShareMode,
) -> RuntimeResult<Arc<audio_core::AudioStreamBinding>> {
    if !backend_stream_supported(device_info.backend) {
        return Err(backend_not_supported(
            "destack.audio.stream.open",
            host_backend_name(device_info.backend),
        ));
    }

    match device_info.backend {
        audio_core::AudioBackend::Alsa => alsa::open_host_stream(device_info, config, share_mode),
        audio_core::AudioBackend::PulseAudio => {
            pulseaudio::open_host_stream(device_info, config, share_mode)
        }
        audio_core::AudioBackend::PipeWire => {
            pipewire::open_host_stream(device_info, config, share_mode)
        }
        audio_core::AudioBackend::CoreAudio => {
            coreaudio::open_host_stream(device_info, config, share_mode)
        }
        audio_core::AudioBackend::AAudio => {
            aaudio::open_host_stream(device_info, config, share_mode)
        }
        audio_core::AudioBackend::OpenSLES => {
            opensles::open_host_stream(device_info, config, share_mode)
        }
        audio_core::AudioBackend::Jack => jack::open_host_stream(device_info, config, share_mode),
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
            audio_core::audio_not_found(
                "destack.audio.device.open",
                format!("audio device id not found: {id}"),
            )
        })
}
