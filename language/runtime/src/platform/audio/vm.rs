use destack_vm::{BindingContext, StringHandle};

use super::{
    AudioBackend, AudioBackendDescriptor, AudioBackendDescriptorVm, AudioBackendSelectionPolicy,
    AudioClockDomain, AudioClockSnapshotVm, AudioDeviceDescriptor, AudioDeviceDescriptorVm,
    AudioDeviceDirection, AudioDeviceListRequestVm, AudioDeviceOpenOptions,
    AudioDeviceOpenOptionsVm, AudioEvent, AudioEventSubscriptionOptionsVm, AudioEventVm,
    AudioStreamAvailabilityVm, AudioStreamClockDomain, AudioStreamConfigVm, AudioStreamDescriptor,
    AudioStreamDescriptorVm, AudioStreamOpenOptionsVm, AudioStreamStateVm, AudioStreamSupportVm,
    AudioStreamTimingVm, native as native_audio,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform;
use crate::platform::abi::NativeSlice;
use crate::platform::core::{
    bytes_to_vm, call_out, intern_string_to_vm as string_to_vm, map_native_slice_to_vm,
    store_bytes_from_vm as bytes_from_vm, store_string_from_vm as string_from_vm,
    store_vm_byte_slices,
};
use crate::platform::{VmSlice, resource};
use crate::runtime::BindingCallContext;

type NativeByteVectors = NativeSlice<NativeSlice<u8>>;
type VmByteVectorList = Vec<VmSlice<u8>>;

/// Convert one native descriptor payload into one vm descriptor payload.
fn device_descriptor_to_vm(
    context: &mut BindingContext<'_>,
    value: AudioDeviceDescriptor,
) -> RuntimeResult<AudioDeviceDescriptorVm> {
    Ok(AudioDeviceDescriptorVm {
        id: context
            .string_handle(unsafe { value.id.as_str()? })
            .map_err(Box::<RuntimeError>::from)?,
        group_id: context
            .string_handle(unsafe { value.group_id.as_str()? })
            .map_err(Box::<RuntimeError>::from)?,
        name: context
            .string_handle(unsafe { value.name.as_str()? })
            .map_err(Box::<RuntimeError>::from)?,
        transport: context
            .string_handle(unsafe { value.transport.as_str()? })
            .map_err(Box::<RuntimeError>::from)?,
        backend: value.backend,
        direction: value.direction,
        connected: value.connected,
        is_raw: value.is_raw,
        is_default_playback: value.is_default_playback,
        is_default_capture: value.is_default_capture,
        is_default_loopback: value.is_default_loopback,
        capability_flags: value.capability_flags,
        supported_device_open_flags: value.supported_device_open_flags,
        supported_stream_flags: value.supported_stream_flags,
        supported_stream_requirement_flags: value.supported_stream_requirement_flags,
        supported_event_subscription_flags: value.supported_event_subscription_flags,
        supported_stream_clock_domains: value.supported_stream_clock_domains,
        preferred_sample_rate: value.preferred_sample_rate,
        min_sample_rate: value.min_sample_rate,
        max_sample_rate: value.max_sample_rate,
        preferred_period_frames: value.preferred_period_frames,
        min_channels: value.min_channels,
        max_channels: value.max_channels,
        preferred_layout: value.preferred_layout,
        preferred_channel_mask: value.preferred_channel_mask,
        supported_channel_mask: value.supported_channel_mask,
        min_period_frames: value.min_period_frames,
        max_period_frames: value.max_period_frames,
        format_mask: value.format_mask,
        share_mode_mask: value.share_mode_mask,
    })
}

/// Convert one native backend descriptor payload into one vm descriptor payload.
fn backend_descriptor_to_vm(
    context: &mut BindingContext<'_>,
    value: AudioBackendDescriptor,
) -> RuntimeResult<AudioBackendDescriptorVm> {
    Ok(AudioBackendDescriptorVm {
        backend: value.backend,
        name: context
            .string_handle(unsafe { value.name.as_str()? })
            .map_err(Box::<RuntimeError>::from)?,
        support: value.support,
        priority: value.priority,
        capability_flags: value.capability_flags,
        supported_device_list_flags: value.supported_device_list_flags,
        supported_device_open_flags: value.supported_device_open_flags,
        supported_stream_flags: value.supported_stream_flags,
        supported_stream_requirement_flags: value.supported_stream_requirement_flags,
        supported_event_subscription_flags: value.supported_event_subscription_flags,
        supported_stream_clock_domains: value.supported_stream_clock_domains,
    })
}

/// Convert one native event payload into one vm event payload.
fn event_to_vm(context: &mut BindingContext<'_>, value: AudioEvent) -> RuntimeResult<AudioEventVm> {
    match value {
        AudioEvent::AudioBackendDisconnectedEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            Ok(AudioEventVm::AudioBackendDisconnectedEvent(
                platform::audio::AudioBackendDisconnectedEventVm {
                    kind,
                    metadata: event.metadata,
                    stream: event.stream,
                },
            ))
        }
        AudioEvent::AudioBackendResetEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            Ok(AudioEventVm::AudioBackendResetEvent(
                platform::audio::AudioBackendResetEventVm {
                    kind,
                    metadata: event.metadata,
                    stream: event.stream,
                },
            ))
        }
        AudioEvent::AudioDefaultCaptureChangedEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            let device_id = event
                .device_id
                .map(|device_id| string_to_vm(context, device_id))
                .transpose()?;
            Ok(AudioEventVm::AudioDefaultCaptureChangedEvent(
                platform::audio::AudioDefaultCaptureChangedEventVm {
                    kind,
                    metadata: event.metadata,
                    device_id,
                },
            ))
        }
        AudioEvent::AudioDefaultLoopbackChangedEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            let device_id = event
                .device_id
                .map(|device_id| string_to_vm(context, device_id))
                .transpose()?;
            Ok(AudioEventVm::AudioDefaultLoopbackChangedEvent(
                platform::audio::AudioDefaultLoopbackChangedEventVm {
                    kind,
                    metadata: event.metadata,
                    device_id,
                },
            ))
        }
        AudioEvent::AudioDefaultPlaybackChangedEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            let device_id = event
                .device_id
                .map(|device_id| string_to_vm(context, device_id))
                .transpose()?;
            Ok(AudioEventVm::AudioDefaultPlaybackChangedEvent(
                platform::audio::AudioDefaultPlaybackChangedEventVm {
                    kind,
                    metadata: event.metadata,
                    device_id,
                },
            ))
        }
        AudioEvent::AudioDeviceAddedEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            let device_id = event
                .device_id
                .map(|device_id| string_to_vm(context, device_id))
                .transpose()?;
            Ok(AudioEventVm::AudioDeviceAddedEvent(
                platform::audio::AudioDeviceAddedEventVm {
                    kind,
                    metadata: event.metadata,
                    device_id,
                },
            ))
        }
        AudioEvent::AudioDeviceFormatChangedEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            let device_id = event
                .device_id
                .map(|device_id| string_to_vm(context, device_id))
                .transpose()?;
            Ok(AudioEventVm::AudioDeviceFormatChangedEvent(
                platform::audio::AudioDeviceFormatChangedEventVm {
                    kind,
                    metadata: event.metadata,
                    device_id,
                },
            ))
        }
        AudioEvent::AudioDeviceRemovedEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            let device_id = event
                .device_id
                .map(|device_id| string_to_vm(context, device_id))
                .transpose()?;
            Ok(AudioEventVm::AudioDeviceRemovedEvent(
                platform::audio::AudioDeviceRemovedEventVm {
                    kind,
                    metadata: event.metadata,
                    device_id,
                },
            ))
        }
        AudioEvent::AudioDeviceReroutedEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            let device_id = event
                .device_id
                .map(|device_id| string_to_vm(context, device_id))
                .transpose()?;
            Ok(AudioEventVm::AudioDeviceReroutedEvent(
                platform::audio::AudioDeviceReroutedEventVm {
                    kind,
                    metadata: event.metadata,
                    device_id,
                },
            ))
        }
        AudioEvent::AudioInterruptionBeganEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            Ok(AudioEventVm::AudioInterruptionBeganEvent(
                platform::audio::AudioInterruptionBeganEventVm {
                    kind,
                    metadata: event.metadata,
                    stream: event.stream,
                },
            ))
        }
        AudioEvent::AudioInterruptionEndedEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            Ok(AudioEventVm::AudioInterruptionEndedEvent(
                platform::audio::AudioInterruptionEndedEventVm {
                    kind,
                    metadata: event.metadata,
                    stream: event.stream,
                },
            ))
        }
        AudioEvent::AudioStreamDeviceChangedEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            let device_id = event
                .device_id
                .map(|device_id| string_to_vm(context, device_id))
                .transpose()?;
            Ok(AudioEventVm::AudioStreamDeviceChangedEvent(
                platform::audio::AudioStreamDeviceChangedEventVm {
                    kind,
                    metadata: event.metadata,
                    stream: event.stream,
                    status_flags: event.status_flags,
                    device_id,
                },
            ))
        }
        AudioEvent::AudioStreamStateChangedEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            Ok(AudioEventVm::AudioStreamStateChangedEvent(
                platform::audio::AudioStreamStateChangedEventVm {
                    kind,
                    metadata: event.metadata,
                    stream: event.stream,
                    status_flags: event.status_flags,
                },
            ))
        }
        AudioEvent::AudioStreamXRunEvent(event) => {
            let kind = string_to_vm(context, event.kind)?;
            let device_id = event
                .device_id
                .map(|device_id| string_to_vm(context, device_id))
                .transpose()?;
            Ok(AudioEventVm::AudioStreamXRunEvent(
                platform::audio::AudioStreamXRunEventVm {
                    kind,
                    metadata: event.metadata,
                    stream: event.stream,
                    status_flags: event.status_flags,
                    xrun_count_delta: event.xrun_count_delta,
                    device_id,
                },
            ))
        }
    }
}

