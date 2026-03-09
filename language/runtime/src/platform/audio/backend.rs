use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::{
    AudioBackend, AudioBackendCapabilityFlags, AudioBackendDescriptor, AudioBackendSelectionPolicy,
    AudioDeviceListFlags, AudioDeviceOpenFlags, AudioSupportedEventSubscriptionFlags,
    AudioSupportedStreamClockDomains, AudioSupportedStreamFlags,
    AudioSupportedStreamRequirementFlags, MidiMessage, MidiPortDescriptor, MidiPortDirection,
    core as audio_core,
};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError, resource};
use crate::runtime::BindingCallContext;

use super::native as native_audio;

/// Return the first available host backend on this target.
fn active_host_backend() -> Option<AudioBackend> {
    native_audio::preferred_host_backends()
        .iter()
        .copied()
        .find(|backend| native_audio::backend_supported(*backend))
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

/// Return one not-supported error for one backend operation.
pub(crate) fn backend_not_supported(
    operation: &'static str,
    backend_name: &str,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: backend {backend_name} is not available on this host",
    )))
    .boxed()
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
    if !native_audio::backend_stream_supported(backend) {
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

/// Return one device-list support mask for one backend.
fn supported_device_list_flags(backend: AudioBackend) -> AudioDeviceListFlags {
    audio_core::supported_backend_device_list_flags(backend)
}

/// Return one device-open support mask for one backend.
fn supported_device_open_flags(backend: AudioBackend) -> AudioDeviceOpenFlags {
    audio_core::supported_backend_device_open_flags(backend)
}

/// Return one stream-option support mask for one backend.
fn supported_stream_flags(backend: AudioBackend) -> AudioSupportedStreamFlags {
    audio_core::supported_backend_stream_flags(backend)
}

/// Return one stream-requirement support mask for one backend.
fn supported_stream_requirement_flags(
    backend: AudioBackend,
) -> AudioSupportedStreamRequirementFlags {
    audio_core::supported_backend_stream_requirement_flags(backend)
}

/// Return one event-subscription support mask for one backend.
fn supported_event_subscription_flags(
    backend: AudioBackend,
) -> AudioSupportedEventSubscriptionFlags {
    audio_core::supported_backend_event_subscription_flags(backend)
}

/// Return one stream-clock support mask for one backend.
fn supported_stream_clock_domains(backend: AudioBackend) -> AudioSupportedStreamClockDomains {
    audio_core::supported_backend_stream_clock_domains(backend)
}

/// Build backend descriptors for the current host family.
pub(crate) fn backend_descriptors(binding: &BindingCallContext) -> Vec<AudioBackendDescriptor> {
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
            _ => native_audio::backend_supported(*backend),
        };
        let capability_backend = if *backend == AudioBackend::Auto {
            active_backend.unwrap_or(AudioBackend::Auto)
        } else {
            *backend
        };

        rows.push(AudioBackendDescriptor {
            backend: *backend,
            name: binding.store_string(backend_name(*backend)),
            available,
            priority: index as u16,
            capability_flags: backend_capability_flags(capability_backend, available),
            supported_device_list_flags: supported_device_list_flags(capability_backend),
            supported_device_open_flags: supported_device_open_flags(capability_backend),
            supported_stream_flags: supported_stream_flags(capability_backend),
            supported_stream_requirement_flags: supported_stream_requirement_flags(
                capability_backend,
            ),
            supported_event_subscription_flags: supported_event_subscription_flags(
                capability_backend,
            ),
            supported_stream_clock_domains: supported_stream_clock_domains(capability_backend),
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

    if native_audio::backend_supported(backend) {
        return Ok(backend);
    }

    if backend_policy == AudioBackendSelectionPolicy::AllowFallback
        && let Some(active_backend) = active_host_backend()
    {
        return Ok(active_backend);
    }

    Err(backend_not_supported(operation, backend_name(backend)))
}

/// Return whether one backend supports native device-event monitoring.
pub(crate) fn backend_supports_native_device_monitor(backend: AudioBackend) -> bool {
    native_audio::backend_supports_native_device_monitor(backend)
}

/// Start one backend native device-event monitor.
pub(crate) fn start_backend_native_device_events(
    backend: AudioBackend,
) -> RuntimeResult<Box<dyn audio_core::AudioMonitorHandle>> {
    if !backend_supports_native_device_monitor(backend) {
        return Err(backend_not_supported(
            "destack.audio.event.open",
            backend_name(backend),
        ));
    }

    native_audio::start_backend_native_device_events_impl(backend)
}

/// Enumerate host devices for one resolved backend.
pub(crate) fn enumerate_host_devices(
    backend: AudioBackend,
) -> RuntimeResult<Vec<audio_core::HostDeviceDescriptor>> {
    native_audio::enumerate_host_devices(backend)
}

/// Return one standardized unsupported error for host MIDI lanes.
fn unsupported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Flush queued MIDI output.
pub(crate) unsafe fn destack_audio_midi_flush(
    _ctx: &BindingCallContext,
    handle: resource::MidiPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(unsupported("destack.audio.midi.flush"))
}

/// Close one MIDI endpoint.
pub(crate) unsafe fn destack_audio_midi_port_close(
    _ctx: &BindingCallContext,
    handle: resource::MidiPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(unsupported("destack.audio.midi.portClose"))
}

/// List available MIDI endpoints.
pub(crate) unsafe fn destack_audio_midi_port_list(
    _ctx: &BindingCallContext,
    out: *mut NativeSlice<MidiPortDescriptor>,
    direction: MidiPortDirection,
) -> RuntimeResult<()> {
    let _ = (out, direction);

    Err(unsupported("destack.audio.midi.portList"))
}

/// Open one MIDI endpoint.
pub(crate) unsafe fn destack_audio_midi_port_open(
    _ctx: &BindingCallContext,
    out: *mut resource::MidiPortHandle,
    id: NativeStringRef,
    direction: MidiPortDirection,
) -> RuntimeResult<()> {
    let _ = (out, id, direction);

    Err(unsupported("destack.audio.midi.portOpen"))
}

/// Read MIDI messages.
pub(crate) unsafe fn destack_audio_midi_read(
    _ctx: &BindingCallContext,
    out: *mut NativeArray<MidiMessage>,
    handle: resource::MidiPortHandle,
    max_messages: u32,
    timeout_ns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, max_messages, timeout_ns);

    Err(unsupported("destack.audio.midi.read"))
}

/// Poll MIDI messages without blocking.
pub(crate) unsafe fn destack_audio_midi_try_read(
    _ctx: &BindingCallContext,
    out: *mut NativeArray<MidiMessage>,
    handle: resource::MidiPortHandle,
    max_messages: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, max_messages);

    Err(unsupported("destack.audio.midi.tryRead"))
}

/// Write MIDI messages.
pub(crate) unsafe fn destack_audio_midi_write(
    _ctx: &BindingCallContext,
    out: *mut u32,
    handle: resource::MidiPortHandle,
    messages: NativeArray<MidiMessage>,
) -> RuntimeResult<()> {
    let _ = (out, handle, messages);

    Err(unsupported("destack.audio.midi.write"))
}
