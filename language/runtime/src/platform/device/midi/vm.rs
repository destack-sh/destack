use destack_vm as vm;

use super::{core as midi_core, host};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::device::{
    MidiBackendDescriptorVm, MidiEventSubscriptionOptionsVm, MidiEventVm,
    MidiInputPortOpenOptionsVm, MidiInputRecordVm, MidiOutputPortOpenOptionsVm, MidiOutputRecordVm,
    MidiPortDescriptorVm, MidiPortListOptionsVm, MidiVirtualInputCreateOptions,
    MidiVirtualInputCreateOptionsVm, MidiVirtualOutputCreateOptions,
    MidiVirtualOutputCreateOptionsVm,
};
use crate::platform::{VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;

/// Decode one VM string handle into one borrowed runtime string.
fn vm_string(
    context: &mut vm::BindingContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<String> {
    let value = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    Ok(value.as_str().to_string())
}

/// Decode one optional VM string handle.
fn vm_optional_string(
    context: &mut vm::BindingContext<'_>,
    value: Option<vm::StringHandle>,
) -> RuntimeResult<Option<String>> {
    match value {
        Some(value) => Ok(Some(vm_string(context, value)?)),
        None => Ok(None),
    }
}

/// Convert one VM virtual-input options payload into one native payload.
fn virtual_input_options_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    value: MidiVirtualInputCreateOptionsVm,
) -> RuntimeResult<MidiVirtualInputCreateOptions> {
    let name = vm_string(context, value.name)?;
    let manufacturer = vm_optional_string(context, value.manufacturer)?;
    let model = vm_optional_string(context, value.model)?;
    let version = vm_optional_string(context, value.version)?;

    Ok(MidiVirtualInputCreateOptions {
        backend: value.backend,
        backend_policy: value.backend_policy,
        name: binding.store_string(&name),
        manufacturer: manufacturer
            .as_ref()
            .map(|value| binding.store_string(value)),
        model: model.as_ref().map(|value| binding.store_string(value)),
        version: version.as_ref().map(|value| binding.store_string(value)),
        data_format: value.data_format,
        protocol: value.protocol,
        queue_capacity: value.queue_capacity,
    })
}

/// Convert one VM virtual-output options payload into one native payload.
fn virtual_output_options_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    value: MidiVirtualOutputCreateOptionsVm,
) -> RuntimeResult<MidiVirtualOutputCreateOptions> {
    let name = vm_string(context, value.name)?;
    let manufacturer = vm_optional_string(context, value.manufacturer)?;
    let model = vm_optional_string(context, value.model)?;
    let version = vm_optional_string(context, value.version)?;

    Ok(MidiVirtualOutputCreateOptions {
        backend: value.backend,
        backend_policy: value.backend_policy,
        name: binding.store_string(&name),
        manufacturer: manufacturer
            .as_ref()
            .map(|value| binding.store_string(value)),
        model: model.as_ref().map(|value| binding.store_string(value)),
        version: version.as_ref().map(|value| binding.store_string(value)),
        data_format: value.data_format,
        protocol: value.protocol,
    })
}

/// Decode one VM output-record array into owned records.
fn output_records_from_vm(
    context: &mut vm::BindingContext<'_>,
    records: VmArray<MidiOutputRecordVm>,
) -> RuntimeResult<Vec<midi_core::MidiOutputRecordValue>> {
    let records = records.read_values(&context.read())?;
    let mut decoded = Vec::with_capacity(records.len());

    // decode each record into owned bytes
    for record in records {
        decoded.push(midi_core::MidiOutputRecordValue::from_vm(context, record)?);
    }

    Ok(decoded)
}

/// List host MIDI backends.
pub(crate) fn destack_device_midi_backend_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<MidiBackendDescriptorVm>> {
    let descriptors = host::midi_backend_list(binding)?;

    midi_core::store_backend_descriptors_vm(context, descriptors)
}

/// Open one MIDI topology event subscription.
pub(crate) fn destack_device_midi_event_open(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    options: MidiEventSubscriptionOptionsVm,
) -> RuntimeResult<resource::MidiEventHandle> {
    host::midi_event_open(binding, options)
}

/// Close one MIDI topology event subscription.
pub(crate) fn destack_device_midi_event_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    host::midi_event_close(binding, handle)
}

/// Wait for one MIDI topology event.
pub(crate) fn destack_device_midi_event_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::MidiEventHandle,
    timeoutns: u64,
) -> RuntimeResult<MidiEventVm> {
    let event = host::midi_event_read(binding, handle, timeoutns)?;

    midi_core::store_event_vm(context, event)
}

