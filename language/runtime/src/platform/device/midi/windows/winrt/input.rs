use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform, BoundedQueue};
use crate::platform::device::midi::core::{
    MidiInputRecordValue, MidiPortDescriptorValue, input_queue_capacity, read_queued_batch,
    read_queued_item, remove_labeled_resource, resolve_descriptor_open_transport,
    surface_terminal_error, try_read_queued_batch, try_read_queued_item,
};
use crate::platform::device::{
    MidiDataFormat, MidiInputPortOpenOptions, MidiPortDirection, MidiPortListOptions,
    MidiVirtualInputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::core::{WinRtInputRepository, insert_input_resource};
use super::descriptor::{filtered_descriptors, resolve_endpoint};
use super::resource::input_resource;

/// List WinRT input ports.
pub(crate) fn midi_input_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let service = binding
        .worker()
        .platform_state
        .device
        .winrt_service("destack.device.midi.input.port.list")?;

    Ok(filtered_descriptors(
        &service,
        MidiPortDirection::Input,
        options.flags,
    ))
}

/// Open one WinRT input session.
pub(crate) fn midi_input_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiInputPortOpenOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    let service = binding
        .worker()
        .platform_state
        .device
        .winrt_service("destack.device.midi.input.port.open")?;

    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Input,
        id,
        "destack.device.midi.input.port.open",
    )?;
    let descriptor = endpoint.descriptor.clone();
    let (_data_format, _protocol) = resolve_descriptor_open_transport(
        "destack.device.midi.input.port.open",
        &descriptor,
        options.data_format,
        options.protocol,
        MidiDataFormat::Midi1Bytes,
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
        queue.clone(),
        terminal_error.clone(),
        "destack.device.midi.input.port.open",
    )?;

    let session = Arc::new(WinRtInputRepository {
        _service: service,
        descriptor,
        host_session_id,
        queue,
        terminal_error,
    });

    Ok(insert_input_resource(binding, session))
}

/// Describe one opened WinRT input session.
pub(crate) fn midi_input_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let session = input_resource(binding, handle, "destack.device.midi.input.port.descriptor")?;

    Ok(session.descriptor.clone())
}

/// Close one opened WinRT input session.
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

/// Wait for one WinRT input record.
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

/// Wait for one batch of WinRT input records.
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

/// Poll one WinRT input record.
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

/// Poll one batch of WinRT input records.
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

/// Reject virtual input creation on WinRT.
pub(crate) fn midi_input_virtual_create(
    _binding: &BindingCallContext,
    _options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    Err(core_platform::not_supported(
        "destack.device.midi.input.virtual.create",
    ))
}
