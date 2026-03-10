use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiOutputRecordValue, MidiPortDescriptorValue, validate_record_shape,
};
use crate::platform::midi::shared::remove_labeled_resource;
use crate::platform::midi::{
    MidiDataFormat, MidiEventSource, MidiOutputPortOpenOptions, MidiPortDirection,
    MidiPortListOptions, MidiVirtualOutputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::abi::{
    MIDIEventList, MIDIEventListAdd, MIDIEventListInit, MIDIFlushOutput,
    MIDIObjectSetIntegerProperty, MIDIObjectSetStringProperty, MIDIOutputPortCreate,
    MIDIPacketList, MIDIPacketListAdd, MIDIPacketListInit, MIDIReceived, MIDIReceivedEventList,
    MIDISend, MIDISendEventList, MIDISourceCreate, MIDISourceCreateWithProtocol, create_cf_string,
    kMIDIPropertyDriverVersion, kMIDIPropertyManufacturer, kMIDIPropertyModel, release_cf,
};
use super::backend::resolve_backend;
use super::core::{
    CoreMidiOutputSession, CoreMidiOutputSessionKind, core_midi_mono_ns_to_host_time,
    core_midi_status_error, insert_output_resource, native_optional_string, native_string,
    selected_protocol_id,
};
use super::descriptor::{
    endpoint_descriptor, filtered_descriptors, register_endpoint_override, resolve_endpoint,
    validate_endpoint_transport_request,
};
use super::event::refresh_native_event_sessions;
use super::resource::output_resource;
use super::service::core_midi_service;

/// Build one legacy packet list buffer from one byte-stream record.
fn legacy_packet_buffer(record: &MidiOutputRecordValue) -> RuntimeResult<Vec<u8>> {
    let buffer_size = 1024usize.max(std::mem::size_of::<MIDIPacketList>() + record.data.len() + 16);
    let mut buffer = vec![0u8; buffer_size];
    let packet_list = buffer.as_mut_ptr().cast::<MIDIPacketList>();

    let current_packet = unsafe { MIDIPacketListInit(packet_list) };
    let time_stamp = core_midi_mono_ns_to_host_time(record.send_at_ns);
    let added = unsafe {
        MIDIPacketListAdd(
            packet_list,
            buffer_size,
            current_packet,
            time_stamp,
            record.data.len(),
            record.data.as_ptr(),
        )
    };
    if added.is_null() {
        return Err(core_platform::invalid_argument(
            "records",
            "failed to encode one CoreMIDI packet list",
        ));
    }

    Ok(buffer)
}

/// Build one UMP event-list buffer from one record.
fn modern_event_buffer(record: &MidiOutputRecordValue) -> RuntimeResult<Vec<u8>> {
    if record.data.len() % 4 != 0 {
        return Err(core_platform::invalid_argument(
            "records",
            "UMP record payload must be a multiple of 4 bytes",
        ));
    }

    let word_count = record.data.len() / 4;
    let buffer_size = 2048usize.max(std::mem::size_of::<MIDIEventList>() + word_count * 4 + 32);
    let mut buffer = vec![0u8; buffer_size];
    let event_list = buffer.as_mut_ptr().cast::<MIDIEventList>();
    let protocol = selected_protocol_id(record.protocol, record.data_format);

    let mut words = Vec::with_capacity(word_count);

    for chunk in record.data.chunks_exact(4) {
        words.push(u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }

    let current_packet = unsafe { MIDIEventListInit(event_list, protocol) };
    let time_stamp = core_midi_mono_ns_to_host_time(record.send_at_ns);
    let added = unsafe {
        MIDIEventListAdd(
            event_list,
            buffer_size,
            current_packet,
            time_stamp,
            words.len(),
            words.as_ptr(),
        )
    };
    if added.is_null() {
        return Err(core_platform::invalid_argument(
            "records",
            "failed to encode one CoreMIDI event list",
        ));
    }

    Ok(buffer)
}

/// List CoreMIDI output ports.
pub(crate) fn midi_output_port_list(
    _binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.output.port.list",
    )?;
    core_midi_service("destack.midi.output.port.list")?;

    Ok(filtered_descriptors(
        MidiPortDirection::Output,
        options.flags,
    ))
}

/// Open one CoreMIDI output session.
pub(crate) fn midi_output_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiOutputPortOpenOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    let _runtime_state = binding.agent().platform_state.midi.runtime_state(binding);

    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.output.port.open",
    )?;
    let service = core_midi_service("destack.midi.output.port.open")?;

    validate_record_shape(
        "destack.midi.output.port.open",
        options.data_format.unwrap_or(MidiDataFormat::Midi1Bytes),
        options.protocol,
    )?;

    let (endpoint, descriptor) = resolve_endpoint(
        MidiPortDirection::Output,
        id,
        "destack.midi.output.port.open",
    )?;
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

    let Some(name) = create_cf_string("Destack MIDI Output") else {
        return Err(core_platform::io_operation_error(
            "destack.midi.output.port.open",
            None,
            "failed to encode one CoreMIDI output port name",
        ));
    };
    let mut port = 0u32;

    let status = unsafe { MIDIOutputPortCreate(service.operation_client, name, &mut port) };
    release_cf(name.cast());
    if status != 0 || port == 0 {
        return Err(core_midi_status_error(
            "destack.midi.output.port.open",
            "MIDIOutputPortCreate",
            status,
        ));
    }

    let session = Arc::new(CoreMidiOutputSession {
        _service: service,
        descriptor,
        data_format,
        protocol,
        kind: CoreMidiOutputSessionKind::Destination {
            port,
            destination: endpoint,
        },
    });

    Ok(insert_output_resource(binding, session))
}