/// Convert one native stream snapshot payload into one vm payload.
fn stream_descriptor_to_vm(
    context: &mut BindingContext<'_>,
    value: AudioStreamDescriptor,
) -> RuntimeResult<AudioStreamDescriptorVm> {
    Ok(AudioStreamDescriptorVm {
        backend: value.backend,
        backend_id: context
            .string_handle(unsafe { value.backend_id.as_str()? })
            .map_err(Box::<RuntimeError>::from)?,
        device_id: context
            .string_handle(unsafe { value.device_id.as_str()? })
            .map_err(Box::<RuntimeError>::from)?,
        sample_rate: value.sample_rate,
        channels: value.channels,
        channel_layout: value.channel_layout,
        channel_mask: value.channel_mask,
        format: value.format,
        period_frames: value.period_frames,
        transfer_mode: value.transfer_mode,
        share_mode: value.share_mode,
        requested_flags: value.requested_flags,
        requested_requirements: value.requested_requirements,
        effective_flags: value.effective_flags,
        effective_requirements: value.effective_requirements,
        period_jitter_ns: value.period_jitter_ns,
        non_interleaved: value.non_interleaved,
        supports_write_at: value.supports_write_at,
        supports_pause: value.supports_pause,
        supports_non_interleaved: value.supports_non_interleaved,
        supports_volume: value.supports_volume,
        supports_mute: value.supports_mute,
        supports_hardware_timestamps: value.supports_hardware_timestamps,
    })
}

