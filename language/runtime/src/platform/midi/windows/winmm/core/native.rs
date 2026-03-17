use std::mem::size_of;
use std::ptr::addr_of;
use std::sync::Arc;

use windows_sys::Win32::Media::Audio::{
    MIDIHDR, MIDIINCAPSW, MIDIOUTCAPSW, midiInGetDevCapsW, midiInGetNumDevs, midiOutGetDevCapsW,
    midiOutGetNumDevs,
};
use windows_sys::Win32::Media::MMSYSERR_NOERROR;
use windows_sys::Win32::System::Threading::WaitForSingleObject;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{self as core_platform};
use crate::platform::midi::core::{MidiRecordBytes, canonical_midi1_message_length};
use crate::platform::midi::{MidiDataFormat, MidiProtocol, MidiRecordFraming};

use super::session::{WinMmInputBuffer, WinMmInputCallbackContext, WinMmInputTerminalError};
use super::transport::winmm_timestamp_to_mono_ns;

/// Default WinMM SysEx buffer size.
const DEFAULT_SYSEX_BUFFER_SIZE: usize = 1024;

/// Return one UTF-16 device name.
pub(crate) fn wide_name(name: &[u16]) -> String {
    let length = name
        .iter()
        .position(|code_unit| *code_unit == 0)
        .unwrap_or(name.len());

    String::from_utf16_lossy(&name[..length])
}

/// Return one current WinMM input descriptor snapshot.
pub(crate) fn query_input_descriptors(
    operation: &'static str,
) -> RuntimeResult<Vec<(u32, MIDIINCAPSW)>> {
    let count = unsafe { midiInGetNumDevs() };
    let mut descriptors = Vec::with_capacity(count as usize);

    // enumerate each WinMM input device
    for device_id in 0..count {
        let mut caps = unsafe { std::mem::zeroed::<MIDIINCAPSW>() };
        let status = unsafe {
            midiInGetDevCapsW(
                device_id as usize,
                &mut caps,
                size_of::<MIDIINCAPSW>() as u32,
            )
        };
        if status != MMSYSERR_NOERROR {
            return Err(winmm_error(operation, "midiInGetDevCapsW", status));
        }

        descriptors.push((device_id, caps));
    }

    Ok(descriptors)
}

/// Return one current WinMM output descriptor snapshot.
pub(crate) fn query_output_descriptors(
    operation: &'static str,
) -> RuntimeResult<Vec<(u32, MIDIOUTCAPSW)>> {
    let count = unsafe { midiOutGetNumDevs() };
    let mut descriptors = Vec::with_capacity(count as usize);

    // enumerate each WinMM output device
    for device_id in 0..count {
        let mut caps = unsafe { std::mem::zeroed::<MIDIOUTCAPSW>() };
        let status = unsafe {
            midiOutGetDevCapsW(
                device_id as usize,
                &mut caps,
                size_of::<MIDIOUTCAPSW>() as u32,
            )
        };
        if status != MMSYSERR_NOERROR {
            return Err(winmm_error(operation, "midiOutGetDevCapsW", status));
        }

        descriptors.push((device_id, caps));
    }

    Ok(descriptors)
}

/// Allocate one stable WinMM SysEx input buffer.
pub(crate) fn allocate_input_buffer() -> WinMmInputBuffer {
    let mut data = vec![0u8; DEFAULT_SYSEX_BUFFER_SIZE];
    let header = MIDIHDR {
        lpData: data.as_mut_ptr(),
        dwBufferLength: data.len() as u32,
        dwBytesRecorded: 0,
        dwUser: 0,
        dwFlags: 0,
        lpNext: std::ptr::null_mut(),
        reserved: 0,
        dwOffset: 0,
        dwReserved: [0; 8],
    };

    WinMmInputBuffer {
        header,
        _data: data,
    }
}

/// Retain one input callback context for the WinMM callback lifetime.
pub(crate) fn retain_input_callback_context(context: &Arc<WinMmInputCallbackContext>) -> usize {
    let context_ptr = Arc::as_ptr(context);

    unsafe {
        Arc::increment_strong_count(context_ptr);
    }

    context_ptr as usize
}

/// Release one retained input callback context.
pub(crate) fn release_input_callback_context(callback_context_token: usize) {
    if callback_context_token == 0 {
        return;
    }

    unsafe {
        Arc::decrement_strong_count(callback_context_token as *const WinMmInputCallbackContext);
    }
}

