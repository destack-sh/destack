use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiOutputRecordValue, MidiPortDescriptorValue, remove_labeled_resource,
    resolve_descriptor_open_transport, validate_output_record_payload, validate_record_shape,
};
use crate::platform::midi::{
    MidiDataFormat, MidiOutputPortOpenOptions, MidiPortDirection, MidiPortListOptions,
    MidiVirtualOutputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::core::{WindowsMidiOutputSession, insert_output_resource};
use super::descriptor::{filtered_descriptors, resolve_endpoint};
use super::resource::output_resource;

/// List Windows MIDI output ports.
pub(crate) fn midi_output_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let service = binding
        .agent()
        .platform_state
        .midi
        .windows_midi_service("destack.midi.output.port.list")?;

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
        .agent()
        .platform_state
        .midi
        .windows_midi_service("destack.midi.output.port.open")?;

    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Output,
        id,
        "destack.midi.output.port.open",
    )?;
    let descriptor = endpoint.descriptor.clone();
    let (data_format, protocol) = resolve_descriptor_open_transport(
        "destack.midi.output.port.open",
        &descriptor,
        options.data_format,
        options.protocol,
        MidiDataFormat::Ump,
        None,
    )?;
    let host_session_id =
        service.open_output_session(endpoint.backend_id, "destack.midi.output.port.open")?;

    let session = Arc::new(WindowsMidiOutputSession {
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
    let session = output_resource(binding, handle, "destack.midi.output.port.descriptor")?;

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
        "destack.midi.output.port.close",
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
    let session = output_resource(binding, handle, "destack.midi.output.write")?;

    for record in &records {
        // transport shape
        validate_record_shape(
            "destack.midi.output.write",
            record.data_format,
            record.protocol,
        )?;

        // transport payload
        validate_output_record_payload("destack.midi.output.write", record)?;

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
        "destack.midi.output.write",
    )
}

/// Reject virtual output creation on Windows MIDI.
pub(crate) fn midi_output_virtual_create(
    _binding: &BindingCallContext,
    _options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    Err(core_platform::not_supported(
        "destack.midi.output.virtual.create",
    ))
}
