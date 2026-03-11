use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiInputRecordValue, MidiPortDescriptorValue, validate_record_shape,
};
use crate::platform::midi::{
    MidiDataFormat, MidiEventSource, MidiInputPortOpenOptions, MidiPortDirection,
    MidiPortListOptions, MidiProtocol, MidiVirtualInputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::abi::{POLLIN, SND_SEQ_OPEN_DUPLEX, poll, pollfd, snd_seq_event_t};
use super::backend::resolve_backend;
use super::core::{
    AlsaInputSession, AlsaInputSessionKind, AlsaInputTerminalError, BoundedQueue,
    alsa_record_framing, binding_timestamp_now, connect_from, create_midi_parser,
    create_simple_port, default_port_type, hidden_destination_port_capability,
    input_queue_capacity, insert_input_resource, native_optional_string, native_string,
    open_sequencer_handle, virtual_destination_port_capability,
};
use super::descriptor::{
    endpoint_address, filtered_descriptors, resolve_endpoint, validate_endpoint_transport_request,
    virtual_input_descriptor,
};
use super::event::refresh_native_event_sessions;
use super::resource::{input_resource, remove_input_resource};

/// Surface one deferred ALSA reader failure after the queue drains.
fn surface_reader_failure(
    session: &AlsaInputSession,
    operation: &'static str,
) -> RuntimeResult<()> {
    let terminal_error = session.terminal_error.lock();
    let Some(terminal_error) = terminal_error.as_ref() else {
        return Ok(());
    };

    Err(core_platform::io_operation_error(
        operation,
        None,
        terminal_error.message(),
    ))
}
/// List ALSA sequencer input endpoints.
pub(crate) fn midi_input_port_list(
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
        "destack.midi.input.port.list",
    )?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .alsa_service("destack.midi.input.port.list")?;

    Ok(filtered_descriptors(
        &service,
        MidiPortDirection::Input,
        options.flags,
    ))
}

/// Open one ALSA sequencer input endpoint.
pub(crate) fn midi_input_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiInputPortOpenOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    binding
        .agent()
        .platform_state
        .midi
        .mark_runtime_active(binding);

    let _backend = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.input.port.open",
    )?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .alsa_service("destack.midi.input.port.open")?;
    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Input,
        id,
        "destack.midi.input.port.open",
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

    validate_record_shape("destack.midi.input.port.open", data_format, Some(protocol))?;
    validate_endpoint_transport_request(
        "destack.midi.input.port.open",
        &endpoint.descriptor,
        data_format,
        Some(protocol),
    )?;

    let queue = Arc::new(BoundedQueue::new(input_queue_capacity(
        options.queue_capacity,
    )));
    let handle = open_sequencer_handle(
        &service.library,
        "Destack MIDI Internal Input Session",
        SND_SEQ_OPEN_DUPLEX,
        "destack.midi.input.port.open",
    )?;
    let local_port_id = create_simple_port(
        &handle,
        "Destack MIDI Internal Input Session",
        hidden_destination_port_capability(),
        default_port_type(),
        "destack.midi.input.port.open",
    )?;
    let (remote_client_id, remote_port_id) =
        endpoint_address(&endpoint.descriptor, "destack.midi.input.port.open")?;
    connect_from(&handle, local_port_id, remote_client_id, remote_port_id)?;

    let stop_flag = Arc::new(AtomicBool::new(false));
    let terminal_error = Arc::new(parking_lot::Mutex::new(None));
    let reader_thread = spawn_input_reader(
        handle.library.clone(),
        handle.raw as usize,
        Some(endpoint.descriptor.id.clone()),
        queue.clone(),
        terminal_error.clone(),
        stop_flag.clone(),
    )?;

    let session = Arc::new(AlsaInputSession {
        _service: service,
        descriptor: endpoint.descriptor,
        queue,
        terminal_error,
        kind: parking_lot::Mutex::new(AlsaInputSessionKind::Source {
            handle,
            local_port_id,
            remote_client_id,
            remote_port_id,
            stop_flag,
            reader_thread: Some(reader_thread),
        }),
    });

    Ok(insert_input_resource(binding, session))
}

/// Describe one opened ALSA sequencer input endpoint.
pub(crate) fn midi_input_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let session = input_resource(binding, handle, "destack.midi.input.port.descriptor")?;

    Ok(session.descriptor.clone())
}

