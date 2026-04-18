use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
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

use super::core::{WinRtOutputRepository, insert_output_resource};
use super::descriptor::{filtered_descriptors, resolve_endpoint};
use super::resource::output_resource;

/// List WinRT output ports.
pub(crate) fn midi_output_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let service = binding
        .worker()
        .platform_state
        .device
        .winrt_service("destack.device.midi.output.port.list")?;

    Ok(filtered_descriptors(
        &service,
        MidiPortDirection::Output,
        options.flags,
    ))
}

/// Open one WinRT output session.
pub(crate) fn midi_output_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiOutputPortOpenOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    let service = binding
        .worker()
        .platform_state
        .device
        .winrt_service("destack.device.midi.output.port.open")?;

    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Output,
        id,
        "destack.device.midi.output.port.open",
    )?;
    let descriptor = endpoint.descriptor.clone();
    let (_data_format, _protocol) = resolve_descriptor_open_transport(
        "destack.device.midi.output.port.open",
        &descriptor,
        options.data_format,
        options.protocol,
        MidiDataFormat::Midi1Bytes,
        None,
    )?;
    let host_session_id =
        service.open_output_session(endpoint.backend_id, "destack.device.midi.output.port.open")?;

    let session = Arc::new(WinRtOutputRepository {
        _service: service,
        descriptor,
        host_session_id,
    });

    Ok(insert_output_resource(binding, session))
}

/// Describe one opened WinRT output session.
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

/// Close one opened WinRT output session.
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

/// Write outbound WinRT MIDI records.
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

        // winrt transport constraints
        if record.send_at_ns.is_some() {
            return Err(core_platform::invalid_argument(
                "records",
                "winrt midi output does not support scheduled send timestamps",
            ));
        }

        if record.data_format != MidiDataFormat::Midi1Bytes {
            return Err(core_platform::invalid_argument(
                "records",
                "winrt midi output only supports MIDI 1 byte-stream transport",
            ));
        }

        if record.protocol != Some(MidiProtocol::Midi1) && record.protocol.is_some() {
            return Err(core_platform::invalid_argument(
                "records",
                "winrt midi output only supports MIDI 1 protocol semantics",
            ));
        }
    }

    let _ = binding;

    session._service.write_output_records(
        session.host_session_id,
        records,
        "destack.device.midi.output.write",
    )
}

/// Reject virtual output creation on WinRT.
pub(crate) fn midi_output_virtual_create(
    _binding: &BindingCallContext,
    _options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    Err(core_platform::not_supported(
        "destack.device.midi.output.virtual.create",
    ))
}