/// Convert one VM vectorized byte-slice payload into one runtime vectorized payload.
fn byte_vectors_from_vm(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    values: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<NativeSlice<NativeSlice<u8>>> {
    store_vm_byte_slices(binding, context, values, "buffers")
}

/// Prepare writable native vectorized buffers from one VM vectorized payload.
fn writable_byte_vectors_from_vm(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    values: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<(VmByteVectorList, NativeByteVectors)> {
    let vm_values = values.read_values(&context.read())?;
    let mut native = Vec::with_capacity(vm_values.len());
    for value in &vm_values {
        native.push(binding.store_slice(vec![0u8; value.len as usize]));
    }

    Ok((vm_values, binding.store_slice(native)))
}

/// Copy written native vectorized bytes back into one VM vectorized payload.
fn copy_written_vectors_to_vm(
    context: &mut BindingContext<'_>,
    vm_values: &[VmSlice<u8>],
    native_values: NativeSlice<NativeSlice<u8>>,
    written: u64,
) -> RuntimeResult<()> {
    let native_values = unsafe { native_values.as_slice()? };
    let mut remaining = written as usize;

    for (vm_value, native_value) in vm_values.iter().zip(native_values.iter()) {
        let native_bytes = unsafe { native_value.as_slice()? };
        let copy_len = remaining.min(native_bytes.len());
        let mut vm_bytes = vec![0u8; vm_value.len as usize];
        vm_bytes[..copy_len].copy_from_slice(&native_bytes[..copy_len]);
        vm_value.write_bytes(&mut context.write(), &vm_bytes)?;

        remaining = remaining.saturating_sub(copy_len);
        if remaining == 0 {
            break;
        }
    }

    Ok(())
}

/// Read one timestamp in one selected clock domain.
///
/// Read one clock timestamp for one process-wide domain.
/// Domain availability and precision follow host platform behavior.
///
/// # Platform
/// Unix and Windows.
/// Mirrors host monotonic and wall clock query semantics.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_clock_now(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    domain: AudioClockDomain,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { native_audio::destack_audio_clock_now(binding, out, domain) })
}

/// List host audio backends.
///
/// Enumerate available backend implementations and backend-level feature flags.
///
/// # Platform
/// Unix and Windows.
/// Mirrors cubeb `cubeb_get_backend_names`, libsoundio `soundio_backend_count` plus `soundio_get_backend`, and miniaudio `ma_get_enabled_backends`.
/// Mirrors PortAudio host-api enumeration through `PaHostApiTypeId`.
///
/// # Errors
/// Returns ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_backend_list(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
) -> RuntimeResult<VmSlice<AudioBackendDescriptorVm>> {
    let values = call_out(|out| unsafe { native_audio::destack_audio_backend_list(binding, out) })?;

    map_native_slice_to_vm(context, values, |context, value| {
        backend_descriptor_to_vm(context, *value)
    })
}

