use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeStringRef;
use crate::platform::core::{self as core_platform};
use crate::platform::device::midi::core::{
    MidiOutputRecordValue, MidiPortDescriptorValue, remove_labeled_resource,
    resolve_descriptor_open_transport, validate_output_record_payload, validate_record_shape,
};
use crate::platform::device::{
    MidiDataFormat, MidiOutputPortOpenOptions, MidiPortDirection, MidiPortListOptions,
    MidiProtocol, MidiVirtualOutputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::core::{WindowsMidiOutputRepository, insert_output_resource};
use super::descriptor::{filtered_descriptors, resolve_endpoint};
use super::resource::output_resource;

/// Decode one native string into one owned Rust string.
fn owned_native_string(value: NativeStringRef) -> RuntimeResult<String> {
    unsafe { value.as_str().map(str::to_owned) }
}

/// Decode one optional native string into one owned Rust string.
fn owned_optional_native_string(value: Option<NativeStringRef>) -> RuntimeResult<Option<String>> {
    value.map(owned_native_string).transpose()
}

/// List Windows MIDI output ports.
pub(crate) fn midi_output_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let service = binding
        .worker()
        .platform_state
        .device
        .windows_midi_service("destack.device.midi.output.port.list")?;

    Ok(filtered_descriptors(
        &service,
        MidiPortDirection::Output,
        options.flags,
    ))
}

/// Open one Windows MIDI output session.
pub(crate) fn midi_output_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiOutputPortOpenOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    let service = binding
        .worker()
        .platform_state
        .device
        .windows_midi_service("destack.device.midi.output.port.open")?;

    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Output,
        id,
        "destack.device.midi.output.port.open",
    )?;
    let descriptor = endpoint.descriptor.clone();
    let (data_format, protocol) = resolve_descriptor_open_transport(
        "destack.device.midi.output.port.open",
        &descriptor,
        options.data_format,
        options.protocol,
        MidiDataFormat::Ump,
        None,
    )?;
    let host_session_id =
        service.open_output_session(endpoint.backend_id, "destack.device.midi.output.port.open")?;

    let session = Arc::new(WindowsMidiOutputRepository {
        _service: service,
        descriptor,
        data_format,
        protocol,
        host_session_id,
    });

    Ok(insert_output_resource(binding, session))
}

/// Describe one opened Windows MIDI output session.
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

/// Close one opened Windows MIDI output session.
pub(crate) fn midi_output_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    remove_labeled_resource(
        binding,
        handle.0,
        "destack.device.midi.output.port.close",
        "midi output port",
    )?;

    Ok(())
}

/// Write outbound Windows MIDI records.
pub(crate) fn midi_output_write(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    records: Vec<MidiOutputRecordValue>,
) -> RuntimeResult<u32> {
    let session = output_resource(binding, handle, "destack.device.midi.output.write")?;

    for record in &records {
        // transport shape
        validate_record_shape(
            "destack.device.midi.output.write",
            record.data_format,
            record.protocol,
        )?;

        // transport payload
        validate_output_record_payload("destack.device.midi.output.write", record)?;

        // opened transport
        if record.data_format != session.data_format {
            return Err(core_platform::invalid_argument(
                "records",
                "record data format does not match the opened output session",
            ));
        }

        if record.protocol != session.protocol {
            return Err(core_platform::invalid_argument(
                "records",
                "record protocol does not match the opened output session",
            ));
        }
    }

    session._service.write_output_records(
        session.host_session_id,
        records,
        "destack.device.midi.output.write",
    )
}

/// Reject virtual output creation on Windows MIDI.
pub(crate) fn midi_output_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    // requested transport
    validate_virtual_output_transport(options.data_format, options.protocol)?;

    // shared service and host strings
    let service = binding
        .worker()
        .platform_state
        .device
        .windows_midi_service("destack.device.midi.output.virtual.create")?;
    let name = owned_native_string(options.name)?;
    let manufacturer = owned_optional_native_string(options.manufacturer)?;
    let model = owned_optional_native_string(options.model)?;
    let version = owned_optional_native_string(options.version)?;
    let (host_session_id, descriptor) = service.create_virtual_output_session(
        name,
        manufacturer,
        model,
        version,
        options.protocol,
        "destack.device.midi.output.virtual.create",
    )?;

    let session = Arc::new(WindowsMidiOutputRepository {
        _service: service,
        descriptor,
        data_format: options.data_format,
        protocol: Some(options.protocol),
        host_session_id,
    });

    Ok(insert_output_resource(binding, session))
}

/// Reject unsupported Windows MIDI virtual-output transport requests.
fn validate_virtual_output_transport(
    data_format: MidiDataFormat,
    _protocol: MidiProtocol,
) -> RuntimeResult<()> {
    if data_format != MidiDataFormat::Ump {
        return Err(core_platform::not_supported(
            "destack.device.midi.output.virtual.create",
        ));
    }

    Ok(())
}
