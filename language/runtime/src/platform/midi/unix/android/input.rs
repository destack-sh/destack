use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::midi::core::{
    MidiInputRecordValue, MidiPortDescriptorValue, filter_port_descriptors, input_queue_capacity,
    remove_midi_input_resource, resolve_descriptor_open_transport, resolve_descriptor_row,
    validate_record_shape,
};
use crate::platform::midi::{
    MidiBackend, MidiDataFormat, MidiInputPortOpenOptions, MidiPortListFlags, MidiPortListOptions,
    MidiProtocol, MidiVirtualInputCreateOptions,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::{BindingCallContext, NativeStringRef};

use super::backend::resolve_backend;
use super::core::{
    AndroidInputSession, close_input_session, create_virtual_input_session, insert_input_resource,
    open_input_session, read_input_port_descriptors, read_input_records,
};
use super::resource::input_resource;

/// List Android MIDI input endpoints with one explicit flag set.
pub(super) fn list_input_descriptors(
    binding: &BindingCallContext,
    flags: MidiPortListFlags,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let descriptors = read_input_port_descriptors(binding, flags.0, operation)?;

    Ok(filter_port_descriptors(descriptors, flags))
}

/// Resolve one Android input descriptor by stable runtime id.
fn resolve_input_descriptor(
    binding: &BindingCallContext,
    id: &str,
    flags: MidiPortListFlags,
    operation: &'static str,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let descriptors = list_input_descriptors(binding, flags, operation)?;

    resolve_descriptor_row(descriptors, id, operation, |descriptor| descriptor)
}

/// List Android MIDI input endpoints.
pub(crate) fn midi_input_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    // backend selection
    let backend = resolve_backend(
        binding,
        options.backend,
        options.backend_policy,
        "destack.midi.input.port.list",
    )?;

    debug_assert_eq!(backend, MidiBackend::AndroidMidi);

    list_input_descriptors(binding, options.flags, "destack.midi.input.port.list")
}

/// Open one Android MIDI input endpoint.
pub(crate) fn midi_input_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiInputPortOpenOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    // backend selection
    let backend = resolve_backend(
        binding,
        options.backend,
        options.backend_policy,
        "destack.midi.input.port.open",
    )?;

    debug_assert_eq!(backend, MidiBackend::AndroidMidi);

    // current descriptor and transport selection
    let descriptor = resolve_input_descriptor(
        binding,
        id,
        MidiPortListFlags(0),
        "destack.midi.input.port.open",
    )?;
    let (data_format, protocol) = resolve_descriptor_open_transport(
        "destack.midi.input.port.open",
        &descriptor,
        options.data_format,
        options.protocol,
        MidiDataFormat::Midi1Bytes,
        Some(MidiProtocol::Midi1),
    )?;

    // host open
    let id = binding.store_string(id);
    let queue_capacity = input_queue_capacity(options.queue_capacity);
    let queue_capacity = u32::try_from(queue_capacity).map_err(|_| {
        core_platform::invalid_argument(
            "queueCapacity",
            "destack.midi.input.port.open: queueCapacity exceeds the Android host ABI width",
        )
    })?;
    let (host_session_id, descriptor) = open_input_session(
        binding,
        id,
        data_format,
        protocol,
        queue_capacity,
        "destack.midi.input.port.open",
    )?;

    let session = Arc::new(AndroidInputSession {
        descriptor,
        host_session_id,
    });

    Ok(insert_input_resource(binding, session))
}

/// Describe one opened Android MIDI input endpoint.
pub(crate) fn midi_input_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let session = input_resource(binding, handle, "destack.midi.input.port.descriptor")?;

    Ok(session.descriptor.clone())
}

/// Close one opened Android MIDI input endpoint.
pub(crate) fn midi_input_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    let session = input_resource(binding, handle, "destack.midi.input.port.close")?;

    // close the host session before removing the handle
    close_input_session(
        binding,
        session.host_session_id,
        "destack.midi.input.port.close",
    )?;

    remove_midi_input_resource(
        binding,
        handle.0,
        "destack.midi.input.port.close",
        "midi input",
    )
}