/// Close one opened ALSA sequencer input endpoint.
pub(crate) fn midi_input_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    remove_input_resource(binding, handle, "destack.midi.input.port.close")
}

/// Wait for one ALSA MIDI input record.
pub(crate) fn midi_input_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    timeout_ns: u64,
) -> RuntimeResult<MidiInputRecordValue> {
    let session = input_resource(binding, handle, "destack.midi.input.read")?;

    // queued record
    let record = session
        .queue
        .pop_with_timeout(std::time::Duration::from_nanos(timeout_ns));
    if let Some(record) = record {
        return Ok(record);
    }

    // deferred backend failure
    surface_reader_failure(&session, "destack.midi.input.read")?;

    Err(core_platform::io_would_block(
        "destack.midi.input.read",
        "no queued MIDI input record is available",
    ))
}

/// Wait for one ALSA MIDI input record batch.
pub(crate) fn midi_input_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    max_records: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    let session = input_resource(binding, handle, "destack.midi.input.readBatch")?;

    let records = session.queue.pop_batch_with_timeout(
        max_records as usize,
        std::time::Duration::from_nanos(timeout_ns),
    );
    if !records.is_empty() {
        return Ok(records);
    }

    // deferred backend failure
    surface_reader_failure(&session, "destack.midi.input.readBatch")?;

    Err(core_platform::io_would_block(
        "destack.midi.input.readBatch",
        "no queued MIDI input records are available",
    ))
}

/// Poll one ALSA MIDI input record without blocking.
pub(crate) fn midi_input_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiInputRecordValue> {
    let session = input_resource(binding, handle, "destack.midi.input.tryRead")?;

    // queued record
    let record = session.queue.try_pop();
    if let Some(record) = record {
        return Ok(record);
    }

    // deferred backend failure
    surface_reader_failure(&session, "destack.midi.input.tryRead")?;

    Err(core_platform::io_would_block(
        "destack.midi.input.tryRead",
        "no queued MIDI input record is available",
    ))
}

/// Poll one ALSA MIDI input record batch without blocking.
pub(crate) fn midi_input_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    max_records: u32,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    let session = input_resource(binding, handle, "destack.midi.input.tryReadBatch")?;
    let records = session.queue.try_pop_batch(max_records as usize);
    if !records.is_empty() {
        return Ok(records);
    }

    // deferred backend failure
    surface_reader_failure(&session, "destack.midi.input.tryReadBatch")?;

    Err(core_platform::io_would_block(
        "destack.midi.input.tryReadBatch",
        "no queued MIDI input records are available",
    ))
}

/// Create one virtual ALSA sequencer input endpoint.
pub(crate) fn midi_input_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
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
        "destack.midi.input.virtual.create",
    )?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .alsa_service("destack.midi.input.virtual.create")?;

    validate_record_shape(
        "destack.midi.input.virtual.create",
        options.data_format,
        Some(options.protocol),
    )?;

    let queue = Arc::new(BoundedQueue::new(input_queue_capacity(
        options.queue_capacity,
    )));
    let handle = open_sequencer_handle(
        &service.library,
        &name,
        SND_SEQ_OPEN_DUPLEX,
        "destack.midi.input.virtual.create",
    )?;
    let local_port_id = create_simple_port(
        &handle,
        &name,
        virtual_destination_port_capability(),
        default_port_type(),
        "destack.midi.input.virtual.create",
    )?;

    let stop_flag = Arc::new(AtomicBool::new(false));
    let terminal_error = Arc::new(parking_lot::Mutex::new(None));
    let reader_thread = spawn_input_reader(
        handle.library.clone(),
        handle.raw as usize,
        None,
        queue.clone(),
        terminal_error.clone(),
        stop_flag.clone(),
    )?;

    let descriptor = virtual_input_descriptor(
        name,
        handle.client_id,
        local_port_id,
        options.data_format,
        options.protocol,
    );
    let session = Arc::new(AlsaInputSession {
        _service: service,
        descriptor,
        queue,
        terminal_error,
        kind: parking_lot::Mutex::new(AlsaInputSessionKind::VirtualDestination {
            handle,
            local_port_id,
            stop_flag,
            reader_thread: Some(reader_thread),
        }),
    });
    let handle = insert_input_resource(binding, session.clone());

    refresh_native_event_sessions(
        &session._service.topology,
        &session._service.native_event_registry,
        MidiEventSource::Native,
    );

    Ok(handle)
}

