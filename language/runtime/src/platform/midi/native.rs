use super::{core as midi_core, host};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::midi::{
    MidiBackendDescriptor, MidiEvent, MidiEventSubscriptionOptions, MidiInputPortOpenOptions,
    MidiInputRecord, MidiOutputPortOpenOptions, MidiOutputRecord, MidiPortDescriptor,
    MidiPortListOptions, MidiVirtualInputCreateOptions, MidiVirtualOutputCreateOptions,
};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Write one value through one native out pointer.
fn write_out<T>(out: *mut T, value: T) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Decode one native output-record array into owned records.
fn output_records_from_native(
    records: NativeArray<MidiOutputRecord>,
) -> RuntimeResult<Vec<midi_core::MidiOutputRecordValue>> {
    let records = unsafe { records.as_slice()? };
    let mut decoded = Vec::with_capacity(records.len());

    // decode each record into owned bytes
    for record in records {
        decoded.push(midi_core::MidiOutputRecordValue::from_native(*record)?);
    }

    Ok(decoded)
}

/// Decode one borrowed native string reference.
fn native_string(value: NativeStringRef) -> RuntimeResult<String> {
    let value = unsafe { value.as_str()? };

    Ok(value.to_string())
}

/// List host MIDI backends.
pub(crate) unsafe fn destack_midi_backend_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<MidiBackendDescriptor>,
) -> RuntimeResult<()> {
    let descriptors = host::midi_backend_list(binding)?;
    let descriptors = midi_core::store_backend_descriptors_native(binding, descriptors);

    write_out(out, descriptors)
}

/// Open one MIDI topology event subscription.
pub(crate) unsafe fn destack_midi_event_open(
    binding: &BindingCallContext,
    out: *mut resource::MidiEventHandle,
    options: MidiEventSubscriptionOptions,
) -> RuntimeResult<()> {
    let handle = host::midi_event_open(binding, options)?;

    write_out(out, handle)
}

/// Close one MIDI topology event subscription.
pub(crate) unsafe fn destack_midi_event_close(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    host::midi_event_close(binding, handle)
}

/// Wait for one MIDI topology event.
pub(crate) unsafe fn destack_midi_event_read(
    binding: &BindingCallContext,
    out: *mut MidiEvent,
    handle: resource::MidiEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let event = host::midi_event_read(binding, handle, timeoutns)?;
    let event = midi_core::store_event_native(binding, event);

    write_out(out, event)
}

/// Wait for one batch of MIDI topology events.
pub(crate) unsafe fn destack_midi_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeSlice<MidiEvent>,
    handle: resource::MidiEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let events = host::midi_event_read_batch(binding, handle, maxevents, timeoutns)?;
    let events = midi_core::store_events_native(binding, events);

    write_out(out, events)
}

/// Poll one MIDI topology event without blocking.
pub(crate) unsafe fn destack_midi_event_try_read(
    binding: &BindingCallContext,
    out: *mut MidiEvent,
    handle: resource::MidiEventHandle,
) -> RuntimeResult<()> {
    let event = host::midi_event_try_read(binding, handle)?;
    let event = midi_core::store_event_native(binding, event);

    write_out(out, event)
}

/// Poll one batch of MIDI topology events without blocking.
pub(crate) unsafe fn destack_midi_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeSlice<MidiEvent>,
    handle: resource::MidiEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    let events = host::midi_event_try_read_batch(binding, handle, maxevents)?;
    let events = midi_core::store_events_native(binding, events);

    write_out(out, events)
}

/// List available MIDI input endpoints.
pub(crate) unsafe fn destack_midi_input_port_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<MidiPortDescriptor>,
    options: MidiPortListOptions,
) -> RuntimeResult<()> {
    let descriptors = host::midi_input_port_list(binding, options)?;
    let descriptors = midi_core::store_port_descriptors_native(binding, descriptors);

    write_out(out, descriptors)
}

/// Open one MIDI input endpoint.
pub(crate) unsafe fn destack_midi_input_port_open(
    binding: &BindingCallContext,
    out: *mut resource::MidiInputPortHandle,
    id: NativeStringRef,
    options: MidiInputPortOpenOptions,
) -> RuntimeResult<()> {
    let id = native_string(id)?;
    let handle = host::midi_input_port_open(binding, &id, options)?;

    write_out(out, handle)
}

