use std::ffi::{c_int, c_void};
use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::device::midi::core::{
    MidiOutputRecordValue, MidiPortDescriptorValue, remove_midi_output_resource,
    resolve_descriptor_open_transport, surface_terminal_error, validate_output_record_payload,
    validate_record_shape,
};
use crate::platform::device::{
    MidiDataFormat, MidiOutputPortOpenOptions, MidiPortDirection, MidiPortListOptions,
    MidiProtocol, MidiVirtualOutputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::core::{
    JackOutputCallbackContext, JackOutputRepository, JackOutputRepositoryKind,
    JackOutputTerminalError, activate_client, connect_ports, insert_output_resource,
    internal_output_client_name, native_optional_string, native_string, open_jack_client,
    port_name, register_output_port, release_output_callback_context,
    retain_output_callback_context,
};
use super::descriptor::{filtered_descriptors, resolve_endpoint, virtual_output_descriptor};
use super::resource::output_resource;
use super::service::refresh_local_native_event_sessions;

/// Install one output process callback and shutdown hook.
fn install_output_callbacks(
    client: &super::core::JackClientHandle,
    callback_context_token: usize,
    operation: &'static str,
) -> RuntimeResult<()> {
    let callback_argument = callback_context_token as *mut c_void;

    // process callback
    let process_status = unsafe {
        (client.library.api.jack_set_process_callback)(
            client.raw,
            Some(output_process_callback),
            callback_argument,
        )
    };
    if process_status != 0 {
        return Err(super::core::jack_error(
            operation,
            format!("failed to install JACK output process callback (status {process_status})"),
        ));
    }

    // shutdown callback
    unsafe {
        (client.library.api.jack_on_shutdown)(
            client.raw,
            Some(output_shutdown_callback),
            callback_argument,
        );
    }

    Ok(())
}

/// Handle one JACK output process callback.
unsafe extern "C" fn output_process_callback(frame_count: u32, argument: *mut c_void) -> c_int {
    let context = argument.cast::<JackOutputCallbackContext>();
    if context.is_null() {
        return 0;
    }

    let context = unsafe { &*context };
    if context.port.is_null() {
        return 0;
    }

    // output buffer
    let buffer = unsafe { (context.library.api.jack_port_get_buffer)(context.port, frame_count) };
    if buffer.is_null() {
        return 0;
    }

    unsafe {
        (context.library.api.jack_midi_clear_buffer)(buffer);
    }

    // pending records
    let mut pending_records = context.pending_records.lock();
    while let Some(record) = pending_records.front() {
        let status = unsafe {
            (context.library.api.jack_midi_event_write)(
                buffer,
                0,
                record.data.as_ptr(),
                record.data.len(),
            )
        };
        if status != 0 {
            break;
        }

        pending_records.pop_front();
    }

    0
}

/// Handle one JACK output shutdown callback.
unsafe extern "C" fn output_shutdown_callback(argument: *mut c_void) {
    let context = argument.cast::<JackOutputCallbackContext>();
    if context.is_null() {
        return;
    }

    let context = unsafe { &*context };

    // terminal failure
    let mut terminal_error = context.terminal_error.lock();
    *terminal_error = Some(JackOutputTerminalError::BackendDisconnected);
}

/// List JACK MIDI output endpoints.
pub(crate) fn midi_output_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let service = binding
        .worker()
        .platform_state
        .device
        .jack_service("destack.device.midi.output.port.list")?;

    Ok(filtered_descriptors(
        &service,
        MidiPortDirection::Output,
        options.flags,
    ))
}

/// Open one JACK MIDI output endpoint.
pub(crate) fn midi_output_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiOutputPortOpenOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    let service = binding
        .worker()
        .platform_state
        .device
        .jack_service("destack.device.midi.output.port.open")?;
    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Output,
        id,
        "destack.device.midi.output.port.open",
    )?;

    // transport selection
    let (data_format, protocol) = resolve_descriptor_open_transport(
        "destack.device.midi.output.port.open",
        &endpoint.descriptor,
        options.data_format,
        options.protocol,
        MidiDataFormat::Midi1Bytes,
        Some(MidiProtocol::Midi1),
    )?;

    let pending_records = Arc::new(parking_lot::Mutex::new(std::collections::VecDeque::new()));
    let terminal_error = Arc::new(parking_lot::Mutex::new(None));

    // client and local port
    let client = open_jack_client(
        service.library.clone(),
        "destack.device.midi.output.port.open",
        &internal_output_client_name(),
    )?;
    let port = register_output_port(&client, "output", "destack.device.midi.output.port.open")?;
    let context = Arc::new(JackOutputCallbackContext {
        library: client.library.clone(),
        port,
        pending_records: pending_records.clone(),
        terminal_error: terminal_error.clone(),
    });
    let callback_context_token = retain_output_callback_context(&context);

    // callback install and activation
    if let Err(error) = install_output_callbacks(
        &client,
        callback_context_token,
        "destack.device.midi.output.port.open",
    ) {
        release_output_callback_context(callback_context_token);
        return Err(error);
    }

    if let Err(error) = activate_client(&client, "destack.device.midi.output.port.open") {
        release_output_callback_context(callback_context_token);
        return Err(error);
    }

    // destination connection
    let local_port_name = port_name(&client, port, "destack.device.midi.output.port.open")?;
    if let Err(error) = connect_ports(
        &client,
        &local_port_name,
        &endpoint.backend_port_name,
        "destack.device.midi.output.port.open",
    ) {
        release_output_callback_context(callback_context_token);
        return Err(error);
    }

    let session = Arc::new(JackOutputRepository {
        _service: service,
        descriptor: endpoint.descriptor,
        data_format,
        protocol,
        pending_records,
        terminal_error,
        kind: parking_lot::Mutex::new(JackOutputRepositoryKind::Destination {
            client,
            _port: port,
            _context_owner: context,
            callback_context_token,
        }),
    });

    Ok(insert_output_resource(binding, session))
}

