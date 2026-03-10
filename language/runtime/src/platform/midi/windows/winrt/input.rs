use std::sync::Arc;
use std::time::Duration;

use windows::Devices::Midi::{MidiInPort, MidiMessageReceivedEventArgs};
use windows::Foundation::TypedEventHandler;
use windows::core::Ref;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{
    MidiInputRecordValue, MidiPortDescriptorValue, validate_record_shape,
};
use crate::platform::midi::shared::remove_labeled_resource;
use crate::platform::midi::{
    MidiDataFormat, MidiInputPortOpenOptions, MidiPortDirection, MidiPortListOptions, MidiProtocol,
    MidiRecordFraming, MidiVirtualInputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::backend::resolve_backend;
use super::core::{
    SharedQueue, WinRtInputSession, binding_timestamp_now, input_queue_capacity,
    insert_input_resource, missing_handle, winrt_relative_timestamp_to_mono_ns,
};
use super::descriptor::{
    filtered_descriptors, resolve_endpoint, validate_endpoint_transport_request,
};
use super::resource::input_resource;
use super::service::{ensure_current_thread_winrt_apartment, winrt_error, winrt_service};

/// Decode one WinRT input message into one inbound record.
fn decode_message(
    open_epoch_ns: u64,
    source_id: String,
    args: &MidiMessageReceivedEventArgs,
) -> windows::core::Result<MidiInputRecordValue> {
    let message = args.Message()?;
    let buffer = message.RawData()?;
    let reader = windows::Storage::Streams::DataReader::FromBuffer(&buffer)?;
    let mut data = vec![0u8; buffer.Length()? as usize];
    reader.ReadBytes(&mut data)?;

    Ok(MidiInputRecordValue {
        received_at_ns: winrt_relative_timestamp_to_mono_ns(
            open_epoch_ns,
            message.Timestamp()?.Duration,
        ),
        source_id: Some(source_id),
        data_format: MidiDataFormat::Midi1Bytes,
        protocol: Some(MidiProtocol::Midi1),
        framing: MidiRecordFraming::Complete,
        data,
    })
}

/// List WinRT input ports.
pub(crate) fn midi_input_port_list(
    _binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.input.port.list",
    )?;
    let service = winrt_service("destack.midi.input.port.list")?;

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
    let _runtime_state = binding.agent().platform_state.midi.runtime_state(binding);

    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.midi.input.port.open",
    )?;
    let service = winrt_service("destack.midi.input.port.open")?;

    validate_record_shape(
        "destack.midi.input.port.open",
        options.data_format.unwrap_or(MidiDataFormat::Midi1Bytes),
        options.protocol,
    )?;

    // open the WinRT port on an initialized caller thread
    ensure_current_thread_winrt_apartment("destack.midi.input.port.open")?;

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

    // anchor the WinRT relative timestamp domain to one process monotonic epoch
    let before_open_ns = binding_timestamp_now();
    let port = MidiInPort::FromIdAsync(&windows::core::HSTRING::from(endpoint.backend_id.as_str()))
        .map_err(|error| {
            winrt_error(
                "destack.midi.input.port.open",
                "MidiInPort::FromIdAsync",
                &error,
            )
        })?
        .get()
        .map_err(|error| {
            winrt_error(
                "destack.midi.input.port.open",
                "IAsyncOperation::get",
                &error,
            )
        })?;
    let after_open_ns = binding_timestamp_now();
    let open_epoch_ns =
        before_open_ns.saturating_add((after_open_ns.saturating_sub(before_open_ns)) / 2);

    // callback queue
    let queue = Arc::new(SharedQueue::new(input_queue_capacity(
        options.queue_capacity,
    )));
    let source_id = descriptor.id.clone();
    let callback_queue = queue.clone();
    let token = port
        .MessageReceived(&TypedEventHandler::new(
            move |_port: Ref<'_, MidiInPort>, args: Ref<'_, MidiMessageReceivedEventArgs>| {
                if let Some(args) = args.as_ref()
                    && let Ok(record) = decode_message(open_epoch_ns, source_id.clone(), args)
                {
                    callback_queue.push_drop_oldest(record);
                }

                Ok(())
            },
        ))
        .map_err(|error| {
            winrt_error(
                "destack.midi.input.port.open",
                "MidiInPort::MessageReceived",
                &error,
            )
        })?;

    let session = Arc::new(WinRtInputSession {
        _service: service,
        descriptor,
        port,
        token,
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
    let _session = input_resource(binding, handle, "destack.midi.input.port.close")?;

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
    let _runtime_state = binding.agent().platform_state.midi.runtime_state(binding);

    Err(core_platform::not_supported(
        "destack.midi.input.virtual.create",
    ))
}
