use parking_lot::Mutex;
use std::mem::size_of;
use std::ptr::addr_of;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use windows_sys::Win32::Media::Audio::{
    CALLBACK_FUNCTION, HMIDIIN, MIDIHDR, midiInAddBuffer, midiInClose, midiInOpen,
    midiInPrepareHeader, midiInStart,
};
use windows_sys::Win32::Media::{
    MM_MIM_DATA, MM_MIM_ERROR, MM_MIM_LONGDATA, MM_MIM_LONGERROR, MM_MIM_MOREDATA, MMSYSERR_NOERROR,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform, BoundedQueue};
use crate::platform::device::midi::core::{
    MidiInputRecordValue, MidiPortDescriptorValue, binding_timestamp_now, input_queue_capacity,
    read_queued_batch, read_queued_item, remove_midi_input_resource,
    resolve_descriptor_open_transport, surface_terminal_error, try_read_queued_batch,
    try_read_queued_item,
};
use crate::platform::device::{
    MidiDataFormat, MidiInputPortOpenOptions, MidiPortDirection, MidiPortListOptions, MidiProtocol,
    MidiVirtualInputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::core::{
    DEFAULT_SYSEX_BUFFER_COUNT, WinMmInputCallbackContext, WinMmInputRepository,
    WinMmInputRepositoryKind, WinMmInputTerminalError, allocate_input_buffer, decode_long_message,
    decode_short_message, header_bytes_recorded, header_data_ptr, input_runtime_id,
    insert_input_resource, release_input_callback_context, retain_input_callback_context,
    set_terminal_input_error, winmm_error,
};
use super::descriptor::{filtered_descriptors, resolve_endpoint};
use super::resource::input_resource;

/// Queue one prepared SysEx buffer on one WinMM input handle.
fn queue_input_buffer(
    handle: HMIDIIN,
    buffer: &mut MIDIHDR,
    operation: &'static str,
) -> RuntimeResult<()> {
    let status = unsafe {
        midiInAddBuffer(
            handle,
            addr_of!(*buffer).cast_mut(),
            size_of::<MIDIHDR>() as u32,
        )
    };
    if status != MMSYSERR_NOERROR {
        return Err(winmm_error(operation, "midiInAddBuffer", status));
    }

    Ok(())
}

/// Requeue one completed SysEx buffer from the WinMM callback.
fn recycle_input_buffer(
    handle: HMIDIIN,
    header: *mut MIDIHDR,
    context: &WinMmInputCallbackContext,
) {
    // skip recycle during shutdown
    if context
        .is_closing
        .load(std::sync::atomic::Ordering::Acquire)
    {
        return;
    }

    // reset the recorded byte count before requeueing
    unsafe {
        addr_of!((*header).dwBytesRecorded)
            .cast_mut()
            .write_unaligned(0);
    }

    let status = unsafe { midiInAddBuffer(handle, header, size_of::<MIDIHDR>() as u32) };
    if status != MMSYSERR_NOERROR {
        set_terminal_input_error(
            context,
            WinMmInputTerminalError::BufferRecycleFailed(status),
        );
    }
}

/// Handle one WinMM input callback.
unsafe extern "system" fn input_callback(
    handle: HMIDIIN,
    message: u32,
    instance: usize,
    param1: usize,
    param2: usize,
) {
    if instance == 0 {
        return;
    }

    // callback context
    let context = unsafe { super::core::input_callback_context(instance) };

    // short messages
    if matches!(message, MM_MIM_DATA | MM_MIM_MOREDATA) {
        let timestamp_ms = param2 as u32;
        let packed_message = param1 as u32;
        let start_epoch_ns = context.start_epoch_ns.load(Ordering::Acquire);

        if let Some(record) = decode_short_message(
            context.source_id.clone(),
            start_epoch_ns,
            timestamp_ms,
            packed_message,
        ) {
            context.queue.push_drop_oldest(record);
        } else {
            set_terminal_input_error(context, WinMmInputTerminalError::ShortMessageError);
        }

        return;
    }

    // malformed short messages
    if message == MM_MIM_ERROR {
        set_terminal_input_error(context, WinMmInputTerminalError::ShortMessageError);
        return;
    }

    // long data and SysEx buffers
    if matches!(message, MM_MIM_LONGDATA | MM_MIM_LONGERROR) {
        let timestamp_ms = param2 as u32;
        let header = param1 as *mut MIDIHDR;
        if header.is_null() {
            set_terminal_input_error(context, WinMmInputTerminalError::LongMessageError);
            return;
        }

        let byte_count = header_bytes_recorded(header);
        let data_ptr = header_data_ptr(header);

        if message == MM_MIM_LONGERROR || data_ptr.is_null() {
            set_terminal_input_error(context, WinMmInputTerminalError::LongMessageError);
            return;
        }

        // decoded SysEx fragment
        let bytes = unsafe { std::slice::from_raw_parts(data_ptr, byte_count as usize) };
        if let Some(record) = decode_long_message(context, timestamp_ms, bytes) {
            context.queue.push_drop_oldest(record);
        }

        recycle_input_buffer(handle, header, context);
    }
}

/// List WinMM MIDI input endpoints.
pub(crate) fn midi_input_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    // current topology
    let service = binding
        .worker()
        .platform_state
        .device
        .winmm_service("destack.device.midi.input.port.list")?;
    service.refresh_topology("destack.device.midi.input.port.list")?;

    Ok(filtered_descriptors(
        &service,
        MidiPortDirection::Input,
        options.flags,
    ))
}