/// Read one stream clock snapshot.
///
/// Read one synchronized stream-position and selected clock-domain timestamp snapshot.
/// Snapshot values are advisory and can change immediately after read.
/// This is the strict lane-select API.
/// For one full best-effort snapshot without lane-specific errors use `audio.stream.timing`.
/// Domain-specific lanes like `InputAdc`, `OutputDac`, and `Device` can return `notSupported` when the opened stream does not expose them.
///
/// # Platform
/// Unix and Windows.
/// Mirrors PortAudio `PaStreamCallbackTimeInfo` input and output timestamp correlation.
/// Mirrors ASIO `bufferSwitchTimeInfo` and time-info correlation semantics.
/// Mirrors cubeb `cubeb_stream_get_position` plus latency-correlation snapshots.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_clock(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    domain: AudioStreamClockDomain,
) -> RuntimeResult<AudioClockSnapshotVm> {
    let value = call_out(|out| unsafe {
        native_audio::destack_audio_stream_clock(binding, out, handle, domain)
    })?;
    Ok(AudioClockSnapshotVm {
        stream_frames: value.stream_frames,
        clock_ns: value.clock_ns,
        clock_quality: value.clock_quality,
        callback_ns: value.callback_ns,
        callback_quality: value.callback_quality,
        input_adc_ns: value.input_adc_ns,
        input_adc_quality: value.input_adc_quality,
        output_dac_ns: value.output_dac_ns,
        output_dac_quality: value.output_dac_quality,
        device_ns: value.device_ns,
        device_quality: value.device_quality,
        monotonic_ns: value.monotonic_ns,
    })
}

/// Close one audio device endpoint.
///
/// Close one opened audio endpoint and release host resources.
/// Close semantics follow host backend teardown behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available endpoint close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_device_close(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { native_audio::destack_audio_device_close(binding, handle) }
}

/// Read one default device identifier for the selected direction.
///
/// Resolve one default host audio endpoint for the selected direction and backend policy.
/// Default selection can change asynchronously as host policy changes.
///
/// # Platform
/// Unix and Windows.
/// Mirrors cubeb and libsoundio default-endpoint query semantics.
/// Mirrors SDL default logical-device routing behavior for playback and recording defaults.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_device_default(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    direction: AudioDeviceDirection,
    backend: AudioBackend,
    backend_policy: AudioBackendSelectionPolicy,
) -> RuntimeResult<StringHandle> {
    let value = call_out(|out| unsafe {
        native_audio::destack_audio_device_default(binding, out, direction, backend, backend_policy)
    })?;
    string_to_vm(context, value)
}

