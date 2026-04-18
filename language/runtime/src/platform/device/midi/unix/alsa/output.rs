use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::device::midi::core::{
    MidiOutputRecordValue, MidiPortDescriptorValue, remove_midi_output_resource,
    resolve_descriptor_open_transport, validate_output_record_payload, validate_record_shape,
};
use crate::platform::device::{
    MidiDataFormat, MidiEventSource, MidiOutputPortOpenOptions, MidiPortDirection,
    MidiPortListOptions, MidiProtocol, MidiVirtualOutputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::abi::SND_SEQ_OPEN_DUPLEX;
use super::core::{
    AlsaOutputRepository, AlsaOutputRepositoryKind, connect_to, create_midi_parser, create_queue,
    create_simple_port, default_port_type, hidden_source_port_capability, insert_output_resource,
    native_optional_string, native_string, open_sequencer_handle, schedule_output_event,
    virtual_source_port_capability,
};
use super::descriptor::{
    endpoint_address, filtered_descriptors, resolve_endpoint, virtual_output_descriptor,
};
use super::event::refresh_native_event_sessions;
use super::resource::output_resource;

/// The empty ALSA event type used when the parser has not produced one event yet.
const SND_SEQ_EVENT_NONE: u8 = 255;

/// List ALSA sequencer output endpoints.
pub(crate) fn midi_output_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let service = binding
        .worker()
        .platform_state
        .device
        .alsa_service("destack.device.midi.output.port.list")?;

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
    let service = binding
        .worker()
        .platform_state
        .device
        .alsa_service("destack.device.midi.output.port.open")?;
    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Output,
        id,
        "destack.device.midi.output.port.open",
    )?;

    let (data_format, protocol) = resolve_descriptor_open_transport(
        "destack.device.midi.output.port.open",
        &endpoint.descriptor,
        options.data_format,
        options.protocol,
        MidiDataFormat::Midi1Bytes,
        Some(MidiProtocol::Midi1),
    )?;

    let handle = open_sequencer_handle(
        &service.library,
        "Destack MIDI Internal Output Repository",
        SND_SEQ_OPEN_DUPLEX,
        "destack.device.midi.output.port.open",
    )?;
    let local_port_id = create_simple_port(
        &handle,
        "Destack MIDI Internal Output Repository",
        hidden_source_port_capability(),
        default_port_type(),
        "destack.device.midi.output.port.open",
    )?;
    let queue = create_queue(
        &handle,
        "Destack MIDI Output Queue",
        "destack.device.midi.output.port.open",
    )?;
    let (remote_client_id, remote_port_id) =
        endpoint_address(&endpoint.descriptor, "destack.device.midi.output.port.open")?;
    connect_to(&handle, local_port_id, remote_client_id, remote_port_id)?;
    let parser = create_midi_parser(&service.library, "destack.device.midi.output.port.open")?;

    let session = Arc::new(AlsaOutputRepository {
        _service: service,
        descriptor: endpoint.descriptor,
        data_format,
        protocol,
        kind: AlsaOutputRepositoryKind::Destination {
            handle,
            queue,
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
    let session = output_resource(
        binding,
        handle,
        "destack.device.midi.output.port.descriptor",
    )?;

    Ok(session.descriptor.clone())
}

/// Close one opened ALSA sequencer output endpoint.
pub(crate) fn midi_output_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    remove_midi_output_resource(
        binding,
        handle.0,
        "destack.device.midi.output.port.close",
        "midi output",
    )
}

/// Write ALSA sequencer output records.
pub(crate) fn midi_output_write(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    records: Vec<MidiOutputRecordValue>,
) -> RuntimeResult<u32> {
    let session = output_resource(binding, handle, "destack.device.midi.output.write")?;

    // validate first
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

    // encode and send
    match &session.kind {
        AlsaOutputRepositoryKind::Destination {
            handle,
            queue,
            local_port_id,
            parser,
            ..
        }
        | AlsaOutputRepositoryKind::VirtualSource {
            handle,
            queue,
            local_port_id,
            parser,
        } => {
            let parser = parser.lock();
            let mut has_queued_records = false;

            for record in &records {
                let mut remaining = record.data.as_slice();
                let scheduled_delay_ns = record
                    .send_at_ns
                    .map(|send_at_ns| send_at_ns.saturating_sub(core_platform::monotonic_now_ns()))
                    .filter(|delay_ns| *delay_ns != 0);

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
                            "destack.device.midi.output.write",
                            "snd_midi_event_encode",
                            encoded as i32,
                        ));
                    }

                    if encoded == 0 {
                        return Err(core_platform::io_operation_error(
                            "destack.device.midi.output.write",
                            None,
                            "ALSA did not consume any MIDI bytes during encoding",
                        ));
                    }

                    // incomplete or ignored bytes
                    if event.type_ == SND_SEQ_EVENT_NONE {
                        return Err(core_platform::invalid_argument(
                            "records",
                            "destack.device.midi.output.write: ALSA could not encode one complete MIDI transport record from the provided bytes",
                        ));
                    }

                    // direct or scheduled send
                    let status = if let Some(delay_ns) = scheduled_delay_ns {
                        schedule_output_event(&mut event, queue.id, delay_ns);
                        has_queued_records = true;

                        unsafe { (handle.library.api.snd_seq_event_output)(handle.raw, &mut event) }
                    } else {
                        unsafe {
                            (handle.library.api.snd_seq_event_output_direct)(handle.raw, &mut event)
                        }
                    };
                    if status < 0 {
                        return Err(super::core::alsa_operation_error(
                            &handle.library,
                            "destack.device.midi.output.write",
                            "snd_seq_event_output_direct",
                            status,
                        ));
                    }

                    remaining = &remaining[encoded as usize..];
                }
            }

            // submit queued records
            if has_queued_records {
                let status = unsafe { (handle.library.api.snd_seq_drain_output)(handle.raw) };
                if status < 0 {
                    return Err(super::core::alsa_operation_error(
                        &handle.library,
                        "destack.device.midi.output.write",
                        "snd_seq_drain_output",
                        status,
                    ));
                }
            }

            Ok(records.len() as u32)
        }
    }
}

