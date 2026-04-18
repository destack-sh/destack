use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeStringRef;
use crate::platform::core::{self as core_platform};
use crate::platform::device::midi::core::{
    MidiInputRecordValue, MidiPortDescriptorValue, input_queue_capacity, read_queued_batch,
    read_queued_item, remove_labeled_resource, resolve_descriptor_open_transport,
    surface_terminal_error, try_read_queued_batch, try_read_queued_item,
};
use crate::platform::device::{
    MidiDataFormat, MidiInputPortOpenOptions, MidiPortDirection, MidiPortListOptions, MidiProtocol,
    MidiVirtualInputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;
use crate::runtime::control::queue::BoundedQueue;

use super::core::{WindowsMidiInputRepository, insert_input_resource};
use super::descriptor::{filtered_descriptors, resolve_endpoint};
use super::resource::input_resource;

/// Decode one native string into one owned Rust string.
fn owned_native_string(value: NativeStringRef) -> RuntimeResult<String> {
    unsafe { value.as_str().map(str::to_owned) }
}

/// Decode one optional native string into one owned Rust string.
fn owned_optional_native_string(value: Option<NativeStringRef>) -> RuntimeResult<Option<String>> {
    value.map(owned_native_string).transpose()
}

/// List Windows MIDI input ports.
pub(crate) fn midi_input_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let service = binding
        .worker()
        .platform_state
        .device
        .windows_midi_service("destack.device.midi.input.port.list")?;

    Ok(filtered_descriptors(
        &service,
        MidiPortDirection::Input,
        options.flags,
    ))
}

/// Open one Windows MIDI input session.
pub(crate) fn midi_input_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiInputPortOpenOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    let service = binding
        .worker()
        .platform_state
        .device
        .windows_midi_service("destack.device.midi.input.port.open")?;

    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Input,
        id,
        "destack.device.midi.input.port.open",
    )?;
    let descriptor = endpoint.descriptor.clone();
    let (_data_format, protocol) = resolve_descriptor_open_transport(
        "destack.device.midi.input.port.open",
        &descriptor,
        options.data_format,
        options.protocol,
        MidiDataFormat::Ump,
        None,
    )?;

    // callback queue
    let queue = Arc::new(BoundedQueue::new(input_queue_capacity(
        options.queue_capacity,
    )));
    let terminal_error = Arc::new(Mutex::new(None));
    let host_session_id = service.open_input_session(
        endpoint.backend_id,
        descriptor.clone(),
        protocol,
        queue.clone(),
        terminal_error.clone(),
        "destack.device.midi.input.port.open",
    )?;

    let session = Arc::new(WindowsMidiInputRepository {
        _service: service,
        descriptor,
        host_session_id,
        queue,
        terminal_error,
    });

    Ok(insert_input_resource(binding, session))
}

/// Describe one opened Windows MIDI input session.
pub(crate) fn midi_input_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let session = input_resource(binding, handle, "destack.device.midi.input.port.descriptor")?;

    Ok(session.descriptor.clone())
}

/// Close one opened Windows MIDI input session.
pub(crate) fn midi_input_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    remove_labeled_resource(
        binding,
        handle.0,
        "destack.device.midi.input.port.close",
        "midi input port",
    )?;

    Ok(())
}

/// Wait for one Windows MIDI input record.
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
        "midi input queue is empty",
        || {
            surface_terminal_error(
                "destack.device.midi.input.read",
                &session.terminal_error,
                |message| message.clone(),
            )
        },
    )
}

/// Wait for one batch of Windows MIDI input records.
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
        "midi input queue is empty",
        || {
            surface_terminal_error(
                "destack.device.midi.input.readBatch",
                &session.terminal_error,
                |message| message.clone(),
            )
        },
    )
}

/// Poll one Windows MIDI input record.
pub(crate) fn midi_input_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiInputRecordValue> {
    let session = input_resource(binding, handle, "destack.device.midi.input.tryRead")?;

    try_read_queued_item(
        &session.queue,
        "destack.device.midi.input.tryRead",
        "midi input queue is empty",
        || {
            surface_terminal_error(
                "destack.device.midi.input.tryRead",
                &session.terminal_error,
                |message| message.clone(),
            )
        },
    )
}

/// Poll one batch of Windows MIDI input records.
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
        "midi input queue is empty",
        || {
            surface_terminal_error(
                "destack.device.midi.input.tryReadBatch",
                &session.terminal_error,
                |message| message.clone(),
            )
        },
    )
}

/// Reject virtual input creation on Windows MIDI.
pub(crate) fn midi_input_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    // requested transport
    validate_virtual_input_transport(options.data_format, options.protocol)?;

    // shared service and host strings
    let service = binding
        .worker()
        .platform_state
        .device
        .windows_midi_service("destack.device.midi.input.virtual.create")?;
    let name = owned_native_string(options.name)?;
    let manufacturer = owned_optional_native_string(options.manufacturer)?;
    let model = owned_optional_native_string(options.model)?;
    let version = owned_optional_native_string(options.version)?;

    // input queue
    let queue = Arc::new(BoundedQueue::new(input_queue_capacity(
        options.queue_capacity,
    )));
    let terminal_error = Arc::new(Mutex::new(None));
    let (host_session_id, descriptor) = service.create_virtual_input_session(
        name,
        manufacturer,
        model,
        version,
        options.protocol,
        queue.clone(),
        terminal_error.clone(),
        "destack.device.midi.input.virtual.create",
    )?;

    let session = Arc::new(WindowsMidiInputRepository {
        _service: service,
        descriptor,
        host_session_id,
        queue,
        terminal_error,
    });

    Ok(insert_input_resource(binding, session))
}

/// Reject unsupported Windows MIDI virtual-input transport requests.
fn validate_virtual_input_transport(
    data_format: MidiDataFormat,
    _protocol: MidiProtocol,
) -> RuntimeResult<()> {
    if data_format != MidiDataFormat::Ump {
        return Err(core_platform::not_supported(
            "destack.device.midi.input.virtual.create",
        ));
    }

    Ok(())
}
