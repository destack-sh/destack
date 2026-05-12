use std::ffi::{c_int, c_void};
use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::BoundedQueue;
use crate::platform::device::midi::core::{
    MidiInputRecordValue, MidiPortDescriptorValue, MidiRecordBytes, binding_timestamp_now,
    input_queue_capacity, read_queued_batch, read_queued_item, remove_midi_input_resource,
    resolve_descriptor_open_transport, surface_terminal_error, try_read_queued_batch,
    try_read_queued_item, validate_record_shape,
};
use crate::platform::device::{
    MidiDataFormat, MidiInputPortOpenOptions, MidiPortDirection, MidiPortListOptions, MidiProtocol,
    MidiRecordFraming, MidiVirtualInputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::core::{
    JackInputCallbackContext, JackInputRepository, JackInputRepositoryKind, JackInputTerminalError,
    activate_client, connect_ports, insert_input_resource, internal_input_client_name,
    jack_event_time_to_mono_ns, native_optional_string, native_string, open_jack_client, port_name,
    register_input_port, release_input_callback_context, retain_input_callback_context,
};
use super::descriptor::{filtered_descriptors, resolve_endpoint, virtual_input_descriptor};
use super::resource::input_resource;
use super::service::refresh_local_native_event_sessions;

/// Install one input process callback and shutdown hook.
fn install_input_callbacks(
    client: &super::core::JackClientHandle,
    callback_context_token: usize,
    operation: &'static str,
) -> RuntimeResult<()> {
    let callback_argument = callback_context_token as *mut c_void;

    // process callback
    let process_status = unsafe {
        (client.library.api.jack_set_process_callback)(
            client.raw,
            Some(input_process_callback),
            callback_argument,
        )
    };
    if process_status != 0 {
        return Err(super::core::jack_error(
            operation,
            format!("failed to install JACK input process callback (status {process_status})"),
        ));
    }

    // shutdown callback
    unsafe {
        (client.library.api.jack_on_shutdown)(
            client.raw,
            Some(input_shutdown_callback),
            callback_argument,
        );
    }

    Ok(())
}

/// Handle one JACK input process callback.
unsafe extern "C" fn input_process_callback(frame_count: u32, argument: *mut c_void) -> c_int {
    let context = argument.cast::<JackInputCallbackContext>();
    if context.is_null() {
        return 0;
    }

    let context = unsafe { &*context };
    if context.port.is_null() {
        return 0;
    }

    // input buffer
    let buffer = unsafe { (context.library.api.jack_port_get_buffer)(context.port, frame_count) };
    if buffer.is_null() {
        return 0;
    }

    let event_count = unsafe { (context.library.api.jack_midi_get_event_count)(buffer) };

    // queued events
    for index in 0..event_count {
        let mut event = super::abi::JackMidiEvent {
            time: 0,
            size: 0,
            buffer: std::ptr::null_mut(),
        };
        let status =
            unsafe { (context.library.api.jack_midi_event_get)(&mut event, buffer, index) };
        if status != 0 || event.size == 0 || event.buffer.is_null() {
            continue;
        }

        let data = MidiRecordBytes::from_slice(unsafe {
            std::slice::from_raw_parts(event.buffer, event.size)
        });
        context.queue.push_drop_oldest(MidiInputRecordValue {
            received_at_ns: jack_event_time_to_mono_ns(context, frame_count, event.time),
            source_id: context.source_id.clone(),
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data,
        });
    }

    0
}

/// Handle one JACK input shutdown callback.
unsafe extern "C" fn input_shutdown_callback(argument: *mut c_void) {
    let context = argument.cast::<JackInputCallbackContext>();
    if context.is_null() {
        return;
    }

    let context = unsafe { &*context };

    // terminal failure
    {
        let mut terminal_error = context.terminal_error.lock();
        *terminal_error = Some(JackInputTerminalError::BackendDisconnected);
    }

    context.queue.close();
}

/// List JACK MIDI input endpoints.
pub(crate) fn midi_input_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let service = binding
        .worker()
        .platform_state
        .device
        .jack_service("destack.device.midi.input.port.list")?;

    Ok(filtered_descriptors(
        &service,
        MidiPortDirection::Input,
        options.flags,
    ))
}

