use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::runtime::BindingCallContext;

use super::{
    AudioBackend, AudioBackendCapabilityFlags, AudioBackendDescriptor, AudioBackendSelectionPolicy,
    core as audio_core,
};

#[cfg(unix)]
#[path = "unix/mod.rs"]
mod unix;
#[cfg(unix)]
pub(crate) use unix::*;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod windows;
#[cfg(windows)]
pub(crate) use windows::*;

#[cfg(not(any(unix, windows)))]
#[path = "unsupported.rs"]
mod unsupported;
#[cfg(not(any(unix, windows)))]
pub(crate) use unsupported::*;

/// Return the first available host backend on this target.
fn active_host_backend() -> Option<AudioBackend> {
    preferred_host_backends()
        .iter()
        .copied()
        .find(|backend| backend_supported(*backend))
}

/// Return one stable backend name for diagnostics and descriptor rows.
pub(crate) fn backend_name(backend: AudioBackend) -> &'static str {
    match backend {
        AudioBackend::Auto => "auto",
        AudioBackend::Alsa => "alsa",
        AudioBackend::PulseAudio => "pulseaudio",
        AudioBackend::PipeWire => "pipewire",
        AudioBackend::CoreAudio => "coreaudio",
        AudioBackend::Wasapi => "wasapi",
        AudioBackend::AAudio => "aaudio",
        AudioBackend::OpenSLES => "opensles",
        AudioBackend::Jack => "jack",
        AudioBackend::Asio => "asio",
        AudioBackend::Null => "null",
    }
}

/// Return one backend capability mask for one descriptor row.
fn backend_capability_flags(backend: AudioBackend, available: bool) -> AudioBackendCapabilityFlags {
    if !available {
        return AudioBackendCapabilityFlags(0);
    }

    let mut flags = audio_core::BACKEND_CAPABILITY_HOTPLUG_EVENTS.0
        | audio_core::BACKEND_CAPABILITY_DEFAULT_ROUTE_EVENTS.0;

    // null backend: always available with synthetic shared loopback behavior
    if backend == AudioBackend::Null {
        flags |= audio_core::BACKEND_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0;
        flags |= audio_core::BACKEND_CAPABILITY_SHARED_MODE.0;
        flags |= audio_core::BACKEND_CAPABILITY_LOOPBACK.0;
        flags |= audio_core::BACKEND_CAPABILITY_DEVICE_CLOCK.0;
        flags |= audio_core::BACKEND_CAPABILITY_SCHEDULED_WRITE.0;
        return AudioBackendCapabilityFlags(flags);
    }

    // unavailable stream backends only expose availability and route events
    if !backend_stream_supported(backend) {
        return AudioBackendCapabilityFlags(flags);
    }

    // shared mode families
    if backend == AudioBackend::Wasapi
        || backend == AudioBackend::CoreAudio
        || backend == AudioBackend::PipeWire
        || backend == AudioBackend::PulseAudio
        || backend == AudioBackend::Alsa
        || backend == AudioBackend::AAudio
        || backend == AudioBackend::OpenSLES
        || backend == AudioBackend::Jack
    {
        flags |= audio_core::BACKEND_CAPABILITY_SHARED_MODE.0;
    }

    // exclusive mode families
    if backend == AudioBackend::Wasapi
        || backend == AudioBackend::CoreAudio
        || backend == AudioBackend::Asio
        || backend == AudioBackend::Alsa
        || backend == AudioBackend::AAudio
    {
        flags |= audio_core::BACKEND_CAPABILITY_EXCLUSIVE_MODE.0;
    }

    // loopback-capable host backends
    if backend == AudioBackend::Wasapi
        || backend == AudioBackend::CoreAudio
        || backend == AudioBackend::PipeWire
        || backend == AudioBackend::PulseAudio
    {
        flags |= audio_core::BACKEND_CAPABILITY_LOOPBACK.0;
    }

    // non-interleaved support currently comes from ASIO
    if backend == AudioBackend::Asio {
        flags |= audio_core::BACKEND_CAPABILITY_NON_INTERLEAVED.0;
    }

    // device-clock timestamp correlation support
    if backend == AudioBackend::CoreAudio || backend == AudioBackend::Wasapi {
        flags |= audio_core::BACKEND_CAPABILITY_DEVICE_CLOCK.0;
    }

    // scheduled write lane support
    if backend == AudioBackend::CoreAudio || backend == AudioBackend::Wasapi {
        flags |= audio_core::BACKEND_CAPABILITY_SCHEDULED_WRITE.0;
    }

    // backend disconnect and reset notifications
    if backend == AudioBackend::Wasapi
        || backend == AudioBackend::Asio
        || backend == AudioBackend::Alsa
        || backend == AudioBackend::PipeWire
        || backend == AudioBackend::PulseAudio
        || backend == AudioBackend::AAudio
        || backend == AudioBackend::OpenSLES
    {
        flags |= audio_core::BACKEND_CAPABILITY_BACKEND_DISCONNECT_EVENTS.0;
    }

    AudioBackendCapabilityFlags(flags)
}

/// Build backend descriptors for the current host family.
pub(crate) fn backend_descriptors(context: &BindingCallContext) -> Vec<AudioBackendDescriptor> {
    let active_backend = active_host_backend();
    let ordered = [
        AudioBackend::Auto,
        AudioBackend::Wasapi,
        AudioBackend::CoreAudio,
        AudioBackend::PipeWire,
        AudioBackend::PulseAudio,
        AudioBackend::Alsa,
        AudioBackend::AAudio,
        AudioBackend::OpenSLES,
        AudioBackend::Jack,
        AudioBackend::Asio,
        AudioBackend::Null,
    ];
    let mut rows = Vec::with_capacity(ordered.len());

    for (index, backend) in ordered.iter().enumerate() {
        let available = match backend {
            AudioBackend::Auto => active_backend.is_some(),
            AudioBackend::Null => true,
            _ => backend_supported(*backend),
        };

        rows.push(AudioBackendDescriptor {
            backend: *backend,
            name: context.store_string(backend_name(*backend)),
            available,
            priority: index as u16,
            capability_flags: backend_capability_flags(*backend, available),
        });
    }

    rows
}

/// Resolve one requested backend with one explicit selection policy.
pub(crate) fn resolve_requested_backend(
    backend: AudioBackend,
    backend_policy: AudioBackendSelectionPolicy,
    operation: &'static str,
) -> RuntimeResult<AudioBackend> {
    if backend == AudioBackend::Auto {
        return active_host_backend().ok_or_else(|| {
            RuntimeError::from(PlatformError::not_supported(format!(
                "{operation}: no host backend is currently implemented on this target",
            )))
            .boxed()
        });
    }

    if backend == AudioBackend::Null {
        return Ok(backend);
    }

    if backend_supported(backend) {
        return Ok(backend);
    }

    if backend_policy == AudioBackendSelectionPolicy::AllowFallback
        && let Some(active_backend) = active_host_backend()
    {
        return Ok(active_backend);
    }

    Err(RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: backend {} is not available on this host",
        backend_name(backend),
    )))
    .boxed())
}
