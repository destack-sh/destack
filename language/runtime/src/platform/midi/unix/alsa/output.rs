use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiOutputRecordValue, MidiPortDescriptorValue, validate_output_record_payload,
    validate_record_shape,
};
use crate::platform::midi::{
    MidiDataFormat, MidiEventSource, MidiOutputPortOpenOptions, MidiPortDirection,
    MidiPortListOptions, MidiProtocol, MidiVirtualOutputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::abi::SND_SEQ_OPEN_DUPLEX;
use super::backend::resolve_backend;
use super::core::{
    AlsaOutputSession, AlsaOutputSessionKind, connect_to, create_midi_parser, create_simple_port,
    default_port_type, hidden_source_port_capability, insert_output_resource,
    native_optional_string, native_string, open_sequencer_handle, virtual_source_port_capability,
};
use super::descriptor::{
    endpoint_address, filtered_descriptors, resolve_endpoint, validate_endpoint_transport_request,
    virtual_output_descriptor,
};
use super::event::refresh_native_event_sessions;
use super::resource::{output_resource, remove_output_resource};

/// The empty ALSA event type used when the parser has not produced one event yet.
const SND_SEQ_EVENT_NONE: u8 = 255;

/// List ALSA sequencer output endpoints.
pub(crate) fn midi_output_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    binding
        .agent()
        .platform_state
        .midi
        .mark_runtime_active(binding);

    let _backend = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.output.port.list",
    )?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .alsa_service("destack.midi.output.port.list")?;

    Ok(filtered_descriptors(
        &service,
        MidiPortDirection::Output,
        options.flags,
    ))
}

/// Open one ALSA sequencer output endpoint.
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

    let _backend = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.output.port.open",
    )?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .alsa_service("destack.midi.output.port.open")?;
    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Output,
        id,
        "destack.midi.output.port.open",
    )?;

    let data_format = options.data_format.unwrap_or(
        endpoint
            .descriptor
            .default_data_format
            .unwrap_or(MidiDataFormat::Midi1Bytes),
    );
    let protocol = options
        .protocol
        .or(endpoint.descriptor.default_protocol)
        .unwrap_or(MidiProtocol::Midi1);

    validate_record_shape("destack.midi.output.port.open", data_format, Some(protocol))?;
    validate_endpoint_transport_request(
        "destack.midi.output.port.open",
        &endpoint.descriptor,
        data_format,
        Some(protocol),
    )?;

    let handle = open_sequencer_handle(
        &service.library,
        "Destack MIDI Internal Output Session",
        SND_SEQ_OPEN_DUPLEX,
        "destack.midi.output.port.open",
    )?;
    let local_port_id = create_simple_port(
        &handle,
        "Destack MIDI Internal Output Session",
        hidden_source_port_capability(),
        default_port_type(),
        "destack.midi.output.port.open",
    )?;
    let (remote_client_id, remote_port_id) =
        endpoint_address(&endpoint.descriptor, "destack.midi.output.port.open")?;
    connect_to(&handle, local_port_id, remote_client_id, remote_port_id)?;
    let parser = create_midi_parser(&service.library, "destack.midi.output.port.open")?;

    let session = Arc::new(AlsaOutputSession {
        _service: service,
        descriptor: endpoint.descriptor,
        data_format,
        protocol: Some(protocol),
        kind: AlsaOutputSessionKind::Destination {
            handle,
            local_port_id,
            remote_client_id,
            remote_port_id,
            parser: parking_lot::Mutex::new(parser),
        },
    });

    Ok(insert_output_resource(binding, session))
}

/// Describe one opened ALSA sequencer output endpoint.
pub(crate) fn midi_output_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let session = output_resource(binding, handle, "destack.midi.output.port.descriptor")?;

    Ok(session.descriptor.clone())
}

/// Close one opened ALSA sequencer output endpoint.
pub(crate) fn midi_output_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    remove_output_resource(binding, handle, "destack.midi.output.port.close")
}

