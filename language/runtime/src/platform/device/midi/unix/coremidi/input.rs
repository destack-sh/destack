use std::ptr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::device::midi::core::{
    MidiInputRecordValue, MidiPortDescriptorValue, input_queue_capacity, read_queued_batch,
    read_queued_item, remove_labeled_resource, resolve_descriptor_open_transport,
    try_read_queued_batch, try_read_queued_item, validate_record_shape,
};
use crate::platform::device::{
    MidiDataFormat, MidiEventSource, MidiInputPortOpenOptions, MidiPortDirection,
    MidiPortListOptions, MidiVirtualInputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;
use crate::runtime::control::queue::BoundedQueue;

use super::abi::{
    MIDIDestinationCreate, MIDIDestinationCreateWithProtocol, MIDIInputPortCreate,
    MIDIInputPortCreateWithProtocol, MIDIObjectSetIntegerProperty, MIDIObjectSetStringProperty,
    MIDIPortConnectSource, MIDIPortDispose, create_cf_string, kMIDIPropertyDriverVersion,
    kMIDIPropertyManufacturer, kMIDIPropertyModel, release_cf,
};
use super::backend::resolve_backend;
use super::callback::{
    LegacyInputCallbackContext, LegacyInputCallbackToken, legacy_input_read_proc,
    modern_receive_block,
};
use super::core::{
    CoreMidiInputRepository, CoreMidiInputRepositoryKind, core_midi_status_error,
    insert_input_resource, native_optional_string, native_string, selected_protocol_id,
};
use super::descriptor::{
    endpoint_descriptor, filtered_descriptors, register_endpoint_override, resolve_endpoint,
};
use super::event::refresh_native_event_sessions;
use super::resource::input_resource;
/// List CoreMIDI input ports.
pub(crate) fn midi_input_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.device.midi.input.port.list",
    )?;
    let service = binding
        .worker()
        .platform_state
        .device
        .core_midi_service("destack.device.midi.input.port.list")?;

    Ok(filtered_descriptors(
        &service,
        MidiPortDirection::Input,
        options.flags,
    ))
}

/// Open one CoreMIDI input session.
pub(crate) fn midi_input_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiInputPortOpenOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.device.midi.input.port.open",
    )?;
    let service = binding
        .worker()
        .platform_state
        .device
        .core_midi_service("destack.device.midi.input.port.open")?;

    let (endpoint, descriptor) = resolve_endpoint(
        &service,
        MidiPortDirection::Input,
        id,
        "destack.device.midi.input.port.open",
    )?;
    let (data_format, protocol) = resolve_descriptor_open_transport(
        "destack.device.midi.input.port.open",
        &descriptor,
        options.data_format,
        options.protocol,
        MidiDataFormat::Midi1Bytes,
        None,
    )?;

    let queue = Arc::new(BoundedQueue::new(input_queue_capacity(
        options.queue_capacity,
    )));

    let kind = if data_format == MidiDataFormat::Ump {
        let Some(name) = create_cf_string("Destack MIDI Input") else {
            return Err(core_platform::io_operation_error(
                "destack.device.midi.input.port.open",
                None,
                "failed to encode one CoreMIDI input port name",
            ));
        };
        let receive_block =
            modern_receive_block(queue.clone(), Some(Arc::<str>::from(descriptor.id.clone())));
        let mut port = 0u32;

        let status = unsafe {
            MIDIInputPortCreateWithProtocol(
                service.operation_client(),
                name,
                selected_protocol_id(protocol, data_format),
                &mut port,
                &*receive_block.raw,
            )
        };
        release_cf(name.cast());
        if status != 0 || port == 0 {
            return Err(core_midi_status_error(
                "destack.device.midi.input.port.open",
                "MIDIInputPortCreateWithProtocol",
                status,
            ));
        }

        let status = unsafe { MIDIPortConnectSource(port, endpoint, ptr::null_mut()) };
        if status != 0 {
            unsafe {
                let _ = MIDIPortDispose(port);
            }
            return Err(core_midi_status_error(
                "destack.device.midi.input.port.open",
                "MIDIPortConnectSource",
                status,
            ));
        }

        CoreMidiInputRepositoryKind::ModernSource {
            port,
            source: endpoint,
            _receive_block: receive_block,
        }
    } else {
        let Some(name) = create_cf_string("Destack MIDI Input") else {
            return Err(core_platform::io_operation_error(
                "destack.device.midi.input.port.open",
                None,
                "failed to encode one CoreMIDI input port name",
            ));
        };
        let context = Box::new(LegacyInputCallbackContext {
            queue: queue.clone(),
            source_id: Some(Arc::<str>::from(descriptor.id.clone())),
            is_inside_sysex: AtomicBool::new(false),
        });
        let callback_context = LegacyInputCallbackToken::new(context);
        let mut port = 0u32;

        let status = unsafe {
            MIDIInputPortCreate(
                service.operation_client(),
                name,
                Some(legacy_input_read_proc),
                callback_context.as_ptr(),
                &mut port,
            )
        };
        release_cf(name.cast());
        if status != 0 || port == 0 {
            return Err(core_midi_status_error(
                "destack.device.midi.input.port.open",
                "MIDIInputPortCreate",
                status,
            ));
        }

        let status = unsafe { MIDIPortConnectSource(port, endpoint, ptr::null_mut()) };
        if status != 0 {
            unsafe {
                let _ = MIDIPortDispose(port);
            }
            return Err(core_midi_status_error(
                "destack.device.midi.input.port.open",
                "MIDIPortConnectSource",
                status,
            ));
        }

        CoreMidiInputRepositoryKind::LegacySource {
            port,
            source: endpoint,
            _callback_context: callback_context,
        }
    };

    let session = Arc::new(CoreMidiInputRepository {
        _service: service,
        descriptor,
        queue,
        kind,
    });

    Ok(insert_input_resource(binding, session))
}