/// Read metadata for one opened device endpoint.
///
/// Read one normalized snapshot for one opened device handle.
/// Snapshot values are advisory and can change as host routes are updated.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available endpoint information query APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_device_descriptor(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<AudioDeviceDescriptorVm> {
    let value = call_out(|out| unsafe {
        native_audio::destack_audio_device_descriptor(binding, out, handle)
    })?;
    device_descriptor_to_vm(context, value)
}

/// List available audio devices.
///
/// Enumerate host audio endpoints and return stable identifiers for later open operations.
/// Device visibility and ordering follow host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available device enumeration.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_device_list(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    request: AudioDeviceListRequestVm,
) -> RuntimeResult<VmSlice<AudioDeviceDescriptorVm>> {
    let values =
        call_out(|out| unsafe { native_audio::destack_audio_device_list(binding, out, request) })?;

    map_native_slice_to_vm(context, values, |context, value| {
        device_descriptor_to_vm(context, *value)
    })
}

/// Trigger one backend rescan.
///
/// Request one immediate backend device rescan.
/// This allows recovery from stale backend snapshots after hotplug churn.
///
/// # Platform
/// Unix and Windows.
/// Mirrors libsoundio `soundio_force_device_scan` semantics and backend-native refresh flows.
///
/// # Errors
/// Returns ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_device_rescan(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    backend: AudioBackend,
    backend_policy: AudioBackendSelectionPolicy,
) -> RuntimeResult<()> {
    unsafe { native_audio::destack_audio_device_rescan(binding, backend, backend_policy) }
}

/// Open one audio device endpoint.
///
/// Open one host audio endpoint for playback, capture, duplex, or loopback operation.
/// Handle lifetime and exclusivity semantics follow host backend rules.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available endpoint open operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_device_open(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    id: StringHandle,
    options: AudioDeviceOpenOptionsVm,
) -> RuntimeResult<resource::AudioDeviceHandle> {
    let id = string_from_vm(binding, context, id)?;
    let options = AudioDeviceOpenOptions {
        direction: options.direction,
        backend: options.backend,
        backend_policy: options.backend_policy,
        share_mode: options.share_mode,
        flags: options.flags,
    };

    call_out(|out| unsafe { native_audio::destack_audio_device_open(binding, out, id, options) })
}

/// Close one audio event subscription.
///
/// Close one event subscription and release backend notification resources.
/// Pending events are discarded.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available notification unregistration APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_event_close(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<()> {
    unsafe { native_audio::destack_audio_event_close(binding, handle) }
}

/// Open one audio event subscription.
///
/// Open one backend event subscription for device and optional stream events.
/// Subscription routing and queue depth follow host backend behavior.
///
/// # Platform
/// Unix and Windows.
/// Mirrors cubeb device and stream change callbacks.
/// Mirrors libsoundio device-change and backend-disconnect callback families.
/// Mirrors miniaudio `ma_device_notification_proc` notification routing.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_event_open(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    options: AudioEventSubscriptionOptionsVm,
) -> RuntimeResult<resource::AudioEventHandle> {
    call_out(|out| unsafe { native_audio::destack_audio_event_open(binding, out, options) })
}

/// Wait for one audio event.
///
/// Wait for one pending event from one subscription queue.
/// Timeout uses nanoseconds in the runtime monotonic domain.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available event wait or callback-queue drain operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_event_read(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioEventHandle,
    timeoutns: u64,
) -> RuntimeResult<AudioEventVm> {
    let value = call_out(|out| unsafe {
        native_audio::destack_audio_event_read(binding, out, handle, timeoutns)
    })?;

    event_to_vm(context, value)
}

/// Wait for one batch of audio events.
///
/// Wait for pending events from one subscription queue and return up to `maxEvents` events.
/// Timeout uses nanoseconds in the runtime monotonic domain.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO queue-drain operations where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_event_read_batch(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<AudioEventVm>> {
    let values = call_out(|out| unsafe {
        native_audio::destack_audio_event_read_batch(binding, out, handle, maxevents, timeoutns)
    })?;

    map_native_slice_to_vm(context, values, |context, value| {
        event_to_vm(context, *value)
    })
}

