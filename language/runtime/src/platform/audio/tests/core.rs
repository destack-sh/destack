use destack_vm as vm;

use super::super::{
    AudioBackend, AudioBackendCapabilityFlags, AudioBackendDescriptor, AudioBackendDescriptorVm,
    AudioBackendSelectionPolicy, AudioChannelLayout, AudioClockSnapshot, AudioClockSnapshotVm,
    AudioDeviceCapabilityFlags, AudioDeviceDescriptor, AudioDeviceDescriptorVm,
    AudioDeviceDirection, AudioDeviceListFlags, AudioDeviceListRequest, AudioDeviceListRequestVm,
    AudioDeviceOpenFlags, AudioDeviceOpenOptions, AudioDeviceOpenOptionsVm, AudioEvent,
    AudioEventKind, AudioEventSource, AudioEventSubscriptionOptions,
    AudioEventSubscriptionOptionsVm, AudioEventVm, AudioSampleFormat, AudioShareMode,
    AudioStreamConfig, AudioStreamConfigVm, AudioStreamDescriptor, AudioStreamDescriptorVm,
    AudioStreamFlags, AudioStreamOpenOptions, AudioStreamOpenOptionsVm,
    AudioStreamRequirementFlags, AudioStreamState, AudioStreamStateVm, AudioStreamSupport,
    AudioStreamSupportVm, AudioStreamTransferMode, AudioSupportedEventSubscriptionFlags,
    AudioSupportedStreamClockDomains, AudioSupportedStreamFlags,
    AudioSupportedStreamRequirementFlags,
};
use super::{AudioHarnessContext, HarnessValue};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::core::BackendSupport;
use crate::platform::{VmSlice, resource};

type NativeByteVectors = NativeSlice<NativeSlice<u8>>;
type VmByteVectors = VmSlice<VmSlice<u8>>;
type ByteVectorsHarnessValue = HarnessValue<NativeByteVectors, VmByteVectors>;

/// Return whether one backend support state allows live host execution.
fn backend_support_allows_host_execution(support: BackendSupport) -> bool {
    matches!(support, BackendSupport::Available)
}

/// One decoded backend descriptor summary used by backend-list tests.
#[derive(Clone, Copy)]
pub(super) struct AudioBackendDescriptorSummary {
    /// Backend selector.
    pub(super) backend: AudioBackend,
    /// Host support state for the selector.
    pub(super) support: BackendSupport,
    /// Auto-selection priority for the selector.
    pub(super) priority: u16,
    /// Backend capability flags for the selector.
    pub(super) capability_flags: AudioBackendCapabilityFlags,
    /// Supported device-list flags for the selector.
    pub(super) supported_device_list_flags: AudioDeviceListFlags,
    /// Supported device-open flags for the selector.
    pub(super) supported_device_open_flags: AudioDeviceOpenFlags,
    /// Supported stream flags for the selector.
    pub(super) supported_stream_flags: AudioSupportedStreamFlags,
    /// Supported stream requirement flags for the selector.
    pub(super) supported_stream_requirement_flags: AudioSupportedStreamRequirementFlags,
    /// Supported event-subscription flags for the selector.
    pub(super) supported_event_subscription_flags: AudioSupportedEventSubscriptionFlags,
    /// Supported stream-clock domains for the selector.
    pub(super) supported_stream_clock_domains: AudioSupportedStreamClockDomains,
}

/// Deterministic pseudo-random sequence used by stress tests.
pub(super) struct DeterministicSequence {
    /// Internal state for the xorshift64 generator.
    state: u64,
}

impl DeterministicSequence {
    /// Create one deterministic sequence from one nonzero seed.
    pub(super) fn new(seed: u64) -> Self {
        let state = if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        };
        Self { state }
    }

    /// Return one random u64 and advance sequence state.
    pub(super) fn next_u64(&mut self) -> u64 {
        let mut state = self.state;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        self.state = state;
        state
    }

    /// Return one random usize in range `[0, upper_exclusive)`.
    pub(super) fn next_index(&mut self, upper_exclusive: usize) -> usize {
        (self.next_u64() as usize) % upper_exclusive
    }

    /// Return one random bool.
    pub(super) fn next_bool(&mut self) -> bool {
        (self.next_u64() & 1) != 0
    }
}

