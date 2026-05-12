use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform, BoundedQueue};
use crate::platform::device::midi::core::{
    MidiInputRecordValue, MidiPortDescriptorValue, input_queue_capacity, read_queued_batch,
    read_queued_item, remove_midi_input_resource, resolve_descriptor_open_transport,
    surface_terminal_error, try_read_queued_batch, try_read_queued_item, validate_record_shape,
};
use crate::platform::device::{
    MidiDataFormat, MidiEventSource, MidiInputPortOpenOptions, MidiPortDirection,
    MidiPortListOptions, MidiProtocol, MidiVirtualInputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::{BindingCallContext, ExecutionMode, ExecutionPolicy, start_with_policy};

use super::abi::{POLLIN, SND_SEQ_OPEN_DUPLEX, poll, pollfd, snd_seq_event_t};
use super::core::{
    AlsaInputRepository, AlsaInputRepositoryKind, AlsaInputTerminalError, alsa_record_framing,
    create_midi_parser, create_queue, create_simple_port, default_port_type,
    enable_port_realtime_timestamps, hidden_destination_port_capability,
    input_event_received_at_ns, insert_input_resource, native_optional_string, native_string,
    open_sequencer_handle, subscribe_from_with_timestamps, virtual_destination_port_capability,
};
use super::descriptor::{
    endpoint_address, filtered_descriptors, resolve_endpoint, virtual_input_descriptor,
};
use super::event::refresh_native_event_sessions;
use super::resource::input_resource;
/// List ALSA sequencer input endpoints.
pub(crate) fn midi_input_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let service = binding
        .worker()
        .platform_state
        .device
        .alsa_service("destack.device.midi.input.port.list")?;

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
    let service = binding
        .worker()
        .platform_state
        .device
        .alsa_service("destack.device.midi.input.port.open")?;
    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Input,
        id,
        "destack.device.midi.input.port.open",
    )?;

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
    let handle = open_sequencer_handle(
        &service.library,
        "Destack MIDI Internal Input Repository",
        SND_SEQ_OPEN_DUPLEX,
        "destack.device.midi.input.port.open",
    )?;
    let local_port_id = create_simple_port(
        &handle,
        "Destack MIDI Internal Input Repository",
        hidden_destination_port_capability(),
        default_port_type(),
        "destack.device.midi.input.port.open",
    )?;
    let queue_id = create_queue(
        &handle,
        "Destack MIDI Input Queue",
        "destack.device.midi.input.port.open",
    )?;
    let (remote_client_id, remote_port_id) =
        endpoint_address(&endpoint.descriptor, "destack.device.midi.input.port.open")?;
    subscribe_from_with_timestamps(
        &handle,
        local_port_id,
        remote_client_id,
        remote_port_id,
        queue_id.id,
    )?;

    let stop_flag = Arc::new(AtomicBool::new(false));
    let terminal_error = Arc::new(parking_lot::Mutex::new(None));
    let reader_thread = spawn_input_reader(
        handle.library.clone(),
        handle.raw as usize,
        queue_id.start_epoch_ns,
        Some(Arc::<str>::from(endpoint.descriptor.id.clone())),
        queue.clone(),
        terminal_error.clone(),
        stop_flag.clone(),
    )?;

    let session = Arc::new(AlsaInputRepository {
        _service: service,
        descriptor: endpoint.descriptor,
        queue,
        terminal_error,
        kind: parking_lot::Mutex::new(AlsaInputRepositoryKind::Source {
            handle,
            queue: queue_id,
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
    let session = input_resource(binding, handle, "destack.device.midi.input.port.descriptor")?;

    Ok(session.descriptor.clone())
}

/// Close one opened ALSA sequencer input endpoint.
pub(crate) fn midi_input_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    remove_midi_input_resource(
        binding,
        handle.0,
        "destack.device.midi.input.port.close",
        "midi input",
    )
}

/// Wait for one ALSA MIDI input record.
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
                |terminal_error| terminal_error.message(),
            )
        },
    )
}

/// Wait for one ALSA MIDI input record batch.
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
                |terminal_error| terminal_error.message(),
            )
        },
    )
}

/// Poll one ALSA MIDI input record without blocking.
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
                |terminal_error| terminal_error.message(),
            )
        },
    )
}

/// Poll one ALSA MIDI input record batch without blocking.
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
                |terminal_error| terminal_error.message(),
            )
        },
    )
}

/// Create one virtual ALSA sequencer input endpoint.
pub(crate) fn midi_input_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    let name = native_string(options.name)?;
    let _manufacturer = native_optional_string(options.manufacturer)?;
    let _model = native_optional_string(options.model)?;
    let _version = native_optional_string(options.version)?;

    let service = binding
        .worker()
        .platform_state
        .device
        .alsa_service("destack.device.midi.input.virtual.create")?;

    validate_record_shape(
        "destack.device.midi.input.virtual.create",
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
        "destack.device.midi.input.virtual.create",
    )?;
    let local_port_id = create_simple_port(
        &handle,
        &name,
        virtual_destination_port_capability(),
        default_port_type(),
        "destack.device.midi.input.virtual.create",
    )?;
    let queue_id = create_queue(
        &handle,
        "Destack MIDI Virtual Input Queue",
        "destack.device.midi.input.virtual.create",
    )?;
    enable_port_realtime_timestamps(
        &handle,
        local_port_id,
        queue_id.id,
        "destack.device.midi.input.virtual.create",
    )?;

    let stop_flag = Arc::new(AtomicBool::new(false));
    let terminal_error = Arc::new(parking_lot::Mutex::new(None));
    let reader_thread = spawn_input_reader(
        handle.library.clone(),
        handle.raw as usize,
        queue_id.start_epoch_ns,
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
    let session = Arc::new(AlsaInputRepository {
        _service: service,
        descriptor,
        queue,
        terminal_error,
        kind: parking_lot::Mutex::new(AlsaInputRepositoryKind::VirtualDestination {
            handle,
            queue: queue_id,
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
    queue_start_ns: u64,
    source_id: Option<Arc<str>>,
    queue: Arc<BoundedQueue<MidiInputRecordValue>>,
    terminal_error: Arc<parking_lot::Mutex<Option<AlsaInputTerminalError>>>,
    stop_flag: Arc<AtomicBool>,
) -> RuntimeResult<std::thread::JoinHandle<()>> {
    start_with_policy(
        "destack-midi-alsa-input",
        "destack.device.midi.input.reader",
        ExecutionPolicy::resource(ExecutionMode::Loop),
        move || {
            let raw_handle = raw_handle as *mut super::abi::snd_seq_t;
            let Some(raw_handle) = (!raw_handle.is_null()).then_some(raw_handle) else {
                return;
            };

            let mut is_inside_sysex = false;

            // parser creation
            let parser = create_midi_parser(&library, "destack.device.midi.input.reader");
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
                    let Some(received_at_ns) =
                        input_event_received_at_ns(unsafe { &*event }, queue_start_ns)
                    else {
                        unsafe {
                            (library.api.snd_seq_free_event)(event);
                        }

                        let mut terminal_error = terminal_error.lock();
                        *terminal_error = Some(AlsaInputTerminalError::MissingRealtimeTimestamp);
                        queue.close();
                        break;
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
                        received_at_ns,
                        source_id: source_id.clone(),
                        data_format: MidiDataFormat::Midi1Bytes,
                        protocol: Some(MidiProtocol::Midi1),
                        framing,
                        data: buffer.into(),
                    });
                }
            }

            queue.close();
        },
    )
}