/// Poll one audio event without blocking.
///
/// Poll one pending event from one subscription queue.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available nonblocking event queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_event_try_read(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<AudioEventVm> {
    let value = call_out(|out| unsafe {
        native_audio::destack_audio_event_try_read(binding, out, handle)
    })?;

    event_to_vm(context, value)
}

/// Poll one batch of audio events without blocking.
///
/// Poll pending events from one subscription queue and return up to `maxEvents` events.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO nonblocking queue-drain operations where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.device.monitor`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_event_try_read_batch(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioEventHandle,
    maxevents: u32,
) -> RuntimeResult<VmSlice<AudioEventVm>> {
    let values = call_out(|out| unsafe {
        native_audio::destack_audio_event_try_read_batch(binding, out, handle, maxevents)
    })?;

    map_native_slice_to_vm(context, values, |context, value| {
        event_to_vm(context, *value)
    })
}

/// Read one stream immediate availability sample.
///
/// Read one point-in-time sample of immediately readable and writable frame counts.
/// Values are advisory and can change immediately after read.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream-space query operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_availability(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamAvailabilityVm> {
    call_out(|out| unsafe { native_audio::destack_audio_stream_availability(binding, out, handle) })
}

/// Close one audio stream.
///
/// Close one host audio stream and release backend buffers and synchronization state.
/// Stream handle becomes invalid after close completes.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_close(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    unsafe { native_audio::destack_audio_stream_close(binding, handle) }
}

/// Drain one playback stream.
///
/// Wait for one playback stream to consume currently queued samples.
/// Drain timeout is expressed in nanoseconds in the runtime monotonic domain.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available drain or synchronized-stop operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_drain(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { native_audio::destack_audio_stream_drain(binding, handle, timeoutns) }
}

/// Flush buffered stream data.
///
/// Drop pending buffered data for one stream without closing it.
/// Flushing semantics are backend-defined for capture and duplex streams.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream flush or reset operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_flush(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    unsafe { native_audio::destack_audio_stream_flush(binding, handle) }
}

/// Read one stream negotiated configuration descriptor.
///
/// Read one normalized view of negotiated stream parameters and backend mode.
/// Values reflect backend negotiation outcomes and can differ from open-time requests.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream-parameter query operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_descriptor(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamDescriptorVm> {
    let value = call_out(|out| unsafe {
        native_audio::destack_audio_stream_descriptor(binding, out, handle)
    })?;

    stream_descriptor_to_vm(context, value)
}

/// Open one audio stream on one device.
///
/// Create one host audio stream with explicit sample format, channel, and period configuration.
/// Open options carry optional tuning hints and strict requirement lanes.
/// Any unsatisfied requirement must fail open with `notSupported`.
/// Buffering and latency behavior follow host backend contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream creation APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_open(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfigVm,
    options: AudioStreamOpenOptionsVm,
) -> RuntimeResult<resource::AudioStreamHandle> {
    call_out(|out| unsafe {
        native_audio::destack_audio_stream_open(binding, out, device, config, options)
    })
}

/// Check one audio stream configuration for backend support.
///
/// Check one stream configuration and return backend negotiation results without opening one long-lived stream handle.
/// Requirement flags are resolved into `satisfiedRequirements` and `unsatisfiedRequirements`.
///
/// # Platform
/// Unix and Windows.
/// Mirrors PortAudio `Pa_IsFormatSupported` intent and miniaudio native-format probing behavior.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_support(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfigVm,
    options: AudioStreamOpenOptionsVm,
) -> RuntimeResult<AudioStreamSupportVm> {
    let value = call_out(|out| unsafe {
        native_audio::destack_audio_stream_support(binding, out, device, config, options)
    })?;

    let descriptor = stream_descriptor_to_vm(context, value.descriptor)?;
    Ok(AudioStreamSupportVm {
        supported: value.supported,
        descriptor,
        satisfied_requirements: value.satisfied_requirements,
        unsatisfied_requirements: value.unsatisfied_requirements,
    })
}

/// Read one packet of captured audio frames.
///
/// Read one packet of captured interleaved audio frames from the capture stream.
/// Packet sizing and buffering follow host backend capture contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream read or capture-client operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_read(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let value = call_out(|out| unsafe {
        native_audio::destack_audio_stream_read(binding, out, handle, maxbytes)
    })?;
    bytes_to_vm(context, value)
}