/// Describe one opened CoreMIDI input session.
pub(crate) fn midi_input_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let session = input_resource(binding, handle, "destack.device.midi.input.port.descriptor")?;

    Ok(session.descriptor.clone())
}

/// Close one opened CoreMIDI input session.
pub(crate) fn midi_input_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<()> {
    let session = input_resource(binding, handle, "destack.device.midi.input.port.close")?;
    let is_virtual_endpoint = session.is_virtual_endpoint();
    let native_event_registry = session._service.native_event_registry.clone();
    drop(session);

    remove_labeled_resource(
        binding,
        handle.0,
        "destack.device.midi.input.port.close",
        "midi input port",
    )?;

    if is_virtual_endpoint {
        refresh_native_event_sessions(&native_event_registry, MidiEventSource::Native);
    }

    Ok(())
}

/// Wait for one CoreMIDI input record.
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
        || Ok(()),
    )
}

/// Wait for one batch of CoreMIDI input records.
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
        || Ok(()),
    )
}

/// Poll one CoreMIDI input record.
pub(crate) fn midi_input_try_read(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiInputRecordValue> {
    let session = input_resource(binding, handle, "destack.device.midi.input.tryRead")?;

    try_read_queued_item(
        &session.queue,
        "destack.device.midi.input.tryRead",
        "midi input queue is empty",
        || Ok(()),
    )
}

/// Poll one batch of CoreMIDI input records.
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
        || Ok(()),
    )
}

/// Create one CoreMIDI virtual input session.
pub(crate) fn midi_input_virtual_create(
    binding: &BindingCallContext,
    options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    let name = native_string(options.name)?;
    let manufacturer = native_optional_string(options.manufacturer)?;
    let model = native_optional_string(options.model)?;
    let version = native_optional_string(options.version)?;

    let _ = resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.device.midi.input.virtual.create",
    )?;
    let service = binding
        .worker()
        .platform_state
        .device
        .core_midi_service("destack.device.midi.input.virtual.create")?;

    validate_record_shape(
        "destack.device.midi.input.virtual.create",
        options.data_format,
        Some(options.protocol),
    )?;

    let queue = Arc::new(BoundedQueue::new(input_queue_capacity(
        options.queue_capacity,
    )));
    let Some(name) = create_cf_string(&name) else {
        return Err(core_platform::io_operation_error(
            "destack.device.midi.input.virtual.create",
            None,
            "failed to encode one CoreMIDI virtual destination name",
        ));
    };

    let (endpoint, kind) = if options.data_format == MidiDataFormat::Ump {
        let receive_block = modern_receive_block(queue.clone(), None);
        let mut endpoint = 0u32;

        let status = unsafe {
            MIDIDestinationCreateWithProtocol(
                service.operation_client(),
                name,
                selected_protocol_id(Some(options.protocol), options.data_format),
                &mut endpoint,
                &*receive_block.raw,
            )
        };
        if status != 0 || endpoint == 0 {
            release_cf(name.cast());
            return Err(core_midi_status_error(
                "destack.device.midi.input.virtual.create",
                "MIDIDestinationCreateWithProtocol",
                status,
            ));
        }

        (
            endpoint,
            CoreMidiInputRepositoryKind::ModernVirtualDestination {
                endpoint,
                _receive_block: receive_block,
            },
        )
    } else {
        let context = Box::new(LegacyInputCallbackContext {
            queue: queue.clone(),
            source_id: None,
            is_inside_sysex: AtomicBool::new(false),
        });
        let callback_context = LegacyInputCallbackToken::new(context);
        let mut endpoint = 0u32;

        let status = unsafe {
            MIDIDestinationCreate(
                service.operation_client(),
                name,
                Some(legacy_input_read_proc),
                callback_context.as_ptr(),
                &mut endpoint,
            )
        };
        if status != 0 || endpoint == 0 {
            release_cf(name.cast());
            return Err(core_midi_status_error(
                "destack.device.midi.input.virtual.create",
                "MIDIDestinationCreate",
                status,
            ));
        }

        (
            endpoint,
            CoreMidiInputRepositoryKind::LegacyVirtualDestination {
                endpoint,
                _callback_context: callback_context,
            },
        )
    };

    if let Some(manufacturer) = manufacturer.as_ref()
        && let Some(value) = create_cf_string(manufacturer)
    {
        unsafe {
            let _ = MIDIObjectSetStringProperty(endpoint, kMIDIPropertyManufacturer, value);
        }
        release_cf(value.cast());
    }
    if let Some(model) = model.as_ref()
        && let Some(value) = create_cf_string(model)
    {
        unsafe {
            let _ = MIDIObjectSetStringProperty(endpoint, kMIDIPropertyModel, value);
        }
        release_cf(value.cast());
    }
    if let Some(version) = version.as_ref()
        && let Ok(version_value) = version.parse::<i32>()
    {
        unsafe {
            let _ =
                MIDIObjectSetIntegerProperty(endpoint, kMIDIPropertyDriverVersion, version_value);
        }
    }
    release_cf(name.cast());

    register_endpoint_override(&service, endpoint, options.data_format, options.protocol);

    let descriptor = endpoint_descriptor(&service, MidiPortDirection::Input, endpoint);
    let session = Arc::new(CoreMidiInputRepository {
        _service: service,
        descriptor,
        queue,
        kind,
    });
    let handle = insert_input_resource(binding, session.clone());

    refresh_native_event_sessions(
        &session._service.native_event_registry,
        MidiEventSource::Native,
    );

    Ok(handle)
}
