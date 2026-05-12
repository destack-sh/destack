use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::{
    AudioBackend, AudioBackendCapabilityFlags, AudioBackendDescriptor, AudioBackendSelectionPolicy,
    AudioDeviceListFlags, AudioDeviceOpenFlags, AudioSupportedEventSubscriptionFlags,
    AudioSupportedStreamClockDomains, AudioSupportedStreamFlags,
    AudioSupportedStreamRequirementFlags, core as audio_core,
};
use crate::platform::core::{BackendSupport, aggregate_backend_support, backend_support_error};
use crate::runtime::BindingCallContext;

use super::host as host_audio;

/// Audio backend selectors exposed through the platform surface.
const AUDIO_BACKEND_SELECTORS: &[AudioBackend] = &[
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

/// Advertised support lanes for one backend descriptor row.
#[derive(Clone, Copy)]
struct AudioBackendDescriptorLanes {
    /// The supported device-list flags.
    supported_device_list_flags: AudioDeviceListFlags,
    /// The supported device-open flags.
    supported_device_open_flags: AudioDeviceOpenFlags,
    /// The supported stream option flags.
    supported_stream_flags: AudioSupportedStreamFlags,
    /// The supported stream requirement flags.
    supported_stream_requirement_flags: AudioSupportedStreamRequirementFlags,
    /// The supported event-subscription flags.
    supported_event_subscription_flags: AudioSupportedEventSubscriptionFlags,
    /// The supported stream clock domains.
    supported_stream_clock_domains: AudioSupportedStreamClockDomains,
}

impl AudioBackendDescriptorLanes {
    /// Return one zeroed descriptor lane set.
    fn unavailable() -> Self {
        Self {
            supported_device_list_flags: AudioDeviceListFlags(0),
            supported_device_open_flags: AudioDeviceOpenFlags(0),
            supported_stream_flags: AudioSupportedStreamFlags(0),
            supported_stream_requirement_flags: AudioSupportedStreamRequirementFlags(0),
            supported_event_subscription_flags: AudioSupportedEventSubscriptionFlags(0),
            supported_stream_clock_domains: AudioSupportedStreamClockDomains(0),
        }
    }

    /// Return one descriptor lane set for one resolved backend.
    fn for_backend(backend: AudioBackend, is_stream_supported: bool) -> Self {
        // advertise descriptor lanes that are independent of stream support
        let mut lanes = Self {
            supported_device_list_flags: audio_core::supported_backend_device_list_flags(backend),
            supported_device_open_flags: audio_core::supported_backend_device_open_flags(backend),
            supported_stream_flags: AudioSupportedStreamFlags(0),
            supported_stream_requirement_flags: AudioSupportedStreamRequirementFlags(0),
            supported_event_subscription_flags:
                audio_core::supported_backend_event_subscription_flags(backend),
            supported_stream_clock_domains: AudioSupportedStreamClockDomains(0),
        };

        // advertise stream lanes only when stream creation is actually supported
        if is_stream_supported {
            lanes.supported_stream_flags = audio_core::supported_backend_stream_flags(backend);
            lanes.supported_stream_requirement_flags =
                audio_core::supported_backend_stream_requirement_flags(backend);
            lanes.supported_stream_clock_domains =
                audio_core::supported_backend_stream_clock_domains(backend);
        }

        lanes
    }
}

/// Return the first available host backend on this target.
fn active_host_backend() -> Option<AudioBackend> {
    host_audio::preferred_host_backends()
        .iter()
        .copied()
        .find(|backend| host_audio::backend_support(*backend).is_available())
}

/// Return combined support for the default host backend lane.
fn auto_backend_support() -> BackendSupport {
    aggregate_backend_support(
        host_audio::preferred_host_backends()
            .iter()
            .copied()
            .map(host_audio::backend_support),
    )
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

/// Build one standardized unsupported error for backend-local stub paths.
#[allow(dead_code)]
pub(crate) fn backend_not_supported(
    operation: &'static str,
    backend_name: &'static str,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: backend {backend_name} is not supported on this target",
    )))
    .boxed()
}

/// Return host backend support for one audio backend selector.
pub(crate) fn backend_support(backend: AudioBackend) -> BackendSupport {
    match backend {
        AudioBackend::Auto => auto_backend_support(),
        AudioBackend::Null => BackendSupport::Available,
        _ => host_audio::backend_support(backend),
    }
}

