use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeStringRef;
use crate::platform::device::midi::core::{
    MidiOutputRecordValue, MidiPortDescriptorValue, filter_port_descriptors,
    remove_midi_output_resource, resolve_descriptor_open_transport, resolve_descriptor_row,
    validate_output_record_payload, validate_record_shape,
};
use crate::platform::device::{
    MIDI_BACKEND_CAP_SCHEDULED_OUTPUT, MidiBackend, MidiDataFormat, MidiOutputPortOpenOptions,
    MidiPortListFlags, MidiPortListOptions, MidiProtocol, MidiVirtualOutputCreateOptions,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::backend::resolve_backend;
use super::core::{
    AndroidOutputRepository, close_output_session, create_virtual_output_session,
    insert_output_resource, open_output_session, read_output_port_descriptors,
    write_output_records,
};
use super::resource::output_resource;

/// List Android MIDI output endpoints with one explicit flag set.
pub(super) fn list_output_descriptors(
    binding: &BindingCallContext,
    flags: MidiPortListFlags,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let descriptors = read_output_port_descriptors(binding, flags.0, operation)?;

    Ok(filter_port_descriptors(descriptors, flags))
}

/// Resolve one Android output descriptor by stable runtime id.
fn resolve_output_descriptor(
    binding: &BindingCallContext,
    id: &str,
    flags: MidiPortListFlags,
    operation: &'static str,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let descriptors = list_output_descriptors(binding, flags, operation)?;

    resolve_descriptor_row(descriptors, id, operation, |descriptor| descriptor)
}

/// List Android MIDI output endpoints.
pub(crate) fn midi_output_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    // backend selection
    let backend = resolve_backend(
        binding,
        options.backend,
        options.backend_policy,
        "destack.device.midi.output.port.list",
    )?;

    debug_assert_eq!(backend, MidiBackend::AndroidMidi);

    list_output_descriptors(
        binding,
        options.flags,
        "destack.device.midi.output.port.list",
    )
}

/// Open one Android MIDI output endpoint.
pub(crate) fn midi_output_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiOutputPortOpenOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    // backend selection
    let backend = resolve_backend(
        binding,
        options.backend,
        options.backend_policy,
        "destack.device.midi.output.port.open",
    )?;

    debug_assert_eq!(backend, MidiBackend::AndroidMidi);

    // current descriptor and transport selection
    let descriptor = resolve_output_descriptor(
        binding,
        id,
        MidiPortListFlags(0),
        "destack.device.midi.output.port.open",
    )?;
    let (data_format, protocol) = resolve_descriptor_open_transport(
        "destack.device.midi.output.port.open",
        &descriptor,
        options.data_format,
        options.protocol,
        MidiDataFormat::Midi1Bytes,
        Some(MidiProtocol::Midi1),
    )?;

    // host open
    let id = binding.store_string(id);
    let (host_session_id, descriptor) = open_output_session(
        binding,
        id,
        data_format,
        protocol,
        "destack.device.midi.output.port.open",
    )?;

    let session = Arc::new(AndroidOutputRepository {
        descriptor,
        data_format,
        protocol,
        host_session_id,
    });

    Ok(insert_output_resource(binding, session))
}

/// Describe one opened Android MIDI output endpoint.
pub(crate) fn midi_output_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let session = output_resource(
        binding,
        handle,
        "destack.device.midi.output.port.descriptor",
    )?;

    Ok(session.descriptor.clone())
}

/// Close one opened Android MIDI output endpoint.
pub(crate) fn midi_output_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let session = output_resource(binding, handle, "destack.device.midi.output.port.close")?;

    // close the host session before removing the handle
    close_output_session(
        binding,
        session.host_session_id,
        "destack.device.midi.output.port.close",
    )?;

    remove_midi_output_resource(
        binding,
        handle.0,
        "destack.device.midi.output.port.close",
        "midi output",
    )
}

/// Write Android MIDI output records.
pub(crate) fn midi_output_write(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    records: Vec<MidiOutputRecordValue>,
) -> RuntimeResult<u32> {
    let session = output_resource(binding, handle, "destack.device.midi.output.write")?;
    let description = binding
        .worker()
        .platform_state
        .device
        .describe_android_backend(binding, "destack.device.midi.output.write")?;
    let scheduled_output_supported =
        description.capability_flags.0 & MIDI_BACKEND_CAP_SCHEDULED_OUTPUT.0 != 0;

    // validate each record against the opened transport
    for record in &records {
        validate_record_shape(
            "destack.device.midi.output.write",
            record.data_format,
            record.protocol,
        )?;
        validate_output_record_payload("destack.device.midi.output.write", record)?;

        if record.send_at_ns.is_some() && !scheduled_output_supported {
            return Err(core_platform::invalid_argument(
                "records",
                "android midi output does not support scheduled send timestamps",
            ));
        }

        if record.data_format != session.data_format {
            return Err(core_platform::invalid_argument(
                "records",
                "destack.device.midi.output.write: record data format does not match the opened port",
            ));
        }

        if record.protocol != session.protocol {
            return Err(core_platform::invalid_argument(
                "records",
                "destack.device.midi.output.write: record protocol does not match the opened port",
            ));
        }
    }

    write_output_records(
        binding,
        session.host_session_id,
        &records,
        "destack.device.midi.output.write",
    )?;

    Ok(records.len() as u32)
}

/// Create one Android virtual MIDI output endpoint.
pub(crate) fn midi_output_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    // backend selection
    let backend = resolve_backend(
        binding,
        options.backend,
        options.backend_policy,
        "destack.device.midi.output.virtual.create",
    )?;

    debug_assert_eq!(backend, MidiBackend::AndroidMidi);

    // requested transport
    let data_format = options.data_format;
    let protocol = Some(options.protocol);

    validate_record_shape(
        "destack.device.midi.output.virtual.create",
        data_format,
        protocol,
    )?;

    // host create
    let name = options.name;
    let manufacturer = options.manufacturer.unwrap_or(NativeStringRef::from(""));
    let model = options.model.unwrap_or(NativeStringRef::from(""));
    let version = options.version.unwrap_or(NativeStringRef::from(""));
    let (host_session_id, descriptor) = create_virtual_output_session(
        binding,
        name,
        manufacturer,
        model,
        version,
        data_format,
        protocol,
        "destack.device.midi.output.virtual.create",
    )?;

    let session = Arc::new(AndroidOutputRepository {
        descriptor,
        data_format,
        protocol,
        host_session_id,
    });

    Ok(insert_output_resource(binding, session))
}