/// Describe one opened CoreMIDI output session.
pub(crate) fn midi_output_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let session = output_resource(binding, handle, "destack.midi.output.port.descriptor")?;

    Ok(session.descriptor.clone())
}

/// Close one opened CoreMIDI output session.
pub(crate) fn midi_output_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let session = output_resource(binding, handle, "destack.midi.output.port.close")?;
    let is_virtual_endpoint = session.is_virtual_endpoint();
    let native_event_registry = session._service.native_event_registry.clone();
    drop(session);

    remove_labeled_resource(
        binding,
        handle.0,
        "destack.midi.output.port.close",
        "midi output port",
    )?;

    if is_virtual_endpoint {
        refresh_native_event_sessions(&native_event_registry, MidiEventSource::Native);
    }

    Ok(())
}

/// Write outbound CoreMIDI records.
pub(crate) fn midi_output_write(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    records: Vec<MidiOutputRecordValue>,
) -> RuntimeResult<u32> {
    let session = output_resource(binding, handle, "destack.midi.output.write")?;

    for record in &records {
        validate_record_shape(
            "destack.midi.output.write",
            record.data_format,
            record.protocol,
        )?;

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

        match &session.kind {
            CoreMidiOutputSessionKind::Destination { port, destination } => {
                if record.data_format == MidiDataFormat::Ump {
                    let buffer = modern_event_buffer(record)?;
                    let status = unsafe {
                        MIDISendEventList(
                            *port,
                            *destination,
                            buffer.as_ptr().cast::<MIDIEventList>(),
                        )
                    };
                    if status != 0 {
                        return Err(core_midi_status_error(
                            "destack.midi.output.write",
                            "MIDISendEventList",
                            status,
                        ));
                    }
                } else {
                    let buffer = legacy_packet_buffer(record)?;
                    let status = unsafe {
                        MIDISend(
                            *port,
                            *destination,
                            buffer.as_ptr().cast::<MIDIPacketList>(),
                        )
                    };
                    if status != 0 {
                        return Err(core_midi_status_error(
                            "destack.midi.output.write",
                            "MIDISend",
                            status,
                        ));
                    }
                }
            }
            CoreMidiOutputSessionKind::VirtualSource { endpoint } => {
                if record.data_format == MidiDataFormat::Ump {
                    let buffer = modern_event_buffer(record)?;
                    let status = unsafe {
                        MIDIReceivedEventList(*endpoint, buffer.as_ptr().cast::<MIDIEventList>())
                    };
                    if status != 0 {
                        return Err(core_midi_status_error(
                            "destack.midi.output.write",
                            "MIDIReceivedEventList",
                            status,
                        ));
                    }
                } else {
                    let buffer = legacy_packet_buffer(record)?;
                    let status = unsafe {
                        MIDIReceived(*endpoint, buffer.as_ptr().cast::<MIDIPacketList>())
                    };
                    if status != 0 {
                        return Err(core_midi_status_error(
                            "destack.midi.output.write",
                            "MIDIReceived",
                            status,
                        ));
                    }
                }
            }
        }
    }

    Ok(records.len() as u32)
}