/// Wait for one batch of MIDI topology events.
pub(crate) fn destack_device_midi_event_read_batch(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::MidiEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<MidiEventVm>> {
    let events = host::midi_event_read_batch(binding, handle, maxevents, timeoutns)?;

    midi_core::store_events_vm(context, events)
}

/// Poll one MIDI topology event without blocking.
pub(crate) fn destack_device_midi_event_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<MidiEventVm> {
    let event = host::midi_event_try_read(binding, handle)?;

    midi_core::store_event_vm(context, event)
}

/// Poll one batch of MIDI topology events without blocking.
pub(crate) fn destack_device_midi_event_try_read_batch(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::MidiEventHandle,
    maxevents: u32,
) -> RuntimeResult<VmSlice<MidiEventVm>> {
    let events = host::midi_event_try_read_batch(binding, handle, maxevents)?;

    midi_core::store_events_vm(context, events)
}

/// List available MIDI input endpoints.
pub(crate) fn destack_device_midi_input_port_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    options: MidiPortListOptionsVm,
) -> RuntimeResult<VmSlice<MidiPortDescriptorVm>> {
    let descriptors = host::midi_input_port_list(binding, options)?;

    midi_core::store_port_descriptors_vm(context, descriptors)
}

/// Open one MIDI input endpoint.
pub(crate) fn destack_device_midi_input_port_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
    options: MidiInputPortOpenOptionsVm,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    let id = vm_string(context, id)?;

    host::midi_input_port_open(binding, &id, options)
}

/// Describe one opened MIDI input endpoint.
pub(crate) fn destack_device_midi_input_port_descriptor(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiPortDescriptorVm> {
    let descriptor = host::midi_input_port_descriptor(binding, handle)?;

    descriptor.into_vm(context)
}

/// Close one opened MIDI input endpoint.
pub(crate) fn destack_device_midi_input_port_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    host::midi_input_port_close(binding, handle)
}

/// Wait for one MIDI input record.
pub(crate) fn destack_device_midi_input_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::MidiInputPortHandle,
    timeoutns: u64,
) -> RuntimeResult<MidiInputRecordVm> {
    let record = host::midi_input_read(binding, handle, timeoutns)?;

    midi_core::store_input_record_vm(context, record)
}

/// Wait for one batch of MIDI input records.
pub(crate) fn destack_device_midi_input_read_batch(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::MidiInputPortHandle,
    maxrecords: u32,
    timeoutns: u64,
) -> RuntimeResult<VmArray<MidiInputRecordVm>> {
    let records = host::midi_input_read_batch(binding, handle, maxrecords, timeoutns)?;

    midi_core::store_input_records_vm(context, records)
}

/// Poll one MIDI input record without blocking.
pub(crate) fn destack_device_midi_input_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiInputRecordVm> {
    let record = host::midi_input_try_read(binding, handle)?;

    midi_core::store_input_record_vm(context, record)
}

/// Poll one batch of MIDI input records without blocking.
pub(crate) fn destack_device_midi_input_try_read_batch(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::MidiInputPortHandle,
    maxrecords: u32,
) -> RuntimeResult<VmArray<MidiInputRecordVm>> {
    let records = host::midi_input_try_read_batch(binding, handle, maxrecords)?;

    midi_core::store_input_records_vm(context, records)
}

/// Create one virtual MIDI input endpoint.
pub(crate) fn destack_device_midi_input_virtual_create(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    options: MidiVirtualInputCreateOptionsVm,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    let options = virtual_input_options_from_vm(binding, context, options)?;

    host::midi_input_virtual_create(binding, options)
}

/// List available MIDI output endpoints.
pub(crate) fn destack_device_midi_output_port_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    options: MidiPortListOptionsVm,
) -> RuntimeResult<VmSlice<MidiPortDescriptorVm>> {
    let descriptors = host::midi_output_port_list(binding, options)?;

    midi_core::store_port_descriptors_vm(context, descriptors)
}

/// Open one MIDI output endpoint.
pub(crate) fn destack_device_midi_output_port_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
    options: MidiOutputPortOpenOptionsVm,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    let id = vm_string(context, id)?;

    host::midi_output_port_open(binding, &id, options)
}

/// Describe one opened MIDI output endpoint.
pub(crate) fn destack_device_midi_output_port_descriptor(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<MidiPortDescriptorVm> {
    let descriptor = host::midi_output_port_descriptor(binding, handle)?;

    descriptor.into_vm(context)
}

/// Close one opened MIDI output endpoint.
pub(crate) fn destack_device_midi_output_port_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    host::midi_output_port_close(binding, handle)
}

/// Write one batch of outbound MIDI records.
pub(crate) fn destack_device_midi_output_write(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::MidiOutputPortHandle,
    records: VmArray<MidiOutputRecordVm>,
) -> RuntimeResult<u32> {
    let records = output_records_from_vm(context, records)?;

    host::midi_output_write(binding, handle, records)
}

/// Create one virtual MIDI output endpoint.
pub(crate) fn destack_device_midi_output_virtual_create(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    options: MidiVirtualOutputCreateOptionsVm,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    let options = virtual_output_options_from_vm(binding, context, options)?;

    host::midi_output_virtual_create(binding, options)
}