/// Read one packet into vectorized buffers.
///
/// Read one packet of captured audio frames into multiple byte slices.
/// Buffers can represent segmented interleaved payloads or channel planes when non-interleaved mode is active.
///
/// # Platform
/// Unix and Windows.
/// Mirrors readv-style capture behavior and backend non-interleaved lanes where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_readv(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let (vm_buffers, native_buffers) = writable_byte_vectors_from_vm(binding, context, buffers)?;
    let written = call_out(|out| unsafe {
        native_audio::destack_audio_stream_readv(binding, out, handle, native_buffers)
    })?;
    copy_written_vectors_to_vm(context, &vm_buffers, native_buffers, written)?;

    Ok(written)
}

/// Set one stream mute state.
///
/// Apply one mute state for one stream processing lane.
/// This controls stream-level mute and does not imply global endpoint mute ownership.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO stream-level mute paths where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_set_mute(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    muted: bool,
) -> RuntimeResult<()> {
    unsafe { native_audio::destack_audio_stream_set_mute(binding, handle, muted) }
}

/// Set one stream name.
///
/// Apply one stream label for host mixers and diagnostics where supported.
/// Backend label visibility and truncation follow host policy.
///
/// # Platform
/// Unix and Windows.
/// Mirrors cubeb `cubeb_stream_set_name` behavior where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_set_name(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    name: StringHandle,
) -> RuntimeResult<()> {
    let name = string_from_vm(binding, context, name)?;
    unsafe { native_audio::destack_audio_stream_set_name(binding, handle, name) }
}

/// Set one stream gain multiplier.
///
/// Apply one linear gain multiplier for one stream processing lane.
/// This controls stream-level gain and does not imply global endpoint mixer ownership.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO stream-level gain paths where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_set_volume(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    lineargain: f64,
) -> RuntimeResult<()> {
    unsafe { native_audio::destack_audio_stream_set_volume(binding, handle, lineargain) }
}

/// Start one audio stream.
///
/// Transition one opened stream to running state and begin host DMA or scheduler processing.
/// Start timing follows host backend scheduling semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream start operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_start(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    unsafe { native_audio::destack_audio_stream_start(binding, handle) }
}

/// Pause or resume one audio stream.
///
/// Transition one running stream into paused state and back.
/// Pause support is backend-dependent.
///
/// # Platform
/// Unix and Windows.
/// Mirrors libsoundio `soundio_outstream_pause` and SDL stream-device pause semantics.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_pause(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    pause: bool,
) -> RuntimeResult<()> {
    unsafe { native_audio::destack_audio_stream_pause(binding, handle, pause) }
}

/// Abort one audio stream immediately.
///
/// Request one immediate stream stop without graceful drain.
/// Pending buffered data can be discarded.
///
/// # Platform
/// Unix and Windows.
/// Mirrors PortAudio `Pa_AbortStream` semantics.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_abort(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    unsafe { native_audio::destack_audio_stream_abort(binding, handle) }
}

/// Read one stream state.
///
/// Read one point-in-time state sample of stream run state and backend buffering metrics.
/// State values are advisory and can change immediately after read.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream query primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_state(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamStateVm> {
    call_out(|out| unsafe { native_audio::destack_audio_stream_state(binding, out, handle) })
}

/// Stop one audio stream.
///
/// Transition one running stream to stopped state and flush host backend scheduling.
/// Buffered frames can be discarded based on host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream stop operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `audio.stream`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_stop(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    unsafe { native_audio::destack_audio_stream_stop(binding, handle) }
}

/// Read one stream timing sample.
///
/// Read one full timing sample that correlates stream position and available backend clocks.
/// Missing optional lanes are reported through `has*` fields instead of `notSupported`.
/// Timing values are intended for drift correction and synchronization.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream-clock query operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.control`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_timing(
    binding: &BindingCallContext,
    _context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamTimingVm> {
    call_out(|out| unsafe { native_audio::destack_audio_stream_timing(binding, out, handle) })
}

