use destack_vm as vm;

use super::{
    AudioBackend, AudioBackendDescriptor, AudioBackendDescriptorVm, AudioBackendSelectionPolicy,
    AudioClockDomain, AudioClockSnapshotVm, AudioDeviceDescriptor, AudioDeviceDescriptorVm,
    AudioDeviceDirection, AudioDeviceListRequestVm, AudioDeviceOpenOptions,
    AudioDeviceOpenOptionsVm, AudioEvent, AudioEventSubscriptionOptionsVm, AudioEventVm,
    AudioStreamAvailabilityVm, AudioStreamClockDomain, AudioStreamConfigVm, AudioStreamDescriptor,
    AudioStreamDescriptorVm, AudioStreamOpenOptionsVm, AudioStreamStateVm, AudioStreamSupportVm,
    AudioStreamTimingVm, MidiMessageVm, MidiPortDescriptorVm, MidiPortDirection,
    host as host_audio,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{NativeSlice, NativeStringRef, PlatformError, VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;

type NativeByteVectors = NativeSlice<NativeSlice<u8>>;
type VmByteVectorList = Vec<VmSlice<u8>>;

/// Invoke one host call that writes through an out pointer.
fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    let mut out = std::mem::MaybeUninit::<T>::uninit();
    call(out.as_mut_ptr())?;
    Ok(unsafe { out.assume_init() })
}

/// Convert one VM string into one runtime string reference.
fn string_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    let value = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    Ok(runtime.store_string(value.as_str()))
}

/// Convert one native string into one VM string handle.
fn string_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeStringRef,
) -> RuntimeResult<vm::StringHandle> {
    let value = unsafe { value.as_str()? };
    Ok(vm::StringHandle::new(context.intern_string(value)))
}

/// Convert one VM device-open options payload into one native payload.
fn device_open_options_from_vm(
    value: AudioDeviceOpenOptionsVm,
) -> RuntimeResult<AudioDeviceOpenOptions> {
    Ok(AudioDeviceOpenOptions {
        direction: value.direction,
        backend: value.backend,
        backend_policy: value.backend_policy,
        share_mode: value.share_mode,
        flags: value.flags,
    })
}