/// Spawn one ALSA reader thread for one opened input session.
fn spawn_input_reader(
    library: Arc<super::core::AlsaLibrary>,
    raw_handle: usize,
    source_id: Option<String>,
    queue: Arc<BoundedQueue<MidiInputRecordValue>>,
    terminal_error: Arc<parking_lot::Mutex<Option<AlsaInputTerminalError>>>,
    stop_flag: Arc<AtomicBool>,
) -> RuntimeResult<std::thread::JoinHandle<()>> {
    let builder = thread::Builder::new().name("destack-midi-alsa-input".to_string());

    builder
        .spawn(move || {
            let raw_handle = raw_handle as *mut super::abi::snd_seq_t;
            let Some(raw_handle) = (!raw_handle.is_null()).then_some(raw_handle) else {
                return;
            };

            let mut is_inside_sysex = false;

            // parser creation
            let parser = create_midi_parser(&library, "destack.midi.input.reader");
            let Ok(parser) = parser else {
                let mut terminal_error = terminal_error.lock();
                *terminal_error = Some(AlsaInputTerminalError::ParserInitializationFailed);
                queue.close();
                return;
            };

            // poll descriptors
            let mut poll_fds = {
                let count =
                    unsafe { (library.api.snd_seq_poll_descriptors_count)(raw_handle, POLLIN) };
                if count <= 0 {
                    let mut terminal_error = terminal_error.lock();
                    *terminal_error = Some(AlsaInputTerminalError::PollDescriptorUnavailable);
                    queue.close();
                    return;
                }

                let mut poll_fds = vec![pollfd::default(); count as usize];
                let status = unsafe {
                    (library.api.snd_seq_poll_descriptors)(
                        raw_handle,
                        poll_fds.as_mut_ptr(),
                        poll_fds.len() as u32,
                        POLLIN,
                    )
                };
                if status < 0 {
                    let mut terminal_error = terminal_error.lock();
                    *terminal_error = Some(AlsaInputTerminalError::PollDescriptorQueryFailed);
                    queue.close();
                    return;
                }

                poll_fds
            };

            loop {
                // stop request
                if stop_flag.load(Ordering::Acquire) {
                    break;
                }

                // wait for inbound midi events
                let poll_status = unsafe { poll(poll_fds.as_mut_ptr(), poll_fds.len() as _, 100) };
                if poll_status <= 0 {
                    continue;
                }

                // drain all queued events
                loop {
                    let mut event = std::ptr::null_mut::<snd_seq_event_t>();
                    let status =
                        unsafe { (library.api.snd_seq_event_input)(raw_handle, &mut event) };
                    if status == -11 {
                        break;
                    }

                    // reader failure
                    if status < 0 {
                        let error = super::core::alsa_error_string(&library, status);
                        let mut terminal_error = terminal_error.lock();
                        *terminal_error = Some(AlsaInputTerminalError::SequencerInputFailed(error));
                        break;
                    }

                    if event.is_null() {
                        break;
                    }

                    let length = unsafe { (library.api.snd_seq_event_length)(event) };
                    let buffer_len = length.max(4096) as usize;
                    let mut buffer = vec![0u8; buffer_len];
                    let decoded = unsafe {
                        (library.api.snd_midi_event_decode)(
                            parser.raw,
                            buffer.as_mut_ptr(),
                            buffer.len() as _,
                            event,
                        )
                    };

                    unsafe {
                        (library.api.snd_seq_free_event)(event);
                    }

                    if decoded <= 0 {
                        continue;
                    }

                    buffer.truncate(decoded as usize);
                    let (framing, next_is_inside_sysex) =
                        alsa_record_framing(&buffer, is_inside_sysex);
                    is_inside_sysex = next_is_inside_sysex;

                    queue.push_drop_oldest(MidiInputRecordValue {
                        received_at_ns: binding_timestamp_now(),
                        source_id: source_id.clone(),
                        data_format: MidiDataFormat::Midi1Bytes,
                        protocol: Some(MidiProtocol::Midi1),
                        framing,
                        data: buffer,
                    });
                }
            }

            queue.close();
        })
        .map_err(|error| {
            core_platform::io_operation_error(
                "destack.midi.input.reader",
                None,
                format!("failed to spawn ALSA input reader thread: {error}"),
            )
        })
}