/// Open one WinMM MIDI input endpoint.
pub(crate) fn midi_input_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiInputPortOpenOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    // service and topology
    let service = binding
        .worker()
        .platform_state
        .device
        .winmm_service("destack.device.midi.input.port.open")?;
    service.refresh_topology("destack.device.midi.input.port.open")?;

    // transport selection
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
        Some(MidiProtocol::Midi1),
    )?;

    // callback state
    let queue = Arc::new(BoundedQueue::new(input_queue_capacity(
        options.queue_capacity,
    )));
    let terminal_error = Arc::new(Mutex::new(None));
    let is_inside_sysex = Arc::new(Mutex::new(false));
    let is_closing = Arc::new(AtomicBool::new(false));
    let context = Arc::new(WinMmInputCallbackContext {
        source_id: Arc::<str>::from(input_runtime_id(endpoint.device_id)),
        start_epoch_ns: AtomicU64::new(0),
        queue: queue.clone(),
        terminal_error: terminal_error.clone(),
        is_inside_sysex,
        is_closing: is_closing.clone(),
    });
    let callback_context_token = retain_input_callback_context(&context);

    // raw input handle
    let mut handle: HMIDIIN = 0;
    let open_status = unsafe {
        midiInOpen(
            &mut handle,
            endpoint.device_id,
            input_callback as *const () as usize,
            callback_context_token,
            CALLBACK_FUNCTION,
        )
    };
    if open_status != MMSYSERR_NOERROR {
        release_input_callback_context(callback_context_token);
        return Err(winmm_error(
            "destack.device.midi.input.port.open",
            "midiInOpen",
            open_status,
        ));
    }

    // prepared SysEx buffers
    let mut buffers = Vec::with_capacity(DEFAULT_SYSEX_BUFFER_COUNT);
    for _ in 0..DEFAULT_SYSEX_BUFFER_COUNT {
        buffers.push(allocate_input_buffer());
    }

    // prepare and queue the stable buffer storage
    for buffer in &mut buffers {
        let prepare_status = unsafe {
            midiInPrepareHeader(
                handle,
                addr_of!(buffer.header).cast_mut(),
                size_of::<MIDIHDR>() as u32,
            )
        };
        if prepare_status != MMSYSERR_NOERROR {
            unsafe {
                let _ = midiInClose(handle);
            }
            release_input_callback_context(callback_context_token);
            return Err(winmm_error(
                "destack.device.midi.input.port.open",
                "midiInPrepareHeader",
                prepare_status,
            ));
        }

        queue_input_buffer(
            handle,
            &mut buffer.header,
            "destack.device.midi.input.port.open",
        )?;
    }

    // input start
    let start_epoch_ns = binding_timestamp_now();
    context
        .start_epoch_ns
        .store(start_epoch_ns, Ordering::Release);
    let start_status = unsafe { midiInStart(handle) };
    if start_status != MMSYSERR_NOERROR {
        unsafe {
            let _ = midiInClose(handle);
        }
        release_input_callback_context(callback_context_token);
        return Err(winmm_error(
            "destack.device.midi.input.port.open",
            "midiInStart",
            start_status,
        ));
    }

    let session = Arc::new(WinMmInputRepository {
        _service: service,
        descriptor,
        queue,
        terminal_error,
        kind: Mutex::new(WinMmInputRepositoryKind {
            handle,
            buffers,
            _context_owner: context,
            callback_context_token,
            is_closing,
        }),
    });

    Ok(insert_input_resource(binding, session))
}

/// Describe one opened WinMM MIDI input endpoint.
pub(crate) fn midi_input_port_descriptor(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
) -> RuntimeResult<MidiPortDescriptorValue> {
    let session = input_resource(binding, handle, "destack.device.midi.input.port.descriptor")?;

    Ok(session.descriptor.clone())
}

/// Close one opened WinMM MIDI input endpoint.
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

/// Wait for one WinMM MIDI input record.
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
                |terminal_error| terminal_error.message(),
            )
        },
    )
}

/// Wait for one batch of WinMM MIDI input records.
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
                |terminal_error| terminal_error.message(),
            )
        },
    )
}

/// Poll one WinMM MIDI input record.
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
                |terminal_error| terminal_error.message(),
            )
        },
    )
}

/// Poll one batch of WinMM MIDI input records.
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
                |terminal_error| terminal_error.message(),
            )
        },
    )
}

/// Reject virtual input creation on WinMM.
pub(crate) fn midi_input_virtual_create(
    _binding: &BindingCallContext,
    _options: MidiVirtualInputCreateOptions,
) -> RuntimeResult<resource::MidiInputPortHandle> {
    Err(core_platform::not_supported(
        "destack.device.midi.input.virtual.create",
    ))
}