/// Wait for one Android MIDI input record.
pub(crate) fn midi_input_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiInputRecordValue> {
    let session = input_resource(binding, handle, "destack.midi.input.read")?;
    let mut records = read_input_records(
        binding,
        session.host_session_id,
        1,
        timeout_ns,
        "destack.midi.input.read",
    )?;

    // report would-block when the host returns no records
    if records.is_empty() {
        return Err(core_platform::io_would_block(
            "destack.midi.input.read",
            "android host midi input read returned no records",
        ));
    }

    Ok(records.remove(0))
}

/// Wait for one batch of Android MIDI input records.
pub(crate) fn midi_input_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    max_records: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    let session = input_resource(binding, handle, "destack.midi.input.readBatch")?;
    let records = read_input_records(
        binding,
        session.host_session_id,
        max_records.max(1),
        timeout_ns,
        "destack.midi.input.readBatch",
    )?;

    // report would-block when the host returns no records
    if records.is_empty() {
        return Err(core_platform::io_would_block(
            "destack.midi.input.readBatch",
            "android host midi input read returned no records",
        ));
    }

    Ok(records)
}

/// Poll one Android MIDI input record without blocking.
pub(crate) fn midi_input_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiInputRecordValue> {
    let session = input_resource(binding, handle, "destack.midi.input.tryRead")?;
    let mut records = read_input_records(
        binding,
        session.host_session_id,
        1,
        0,
        "destack.midi.input.tryRead",
    )?;

    let Some(record) = records.pop() else {
        return Err(core_platform::io_would_block(
            "destack.midi.input.tryRead",
            "android host midi input read returned no records",
        ));
    };

    Ok(record)
}

/// Poll one batch of Android MIDI input records without blocking.
pub(crate) fn midi_input_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    max_records: u32,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    let session = input_resource(binding, handle, "destack.midi.input.tryReadBatch")?;

    let records = read_input_records(
        binding,
        session.host_session_id,
        max_records.max(1),
        0,
        "destack.midi.input.tryReadBatch",
    )?;

    if records.is_empty() {
        return Err(core_platform::io_would_block(
            "destack.midi.input.tryReadBatch",
            "android host midi input read returned no records",
        ));
    }

    Ok(records)
}

/// Create one Android virtual MIDI input endpoint.
pub(crate) fn midi_input_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    // backend selection
    let backend = resolve_backend(
        binding,
        options.backend,
        options.backend_policy,
        "destack.midi.input.virtual.create",
    )?;

    debug_assert_eq!(backend, MidiBackend::AndroidMidi);

    // requested transport
    let data_format = options.data_format;
    let protocol = Some(options.protocol);

    validate_record_shape("destack.midi.input.virtual.create", data_format, protocol)?;

    // host create
    let name = options.name;
    let manufacturer = options.manufacturer.unwrap_or(NativeStringRef::from(""));
    let model = options.model.unwrap_or(NativeStringRef::from(""));
    let version = options.version.unwrap_or(NativeStringRef::from(""));
    let queue_capacity = input_queue_capacity(options.queue_capacity);
    let queue_capacity = u32::try_from(queue_capacity).map_err(|_| {
        core_platform::invalid_argument(
            "queueCapacity",
            "destack.midi.input.virtual.create: queueCapacity exceeds the Android host ABI width",
        )
    })?;
    let (host_session_id, descriptor) = create_virtual_input_session(
        binding,
        name,
        manufacturer,
        model,
        version,
        data_format,
        protocol,
        queue_capacity,
        "destack.midi.input.virtual.create",
    )?;

    let session = Arc::new(AndroidInputSession {
        descriptor,
        host_session_id,
    });

    Ok(insert_input_resource(binding, session))
}