/// Open one JACK MIDI input endpoint.
pub(crate) fn midi_input_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiInputPortOpenOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    let service = binding
        .worker()
        .platform_state
        .device
        .jack_service("destack.device.midi.input.port.open")?;
    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Input,
        id,
        "destack.device.midi.input.port.open",
    )?;

    // transport selection
    let (_data_format, _protocol) = resolve_descriptor_open_transport(
        "destack.device.midi.input.port.open",
        &endpoint.descriptor,
        options.data_format,
        options.protocol,
        MidiDataFormat::Midi1Bytes,
        Some(MidiProtocol::Midi1),
    )?;

    let queue = Arc::new(BoundedQueue::new(input_queue_capacity(
        options.queue_capacity,
    )));
    let terminal_error = Arc::new(parking_lot::Mutex::new(None));

    // client and local port
    let client = open_jack_client(
        service.library.clone(),
        "destack.device.midi.input.port.open",
        &internal_input_client_name(),
    )?;
    let open_epoch_ns = binding_timestamp_now();
    let open_jack_time_usecs = unsafe { (client.library.api.jack_get_time)() };
    let port = register_input_port(&client, "input", "destack.device.midi.input.port.open")?;
    let context = Arc::new(JackInputCallbackContext {
        library: client.library.clone(),
        client: client.raw,
        port,
        open_epoch_ns,
        open_jack_time_usecs,
        queue: queue.clone(),
        source_id: Some(Arc::<str>::from(endpoint.descriptor.id.clone())),
        terminal_error: terminal_error.clone(),
    });
    let callback_context_token = retain_input_callback_context(&context);

    // callback install and activation
    if let Err(error) = install_input_callbacks(
        &client,
        callback_context_token,
        "destack.device.midi.input.port.open",
    ) {
        release_input_callback_context(callback_context_token);
        return Err(error);
    }

    if let Err(error) = activate_client(&client, "destack.device.midi.input.port.open") {
        release_input_callback_context(callback_context_token);
        return Err(error);
    }

    // source connection
    let local_port_name = port_name(&client, port, "destack.device.midi.input.port.open")?;
    if let Err(error) = connect_ports(
        &client,
        &endpoint.backend_port_name,
        &local_port_name,
        "destack.device.midi.input.port.open",
    ) {
        release_input_callback_context(callback_context_token);
        return Err(error);
    }

    let session = Arc::new(JackInputRepository {
        _service: service,
        descriptor: endpoint.descriptor,
        queue,
        terminal_error,
        kind: parking_lot::Mutex::new(JackInputRepositoryKind::Source {
            client,
            _port: port,
            _context_owner: context,
            callback_context_token,
        }),
    });

    Ok(insert_input_resource(binding, session))
}

/// Describe one opened JACK MIDI input endpoint.
pub(crate) fn midi_input_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let session = input_resource(binding, handle, "destack.device.midi.input.port.descriptor")?;

    Ok(session.descriptor.clone())
}

/// Close one opened JACK MIDI input endpoint.
pub(crate) fn midi_input_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    let session = input_resource(binding, handle, "destack.device.midi.input.port.close")?;
    let service = session._service.clone();
    let is_virtual_endpoint = session.is_virtual_endpoint();

    remove_midi_input_resource(
        binding,
        handle.0,
        "destack.device.midi.input.port.close",
        "midi input",
    )?;

    // local virtual endpoints change visible topology immediately
    if is_virtual_endpoint {
        refresh_local_native_event_sessions(&service);
    }

    Ok(())
}

/// Wait for one JACK MIDI input record.
pub(crate) fn midi_input_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiInputRecordValue> {
    let session = input_resource(binding, handle, "destack.device.midi.input.read")?;

    read_queued_item(
        &session.queue,
        timeout_ns,
        "destack.device.midi.input.read",
        "no queued MIDI input record is available",
        || {
            surface_terminal_error(
                "destack.device.midi.input.read",
                &session.terminal_error,
                |terminal_error| terminal_error.message().to_string(),
            )
        },
    )
}

