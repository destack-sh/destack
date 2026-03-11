use std::sync::Arc;
use std::time::Duration;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiInputRecordValue, MidiPortDescriptorValue, remove_labeled_resource, validate_record_shape,
};
use crate::platform::midi::{
    MidiDataFormat, MidiInputPortOpenOptions, MidiPortDirection, MidiPortListOptions,
    MidiVirtualInputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::backend::resolve_backend;
use super::core::{BoundedQueue, WinRtInputSession, input_queue_capacity, insert_input_resource};
use super::descriptor::{
    filtered_descriptors, resolve_endpoint, validate_endpoint_transport_request,
};
use super::resource::input_resource;
/// List WinRT input ports.
pub(crate) fn midi_input_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.input.port.list",
    )?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .winrt_service("destack.midi.input.port.list")?;

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
    binding
        .agent()
        .platform_state
        .midi
        .mark_runtime_active(binding);

    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.input.port.open",
    )?;
    let service = binding
        .agent()
        .platform_state
        .midi
        .winrt_service("destack.midi.input.port.open")?;

    validate_record_shape(
        "destack.midi.input.port.open",
        options.data_format.unwrap_or(MidiDataFormat::Midi1Bytes),
        options.protocol,
    )?;

    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Input,
        id,
        "destack.midi.input.port.open",
    )?;
    let descriptor = endpoint.descriptor.clone();
    let data_format = options
        .data_format
        .or(descriptor.default_data_format)
        .unwrap_or(MidiDataFormat::Midi1Bytes);
    let protocol = options.protocol.or(descriptor.default_protocol);

    validate_endpoint_transport_request(
        "destack.midi.input.port.open",
        &descriptor,
        data_format,
        protocol,
    )?;

    // callback queue
    let queue = Arc::new(BoundedQueue::new(input_queue_capacity(
        options.queue_capacity,
    )));
    let host_session_id = service.open_input_session(
        endpoint.backend_id,
        descriptor.clone(),
        queue.clone(),
        "destack.midi.input.port.open",
    )?;

    let session = Arc::new(WinRtInputSession {
        _service: service,
        descriptor,
        host_session_id,
        queue,
    });

    Ok(insert_input_resource(binding, session))
}

/// Describe one opened WinRT input session.
pub(crate) fn midi_input_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let session = input_resource(binding, handle, "destack.midi.input.port.descriptor")?;

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
        "destack.midi.input.port.close",
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
    let session = input_resource(binding, handle, "destack.midi.input.read")?;
    let timeout = Duration::from_nanos(timeout_ns);

    match session.queue.pop_with_timeout(timeout) {
        Some(record) => Ok(record),
        None => Err(core_platform::io_would_block(
            "destack.midi.input.read",
            "midi input queue is empty",
        )),
    }
}

/// Wait for one batch of WinRT input records.
pub(crate) fn midi_input_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    max_records: u32,
    timeout_ns: u64,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    let session = input_resource(binding, handle, "destack.midi.input.readBatch")?;
    let batch = session.queue.pop_batch_with_timeout(
        max_records.max(1) as usize,
        Duration::from_nanos(timeout_ns),
    );

    if batch.is_empty() {
        return Err(core_platform::io_would_block(
            "destack.midi.input.readBatch",
            "midi input queue is empty",
        ));
    }

    Ok(batch)
}

/// Poll one WinRT input record.
pub(crate) fn midi_input_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiInputRecordValue> {
    let session = input_resource(binding, handle, "destack.midi.input.tryRead")?;

    match session.queue.try_pop() {
        Some(record) => Ok(record),
        None => Err(core_platform::io_would_block(
            "destack.midi.input.tryRead",
            "midi input queue is empty",
        )),
    }
}

/// Poll one batch of WinRT input records.
pub(crate) fn midi_input_try_read_batch(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    max_records: u32,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    let session = input_resource(binding, handle, "destack.midi.input.tryReadBatch")?;
    let batch = session.queue.try_pop_batch(max_records.max(1) as usize);

    if batch.is_empty() {
        return Err(core_platform::io_would_block(
            "destack.midi.input.tryReadBatch",
            "midi input queue is empty",
        ));
    }

    Ok(batch)
}

/// Reject virtual input creation on WinRT.
pub(crate) fn midi_input_virtual_create(
    binding: &BindingCallContext,
    _options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    binding
        .agent()
        .platform_state
        .midi
        .mark_runtime_active(binding);

    Err(core_platform::not_supported(
        "destack.midi.input.virtual.create",
    ))
}
