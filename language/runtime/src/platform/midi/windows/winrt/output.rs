use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiOutputRecordValue, MidiPortDescriptorValue, remove_labeled_resource,
    validate_output_record_payload, validate_record_shape,
};
use crate::platform::midi::{
    MidiDataFormat, MidiOutputPortOpenOptions, MidiPortDirection, MidiPortListOptions,
    MidiProtocol, MidiVirtualOutputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::backend::resolve_backend;
use super::core::{WinRtOutputSession, insert_output_resource};
use super::descriptor::{
    filtered_descriptors, resolve_endpoint, validate_endpoint_transport_request,
};
use super::resource::output_resource;
/// List WinRT output ports.
pub(crate) fn midi_output_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.output.port.list",
    )?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .winrt_service("destack.midi.output.port.list")?;

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
    binding
        .agent()
        .platform_state
        .midi
        .mark_runtime_active(binding);

    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.output.port.open",
    )?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .winrt_service("destack.midi.output.port.open")?;

    validate_record_shape(
        "destack.midi.output.port.open",
        options.data_format.unwrap_or(MidiDataFormat::Midi1Bytes),
        options.protocol,
    )?;

    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Output,
        id,
        "destack.midi.output.port.open",
    )?;
    let descriptor = endpoint.descriptor.clone();
    let data_format = options
        .data_format
        .or(descriptor.default_data_format)
        .unwrap_or(MidiDataFormat::Midi1Bytes);
    let protocol = options.protocol.or(descriptor.default_protocol);

    validate_endpoint_transport_request(
        "destack.midi.output.port.open",
        &descriptor,
        data_format,
        protocol,
    )?;
    let host_session_id =
        service.open_output_session(endpoint.backend_id, "destack.midi.output.port.open")?;

    let session = Arc::new(WinRtOutputSession {
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
    let session = output_resource(binding, handle, "destack.midi.output.port.descriptor")?;

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
        "destack.midi.output.port.close",
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
        "destack.midi.output.write",
    )
}

/// Flush queued outbound WinRT MIDI records.
pub(crate) fn midi_output_flush(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let _session = output_resource(binding, handle, "destack.midi.output.flush")?;
    let _ = binding;

    Ok(())
}

/// Reject virtual output creation on WinRT.
pub(crate) fn midi_output_virtual_create(
    binding: &BindingCallContext,
    _options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    binding
        .agent()
        .platform_state
        .midi
        .mark_runtime_active(binding);

    Err(core_platform::not_supported(
        "destack.midi.output.virtual.create",
    ))
}