/// Describe one opened JACK MIDI output endpoint.
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

/// Close one opened JACK MIDI output endpoint.
pub(crate) fn midi_output_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    let session = output_resource(binding, handle, "destack.device.midi.output.port.close")?;
    let service = session._service.clone();
    let is_virtual_endpoint = session.is_virtual_endpoint();

    remove_midi_output_resource(
        binding,
        handle.0,
        "destack.device.midi.output.port.close",
        "midi output",
    )?;

    // local virtual endpoints change visible topology immediately
    if is_virtual_endpoint {
        refresh_local_native_event_sessions(&service);
    }

    Ok(())
}

/// Write JACK MIDI output records.
pub(crate) fn midi_output_write(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    records: Vec<MidiOutputRecordValue>,
) -> RuntimeResult<u32> {
    let session = output_resource(binding, handle, "destack.device.midi.output.write")?;

    // deferred backend failure
    surface_terminal_error(
        "destack.device.midi.output.write",
        &session.terminal_error,
        |terminal_error| terminal_error.message().to_string(),
    )?;

    // transport validation
    for record in &records {
        validate_record_shape(
            "destack.device.midi.output.write",
            record.data_format,
            record.protocol,
        )?;
        validate_output_record_payload("destack.device.midi.output.write", record)?;

        if record.send_at_ns.is_some() {
            return Err(core_platform::invalid_argument(
                "records",
                "jack midi output does not support scheduled send timestamps",
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

    // enqueue records for the process callback
    let written_count = records.len() as u32;

    {
        let mut pending_records = session.pending_records.lock();
        pending_records.extend(records);
    }

    Ok(written_count)
}

/// Create one virtual JACK MIDI output endpoint.
pub(crate) fn midi_output_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    // metadata decode
    let name = native_string(options.name)?;
    let _manufacturer = native_optional_string(options.manufacturer)?;
    let _model = native_optional_string(options.model)?;
    let _version = native_optional_string(options.version)?;

    // transport selection
    validate_record_shape(
        "destack.device.midi.output.virtual.create",
        options.data_format,
        Some(options.protocol),
    )?;

    let service = binding
        .worker()
        .platform_state
        .device
        .jack_service("destack.device.midi.output.virtual.create")?;
    let pending_records = Arc::new(parking_lot::Mutex::new(std::collections::VecDeque::new()));
    let terminal_error = Arc::new(parking_lot::Mutex::new(None));

    // client and visible source port
    let client = open_jack_client(
        service.library.clone(),
        "destack.device.midi.output.virtual.create",
        &name,
    )?;
    let port = register_output_port(&client, &name, "destack.device.midi.output.virtual.create")?;
    let context = Arc::new(JackOutputCallbackContext {
        library: client.library.clone(),
        port,
        pending_records: pending_records.clone(),
        terminal_error: terminal_error.clone(),
    });
    let callback_context_token = retain_output_callback_context(&context);

    // callback install and activation
    if let Err(error) = install_output_callbacks(
        &client,
        callback_context_token,
        "destack.device.midi.output.virtual.create",
    ) {
        release_output_callback_context(callback_context_token);
        return Err(error);
    }

    if let Err(error) = activate_client(&client, "destack.device.midi.output.virtual.create") {
        release_output_callback_context(callback_context_token);
        return Err(error);
    }

    let backend_port_name = port_name(&client, port, "destack.device.midi.output.virtual.create")?;
    let descriptor =
        virtual_output_descriptor(&backend_port_name, options.data_format, options.protocol);
    let session = Arc::new(JackOutputRepository {
        _service: service,
        descriptor,
        data_format: options.data_format,
        protocol: Some(options.protocol),
        pending_records,
        terminal_error,
        kind: parking_lot::Mutex::new(JackOutputRepositoryKind::VirtualSource {
            client,
            _port: port,
            _context_owner: context,
            callback_context_token,
        }),
    });
    let handle = insert_output_resource(binding, session.clone());

    refresh_local_native_event_sessions(&session._service);

    Ok(handle)
}