/// Write ALSA sequencer output records.
pub(crate) fn midi_output_write(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    records: Vec<MidiOutputRecordValue>,
) -> RuntimeResult<u32> {
    let session = output_resource(binding, handle, "destack.midi.output.write")?;

    // validate first
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
                "destack.midi.output.write: record data format does not match the opened port",
            ));
        }

        if record.protocol != session.protocol {
            return Err(core_platform::invalid_argument(
                "records",
                "destack.midi.output.write: record protocol does not match the opened port",
            ));
        }
    }

    // encode and send
    match &session.kind {
        AlsaOutputSessionKind::Destination {
            handle,
            local_port_id,
            parser,
            ..
        }
        | AlsaOutputSessionKind::VirtualSource {
            handle,
            local_port_id,
            parser,
        } => {
            let parser = parser.lock();

            for record in &records {
                let mut remaining = record.data.as_slice();

                while !remaining.is_empty() {
                    let mut event = unsafe { std::mem::zeroed() };
                    super::core::initialize_output_event(&mut event, *local_port_id);

                    // parser step
                    let encoded = unsafe {
                        (handle.library.api.snd_midi_event_encode)(
                            parser.raw,
                            remaining.as_ptr(),
                            remaining.len() as _,
                            &mut event,
                        )
                    };
                    if encoded < 0 {
                        return Err(super::core::alsa_operation_error(
                            &handle.library,
                            "destack.midi.output.write",
                            "snd_midi_event_encode",
                            encoded as i32,
                        ));
                    }

                    if encoded == 0 {
                        return Err(core_platform::io_operation_error(
                            "destack.midi.output.write",
                            None,
                            "ALSA did not consume any MIDI bytes during encoding",
                        ));
                    }

                    // incomplete or ignored bytes
                    if event.type_ == SND_SEQ_EVENT_NONE {
                        return Err(core_platform::invalid_argument(
                            "records",
                            "destack.midi.output.write: ALSA could not encode one complete MIDI transport record from the provided bytes",
                        ));
                    }

                    // direct send
                    let status = unsafe {
                        (handle.library.api.snd_seq_event_output_direct)(handle.raw, &mut event)
                    };
                    if status < 0 {
                        return Err(super::core::alsa_operation_error(
                            &handle.library,
                            "destack.midi.output.write",
                            "snd_seq_event_output_direct",
                            status,
                        ));
                    }

                    remaining = &remaining[encoded as usize..];
                }
            }

            Ok(records.len() as u32)
        }
    }
}

/// Flush queued ALSA sequencer output records.
pub(crate) fn midi_output_flush(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let session = output_resource(binding, handle, "destack.midi.output.flush")?;

    match &session.kind {
        AlsaOutputSessionKind::Destination { handle, .. }
        | AlsaOutputSessionKind::VirtualSource { handle, .. } => {
            let status = unsafe { (handle.library.api.snd_seq_drain_output)(handle.raw) };
            if status < 0 {
                return Err(super::core::alsa_operation_error(
                    &handle.library,
                    "destack.midi.output.flush",
                    "snd_seq_drain_output",
                    status,
                ));
            }
        }
    }

    Ok(())
}

/// Create one virtual ALSA sequencer output endpoint.
pub(crate) fn midi_output_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    binding
        .agent()
        .platform_state
        .midi
        .mark_runtime_active(binding);

    let name = native_string(options.name)?;
    let _manufacturer = native_optional_string(options.manufacturer)?;
    let _model = native_optional_string(options.model)?;
    let _version = native_optional_string(options.version)?;

    let _backend = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.output.virtual.create",
    )?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .alsa_service("destack.midi.output.virtual.create")?;

    validate_record_shape(
        "destack.midi.output.virtual.create",
        options.data_format,
        Some(options.protocol),
    )?;

    let handle = open_sequencer_handle(
        &service.library,
        &name,
        SND_SEQ_OPEN_DUPLEX,
        "destack.midi.output.virtual.create",
    )?;
    let local_port_id = create_simple_port(
        &handle,
        &name,
        virtual_source_port_capability(),
        default_port_type(),
        "destack.midi.output.virtual.create",
    )?;
    let parser = create_midi_parser(&service.library, "destack.midi.output.virtual.create")?;
    let descriptor = virtual_output_descriptor(
        name,
        handle.client_id,
        local_port_id,
        options.data_format,
        options.protocol,
    );
    let session = Arc::new(AlsaOutputSession {
        _service: service,
        descriptor,
        data_format: options.data_format,
        protocol: Some(options.protocol),
        kind: AlsaOutputSessionKind::VirtualSource {
            handle,
            local_port_id,
            parser: parking_lot::Mutex::new(parser),
        },
    });
    let handle = insert_output_resource(binding, session.clone());

    refresh_native_event_sessions(
        &session._service.topology,
        &session._service.native_event_registry,
        MidiEventSource::Native,
    );

    Ok(handle)
}
