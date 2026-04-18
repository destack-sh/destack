use std::mem::size_of;
use std::ptr::{addr_of, null};
use std::sync::Arc;

use windows_sys::Win32::Media::Audio::{
    CALLBACK_EVENT, HMIDIOUT, MIDIHDR, midiOutLongMsg, midiOutOpen, midiOutPrepareHeader,
    midiOutShortMsg, midiOutUnprepareHeader,
};
use windows_sys::Win32::Media::MMSYSERR_NOERROR;
use windows_sys::Win32::System::Threading::{CreateEventW, ResetEvent};

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::device::midi::core::{
    MidiOutputRecordValue, MidiPortDescriptorValue, remove_midi_output_resource,
    resolve_descriptor_open_transport, validate_output_record_payload, validate_record_shape,
};
use crate::platform::device::{
    MidiDataFormat, MidiOutputPortOpenOptions, MidiPortDirection, MidiPortListOptions,
    MidiProtocol, MidiRecordFraming, MidiVirtualOutputCreateOptions,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::core::{
    WinMmOutputRepository, insert_output_resource, wait_for_output_completion_event, winmm_error,
};
use super::descriptor::{filtered_descriptors, resolve_endpoint};
use super::resource::output_resource;

/// Pack one short MIDI1 byte stream into one WinMM DWORD payload.
fn pack_short_message(data: &[u8]) -> Option<u32> {
    if data.is_empty() || data.len() > 3 {
        return None;
    }

    let mut packed = 0u32;
    for (index, byte) in data.iter().copied().enumerate() {
        packed |= u32::from(byte) << (index * 8);
    }

    Some(packed)
}

/// Return whether one outbound record should use the short-message path.
fn uses_short_message_path(record: &MidiOutputRecordValue) -> bool {
    record.data_format == MidiDataFormat::Midi1Bytes
        && record.framing == MidiRecordFraming::Complete
        && record.data.len() <= 3
}

/// Write one short WinMM MIDI1 message.
fn write_short_message(
    handle: HMIDIOUT,
    record: &MidiOutputRecordValue,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(packed_message) = pack_short_message(&record.data) else {
        return Err(core_platform::invalid_argument(
            "records",
            format!("{operation}: malformed short MIDI 1 output record"),
        ));
    };

    let status = unsafe { midiOutShortMsg(handle, packed_message) };
    if status != MMSYSERR_NOERROR {
        return Err(winmm_error(operation, "midiOutShortMsg", status));
    }

    Ok(())
}

/// Write one long WinMM byte payload.
fn write_long_message(
    session: &WinMmOutputRepository,
    record: &MidiOutputRecordValue,
    operation: &'static str,
) -> RuntimeResult<()> {
    let mut data = record.data.clone();
    let header = MIDIHDR {
        lpData: data.as_mut_ptr(),
        dwBufferLength: data.len() as u32,
        dwBytesRecorded: data.len() as u32,
        dwUser: 0,
        dwFlags: 0,
        lpNext: std::ptr::null_mut(),
        reserved: 0,
        dwOffset: 0,
        dwReserved: [0; 8],
    };

    // clear stale backend completion before queueing one new long message
    let reset_status = unsafe { ResetEvent(session.completion_event) };
    if reset_status == 0 {
        return Err(core_platform::io_error("ResetEvent"));
    }

    // prepared header
    let prepare_status = unsafe {
        midiOutPrepareHeader(
            session.handle,
            addr_of!(header).cast_mut(),
            size_of::<MIDIHDR>() as u32,
        )
    };
    if prepare_status != MMSYSERR_NOERROR {
        return Err(winmm_error(
            operation,
            "midiOutPrepareHeader",
            prepare_status,
        ));
    }

    // send long payload
    let send_status = unsafe {
        midiOutLongMsg(
            session.handle,
            addr_of!(header).cast_mut(),
            size_of::<MIDIHDR>() as u32,
        )
    };
    if send_status != MMSYSERR_NOERROR {
        unsafe {
            let _ = midiOutUnprepareHeader(
                session.handle,
                addr_of!(header).cast_mut(),
                size_of::<MIDIHDR>() as u32,
            );
        }
        return Err(winmm_error(operation, "midiOutLongMsg", send_status));
    }

    // wait for backend completion
    wait_for_output_completion_event(session.completion_event, operation)?;

    // header teardown
    let unprepare_status = unsafe {
        midiOutUnprepareHeader(
            session.handle,
            addr_of!(header).cast_mut(),
            size_of::<MIDIHDR>() as u32,
        )
    };
    if unprepare_status != MMSYSERR_NOERROR {
        return Err(winmm_error(
            operation,
            "midiOutUnprepareHeader",
            unprepare_status,
        ));
    }

    Ok(())
}

/// List WinMM MIDI output endpoints.
pub(crate) fn midi_output_port_list(
    binding: &BindingCallContext,
    options: MidiPortListOptions,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    // current topology
    let service = binding
        .worker()
        .platform_state
        .device
        .winmm_service("destack.device.midi.output.port.list")?;
    service.refresh_topology("destack.device.midi.output.port.list")?;

    Ok(filtered_descriptors(
        &service,
        MidiPortDirection::Output,
        options.flags,
    ))
}

/// Open one WinMM MIDI output endpoint.
pub(crate) fn midi_output_port_open(
    binding: &BindingCallContext,
    id: &str,
    options: MidiOutputPortOpenOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    // service and topology
    let service = binding
        .worker()
        .platform_state
        .device
        .winmm_service("destack.device.midi.output.port.open")?;
    service.refresh_topology("destack.device.midi.output.port.open")?;

    // transport selection
    let endpoint = resolve_endpoint(
        &service,
        MidiPortDirection::Output,
        id,
        "destack.device.midi.output.port.open",
    )?;
    let descriptor = endpoint.descriptor.clone();
    let (data_format, protocol) = resolve_descriptor_open_transport(
        "destack.device.midi.output.port.open",
        &descriptor,
        options.data_format,
        options.protocol,
        MidiDataFormat::Midi1Bytes,
        Some(MidiProtocol::Midi1),
    )?;

    // session completion event
    let completion_event = unsafe { CreateEventW(null(), 1, 0, null()) };
    if completion_event == 0 {
        return Err(core_platform::io_error("CreateEventW"));
    }

    // raw output handle
    let mut handle: HMIDIOUT = 0;
    let open_status = unsafe {
        midiOutOpen(
            &mut handle,
            endpoint.device_id,
            completion_event as usize,
            0,
            CALLBACK_EVENT,
        )
    };
    if open_status != MMSYSERR_NOERROR {
        unsafe {
            let _ = windows_sys::Win32::Foundation::CloseHandle(completion_event);
        }
        return Err(winmm_error(
            "destack.device.midi.output.port.open",
            "midiOutOpen",
            open_status,
        ));
    }

    let session = Arc::new(WinMmOutputRepository {
        _service: service,
        descriptor,
        data_format,
        protocol,
        handle,
        completion_event,
    });

    Ok(insert_output_resource(binding, session))
}

/// Describe one opened WinMM MIDI output endpoint.
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

/// Close one opened WinMM MIDI output endpoint.
pub(crate) fn midi_output_port_close(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
) -> RuntimeResult<()> {
    remove_midi_output_resource(
        binding,
        handle.0,
        "destack.device.midi.output.port.close",
        "midi output",
    )
}

/// Write outbound WinMM MIDI records.
pub(crate) fn midi_output_write(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    records: Vec<MidiOutputRecordValue>,
) -> RuntimeResult<u32> {
    let session = output_resource(binding, handle, "destack.device.midi.output.write")?;
    let mut written = 0u32;

    // record validation and send
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
                "winmm midi output does not support scheduled send timestamps",
            ));
        }

        if record.data_format != session.data_format {
            return Err(core_platform::invalid_argument(
                "records",
                "destack.device.midi.output.write: record data format does not match the opened port",
            ));
        }

        if record.protocol != session.protocol && record.protocol.is_some() {
            return Err(core_platform::invalid_argument(
                "records",
                "destack.device.midi.output.write: record protocol does not match the opened port",
            ));
        }

        // short path
        if uses_short_message_path(record) {
            write_short_message(session.handle, record, "destack.device.midi.output.write")?;
            written = written.saturating_add(1);
            continue;
        }

        // long path
        write_long_message(&session, record, "destack.device.midi.output.write")?;
        written = written.saturating_add(1);
    }

    Ok(written)
}

/// Reject virtual output creation on WinMM.
pub(crate) fn midi_output_virtual_create(
    _binding: &BindingCallContext,
    _options: MidiVirtualOutputCreateOptions,
) -> RuntimeResult<resource::MidiOutputPortHandle> {
    Err(core_platform::not_supported(
        "destack.device.midi.output.virtual.create",
    ))
}