/// Create one virtual ALSA sequencer output endpoint.
pub(crate) fn midi_output_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    let name = native_string(options.name)?;
    let _manufacturer = native_optional_string(options.manufacturer)?;
    let _model = native_optional_string(options.model)?;
    let _version = native_optional_string(options.version)?;

    let service = binding
        .worker()
        .platform_state
        .device
        .alsa_service("destack.device.midi.output.virtual.create")?;

    validate_record_shape(
        "destack.device.midi.output.virtual.create",
        options.data_format,
        Some(options.protocol),
    )?;

    let handle = open_sequencer_handle(
        &service.library,
        &name,
        SND_SEQ_OPEN_DUPLEX,
        "destack.device.midi.output.virtual.create",
    )?;
    let local_port_id = create_simple_port(
        &handle,
        &name,
        virtual_source_port_capability(),
        default_port_type(),
        "destack.device.midi.output.virtual.create",
    )?;
    let queue = create_queue(
        &handle,
        "Destack MIDI Virtual Output Queue",
        "destack.device.midi.output.virtual.create",
    )?;
    let parser = create_midi_parser(
        &service.library,
        "destack.device.midi.output.virtual.create",
    )?;
    let descriptor = virtual_output_descriptor(
        name,
        handle.client_id,
        local_port_id,
        options.data_format,
        options.protocol,
    );
    let session = Arc::new(AlsaOutputRepository {
        _service: service,
        descriptor,
        data_format: options.data_format,
        protocol: Some(options.protocol),
        kind: AlsaOutputRepositoryKind::VirtualSource {
            handle,
            queue,
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