/// Try to read one packet of captured audio frames without blocking.
///
/// Read one packet of captured interleaved audio frames without waiting.
/// Empty input state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available nonblocking stream read operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_try_read(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let value = call_out(|out| unsafe {
        native_audio::destack_audio_stream_try_read(binding, out, handle, maxbytes)
    })?;
    bytes_to_vm(context, value)
}

/// Try to read one packet into vectorized buffers without blocking.
///
/// Read one packet of captured audio frames into multiple byte slices without waiting.
/// Empty input state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Mirrors nonblocking readv-style capture behavior.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.capture`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_try_readv(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let (vm_buffers, native_buffers) = writable_byte_vectors_from_vm(binding, context, buffers)?;
    let written = call_out(|out| unsafe {
        native_audio::destack_audio_stream_try_readv(binding, out, handle, native_buffers)
    })?;
    copy_written_vectors_to_vm(context, &vm_buffers, native_buffers, written)?;

    Ok(written)
}

/// Try to write one packet of audio frames without blocking.
///
/// Submit one packet of interleaved audio frames to the playback stream without waiting.
/// Empty output space is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available nonblocking stream write operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_try_write(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    data: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let data = bytes_from_vm(binding, context, data)?;
    call_out(|out| unsafe {
        native_audio::destack_audio_stream_try_write(binding, out, handle, data)
    })
}

/// Try to write one packet from vectorized buffers without blocking.
///
/// Submit one packet of audio frames from multiple byte slices without waiting.
/// Empty output space is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Mirrors nonblocking writev-style submission behavior.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_try_writev(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let buffers = byte_vectors_from_vm(binding, context, buffers)?;
    call_out(|out| unsafe {
        native_audio::destack_audio_stream_try_writev(binding, out, handle, buffers)
    })
}

/// Write one packet of audio frames.
///
/// Submit one packet of interleaved audio frames to the playback stream.
/// Short writes can occur when host buffers are near capacity.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available stream write or render-client operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_write(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    data: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let data = bytes_from_vm(binding, context, data)?;
    call_out(|out| unsafe { native_audio::destack_audio_stream_write(binding, out, handle, data) })
}

/// Write one packet from vectorized buffers.
///
/// Submit one packet of audio frames from multiple byte slices.
/// Buffers can represent segmented interleaved payloads or channel planes when non-interleaved mode is active.
///
/// # Platform
/// Unix and Windows.
/// Mirrors writev-style submission behavior and backend non-interleaved lanes where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_writev(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let buffers = byte_vectors_from_vm(binding, context, buffers)?;
    call_out(|out| unsafe {
        native_audio::destack_audio_stream_writev(binding, out, handle, buffers)
    })
}

/// Write one packet for one target presentation time.
///
/// Submit one packet of interleaved audio frames for one target presentation timestamp.
/// Scheduling precision depends on host backend timing guarantees.
/// This is one optional scheduling lane and can return `notSupported` when unavailable.
///
/// # Platform
/// Unix and Windows.
/// Uses ALSA, PulseAudio, PipeWire, CoreAudio, WASAPI, AAudio, OpenSL ES, JACK, and ASIO where available scheduled-render operations when available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback.schedule`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_write_at(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    data: VmSlice<u8>,
    presentationtimens: u64,
) -> RuntimeResult<u64> {
    let data = bytes_from_vm(binding, context, data)?;
    call_out(|out| unsafe {
        native_audio::destack_audio_stream_write_at(binding, out, handle, data, presentationtimens)
    })
}

/// Write one vectorized packet for one target presentation time.
///
/// Submit one packet of audio frames from multiple byte slices for one target presentation timestamp.
/// Buffers can represent segmented interleaved payloads or channel planes when non-interleaved mode is active.
/// Scheduling precision depends on host backend timing guarantees.
///
/// # Platform
/// Unix and Windows.
/// Mirrors scheduled-render operations where available and extends them for writev-style payload submission.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `audio.playback.schedule`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_audio_stream_write_atv(
    binding: &BindingCallContext,
    context: &mut BindingContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
    presentationtimens: u64,
) -> RuntimeResult<u64> {
    let buffers = byte_vectors_from_vm(binding, context, buffers)?;
    call_out(|out| unsafe {
        native_audio::destack_audio_stream_write_atv(
            binding,
            out,
            handle,
            buffers,
            presentationtimens,
        )
    })
}