/// Describe one opened MIDI input endpoint.
pub(crate) unsafe fn destack_midi_input_port_descriptor(
    binding: &BindingCallContext,
    out: *mut MidiPortDescriptor,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    let descriptor = host::midi_input_port_descriptor(binding, handle)?;
    let descriptor = descriptor.into_native(binding);

    write_out(out, descriptor)
}

/// Close one opened MIDI input endpoint.
pub(crate) unsafe fn destack_midi_input_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    host::midi_input_port_close(binding, handle)
}

/// Wait for one MIDI input record.
pub(crate) unsafe fn destack_midi_input_read(
    binding: &BindingCallContext,
    out: *mut MidiInputRecord,
    handle: resource::MidiInputPortHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let record = host::midi_input_read(binding, handle, timeoutns)?;
    let record = midi_core::store_input_record_native(binding, record);

    write_out(out, record)
}

/// Wait for one batch of MIDI input records.
pub(crate) unsafe fn destack_midi_input_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<MidiInputRecord>,
    handle: resource::MidiInputPortHandle,
    maxrecords: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let records = host::midi_input_read_batch(binding, handle, maxrecords, timeoutns)?;
    let records = midi_core::store_input_records_native(binding, records);

    write_out(out, records)
}

/// Poll one MIDI input record without blocking.
pub(crate) unsafe fn destack_midi_input_try_read(
    binding: &BindingCallContext,
    out: *mut MidiInputRecord,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    let record = host::midi_input_try_read(binding, handle)?;
    let record = midi_core::store_input_record_native(binding, record);

    write_out(out, record)
}

/// Poll one batch of MIDI input records without blocking.
pub(crate) unsafe fn destack_midi_input_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<MidiInputRecord>,
    handle: resource::MidiInputPortHandle,
    maxrecords: u32,
) -> RuntimeResult<()> {
    let records = host::midi_input_try_read_batch(binding, handle, maxrecords)?;
    let records = midi_core::store_input_records_native(binding, records);

    write_out(out, records)
}

/// Create one virtual MIDI input endpoint.
pub(crate) unsafe fn destack_midi_input_virtual_create(
    binding: &BindingCallContext,
    out: *mut resource::MidiInputPortHandle,
    options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<()> {
    let handle = host::midi_input_virtual_create(binding, options)?;

    write_out(out, handle)
}

/// List available MIDI output endpoints.
pub(crate) unsafe fn destack_midi_output_port_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<MidiPortDescriptor>,
    options: MidiPortListOptions,
) -> RuntimeResult<()> {
    let descriptors = host::midi_output_port_list(binding, options)?;
    let descriptors = midi_core::store_port_descriptors_native(binding, descriptors);

    write_out(out, descriptors)
}

/// Open one MIDI output endpoint.
pub(crate) unsafe fn destack_midi_output_port_open(
    binding: &BindingCallContext,
    out: *mut resource::MidiOutputPortHandle,
    id: NativeStringRef,
    options: MidiOutputPortOpenOptions,
) -> RuntimeResult<()> {
    let id = native_string(id)?;
    let handle = host::midi_output_port_open(binding, &id, options)?;

    write_out(out, handle)
}

/// Describe one opened MIDI output endpoint.
pub(crate) unsafe fn destack_midi_output_port_descriptor(
    binding: &BindingCallContext,
    out: *mut MidiPortDescriptor,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let descriptor = host::midi_output_port_descriptor(binding, handle)?;
    let descriptor = descriptor.into_native(binding);

    write_out(out, descriptor)
}

/// Close one opened MIDI output endpoint.
pub(crate) unsafe fn destack_midi_output_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    host::midi_output_port_close(binding, handle)
}

/// Write one batch of outbound MIDI records.
pub(crate) unsafe fn destack_midi_output_write(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::MidiOutputPortHandle,
    records: NativeArray<MidiOutputRecord>,
) -> RuntimeResult<()> {
    let records = output_records_from_native(records)?;
    let written = host::midi_output_write(binding, handle, records)?;

    write_out(out, written)
}

/// Create one virtual MIDI output endpoint.
pub(crate) unsafe fn destack_midi_output_virtual_create(
    binding: &BindingCallContext,
    out: *mut resource::MidiOutputPortHandle,
    options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<()> {
    let handle = host::midi_output_virtual_create(binding, options)?;

    write_out(out, handle)
}
