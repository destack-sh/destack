use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError, resource};
use crate::runtime::BindingCallContext;

use super::{
    AudioBackend, AudioBackendCapabilityFlags, AudioBackendDescriptor, AudioBackendSelectionPolicy,
    MidiMessage, MidiPortDescriptor, MidiPortDirection, core as audio_core,
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

/// Return one device-list support mask for one backend.
fn supported_device_list_flags(backend: AudioBackend) -> super::AudioDeviceListFlags {
    audio_core::supported_backend_device_list_flags(backend)
}

/// Return one device-open support mask for one backend.
fn supported_device_open_flags(backend: AudioBackend) -> super::AudioDeviceOpenFlags {
    audio_core::supported_backend_device_open_flags(backend)
}

/// Return one stream-option support mask for one backend.
fn supported_stream_flags(backend: AudioBackend) -> super::AudioSupportedStreamFlags {
    audio_core::supported_backend_stream_flags(backend)
}

/// Return one stream-requirement support mask for one backend.
fn supported_stream_requirement_flags(
    backend: AudioBackend,
) -> super::AudioSupportedStreamRequirementFlags {
    audio_core::supported_backend_stream_requirement_flags(backend)
}

/// Return one event-subscription support mask for one backend.
fn supported_event_subscription_flags(
    backend: AudioBackend,
) -> super::AudioSupportedEventSubscriptionFlags {
    audio_core::supported_backend_event_subscription_flags(backend)
}

/// Return one stream-clock support mask for one backend.
fn supported_stream_clock_domains(
    backend: AudioBackend,
) -> super::AudioSupportedStreamClockDomains {
    audio_core::supported_backend_stream_clock_domains(backend)
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
        let capability_backend = if *backend == AudioBackend::Auto {
            active_backend.unwrap_or(AudioBackend::Auto)
        } else {
            *backend
        };

        rows.push(AudioBackendDescriptor {
            backend: *backend,
            name: context.store_string(backend_name(*backend)),
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

/// Return whether one backend exposes native device-event subscriptions.
pub(crate) fn backend_native_device_events_supported(backend: AudioBackend) -> bool {
    backend_native_device_events_supported_impl(backend)
}

/// Start one backend native device-event monitor.
pub(crate) fn start_backend_native_device_events(backend: AudioBackend) -> RuntimeResult<()> {
    start_backend_native_device_events_impl(backend)
}

/// Stop one backend native device-event monitor.
pub(crate) fn stop_backend_native_device_events(backend: AudioBackend) {
    stop_backend_native_device_events_impl(backend)
}

/// Return one standardized unsupported error for host MIDI lanes.
fn unsupported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Flush queued MIDI output.
///
/// Request immediate flush of queued outbound MIDI messages for one opened output endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific MIDI flush operations where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.midi`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_midi_flush(
    _context: &BindingCallContext,
    handle: resource::MidiPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(unsupported("destack.audio.midi.flush"))
}

/// Close one MIDI endpoint.
///
/// Close one opened MIDI endpoint and release host resources.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific MIDI close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.midi`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_midi_port_close(
    _context: &BindingCallContext,
    handle: resource::MidiPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(unsupported("destack.audio.midi.portClose"))
}

/// List available MIDI endpoints.
///
/// Enumerate host MIDI endpoints for one selected direction.
/// Endpoint visibility and ordering follow host MIDI subsystem behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses CoreMIDI or ALSA sequencer or WinMM or UWP MIDI APIs depending on backend availability.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.midi`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_midi_port_list(
    _context: &BindingCallContext,
    out: *mut NativeSlice<MidiPortDescriptor>,
    direction: MidiPortDirection,
) -> RuntimeResult<()> {
    let _ = (out, direction);

    Err(unsupported("destack.audio.midi.portList"))
}

/// Open one MIDI endpoint.
///
/// Open one host MIDI endpoint for input or output operations.
/// Endpoint open behavior follows host MIDI session policy and sharing semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific MIDI endpoint open operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.midi`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_midi_port_open(
    _context: &BindingCallContext,
    out: *mut resource::MidiPortHandle,
    id: NativeStringRef,
    direction: MidiPortDirection,
) -> RuntimeResult<()> {
    let _ = (out, id, direction);

    Err(unsupported("destack.audio.midi.portOpen"))
}

/// Read MIDI messages.
///
/// Read up to `maxMessages` queued MIDI messages from one opened input endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend MIDI queue receive operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `audio.midi`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_midi_read(
    _context: &BindingCallContext,
    out: *mut NativeArray<MidiMessage>,
    handle: resource::MidiPortHandle,
    maxmessages: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxmessages, timeoutns);

    Err(unsupported("destack.audio.midi.read"))
}

/// Poll MIDI messages without blocking.
///
/// Read up to `maxMessages` queued MIDI messages from one opened input endpoint without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses backend nonblocking MIDI queue receive operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.midi`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_midi_try_read(
    _context: &BindingCallContext,
    out: *mut NativeArray<MidiMessage>,
    handle: resource::MidiPortHandle,
    maxmessages: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, maxmessages);

    Err(unsupported("destack.audio.midi.tryRead"))
}

/// Write MIDI messages.
///
/// Submit one batch of MIDI messages to one opened output endpoint.
///
/// # Platform
/// Unix and Windows.
/// Uses backend MIDI queue send operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.midi`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_audio_midi_write(
    _context: &BindingCallContext,
    out: *mut u32,
    handle: resource::MidiPortHandle,
    messages: NativeArray<MidiMessage>,
) -> RuntimeResult<()> {
    let _ = (out, handle, messages);

    Err(unsupported("destack.audio.midi.write"))
}