/// Convert one native descriptor payload into one vm descriptor payload.
fn device_descriptor_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: AudioDeviceDescriptor,
) -> RuntimeResult<AudioDeviceDescriptorVm> {
    Ok(AudioDeviceDescriptorVm {
        id: vm::StringHandle::new(context.intern_string(unsafe { value.id.as_str()? })),
        group_id: vm::StringHandle::new(context.intern_string(unsafe { value.group_id.as_str()? })),
        name: vm::StringHandle::new(context.intern_string(unsafe { value.name.as_str()? })),
        transport: vm::StringHandle::new(
            context.intern_string(unsafe { value.transport.as_str()? }),
        ),
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
    context: &mut vm::ExternalCallContext<'_>,
    value: AudioBackendDescriptor,
) -> RuntimeResult<AudioBackendDescriptorVm> {
    Ok(AudioBackendDescriptorVm {
        backend: value.backend,
        name: vm::StringHandle::new(context.intern_string(unsafe { value.name.as_str()? })),
        available: value.available,
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
fn event_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: AudioEvent,
) -> RuntimeResult<AudioEventVm> {
    Ok(AudioEventVm {
        kind: value.kind,
        timestamp_ns: value.timestamp_ns,
        sequence: value.sequence,
        dropped_count: value.dropped_count,
        source: value.source,
        backend: value.backend,
        flags: value.flags,
        status_flags: value.status_flags,
        xrun_count_delta: value.xrun_count_delta,
        has_device_id: value.has_device_id,
        device_id: vm::StringHandle::new(
            context.intern_string(unsafe { value.device_id.as_str()? }),
        ),
        has_stream: value.has_stream,
        stream: value.stream,
    })
}

/// Convert one native stream snapshot payload into one vm payload.
fn stream_descriptor_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: AudioStreamDescriptor,
) -> RuntimeResult<AudioStreamDescriptorVm> {
    Ok(AudioStreamDescriptorVm {
        backend: value.backend,
        backend_id: vm::StringHandle::new(
            context.intern_string(unsafe { value.backend_id.as_str()? }),
        ),
        device_id: vm::StringHandle::new(
            context.intern_string(unsafe { value.device_id.as_str()? }),
        ),
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

/// Convert one native descriptor slice into one vm slice.
fn descriptor_slice_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeSlice<AudioDeviceDescriptor>,
) -> RuntimeResult<VmSlice<AudioDeviceDescriptorVm>> {
    let values = unsafe { values.as_slice()? };
    let mut vm_values = Vec::with_capacity(values.len());
    for value in values {
        vm_values.push(device_descriptor_to_vm(context, *value)?);
    }

    VmSlice::from_values(context, &vm_values)
}

/// Convert one native backend-descriptor slice into one vm slice.
fn backend_slice_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeSlice<AudioBackendDescriptor>,
) -> RuntimeResult<VmSlice<AudioBackendDescriptorVm>> {
    let values = unsafe { values.as_slice()? };
    let mut vm_values = Vec::with_capacity(values.len());
    for value in values {
        vm_values.push(backend_descriptor_to_vm(context, *value)?);
    }

    VmSlice::from_values(context, &vm_values)
}

/// Convert one native event slice into one vm slice.
fn event_slice_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeSlice<AudioEvent>,
) -> RuntimeResult<VmSlice<AudioEventVm>> {
    let values = unsafe { values.as_slice()? };
    let mut vm_values = Vec::with_capacity(values.len());
    for value in values {
        vm_values.push(event_to_vm(context, *value)?);
    }

    VmSlice::from_values(context, &vm_values)
}

/// Convert one vm byte slice into one runtime native byte slice.
fn bytes_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    values: VmSlice<u8>,
) -> RuntimeResult<NativeSlice<u8>> {
    let values = values.read_bytes(context)?;
    Ok(runtime.store_slice(values))
}

/// Convert one native byte slice into one vm byte slice.
fn bytes_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: NativeSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let values = unsafe { values.as_slice()? };
    Ok(VmSlice::from_bytes(context, values))
}

/// Convert one VM vectorized byte-slice payload into one runtime vectorized payload.
fn byte_vectors_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    values: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<NativeSlice<NativeSlice<u8>>> {
    let values = values.read_values(context)?;
    let mut native = Vec::with_capacity(values.len());
    for value in values {
        let bytes = value.read_bytes(context)?;
        native.push(runtime.store_slice(bytes));
    }

    Ok(runtime.store_slice(native))
}

/// Prepare writable native vectorized buffers from one VM vectorized payload.
fn writable_byte_vectors_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    values: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<(VmByteVectorList, NativeByteVectors)> {
    let vm_values = values.read_values(context)?;
    let mut native = Vec::with_capacity(vm_values.len());
    for value in &vm_values {
        native.push(runtime.store_slice(vec![0u8; value.len as usize]));
    }

    Ok((vm_values, runtime.store_slice(native)))
}

/// Copy written native vectorized bytes back into one VM vectorized payload.
fn copy_written_vectors_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
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
        vm_value.write_bytes(context, &vm_bytes)?;

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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    domain: AudioClockDomain,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_audio::destack_audio_clock_now(runtime, out, domain) })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<AudioBackendDescriptorVm>> {
    let values = call_out(|out| unsafe { host_audio::destack_audio_backend_list(runtime, out) })?;
    backend_slice_to_vm(context, values)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    domain: AudioStreamClockDomain,
) -> RuntimeResult<AudioClockSnapshotVm> {
    let value = call_out(|out| unsafe {
        host_audio::destack_audio_stream_clock(runtime, out, handle, domain)
    })?;
    Ok(AudioClockSnapshotVm {
        stream_frames: value.stream_frames,
        clock_ns: value.clock_ns,
        clock_quality: value.clock_quality,
        has_callback_ns: value.has_callback_ns,
        callback_ns: value.callback_ns,
        callback_quality: value.callback_quality,
        has_input_adc_ns: value.has_input_adc_ns,
        input_adc_ns: value.input_adc_ns,
        input_adc_quality: value.input_adc_quality,
        has_output_dac_ns: value.has_output_dac_ns,
        output_dac_ns: value.output_dac_ns,
        output_dac_quality: value.output_dac_quality,
        has_device_ns: value.has_device_ns,
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<()> {
    unsafe { host_audio::destack_audio_device_close(runtime, handle) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    direction: AudioDeviceDirection,
    backend: AudioBackend,
    backend_policy: AudioBackendSelectionPolicy,
) -> RuntimeResult<vm::StringHandle> {
    let value = call_out(|out| unsafe {
        host_audio::destack_audio_device_default(runtime, out, direction, backend, backend_policy)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioDeviceHandle,
) -> RuntimeResult<AudioDeviceDescriptorVm> {
    let value = call_out(|out| unsafe {
        host_audio::destack_audio_device_descriptor(runtime, out, handle)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: AudioDeviceListRequestVm,
) -> RuntimeResult<VmSlice<AudioDeviceDescriptorVm>> {
    let values =
        call_out(|out| unsafe { host_audio::destack_audio_device_list(runtime, out, request) })?;

    descriptor_slice_to_vm(context, values)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    backend: AudioBackend,
    backend_policy: AudioBackendSelectionPolicy,
) -> RuntimeResult<()> {
    unsafe { host_audio::destack_audio_device_rescan(runtime, backend, backend_policy) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
    options: AudioDeviceOpenOptionsVm,
) -> RuntimeResult<resource::AudioDeviceHandle> {
    let id = string_from_vm(runtime, context, id)?;
    let options = device_open_options_from_vm(options)?;
    call_out(|out| unsafe { host_audio::destack_audio_device_open(runtime, out, id, options) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<()> {
    unsafe { host_audio::destack_audio_event_close(runtime, handle) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: AudioEventSubscriptionOptionsVm,
) -> RuntimeResult<resource::AudioEventHandle> {
    call_out(|out| unsafe { host_audio::destack_audio_event_open(runtime, out, options) })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioEventHandle,
    timeoutns: u64,
) -> RuntimeResult<AudioEventVm> {
    let value = call_out(|out| unsafe {
        host_audio::destack_audio_event_read(runtime, out, handle, timeoutns)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<AudioEventVm>> {
    let values = call_out(|out| unsafe {
        host_audio::destack_audio_event_read_batch(runtime, out, handle, maxevents, timeoutns)
    })?;

    event_slice_to_vm(context, values)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioEventHandle,
) -> RuntimeResult<AudioEventVm> {
    let value =
        call_out(|out| unsafe { host_audio::destack_audio_event_try_read(runtime, out, handle) })?;

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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioEventHandle,
    maxevents: u32,
) -> RuntimeResult<VmSlice<AudioEventVm>> {
    let values = call_out(|out| unsafe {
        host_audio::destack_audio_event_try_read_batch(runtime, out, handle, maxevents)
    })?;

    event_slice_to_vm(context, values)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamAvailabilityVm> {
    call_out(|out| unsafe { host_audio::destack_audio_stream_availability(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    unsafe { host_audio::destack_audio_stream_close(runtime, handle) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { host_audio::destack_audio_stream_drain(runtime, handle, timeoutns) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    unsafe { host_audio::destack_audio_stream_flush(runtime, handle) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamDescriptorVm> {
    let value = call_out(|out| unsafe {
        host_audio::destack_audio_stream_descriptor(runtime, out, handle)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfigVm,
    options: AudioStreamOpenOptionsVm,
) -> RuntimeResult<resource::AudioStreamHandle> {
    call_out(|out| unsafe {
        host_audio::destack_audio_stream_open(runtime, out, device, config, options)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    device: resource::AudioDeviceHandle,
    config: AudioStreamConfigVm,
    options: AudioStreamOpenOptionsVm,
) -> RuntimeResult<AudioStreamSupportVm> {
    let value = call_out(|out| unsafe {
        host_audio::destack_audio_stream_support(runtime, out, device, config, options)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let value = call_out(|out| unsafe {
        host_audio::destack_audio_stream_read(runtime, out, handle, maxbytes)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let (vm_buffers, native_buffers) = writable_byte_vectors_from_vm(runtime, context, buffers)?;
    let written = call_out(|out| unsafe {
        host_audio::destack_audio_stream_readv(runtime, out, handle, native_buffers)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    muted: bool,
) -> RuntimeResult<()> {
    unsafe { host_audio::destack_audio_stream_set_mute(runtime, handle, muted) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let name = string_from_vm(runtime, context, name)?;
    unsafe { host_audio::destack_audio_stream_set_name(runtime, handle, name) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    lineargain: f64,
) -> RuntimeResult<()> {
    unsafe { host_audio::destack_audio_stream_set_volume(runtime, handle, lineargain) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    unsafe { host_audio::destack_audio_stream_start(runtime, handle) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    pause: bool,
) -> RuntimeResult<()> {
    unsafe { host_audio::destack_audio_stream_pause(runtime, handle, pause) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    unsafe { host_audio::destack_audio_stream_abort(runtime, handle) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamStateVm> {
    call_out(|out| unsafe { host_audio::destack_audio_stream_state(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<()> {
    unsafe { host_audio::destack_audio_stream_stop(runtime, handle) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
) -> RuntimeResult<AudioStreamTimingVm> {
    call_out(|out| unsafe { host_audio::destack_audio_stream_timing(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let value = call_out(|out| unsafe {
        host_audio::destack_audio_stream_try_read(runtime, out, handle, maxbytes)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let (vm_buffers, native_buffers) = writable_byte_vectors_from_vm(runtime, context, buffers)?;
    let written = call_out(|out| unsafe {
        host_audio::destack_audio_stream_try_readv(runtime, out, handle, native_buffers)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    data: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let data = bytes_from_vm(runtime, context, data)?;
    call_out(|out| unsafe {
        host_audio::destack_audio_stream_try_write(runtime, out, handle, data)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let buffers = byte_vectors_from_vm(runtime, context, buffers)?;
    call_out(|out| unsafe {
        host_audio::destack_audio_stream_try_writev(runtime, out, handle, buffers)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    data: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let data = bytes_from_vm(runtime, context, data)?;
    call_out(|out| unsafe { host_audio::destack_audio_stream_write(runtime, out, handle, data) })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let buffers = byte_vectors_from_vm(runtime, context, buffers)?;
    call_out(|out| unsafe {
        host_audio::destack_audio_stream_writev(runtime, out, handle, buffers)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    data: VmSlice<u8>,
    presentationtimens: u64,
) -> RuntimeResult<u64> {
    let data = bytes_from_vm(runtime, context, data)?;
    call_out(|out| unsafe {
        host_audio::destack_audio_stream_write_at(runtime, out, handle, data, presentationtimens)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::AudioStreamHandle,
    buffers: VmSlice<VmSlice<u8>>,
    presentationtimens: u64,
) -> RuntimeResult<u64> {
    let buffers = byte_vectors_from_vm(runtime, context, buffers)?;
    call_out(|out| unsafe {
        host_audio::destack_audio_stream_write_atv(
            runtime,
            out,
            handle,
            buffers,
            presentationtimens,
        )
    })
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
pub(crate) fn destack_audio_midi_flush(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MidiPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.midi.flush")).boxed())
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
pub(crate) fn destack_audio_midi_port_close(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MidiPortHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.midi.portClose")).boxed())
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
pub(crate) fn destack_audio_midi_port_list(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    direction: MidiPortDirection,
) -> RuntimeResult<VmSlice<MidiPortDescriptorVm>> {
    let _ = direction;

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.midi.portList")).boxed())
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
pub(crate) fn destack_audio_midi_port_open(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
    direction: MidiPortDirection,
) -> RuntimeResult<resource::MidiPortHandle> {
    let _ = (id, direction);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.midi.portOpen")).boxed())
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
pub(crate) fn destack_audio_midi_read(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MidiPortHandle,
    maxmessages: u32,
    timeoutns: u64,
) -> RuntimeResult<VmArray<MidiMessageVm>> {
    let _ = (handle, maxmessages, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.midi.read")).boxed())
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
pub(crate) fn destack_audio_midi_try_read(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MidiPortHandle,
    maxmessages: u32,
) -> RuntimeResult<VmArray<MidiMessageVm>> {
    let _ = (handle, maxmessages);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.midi.tryRead")).boxed())
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
pub(crate) fn destack_audio_midi_write(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MidiPortHandle,
    messages: VmArray<MidiMessageVm>,
) -> RuntimeResult<u32> {
    let _ = (handle, messages);

    Err(RuntimeError::from(PlatformError::not_supported("destack.audio.midi.write")).boxed())
}