/// Return the mutable VM context when the harness is running in VM mode.
pub(super) fn vm_context_mut<'context, 'call>(
    context: &'context mut AudioHarnessContext<'call>,
) -> Option<&'context mut vm::BindingContext<'call>> {
    context
        .vm_context
        .map(|context| unsafe { &mut *(context as *mut vm::BindingContext<'_>) })
}

/// Build one harness string value for the current engine mode.
pub(super) fn harness_string(
    context: &mut AudioHarnessContext<'_>,
    value: &str,
) -> HarnessValue<NativeStringRef, vm::StringHandle> {
    if let Some(vm_context) = vm_context_mut(context) {
        let value = vm_context
            .intern_string(value)
            .expect("vm test string should intern");
        let value = vm::StringHandle::new(value);

        context.harness_value_vm(value)
    } else {
        context.harness_value(context.call_context.store_string(value))
    }
}

/// Build one harness device-open options value for the current engine mode.
pub(super) fn harness_device_options(
    context: &mut AudioHarnessContext<'_>,
    options: AudioDeviceOpenOptions,
) -> HarnessValue<AudioDeviceOpenOptions, AudioDeviceOpenOptionsVm> {
    if vm_context_mut(context).is_some() {
        let vm_options = AudioDeviceOpenOptionsVm {
            direction: options.direction,
            backend: options.backend,
            backend_policy: options.backend_policy,
            share_mode: options.share_mode,
            flags: options.flags,
        };

        context.harness_value_vm(vm_options)
    } else {
        context.harness_value(options)
    }
}

/// Build one harness stream-config value for the current engine mode.
pub(super) fn harness_stream_config(
    context: &mut AudioHarnessContext<'_>,
    config: AudioStreamConfig,
) -> HarnessValue<AudioStreamConfig, AudioStreamConfigVm> {
    if context.vm_context.is_some() {
        context.harness_value_vm(config)
    } else {
        context.harness_value(config)
    }
}

/// Build one harness stream-open options value for the current engine mode.
pub(super) fn harness_stream_options(
    context: &mut AudioHarnessContext<'_>,
    options: AudioStreamOpenOptions,
) -> HarnessValue<AudioStreamOpenOptions, AudioStreamOpenOptionsVm> {
    if context.vm_context.is_some() {
        context.harness_value_vm(options)
    } else {
        context.harness_value(options)
    }
}

/// Return one default stream-open options payload.
pub(super) fn default_stream_open_options() -> AudioStreamOpenOptions {
    AudioStreamOpenOptions {
        flags: AudioStreamFlags(0),
        requirements: AudioStreamRequirementFlags(0),
    }
}

/// Return one stream-open options payload with overridden stream flags.
pub(super) fn default_stream_open_options_with_flags(
    flags: AudioStreamFlags,
) -> AudioStreamOpenOptions {
    AudioStreamOpenOptions {
        flags,
        requirements: AudioStreamRequirementFlags(0),
    }
}

/// Open one stream with default stream-open options.
pub(super) fn stream_open_with_default_options(
    context: &mut AudioHarnessContext<'_>,
    device: resource::AudioDeviceHandle,
    config: HarnessValue<AudioStreamConfig, AudioStreamConfigVm>,
) -> RuntimeResult<resource::AudioStreamHandle> {
    let options = harness_stream_options(context, default_stream_open_options());
    context.destack_audio_stream_open(device, config, options)
}

/// Build one harness device-list request for the current engine mode.
pub(super) fn harness_list_request(
    context: &mut AudioHarnessContext<'_>,
    request: AudioDeviceListRequest,
) -> HarnessValue<AudioDeviceListRequest, AudioDeviceListRequestVm> {
    if context.vm_context.is_some() {
        context.harness_value_vm(request)
    } else {
        context.harness_value(request)
    }
}

/// Build one harness event-subscription options value for the current engine mode.
pub(super) fn harness_event_options(
    context: &mut AudioHarnessContext<'_>,
    options: AudioEventSubscriptionOptions,
) -> HarnessValue<AudioEventSubscriptionOptions, AudioEventSubscriptionOptionsVm> {
    if context.vm_context.is_some() {
        context.harness_value_vm(options)
    } else {
        context.harness_value(options)
    }
}

/// Build one harness byte-slice value for the current engine mode.
pub(super) fn harness_bytes(
    context: &mut AudioHarnessContext<'_>,
    data: &[u8],
) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
    if let Some(vm_context) = vm_context_mut(context) {
        let value = VmSlice::from_bytes(&mut vm_context.write(), data)?;

        Ok(context.harness_value_vm(value))
    } else {
        Ok(context.harness_value(context.call_context.store_slice(data.to_vec())))
    }
}

/// Build one VM nested slice from vm byte slices.
fn vm_slice_of_slices(
    context: &mut vm::BindingContext<'_>,
    slices: &[VmSlice<u8>],
) -> RuntimeResult<VmSlice<VmSlice<u8>>> {
    let values = slices
        .iter()
        .map(|slice| slice.to_value(&mut context.write()))
        .collect::<RuntimeResult<Vec<_>>>()?;
    let data = context
        .allocate_heap_words(values.len())
        .map_err(RuntimeError::from)?;
    for (index, value) in values.into_iter().enumerate() {
        context
            .write_heap_word(data, index, value)
            .map_err(RuntimeError::from)?;
    }

    Ok(VmSlice {
        data: vm::Word::heap_reference(data),
        len: slices.len() as u32,
        _marker: std::marker::PhantomData::<VmSlice<u8>>,
    })
}

/// Build one harness nested byte-slice value for vectorized I/O.
pub(super) fn harness_bytes_slices(
    context: &mut AudioHarnessContext<'_>,
    buffers: &[&[u8]],
) -> RuntimeResult<ByteVectorsHarnessValue> {
    if let Some(vm_context) = vm_context_mut(context) {
        let vm_buffers = buffers
            .iter()
            .map(|buffer| VmSlice::from_bytes(&mut vm_context.write(), buffer))
            .collect::<RuntimeResult<Vec<_>>>()?;
        let values = vm_slice_of_slices(vm_context, &vm_buffers)?;

        Ok(context.harness_value_vm(values))
    } else {
        let native_buffers = buffers
            .iter()
            .map(|buffer| NativeSlice {
                data: buffer.as_ptr() as *mut u8,
                len: buffer.len() as u32,
            })
            .collect::<Vec<_>>();
        Ok(context.harness_value(context.call_context.store_slice(native_buffers)))
    }
}

/// Build one harness mutable nested byte-slice value for vectorized reads.
pub(super) fn harness_mutable_bytes_slices(
    context: &mut AudioHarnessContext<'_>,
    buffers: &mut [Vec<u8>],
) -> RuntimeResult<ByteVectorsHarnessValue> {
    if let Some(vm_context) = vm_context_mut(context) {
        let vm_buffers = buffers
            .iter()
            .map(|buffer| VmSlice::from_bytes(&mut vm_context.write(), &vec![0u8; buffer.len()]))
            .collect::<RuntimeResult<Vec<_>>>()?;
        let values = vm_slice_of_slices(vm_context, &vm_buffers)?;

        Ok(context.harness_value_vm(values))
    } else {
        let native_buffers = buffers
            .iter_mut()
            .map(|buffer| NativeSlice {
                data: buffer.as_mut_ptr(),
                len: buffer.len() as u32,
            })
            .collect::<Vec<_>>();
        Ok(context.harness_value(context.call_context.store_slice(native_buffers)))
    }
}

/// Return the descriptor count for one native or vm list payload.
pub(super) fn descriptor_count(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<NativeSlice<AudioDeviceDescriptor>, VmSlice<AudioDeviceDescriptorVm>>,
) -> RuntimeResult<usize> {
    match value {
        HarnessValue::Native(value) => Ok(unsafe { value.as_slice()? }.len()),
        HarnessValue::Vm(value) => {
            let vm_context = vm_context_mut(context)
                .expect("vm payload requires vm context to decode descriptor slice");
            Ok(value.read_values(&vm_context.read())?.len())
        }
    }
}

/// Decode one device descriptor list into direction and capability rows.
pub(super) fn device_direction_capability_rows(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<NativeSlice<AudioDeviceDescriptor>, VmSlice<AudioDeviceDescriptorVm>>,
) -> RuntimeResult<Vec<(AudioDeviceDirection, AudioDeviceCapabilityFlags)>> {
    match value {
        HarnessValue::Native(value) => {
            let values = unsafe { value.as_slice()? };
            Ok(values
                .iter()
                .map(|value| (value.direction, value.capability_flags))
                .collect::<Vec<_>>())
        }
        HarnessValue::Vm(value) => {
            let vm_context = vm_context_mut(context)
                .expect("vm payload requires vm context to decode device descriptor slice");
            let values = value.read_values(&vm_context.read())?;
            Ok(values
                .iter()
                .map(|value| (value.direction, value.capability_flags))
                .collect::<Vec<_>>())
        }
    }
}

/// Return the payload byte length for one native or vm byte-slice payload.
pub(super) fn byte_len(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<NativeSlice<u8>, VmSlice<u8>>,
) -> RuntimeResult<usize> {
    match value {
        HarnessValue::Native(value) => Ok(unsafe { value.as_slice()? }.len()),
        HarnessValue::Vm(value) => {
            let vm_context = vm_context_mut(context)
                .expect("vm payload requires vm context to decode byte slice");
            Ok(value.read_bytes(&vm_context.read())?.len())
        }
    }
}

/// Decode one backend descriptor list into backend availability rows.
pub(super) fn backend_descriptor_summaries(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<NativeSlice<AudioBackendDescriptor>, VmSlice<AudioBackendDescriptorVm>>,
) -> RuntimeResult<Vec<AudioBackendDescriptorSummary>> {
    match value {
        HarnessValue::Native(value) => {
            let values = unsafe { value.as_slice()? };
            Ok(values
                .iter()
                .map(|value| AudioBackendDescriptorSummary {
                    backend: value.backend,
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
                .collect())
        }
        HarnessValue::Vm(value) => {
            let vm_context = vm_context_mut(context)
                .expect("vm payload requires vm context to decode backend descriptor slice");
            let values = value.read_values(&vm_context.read())?;
            Ok(values
                .iter()
                .map(|value| AudioBackendDescriptorSummary {
                    backend: value.backend,
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
                .collect())
        }
    }
}

/// Decode one backend descriptor list into backend support rows.
pub(super) fn backend_support_rows(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<NativeSlice<AudioBackendDescriptor>, VmSlice<AudioBackendDescriptorVm>>,
) -> RuntimeResult<Vec<(AudioBackend, BackendSupport)>> {
    let rows = backend_descriptor_summaries(context, value)?;
    Ok(rows
        .into_iter()
        .map(|value| (value.backend, value.support))
        .collect())
}

/// Decode one backend descriptor list into backend support and capability rows.
pub(super) fn backend_support_rows_with_capabilities(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<NativeSlice<AudioBackendDescriptor>, VmSlice<AudioBackendDescriptorVm>>,
) -> RuntimeResult<Vec<(AudioBackend, BackendSupport, AudioBackendCapabilityFlags)>> {
    let rows = backend_descriptor_summaries(context, value)?;
    Ok(rows
        .into_iter()
        .map(|value| (value.backend, value.support, value.capability_flags))
        .collect())
}

/// Decode one backend descriptor list into support and event-subscription rows.
pub(super) fn backend_event_support_rows(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<NativeSlice<AudioBackendDescriptor>, VmSlice<AudioBackendDescriptorVm>>,
) -> RuntimeResult<
    Vec<(
        AudioBackend,
        BackendSupport,
        AudioSupportedEventSubscriptionFlags,
    )>,
> {
    let rows = backend_descriptor_summaries(context, value)?;
    Ok(rows
        .into_iter()
        .map(|value| {
            (
                value.backend,
                value.support,
                value.supported_event_subscription_flags,
            )
        })
        .collect())
}

/// Return the reported support state for one backend row.
pub(super) fn backend_support_for(
    rows: &[(AudioBackend, BackendSupport)],
    backend: AudioBackend,
) -> BackendSupport {
    rows.iter()
        .find(|(row_backend, _support)| *row_backend == backend)
        .map(|(_row_backend, support)| *support)
        .unwrap_or_else(|| panic!("audio backend list should include {backend:?} row"))
}

/// Return whether one backend is available for live host execution.
pub(super) fn backend_is_available_for_host_execution(
    rows: &[(AudioBackend, BackendSupport)],
    backend: AudioBackend,
) -> bool {
    let support = backend_support_for(rows, backend);

    backend_support_allows_host_execution(support)
}

/// Return the first available concrete host backend.
pub(super) fn first_available_host_backend(
    rows: &[(AudioBackend, BackendSupport)],
) -> Option<AudioBackend> {
    rows.iter()
        .find(|(backend, support)| {
            *backend != AudioBackend::Auto
                && *backend != AudioBackend::Null
                && backend_support_allows_host_execution(*support)
        })
        .map(|(backend, _support)| *backend)
}

/// Return whether any concrete host backend is available.
pub(super) fn has_available_host_backend(rows: &[(AudioBackend, BackendSupport)]) -> bool {
    first_available_host_backend(rows).is_some()
}

/// Decode one native or VM string payload into one rust string.
pub(super) fn string_from_harness_value(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<NativeStringRef, vm::StringHandle>,
) -> RuntimeResult<String> {
    match value {
        HarnessValue::Native(value) => Ok(unsafe { value.as_str()? }.to_string()),
        HarnessValue::Vm(value) => {
            let vm_context =
                vm_context_mut(context).expect("vm payload requires vm context to decode string");
            let value = vm_context
                .string_ref(value)
                .map_err(|error| RuntimeError::from(error).boxed())?;
            Ok(value.as_str().to_string())
        }
    }
}

/// Decode one stream state payload into one native state value.
pub(super) fn stream_state_from_value(
    value: HarnessValue<AudioStreamState, AudioStreamStateVm>,
) -> AudioStreamState {
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => AudioStreamState {
            state: value.state,
            running: value.running,
            paused: value.paused,
            buffered_frames: value.buffered_frames,
            input_latency_ns: value.input_latency_ns,
            output_latency_ns: value.output_latency_ns,
            total_latency_ns: value.total_latency_ns,
            status_flags: value.status_flags,
            xrun_count: value.xrun_count,
            input_underflow_count: value.input_underflow_count,
            input_overflow_count: value.input_overflow_count,
            output_underflow_count: value.output_underflow_count,
            output_overflow_count: value.output_overflow_count,
            callback_cpu_load: value.callback_cpu_load,
        },
    }
}

/// Decode one device descriptor payload into one direction value.
pub(super) fn device_descriptor_direction_from_value(
    value: HarnessValue<AudioDeviceDescriptor, AudioDeviceDescriptorVm>,
) -> AudioDeviceDirection {
    match value {
        HarnessValue::Native(value) => value.direction,
        HarnessValue::Vm(value) => value.direction,
    }
}

/// Decode one device descriptor payload into stable id and group id strings.
pub(super) fn device_descriptor_identity_from_value(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<AudioDeviceDescriptor, AudioDeviceDescriptorVm>,
) -> RuntimeResult<(String, String)> {
    match value {
        HarnessValue::Native(value) => Ok((
            unsafe { value.id.as_str()? }.to_string(),
            unsafe { value.group_id.as_str()? }.to_string(),
        )),
        HarnessValue::Vm(value) => {
            let vm_context = vm_context_mut(context)
                .expect("vm payload requires vm context to decode device descriptor");
            let id = vm_context
                .string_ref(value.id)
                .map_err(|error| RuntimeError::from(error).boxed())?;
            let group_id = vm_context
                .string_ref(value.group_id)
                .map_err(|error| RuntimeError::from(error).boxed())?;

            Ok((id.as_str().to_string(), group_id.as_str().to_string()))
        }
    }
}

/// Decode one device descriptor payload into one stream-clock support mask.
pub(super) fn device_descriptor_stream_clock_domains_from_value(
    value: HarnessValue<AudioDeviceDescriptor, AudioDeviceDescriptorVm>,
) -> AudioSupportedStreamClockDomains {
    match value {
        HarnessValue::Native(value) => value.supported_stream_clock_domains,
        HarnessValue::Vm(value) => value.supported_stream_clock_domains,
    }
}

/// Decode one stream info payload into support-flag booleans.
pub(super) fn stream_descriptor_flags_from_value(
    value: HarnessValue<AudioStreamDescriptor, AudioStreamDescriptorVm>,
) -> (bool, bool, bool, bool, bool) {
    match value {
        HarnessValue::Native(value) => (
            value.supports_write_at,
            value.supports_pause,
            value.supports_volume,
            value.supports_mute,
            value.supports_hardware_timestamps,
        ),
        HarnessValue::Vm(value) => (
            value.supports_write_at,
            value.supports_pause,
            value.supports_volume,
            value.supports_mute,
            value.supports_hardware_timestamps,
        ),
    }
}

/// Decode one stream descriptor payload into requested and effective option fields.
pub(super) fn stream_descriptor_option_flags_from_value(
    value: HarnessValue<AudioStreamDescriptor, AudioStreamDescriptorVm>,
) -> (
    AudioStreamFlags,
    AudioStreamRequirementFlags,
    AudioStreamFlags,
    AudioStreamRequirementFlags,
) {
    match value {
        HarnessValue::Native(value) => (
            value.requested_flags,
            value.requested_requirements,
            value.effective_flags,
            value.effective_requirements,
        ),
        HarnessValue::Vm(value) => (
            value.requested_flags,
            value.requested_requirements,
            value.effective_flags,
            value.effective_requirements,
        ),
    }
}

/// Decode one stream support payload into support and requirement flags.
pub(super) fn stream_support_from_value(
    value: HarnessValue<AudioStreamSupport, AudioStreamSupportVm>,
) -> (
    bool,
    AudioStreamRequirementFlags,
    AudioStreamRequirementFlags,
) {
    match value {
        HarnessValue::Native(value) => (
            value.supported,
            value.satisfied_requirements,
            value.unsatisfied_requirements,
        ),
        HarnessValue::Vm(value) => (
            value.supported,
            value.satisfied_requirements,
            value.unsatisfied_requirements,
        ),
    }
}

/// Decode one stream support payload into descriptor option fields.
pub(super) fn stream_support_descriptor_option_flags_from_value(
    value: HarnessValue<AudioStreamSupport, AudioStreamSupportVm>,
) -> (
    AudioStreamFlags,
    AudioStreamRequirementFlags,
    AudioStreamFlags,
    AudioStreamRequirementFlags,
) {
    match value {
        HarnessValue::Native(value) => (
            value.descriptor.requested_flags,
            value.descriptor.requested_requirements,
            value.descriptor.effective_flags,
            value.descriptor.effective_requirements,
        ),
        HarnessValue::Vm(value) => (
            value.descriptor.requested_flags,
            value.descriptor.requested_requirements,
            value.descriptor.effective_flags,
            value.descriptor.effective_requirements,
        ),
    }
}

/// Return one event-batch length from one native or vm payload.
pub(super) fn event_batch_len(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<NativeSlice<AudioEvent>, VmSlice<AudioEventVm>>,
) -> RuntimeResult<usize> {
    match value {
        HarnessValue::Native(value) => Ok(unsafe { value.as_slice()? }.len()),
        HarnessValue::Vm(value) => {
            let vm_context = vm_context_mut(context)
                .expect("vm payload requires vm context to decode event slice");
            Ok(value.read_values(&vm_context.read())?.len())
        }
    }
}

/// Decode one native event row into sequence, dropped count, kind, xrun delta, and source.
fn native_event_row(value: &AudioEvent) -> (u64, u64, AudioEventKind, u64, AudioEventSource) {
    match value {
        AudioEvent::AudioBackendDisconnectedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::BackendDisconnected,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioBackendResetEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::BackendReset,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioDefaultCaptureChangedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DefaultCaptureChanged,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioDefaultLoopbackChangedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DefaultLoopbackChanged,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioDefaultPlaybackChangedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DefaultPlaybackChanged,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioDeviceAddedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DeviceAdded,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioDeviceFormatChangedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DeviceFormatChanged,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioDeviceRemovedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DeviceRemoved,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioDeviceReroutedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DeviceRerouted,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioInterruptionBeganEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::InterruptionBegan,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioInterruptionEndedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::InterruptionEnded,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioStreamDeviceChangedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::StreamDeviceChanged,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioStreamStateChangedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::StreamStateChanged,
            0,
            value.metadata.source,
        ),
        AudioEvent::AudioStreamXRunEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::StreamXRun,
            value.xrun_count_delta,
            value.metadata.source,
        ),
    }
}

/// Decode one VM event row into sequence, dropped count, kind, xrun delta, and source.
fn vm_event_row(value: &AudioEventVm) -> (u64, u64, AudioEventKind, u64, AudioEventSource) {
    match value {
        AudioEventVm::AudioBackendDisconnectedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::BackendDisconnected,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioBackendResetEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::BackendReset,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioDefaultCaptureChangedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DefaultCaptureChanged,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioDefaultLoopbackChangedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DefaultLoopbackChanged,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioDefaultPlaybackChangedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DefaultPlaybackChanged,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioDeviceAddedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DeviceAdded,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioDeviceFormatChangedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DeviceFormatChanged,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioDeviceRemovedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DeviceRemoved,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioDeviceReroutedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::DeviceRerouted,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioInterruptionBeganEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::InterruptionBegan,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioInterruptionEndedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::InterruptionEnded,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioStreamDeviceChangedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::StreamDeviceChanged,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioStreamStateChangedEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::StreamStateChanged,
            0,
            value.metadata.source,
        ),
        AudioEventVm::AudioStreamXRunEvent(value) => (
            value.metadata.sequence,
            value.metadata.dropped_count,
            AudioEventKind::StreamXRun,
            value.xrun_count_delta,
            value.metadata.source,
        ),
    }
}

/// Decode one event-batch payload into sequence and dropped-count rows.
pub(super) fn event_batch_sequence_rows(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<NativeSlice<AudioEvent>, VmSlice<AudioEventVm>>,
) -> RuntimeResult<Vec<(u64, u64)>> {
    match value {
        HarnessValue::Native(value) => {
            let values = unsafe { value.as_slice()? };
            Ok(values
                .iter()
                .map(|value| {
                    let (sequence, dropped_count, _, _, _) = native_event_row(value);
                    (sequence, dropped_count)
                })
                .collect::<Vec<_>>())
        }
        HarnessValue::Vm(value) => {
            let vm_context = vm_context_mut(context)
                .expect("vm payload requires vm context to decode event slice");
            let values = value.read_values(&vm_context.read())?;
            Ok(values
                .iter()
                .map(|value| {
                    let (sequence, dropped_count, _, _, _) = vm_event_row(value);
                    (sequence, dropped_count)
                })
                .collect::<Vec<_>>())
        }
    }
}

/// Decode one audio clock snapshot payload into one native snapshot.
pub(super) fn clock_snapshot_from_value(
    value: HarnessValue<AudioClockSnapshot, AudioClockSnapshotVm>,
) -> AudioClockSnapshot {
    match value {
        HarnessValue::Native(value) => value,
        HarnessValue::Vm(value) => AudioClockSnapshot {
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
        },
    }
}

/// Open one null duplex stream and start it.
pub(super) fn open_null_duplex_stream(
    context: &mut AudioHarnessContext<'_>,
) -> RuntimeResult<(resource::AudioDeviceHandle, resource::AudioStreamHandle)> {
    let options = AudioDeviceOpenOptions {
        direction: AudioDeviceDirection::Duplex,
        backend: AudioBackend::Null,
        backend_policy: AudioBackendSelectionPolicy::Strict,
        share_mode: AudioShareMode::Shared,
        flags: AudioDeviceOpenFlags(0),
    };

    let device_id = harness_string(context, "audio:null:duplex");
    let device_options = harness_device_options(context, options);
    let device = context.destack_audio_device_open(device_id, device_options)?;

    let stream_config = AudioStreamConfig {
        sample_rate: 48_000,
        channels: 2,
        channel_layout: AudioChannelLayout::Stereo,
        channel_mask: 0b11,
        format: AudioSampleFormat::F32,
        period_frames: 128,
        transfer_mode: AudioStreamTransferMode::Push,
    };

    let stream_open_config = harness_stream_config(context, stream_config);
    let stream_open_options = harness_stream_options(context, default_stream_open_options());
    let stream =
        context.destack_audio_stream_open(device, stream_open_config, stream_open_options)?;
    context.destack_audio_stream_start(stream)?;

    Ok((device, stream))
}

/// Open one null playback stream and start it.
pub(super) fn open_null_playback_stream(
    context: &mut AudioHarnessContext<'_>,
) -> RuntimeResult<(resource::AudioDeviceHandle, resource::AudioStreamHandle)> {
    let options = AudioDeviceOpenOptions {
        direction: AudioDeviceDirection::Playback,
        backend: AudioBackend::Null,
        backend_policy: AudioBackendSelectionPolicy::Strict,
        share_mode: AudioShareMode::Shared,
        flags: AudioDeviceOpenFlags(0),
    };

    let device_id = harness_string(context, "audio:null:playback");
    let device_options = harness_device_options(context, options);
    let device = context.destack_audio_device_open(device_id, device_options)?;

    let stream_config = AudioStreamConfig {
        sample_rate: 48_000,
        channels: 2,
        channel_layout: AudioChannelLayout::Stereo,
        channel_mask: 0b11,
        format: AudioSampleFormat::F32,
        period_frames: 128,
        transfer_mode: AudioStreamTransferMode::Push,
    };

    let stream_open_config = harness_stream_config(context, stream_config);
    let stream_open_options = harness_stream_options(context, default_stream_open_options());
    let stream =
        context.destack_audio_stream_open(device, stream_open_config, stream_open_options)?;
    context.destack_audio_stream_start(stream)?;

    Ok((device, stream))
}