/// Return one backend capability mask for one descriptor row.
fn backend_capability_flags(
    backend: AudioBackend,
    support: BackendSupport,
) -> AudioBackendCapabilityFlags {
    if !support.is_available() {
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
    if !host_audio::backend_stream_supported(backend) {
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

    // native device-event ingress
    if backend != AudioBackend::Null && host_audio::backend_supports_native_device_monitor(backend)
    {
        flags |= audio_core::BACKEND_CAPABILITY_NATIVE_EVENT_FEED.0;
    }

    AudioBackendCapabilityFlags(flags)
}

/// Return one effective backend lane for one descriptor row.
fn descriptor_backend(
    backend: AudioBackend,
    support: BackendSupport,
    active_backend: Option<AudioBackend>,
) -> Option<AudioBackend> {
    if !support.is_available() {
        return None;
    }

    if backend == AudioBackend::Auto {
        return active_backend;
    }

    Some(backend)
}

/// Return one effective backend for descriptor capability probing.
fn descriptor_capability_backend(
    backend: AudioBackend,
    support: BackendSupport,
    active_backend: Option<AudioBackend>,
) -> AudioBackend {
    descriptor_backend(backend, support, active_backend).unwrap_or(backend)
}

/// Return one auto-selection priority for one descriptor row.
fn backend_priority(backend: AudioBackend) -> u16 {
    if backend == AudioBackend::Auto {
        return u16::MAX;
    }

    host_audio::preferred_host_backends()
        .iter()
        .position(|candidate| *candidate == backend)
        .map(|index| u16::MAX.saturating_sub(index as u16 + 1))
        .unwrap_or(0)
}

/// Return the advertised descriptor lanes for one backend row.
fn descriptor_lanes(
    backend: AudioBackend,
    support: BackendSupport,
    active_backend: Option<AudioBackend>,
) -> AudioBackendDescriptorLanes {
    // zero unavailable rows instead of advertising stale backend masks
    let Some(descriptor_backend) = descriptor_backend(backend, support, active_backend) else {
        return AudioBackendDescriptorLanes::unavailable();
    };

    // advertise stream-related lanes only when stream creation is usable
    let is_stream_supported = descriptor_backend == AudioBackend::Null
        || host_audio::backend_stream_supported(descriptor_backend);

    AudioBackendDescriptorLanes::for_backend(descriptor_backend, is_stream_supported)
}

/// Build one backend descriptor row.
fn build_backend_descriptor(
    binding: &BindingCallContext,
    backend: AudioBackend,
    active_backend: Option<AudioBackend>,
) -> AudioBackendDescriptor {
    // resolve backend support and effective descriptor capabilities
    let support = backend_support(backend);
    let capability_backend = descriptor_capability_backend(backend, support, active_backend);
    let lanes = descriptor_lanes(backend, support, active_backend);

    // return the descriptor row for this selector
    AudioBackendDescriptor {
        backend,
        name: binding.store_string(backend_name(backend)),
        support,
        priority: backend_priority(backend),
        capability_flags: backend_capability_flags(capability_backend, support),
        supported_device_list_flags: lanes.supported_device_list_flags,
        supported_device_open_flags: lanes.supported_device_open_flags,
        supported_stream_flags: lanes.supported_stream_flags,
        supported_stream_requirement_flags: lanes.supported_stream_requirement_flags,
        supported_event_subscription_flags: lanes.supported_event_subscription_flags,
        supported_stream_clock_domains: lanes.supported_stream_clock_domains,
    }
}

/// Build backend descriptors for the current host family.
pub(crate) fn backend_descriptors(binding: &BindingCallContext) -> Vec<AudioBackendDescriptor> {
    // resolve the active host backend for auto rows
    let active_backend = active_host_backend();
    let mut rows = Vec::with_capacity(AUDIO_BACKEND_SELECTORS.len());

    // build rows in stable selector order
    for backend in AUDIO_BACKEND_SELECTORS.iter().copied() {
        rows.push(build_backend_descriptor(binding, backend, active_backend));
    }

    rows
}

/// Resolve one requested backend with one explicit selection policy.
pub(crate) fn resolve_requested_backend(
    backend: AudioBackend,
    backend_policy: AudioBackendSelectionPolicy,
    operation: &'static str,
) -> RuntimeResult<AudioBackend> {
    // auto always resolves through the preferred host lane
    if backend == AudioBackend::Auto {
        return active_host_backend().ok_or_else(|| {
            backend_support_error(
                operation,
                backend_name(AudioBackend::Auto),
                auto_backend_support(),
            )
        });
    }

    // null is always handled directly in-process
    if backend == AudioBackend::Null {
        return Ok(backend);
    }

    // keep explicit backends when they are available
    let support = backend_support(backend);
    if support.is_available() {
        return Ok(backend);
    }

    // fall back to the active host backend only when explicitly allowed
    if backend_policy == AudioBackendSelectionPolicy::AllowFallback
        && let Some(active_backend) = active_host_backend()
    {
        return Ok(active_backend);
    }

    Err(backend_support_error(
        operation,
        backend_name(backend),
        support,
    ))
}

/// Return whether one backend supports native device-event monitoring.
pub(crate) fn backend_supports_native_device_monitor(backend: AudioBackend) -> bool {
    host_audio::backend_supports_native_device_monitor(backend)
}

/// Start one backend native device-event monitor.
pub(crate) fn start_backend_native_device_events(
    backend: AudioBackend,
) -> RuntimeResult<Box<dyn audio_core::AudioMonitorHandle>> {
    if !backend_supports_native_device_monitor(backend) {
        return Err(backend_support_error(
            "destack.audio.event.open",
            backend_name(backend),
            backend_support(backend),
        ));
    }

    host_audio::start_backend_native_device_events_impl(backend)
}

/// Enumerate host devices for one resolved backend.
pub(crate) fn enumerate_host_devices(
    backend: AudioBackend,
) -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    host_audio::enumerate_host_devices(backend)
}
