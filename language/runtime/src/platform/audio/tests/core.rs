use destack_vm as vm;

use super::super::{
    AudioBackend, AudioBackendCapabilityFlags, AudioBackendDescriptor, AudioBackendDescriptorVm,
    AudioBackendOpenFlags, AudioBackendSelectionPolicy, AudioChannelLayout,
    AudioDeviceCapabilityFlags, AudioDeviceDescriptor, AudioDeviceDescriptorVm,
    AudioDeviceDirection, AudioDeviceListRequest, AudioDeviceListRequestVm, AudioDeviceOpenFlags,
    AudioDeviceOpenOptions, AudioDeviceOpenOptionsVm, AudioEventSubscriptionOptions,
    AudioEventSubscriptionOptionsVm, AudioSampleFormat, AudioShareMode, AudioStreamConfig,
    AudioStreamConfigVm, AudioStreamFlags, AudioStreamSnapshot, AudioStreamSnapshotVm,
    AudioStreamState, AudioStreamStateVm, AudioStreamTransferMode,
};
use super::{AudioHarnessContext, HarnessValue};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{NativeSlice, NativeStringRef, VmSlice, resource};

type NativeByteVectors = NativeSlice<NativeSlice<u8>>;
type VmByteVectors = VmSlice<VmSlice<u8>>;
type ByteVectorsHarnessValue = HarnessValue<NativeByteVectors, VmByteVectors>;

/// Return the mutable VM context when the harness is running in VM mode.
pub(super) fn vm_context_mut<'a>(
    context: &'a AudioHarnessContext<'a>,
) -> Option<&'a mut vm::ExternalCallContext<'a>> {
    context
        .vm_context
        .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
}

/// Build one harness string value for the current engine mode.
pub(super) fn harness_string(
    context: &mut AudioHarnessContext<'_>,
    value: &str,
) -> HarnessValue<NativeStringRef, vm::StringHandle> {
    if let Some(vm_context) = vm_context_mut(context) {
        let value = vm::StringHandle::new(vm_context.intern_string(value));
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
    if let Some(vm_context) = vm_context_mut(context) {
        let backend_hint = unsafe {
            options
                .backend_hint
                .as_str()
                .expect("backend hint should decode from native options")
        };

        let vm_options = AudioDeviceOpenOptionsVm {
            direction: options.direction,
            backend: options.backend,
            backend_policy: options.backend_policy,
            share_mode: options.share_mode,
            flags: options.flags,
            backend_flags: options.backend_flags,
            backend_hint: vm::StringHandle::new(vm_context.intern_string(backend_hint)),
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
        let value = VmSlice::from_bytes(vm_context, data);
        Ok(context.harness_value_vm(value))
    } else {
        Ok(context.harness_value(context.call_context.store_slice(data.to_vec())))
    }
}

/// Build one VM nested slice from vm byte slices.
fn vm_slice_of_slices(
    context: &mut vm::ExternalCallContext<'_>,
    slices: &[VmSlice<u8>],
) -> VmSlice<VmSlice<u8>> {
    let values = slices
        .iter()
        .map(|slice| slice.to_value(context))
        .collect::<Vec<_>>();
    let data = context.allocate_raw_values(values);
    VmSlice {
        data,
        len: slices.len() as u32,
        _marker: std::marker::PhantomData::<VmSlice<u8>>,
    }
}

/// Build one harness nested byte-slice value for vectorized I/O.
pub(super) fn harness_bytes_slices(
    context: &mut AudioHarnessContext<'_>,
    buffers: &[&[u8]],
) -> RuntimeResult<ByteVectorsHarnessValue> {
    if let Some(vm_context) = vm_context_mut(context) {
        let vm_buffers = buffers
            .iter()
            .map(|buffer| VmSlice::from_bytes(vm_context, buffer))
            .collect::<Vec<_>>();
        Ok(context.harness_value_vm(vm_slice_of_slices(vm_context, &vm_buffers)))
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
            .map(|buffer| VmSlice::from_bytes(vm_context, &vec![0u8; buffer.len()]))
            .collect::<Vec<_>>();
        Ok(context.harness_value_vm(vm_slice_of_slices(vm_context, &vm_buffers)))
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
            Ok(value.read_values(vm_context)?.len())
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
            let values = value.read_values(vm_context)?;
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
            Ok(value.read_bytes(vm_context)?.len())
        }
    }
}

/// Decode one backend descriptor list into backend availability rows.
pub(super) fn backend_availability_rows(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<NativeSlice<AudioBackendDescriptor>, VmSlice<AudioBackendDescriptorVm>>,
) -> RuntimeResult<Vec<(AudioBackend, bool)>> {
    match value {
        HarnessValue::Native(value) => {
            let values = unsafe { value.as_slice()? };
            Ok(values
                .iter()
                .map(|value| (value.backend, value.available))
                .collect::<Vec<_>>())
        }
        HarnessValue::Vm(value) => {
            let vm_context = vm_context_mut(context)
                .expect("vm payload requires vm context to decode backend descriptor slice");
            let values = value.read_values(vm_context)?;
            Ok(values
                .iter()
                .map(|value| (value.backend, value.available))
                .collect::<Vec<_>>())
        }
    }
}

/// Decode one backend descriptor list into backend availability and capability rows.
pub(super) fn backend_availability_rows_with_capabilities(
    context: &mut AudioHarnessContext<'_>,
    value: HarnessValue<NativeSlice<AudioBackendDescriptor>, VmSlice<AudioBackendDescriptorVm>>,
) -> RuntimeResult<Vec<(AudioBackend, bool, AudioBackendCapabilityFlags)>> {
    match value {
        HarnessValue::Native(value) => {
            let values = unsafe { value.as_slice()? };
            Ok(values
                .iter()
                .map(|value| (value.backend, value.available, value.capability_flags))
                .collect::<Vec<_>>())
        }
        HarnessValue::Vm(value) => {
            let vm_context = vm_context_mut(context)
                .expect("vm payload requires vm context to decode backend descriptor slice");
            let values = value.read_values(vm_context)?;
            Ok(values
                .iter()
                .map(|value| (value.backend, value.available, value.capability_flags))
                .collect::<Vec<_>>())
        }
    }
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

/// Decode one stream snapshot payload into support-flag booleans.
pub(super) fn stream_snapshot_support_from_value(
    value: HarnessValue<AudioStreamSnapshot, AudioStreamSnapshotVm>,
) -> (bool, bool, bool, bool) {
    match value {
        HarnessValue::Native(value) => (
            value.supports_write_at,
            value.supports_pause,
            value.supports_volume,
            value.supports_mute,
        ),
        HarnessValue::Vm(value) => (
            value.supports_write_at,
            value.supports_pause,
            value.supports_volume,
            value.supports_mute,
        ),
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
        backend_flags: AudioBackendOpenFlags(0),
        backend_hint: context.call_context.store_string(""),
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
        flags: AudioStreamFlags(0),
    };

    let stream_open_config = harness_stream_config(context, stream_config);
    let stream = context.destack_audio_stream_open(device, stream_open_config)?;
    context.destack_audio_stream_start(stream)?;

    Ok((device, stream))
}