/// Wait for one batch of JACK MIDI input records.
pub(crate) fn midi_input_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    max_records: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    let session = input_resource(binding, handle, "destack.device.midi.input.readBatch")?;

    read_queued_batch(
        &session.queue,
        max_records as usize,
        timeout_ns,
        "destack.device.midi.input.readBatch",
        "no queued MIDI input records are available",
        || {
            surface_terminal_error(
                "destack.device.midi.input.readBatch",
                &session.terminal_error,
                |terminal_error| terminal_error.message().to_string(),
            )
        },
    )
}

/// Poll one JACK MIDI input record without blocking.
pub(crate) fn midi_input_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiInputRecordValue> {
    let session = input_resource(binding, handle, "destack.device.midi.input.tryRead")?;

    try_read_queued_item(
        &session.queue,
        "destack.device.midi.input.tryRead",
        "no queued MIDI input record is available",
        || {
            surface_terminal_error(
                "destack.device.midi.input.tryRead",
                &session.terminal_error,
                |terminal_error| terminal_error.message().to_string(),
            )
        },
    )
}

/// Poll one batch of JACK MIDI input records without blocking.
pub(crate) fn midi_input_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    max_records: u32,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    let session = input_resource(binding, handle, "destack.device.midi.input.tryReadBatch")?;

    try_read_queued_batch(
        &session.queue,
        max_records as usize,
        "destack.device.midi.input.tryReadBatch",
        "no queued MIDI input records are available",
        || {
            surface_terminal_error(
                "destack.device.midi.input.tryReadBatch",
                &session.terminal_error,
                |terminal_error| terminal_error.message().to_string(),
            )
        },
    )
}

/// Create one virtual JACK MIDI input endpoint.
pub(crate) fn midi_input_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    // metadata decode
    let name = native_string(options.name)?;
    let _manufacturer = native_optional_string(options.manufacturer)?;
    let _model = native_optional_string(options.model)?;
    let _version = native_optional_string(options.version)?;

    // transport selection
    validate_record_shape(
        "destack.device.midi.input.virtual.create",
        options.data_format,
        Some(options.protocol),
    )?;

    let service = binding
        .worker()
        .platform_state
        .device
        .jack_service("destack.device.midi.input.virtual.create")?;
    let queue = Arc::new(BoundedQueue::new(input_queue_capacity(
        options.queue_capacity,
    )));
    let terminal_error = Arc::new(parking_lot::Mutex::new(None));

    // client and visible destination port
    let client = open_jack_client(
        service.library.clone(),
        "destack.device.midi.input.virtual.create",
        &name,
    )?;
    let open_epoch_ns = binding_timestamp_now();
    let open_jack_time_usecs = unsafe { (client.library.api.jack_get_time)() };
    let port = register_input_port(&client, &name, "destack.device.midi.input.virtual.create")?;
    let context = Arc::new(JackInputCallbackContext {
        library: client.library.clone(),
        client: client.raw,
        port,
        open_epoch_ns,
        open_jack_time_usecs,
        queue: queue.clone(),
        source_id: None,
        terminal_error: terminal_error.clone(),
    });
    let callback_context_token = retain_input_callback_context(&context);

    // callback install and activation
    if let Err(error) = install_input_callbacks(
        &client,
        callback_context_token,
        "destack.device.midi.input.virtual.create",
    ) {
        release_input_callback_context(callback_context_token);
        return Err(error);
    }

    if let Err(error) = activate_client(&client, "destack.device.midi.input.virtual.create") {
        release_input_callback_context(callback_context_token);
        return Err(error);
    }

    let backend_port_name = port_name(&client, port, "destack.device.midi.input.virtual.create")?;
    let descriptor =
        virtual_input_descriptor(&backend_port_name, options.data_format, options.protocol);
    let session = Arc::new(JackInputRepository {
        _service: service,
        descriptor,
        queue,
        terminal_error,
        kind: parking_lot::Mutex::new(JackInputRepositoryKind::VirtualDestination {
            client,
            _port: port,
            _context_owner: context,
            callback_context_token,
        }),
    });
    let handle = insert_input_resource(binding, session.clone());

    refresh_local_native_event_sessions(&session._service);

    Ok(handle)
}