/// Flush one CoreMIDI output session.
pub(crate) fn midi_output_flush(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let session = output_resource(binding, handle, "destack.midi.output.flush")?;

    match &session.kind {
        CoreMidiOutputSessionKind::Destination { destination, .. } => {
            let status = unsafe { MIDIFlushOutput(*destination) };
            if status != 0 {
                return Err(core_midi_status_error(
                    "destack.midi.output.flush",
                    "MIDIFlushOutput",
                    status,
                ));
            }
        }
        CoreMidiOutputSessionKind::VirtualSource { .. } => {}
    }

    Ok(())
}

/// Create one CoreMIDI virtual output session.
pub(crate) fn midi_output_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    let _runtime_state = binding.agent().platform_state.midi.runtime_state(binding);

    let name = native_string(options.name)?;
    let manufacturer = native_optional_string(options.manufacturer)?;
    let model = native_optional_string(options.model)?;
    let version = native_optional_string(options.version)?;

    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.output.virtual.create",
    )?;
    let service = core_midi_service("destack.midi.output.virtual.create")?;

    validate_record_shape(
        "destack.midi.output.virtual.create",
        options.data_format,
        Some(options.protocol),
    )?;

    let Some(name) = create_cf_string(&name) else {
        return Err(core_platform::io_operation_error(
            "destack.midi.output.virtual.create",
            None,
            "failed to encode one CoreMIDI virtual source name",
        ));
    };

    let mut endpoint = 0u32;

    let status = if options.data_format == MidiDataFormat::Ump {
        unsafe {
            MIDISourceCreateWithProtocol(
                service.operation_client,
                name,
                selected_protocol_id(Some(options.protocol), options.data_format),
                &mut endpoint,
            )
        }
    } else {
        unsafe { MIDISourceCreate(service.operation_client, name, &mut endpoint) }
    };
    release_cf(name.cast());
    if status != 0 || endpoint == 0 {
        return Err(core_midi_status_error(
            "destack.midi.output.virtual.create",
            "MIDISourceCreate",
            status,
        ));
    }

    if let Some(manufacturer) = manufacturer.as_ref() {
        if let Some(value) = create_cf_string(manufacturer) {
            unsafe {
                let _ = MIDIObjectSetStringProperty(endpoint, kMIDIPropertyManufacturer, value);
            }
            release_cf(value.cast());
        }
    }
    if let Some(model) = model.as_ref() {
        if let Some(value) = create_cf_string(model) {
            unsafe {
                let _ = MIDIObjectSetStringProperty(endpoint, kMIDIPropertyModel, value);
            }
            release_cf(value.cast());
        }
    }
    if let Some(version) = version.as_ref()
        && let Ok(version_value) = version.parse::<i32>()
    {
        unsafe {
            let _ =
                MIDIObjectSetIntegerProperty(endpoint, kMIDIPropertyDriverVersion, version_value);
        }
    }

    register_endpoint_override(endpoint, options.data_format, options.protocol);

    let descriptor = endpoint_descriptor(MidiPortDirection::Output, endpoint);
    let session = Arc::new(CoreMidiOutputSession {
        _service: service,
        descriptor,
        data_format: options.data_format,
        protocol: Some(options.protocol),
        kind: CoreMidiOutputSessionKind::VirtualSource { endpoint },
    });
    let handle = insert_output_resource(binding, session.clone());

    refresh_native_event_sessions(
        &session._service.native_event_registry,
        MidiEventSource::Native,
    );

    Ok(handle)
}