/// Return one borrowed callback context from one WinMM callback token.
pub(crate) unsafe fn input_callback_context(
    callback_context_token: usize,
) -> &'static WinMmInputCallbackContext {
    unsafe { &*(callback_context_token as *const WinMmInputCallbackContext) }
}

/// Return one unaligned WinMM recorded byte count.
pub(crate) fn header_bytes_recorded(header: *const MIDIHDR) -> u32 {
    unsafe { addr_of!((*header).dwBytesRecorded).read_unaligned() }
}

/// Return one unaligned WinMM header data pointer.
pub(crate) fn header_data_ptr(header: *const MIDIHDR) -> *mut u8 {
    unsafe { addr_of!((*header).lpData).read_unaligned() }
}

/// Decode one WinMM short-message payload into one input record.
pub(crate) fn decode_short_message(
    source_id: Arc<str>,
    start_epoch_ns: u64,
    timestamp_ms: u32,
    packed_message: u32,
) -> Option<crate::platform::midi::core::MidiInputRecordValue> {
    let status = (packed_message & 0xFF) as u8;
    let length = canonical_midi1_message_length(status)?;
    let mut data = MidiRecordBytes::with_capacity(length);

    // message bytes
    for index in 0..length {
        let byte = ((packed_message >> (index * 8)) & 0xFF) as u8;
        data.push(byte);
    }

    Some(crate::platform::midi::core::MidiInputRecordValue {
        received_at_ns: winmm_timestamp_to_mono_ns(start_epoch_ns, timestamp_ms),
        source_id: Some(source_id),
        data_format: MidiDataFormat::Midi1Bytes,
        protocol: Some(MidiProtocol::Midi1),
        framing: MidiRecordFraming::Complete,
        data,
    })
}

/// Decode one WinMM long-message payload into one input record.
pub(crate) fn decode_long_message(
    context: &WinMmInputCallbackContext,
    timestamp_ms: u32,
    bytes: &[u8],
) -> Option<crate::platform::midi::core::MidiInputRecordValue> {
    if bytes.is_empty() {
        return None;
    }

    let starts_sysex = bytes.first() == Some(&0xF0);
    let ends_sysex = bytes.last() == Some(&0xF7);
    let mut is_inside_sysex = context.is_inside_sysex.lock();

    // framing
    let framing = if starts_sysex && ends_sysex {
        *is_inside_sysex = false;
        MidiRecordFraming::Complete
    } else if starts_sysex {
        *is_inside_sysex = true;
        MidiRecordFraming::Start
    } else if ends_sysex {
        *is_inside_sysex = false;
        MidiRecordFraming::End
    } else if *is_inside_sysex {
        MidiRecordFraming::Continue
    } else {
        MidiRecordFraming::Complete
    };

    Some(crate::platform::midi::core::MidiInputRecordValue {
        received_at_ns: winmm_timestamp_to_mono_ns(
            context
                .start_epoch_ns
                .load(std::sync::atomic::Ordering::Acquire),
            timestamp_ms,
        ),
        source_id: Some(context.source_id.clone()),
        data_format: MidiDataFormat::Midi1Bytes,
        protocol: Some(MidiProtocol::Midi1),
        framing,
        data: MidiRecordBytes::from_slice(bytes),
    })
}

/// Queue one terminal input error and close the callback queue.
pub(crate) fn set_terminal_input_error(
    context: &WinMmInputCallbackContext,
    terminal_error: WinMmInputTerminalError,
) {
    // terminal error
    {
        let mut error_slot = context.terminal_error.lock();
        *error_slot = Some(terminal_error);
    }

    context.queue.close();
}

/// Map one WinMM error code into one runtime error.
pub(crate) fn winmm_error(
    operation: &'static str,
    api: &'static str,
    status: u32,
) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        None,
        format!("{api} failed with MMRESULT {status}"),
    )
}

/// Wait for one signaled WinMM output completion event.
pub(crate) fn wait_for_output_completion_event(
    completion_event: isize,
    operation: &'static str,
) -> RuntimeResult<()> {
    // block until WinMM signals the session completion event
    let wait_status = unsafe { WaitForSingleObject(completion_event, u32::MAX) };
    let wait_status =
        core_platform::decode_wait_for_single_object_status(wait_status, "WaitForSingleObject")?;

    // successful session output completion
    if matches!(wait_status, core_platform::WaitStatus::Signaled) {
        return Ok(());
    }

    Err(core_platform::io_operation_error(
        operation,
        None,
        format!("WaitForSingleObject returned unexpected output wait status {wait_status:?}"),
    ))
}
