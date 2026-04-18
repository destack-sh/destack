use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::HostStatus;
use crate::host::os::android::abi::midi::ffi::{
    destack_host_android_midi_describe_backend, destack_host_android_midi_event_close,
    destack_host_android_midi_event_open, destack_host_android_midi_event_read,
    destack_host_android_midi_input_port_close, destack_host_android_midi_input_port_list,
    destack_host_android_midi_input_port_open, destack_host_android_midi_input_read,
    destack_host_android_midi_input_virtual_create, destack_host_android_midi_output_port_close,
    destack_host_android_midi_output_port_list, destack_host_android_midi_output_port_open,
    destack_host_android_midi_output_virtual_create, destack_host_android_midi_output_write,
};
use crate::host::os::android::abi::midi::types::{
    AndroidHostMidiEventHeader, AndroidHostMidiInputRecordHeader, AndroidHostMidiOpenedPortHeader,
    AndroidHostMidiPortDescriptorHeader,
};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::core::android::host_status_result;
use crate::platform::device::midi::core::{
    MidiEventValue, MidiInputRecordValue, MidiOutputRecordValue, MidiPortDescriptorValue,
};
use crate::platform::device::{
    MidiBackend, MidiBackendCapabilityFlags, MidiDataFormat, MidiDataFormatFlags,
    MidiEventSubscriptionFlags, MidiPortDirectionFlags, MidiProtocol, MidiProtocolFlags,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

use super::super::descriptor::{
    decode_input_records, decode_native_events, decode_opened_port, decode_port_descriptors,
    encode_output_records,
};
use super::codec::{data_format_code, protocol_code};
use super::session::AndroidBackendDescription;

/// Initial descriptor-row scratch capacity for Android host list calls.
const INITIAL_DESCRIPTOR_CAPACITY: usize = 8;
/// Initial descriptor-string scratch capacity for Android host list calls.
const INITIAL_DESCRIPTOR_STRING_CAPACITY: usize = 512;
/// Initial input-record scratch capacity for Android host read calls.
const INITIAL_INPUT_RECORD_CAPACITY: usize = 8;
/// Initial shared blob scratch capacity for Android host record calls.
const INITIAL_RECORD_BLOB_CAPACITY: usize = 512;
/// Maximum descriptor rows accepted from Android host callbacks.
const MAX_DESCRIPTOR_CAPACITY: usize = 4096;
/// Maximum shared descriptor-string bytes accepted from Android host callbacks.
const MAX_DESCRIPTOR_STRING_CAPACITY: usize = 1024 * 1024;
/// Maximum input-record rows accepted from Android host callbacks.
const MAX_INPUT_RECORD_CAPACITY: usize = 4096;
/// Maximum shared record-blob bytes accepted from Android host callbacks.
const MAX_RECORD_BLOB_CAPACITY: usize = 4 * 1024 * 1024;

/// Return one runtime id for Android host callback routing.
fn host_session_id(
    binding: &BindingCallContext,
    _operation: &'static str,
) -> Result<u64, Box<RuntimeError>> {
    Ok(binding.worker().runtime_id.0)
}

/// Build one ioInvalidData runtime error.
pub(crate) fn invalid_data(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    core_platform::io_operation_error(operation, Some(PlatformErrorCode::IoInvalidData), message)
}

/// Load one Android backend description through the host bridge.
pub(crate) fn describe_backend(
    binding: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<AndroidBackendDescription> {
    let runtime_id = host_session_id(binding, operation)?;

    let mut capability_flags = 0u64;
    let mut supported_data_formats = 0u32;
    let mut supported_protocols = 0u32;

    let status = unsafe {
        destack_host_android_midi_describe_backend(
            runtime_id,
            &mut capability_flags,
            &mut supported_data_formats,
            &mut supported_protocols,
        )
    };

    host_status_result(status, operation, "describe backend")?;

    Ok(AndroidBackendDescription {
        capability_flags: MidiBackendCapabilityFlags(capability_flags),
        supported_data_formats: MidiDataFormatFlags(supported_data_formats),
        supported_protocols: MidiProtocolFlags(supported_protocols),
    })
}

/// Call one Android input-port list callback with dynamic output growth.
pub(crate) fn read_input_port_descriptors(
    binding: &BindingCallContext,
    flags: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let runtime_id = host_session_id(binding, operation)?;

    let mut headers =
        vec![AndroidHostMidiPortDescriptorHeader::default(); INITIAL_DESCRIPTOR_CAPACITY];
    let mut string_bytes = vec![0u8; INITIAL_DESCRIPTOR_STRING_CAPACITY];

    loop {
        let header_capacity = checked_u32_length(headers.len(), operation, "descriptor headers")?;
        let string_capacity =
            checked_u32_length(string_bytes.len(), operation, "descriptor strings")?;
        let mut header_count_written = 0u32;
        let mut string_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_midi_input_port_list(
                runtime_id,
                flags,
                NativeSlice {
                    data: headers.as_mut_ptr(),
                    len: header_capacity,
                },
                &mut header_count_written,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        if status == HostStatus::BufferTooSmall.code() {
            grow_descriptor_buffers(
                &mut headers,
                header_count_written as usize,
                &mut string_bytes,
                string_bytes_written as usize,
                operation,
            )?;
            continue;
        }

        host_status_result(status, operation, "list input ports")?;

        let header_count = header_count_written as usize;
        let string_count = string_bytes_written as usize;

        if header_count > headers.len() || string_count > string_bytes.len() {
            return Err(RuntimeError::from(PlatformError::invalid_data(format!(
                "{operation}: android host midi input port list reported one output larger than the provided buffer"
            )))
            .boxed());
        }

        headers.truncate(header_count);
        string_bytes.truncate(string_count);

        return decode_port_descriptors(
            MidiBackend::AndroidMidi,
            &headers,
            &string_bytes,
            operation,
        );
    }
}

/// Call one Android output-port list callback with dynamic output growth.
pub(crate) fn read_output_port_descriptors(
    binding: &BindingCallContext,
    flags: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let runtime_id = host_session_id(binding, operation)?;

    let mut headers =
        vec![AndroidHostMidiPortDescriptorHeader::default(); INITIAL_DESCRIPTOR_CAPACITY];
    let mut string_bytes = vec![0u8; INITIAL_DESCRIPTOR_STRING_CAPACITY];

    loop {
        let header_capacity = checked_u32_length(headers.len(), operation, "descriptor headers")?;
        let string_capacity =
            checked_u32_length(string_bytes.len(), operation, "descriptor strings")?;
        let mut header_count_written = 0u32;
        let mut string_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_midi_output_port_list(
                runtime_id,
                flags,
                NativeSlice {
                    data: headers.as_mut_ptr(),
                    len: header_capacity,
                },
                &mut header_count_written,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        if status == HostStatus::BufferTooSmall.code() {
            grow_descriptor_buffers(
                &mut headers,
                header_count_written as usize,
                &mut string_bytes,
                string_bytes_written as usize,
                operation,
            )?;
            continue;
        }

        host_status_result(status, operation, "list output ports")?;

        let header_count = header_count_written as usize;
        let string_count = string_bytes_written as usize;

        if header_count > headers.len() || string_count > string_bytes.len() {
            return Err(RuntimeError::from(PlatformError::invalid_data(format!(
                "{operation}: android host midi output port list reported one output larger than the provided buffer"
            )))
            .boxed());
        }

        headers.truncate(header_count);
        string_bytes.truncate(string_count);

        return decode_port_descriptors(
            MidiBackend::AndroidMidi,
            &headers,
            &string_bytes,
            operation,
        );
    }
}

/// Call one Android input-port open callback with dynamic output growth.
pub(crate) fn open_input_session(
    binding: &BindingCallContext,
    id: NativeStringRef,
    data_format: MidiDataFormat,
    protocol: Option<MidiProtocol>,
    queue_capacity: u32,
    operation: &'static str,
) -> RuntimeResult<(u64, MidiPortDescriptorValue)> {
    let runtime_id = host_session_id(binding, operation)?;

    let mut opened_port = AndroidHostMidiOpenedPortHeader::default();
    let mut string_bytes = vec![0u8; INITIAL_DESCRIPTOR_STRING_CAPACITY];

    loop {
        let string_capacity =
            checked_u32_length(string_bytes.len(), operation, "opened port strings")?;
        let mut string_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_midi_input_port_open(
                runtime_id,
                id,
                data_format_code(data_format),
                protocol.map(protocol_code).unwrap_or(0),
                queue_capacity,
                &mut opened_port,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        if status == HostStatus::BufferTooSmall.code() {
            grow_string_buffer(&mut string_bytes, string_bytes_written as usize, operation)?;
            continue;
        }

        host_status_result(status, operation, "open input port")?;

        let string_count = string_bytes_written as usize;
        if string_count > string_bytes.len() {
            return Err(RuntimeError::from(PlatformError::invalid_data(format!(
                "{operation}: android host midi input port open reported one payload larger than the provided buffer"
            )))
            .boxed());
        }

        string_bytes.truncate(string_count);

        return decode_opened_port(
            MidiBackend::AndroidMidi,
            &opened_port,
            &string_bytes,
            operation,
        );
    }
}

/// Call one Android output-port open callback with dynamic output growth.
pub(crate) fn open_output_session(
    binding: &BindingCallContext,
    id: NativeStringRef,
    data_format: MidiDataFormat,
    protocol: Option<MidiProtocol>,
    operation: &'static str,
) -> RuntimeResult<(u64, MidiPortDescriptorValue)> {
    let runtime_id = host_session_id(binding, operation)?;

    let mut opened_port = AndroidHostMidiOpenedPortHeader::default();
    let mut string_bytes = vec![0u8; INITIAL_DESCRIPTOR_STRING_CAPACITY];

    loop {
        let string_capacity =
            checked_u32_length(string_bytes.len(), operation, "opened port strings")?;
        let mut string_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_midi_output_port_open(
                runtime_id,
                id,
                data_format_code(data_format),
                protocol.map(protocol_code).unwrap_or(0),
                &mut opened_port,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        if status == HostStatus::BufferTooSmall.code() {
            grow_string_buffer(&mut string_bytes, string_bytes_written as usize, operation)?;
            continue;
        }

        host_status_result(status, operation, "open output port")?;

        let string_count = string_bytes_written as usize;
        if string_count > string_bytes.len() {
            return Err(RuntimeError::from(PlatformError::invalid_data(format!(
                "{operation}: android host midi output port open reported one payload larger than the provided buffer"
            )))
            .boxed());
        }

        string_bytes.truncate(string_count);

        return decode_opened_port(
            MidiBackend::AndroidMidi,
            &opened_port,
            &string_bytes,
            operation,
        );
    }
}

/// Call one Android virtual input create callback with dynamic output growth.
pub(crate) fn create_virtual_input_session(
    binding: &BindingCallContext,
    name: NativeStringRef,
    manufacturer: NativeStringRef,
    model: NativeStringRef,
    version: NativeStringRef,
    data_format: MidiDataFormat,
    protocol: Option<MidiProtocol>,
    queue_capacity: u32,
    operation: &'static str,
) -> RuntimeResult<(u64, MidiPortDescriptorValue)> {
    let runtime_id = host_session_id(binding, operation)?;

    let mut opened_port = AndroidHostMidiOpenedPortHeader::default();
    let mut string_bytes = vec![0u8; INITIAL_DESCRIPTOR_STRING_CAPACITY];

    loop {
        let string_capacity =
            checked_u32_length(string_bytes.len(), operation, "opened port strings")?;
        let mut string_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_midi_input_virtual_create(
                runtime_id,
                name,
                manufacturer,
                model,
                version,
                data_format_code(data_format),
                protocol.map(protocol_code).unwrap_or(0),
                queue_capacity,
                &mut opened_port,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        if status == HostStatus::BufferTooSmall.code() {
            grow_string_buffer(&mut string_bytes, string_bytes_written as usize, operation)?;
            continue;
        }

        host_status_result(status, operation, "create virtual input port")?;

        let string_count = string_bytes_written as usize;
        if string_count > string_bytes.len() {
            return Err(RuntimeError::from(PlatformError::invalid_data(format!(
                "{operation}: android host midi virtual input create reported one payload larger than the provided buffer"
            )))
            .boxed());
        }

        string_bytes.truncate(string_count);

        return decode_opened_port(
            MidiBackend::AndroidMidi,
            &opened_port,
            &string_bytes,
            operation,
        );
    }
}

/// Call one Android virtual output create callback with dynamic output growth.
pub(crate) fn create_virtual_output_session(
    binding: &BindingCallContext,
    name: NativeStringRef,
    manufacturer: NativeStringRef,
    model: NativeStringRef,
    version: NativeStringRef,
    data_format: MidiDataFormat,
    protocol: Option<MidiProtocol>,
    operation: &'static str,
) -> RuntimeResult<(u64, MidiPortDescriptorValue)> {
    let runtime_id = host_session_id(binding, operation)?;

    let mut opened_port = AndroidHostMidiOpenedPortHeader::default();
    let mut string_bytes = vec![0u8; INITIAL_DESCRIPTOR_STRING_CAPACITY];

    loop {
        let string_capacity =
            checked_u32_length(string_bytes.len(), operation, "opened port strings")?;
        let mut string_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_midi_output_virtual_create(
                runtime_id,
                name,
                manufacturer,
                model,
                version,
                data_format_code(data_format),
                protocol.map(protocol_code).unwrap_or(0),
                &mut opened_port,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        if status == HostStatus::BufferTooSmall.code() {
            grow_string_buffer(&mut string_bytes, string_bytes_written as usize, operation)?;
            continue;
        }

        host_status_result(status, operation, "create virtual output port")?;

        let string_count = string_bytes_written as usize;
        if string_count > string_bytes.len() {
            return Err(RuntimeError::from(PlatformError::invalid_data(format!(
                "{operation}: android host midi virtual output create reported one payload larger than the provided buffer"
            )))
            .boxed());
        }

        string_bytes.truncate(string_count);

        return decode_opened_port(
            MidiBackend::AndroidMidi,
            &opened_port,
            &string_bytes,
            operation,
        );
    }
}

/// Call one Android input-read callback with dynamic output growth.
pub(crate) fn read_input_records(
    binding: &BindingCallContext,
    session_id: u64,
    max_records: u32,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    let runtime_id = host_session_id(binding, operation)?;

    let mut headers =
        vec![AndroidHostMidiInputRecordHeader::default(); INITIAL_INPUT_RECORD_CAPACITY];
    let mut blob_bytes = vec![0u8; INITIAL_RECORD_BLOB_CAPACITY];

    loop {
        let header_capacity = checked_u32_length(headers.len(), operation, "record headers")?;
        let blob_capacity = checked_u32_length(blob_bytes.len(), operation, "record blob")?;
        let mut record_count_written = 0u32;
        let mut blob_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_midi_input_read(
                runtime_id,
                session_id,
                max_records,
                timeout_ns,
                NativeSlice {
                    data: headers.as_mut_ptr(),
                    len: header_capacity,
                },
                &mut record_count_written,
                NativeSlice {
                    data: blob_bytes.as_mut_ptr(),
                    len: blob_capacity,
                },
                &mut blob_bytes_written,
            )
        };

        if status == HostStatus::BufferTooSmall.code() {
            grow_record_buffers(
                &mut headers,
                record_count_written as usize,
                &mut blob_bytes,
                blob_bytes_written as usize,
                operation,
            )?;
            continue;
        }

        host_status_result(status, operation, "read input records")?;

        let record_count = record_count_written as usize;
        let blob_count = blob_bytes_written as usize;

        if record_count > headers.len() || blob_count > blob_bytes.len() {
            return Err(RuntimeError::from(PlatformError::invalid_data(format!(
                "{operation}: android host midi input read reported one output larger than the provided buffer"
            )))
            .boxed());
        }

        headers.truncate(record_count);
        blob_bytes.truncate(blob_count);

        return decode_input_records(&headers, &blob_bytes, operation);
    }
}

/// Call one Android event-open callback.
pub(crate) fn open_event_session(
    binding: &BindingCallContext,
    flags: MidiEventSubscriptionFlags,
    direction_mask: MidiPortDirectionFlags,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let runtime_id = host_session_id(binding, operation)?;
    let mut session_id = 0u64;
    let status = unsafe {
        destack_host_android_midi_event_open(runtime_id, flags.0, direction_mask.0, &mut session_id)
    };

    host_status_result(status, operation, "open event subscription")?;

    Ok(session_id)
}

/// Call one Android event-read callback with dynamic output growth.
pub(crate) fn read_native_events(
    binding: &BindingCallContext,
    session_id: u64,
    max_events: u32,
    timeout_ns: u64,
    next_sequence: &mut u64,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let runtime_id = host_session_id(binding, operation)?;

    let mut headers = vec![AndroidHostMidiEventHeader::default(); INITIAL_DESCRIPTOR_CAPACITY];
    let mut string_bytes = vec![0u8; INITIAL_DESCRIPTOR_STRING_CAPACITY];

    loop {
        let header_capacity = checked_u32_length(headers.len(), operation, "event headers")?;
        let string_capacity = checked_u32_length(string_bytes.len(), operation, "event strings")?;
        let mut event_count_written = 0u32;
        let mut string_bytes_written = 0u32;

        let status = unsafe {
            destack_host_android_midi_event_read(
                runtime_id,
                session_id,
                max_events,
                timeout_ns,
                NativeSlice {
                    data: headers.as_mut_ptr(),
                    len: header_capacity,
                },
                &mut event_count_written,
                NativeSlice {
                    data: string_bytes.as_mut_ptr(),
                    len: string_capacity,
                },
                &mut string_bytes_written,
            )
        };

        if status == HostStatus::BufferTooSmall.code() {
            grow_event_buffers(
                &mut headers,
                event_count_written as usize,
                &mut string_bytes,
                string_bytes_written as usize,
                operation,
            )?;
            continue;
        }

        host_status_result(status, operation, "read events")?;

        let event_count = event_count_written as usize;
        let string_count = string_bytes_written as usize;
        if event_count > headers.len() || string_count > string_bytes.len() {
            return Err(RuntimeError::from(PlatformError::invalid_data(format!(
                "{operation}: android host midi event read reported one output larger than the provided buffer"
            )))
            .boxed());
        }

        headers.truncate(event_count);
        string_bytes.truncate(string_count);

        return decode_native_events(&headers, &string_bytes, next_sequence, operation);
    }
}

/// Call one Android output-write callback.
pub(crate) fn write_output_records(
    binding: &BindingCallContext,
    session_id: u64,
    records: &[MidiOutputRecordValue],
    operation: &'static str,
) -> RuntimeResult<()> {
    let runtime_id = host_session_id(binding, operation)?;

    let (headers, blob_bytes): (Vec<_>, Vec<u8>) = encode_output_records(records, operation)?;
    let header_count = checked_u32_length(headers.len(), operation, "record headers")?;
    let blob_count = checked_u32_length(blob_bytes.len(), operation, "record blob")?;
    let mut records_written = 0u32;

    let status = unsafe {
        destack_host_android_midi_output_write(
            runtime_id,
            session_id,
            NativeSlice {
                data: headers.as_ptr().cast_mut(),
                len: header_count,
            },
            header_count,
            NativeSlice {
                data: blob_bytes.as_ptr().cast_mut(),
                len: blob_count,
            },
            &mut records_written,
        )
    };

    host_status_result(status, operation, "write output records")?;

    if records_written != header_count {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi output write reported one partial write"
        )))
        .boxed());
    }

    Ok(())
}

/// Close one Android input session.
pub(crate) fn close_input_session(
    binding: &BindingCallContext,
    session_id: u64,
    operation: &'static str,
) -> RuntimeResult<()> {
    let runtime_id = host_session_id(binding, operation)?;
    let status = unsafe { destack_host_android_midi_input_port_close(runtime_id, session_id) };

    host_status_result(status, operation, "close input port")
}

/// Close one Android event session.
pub(crate) fn close_event_session(
    binding: &BindingCallContext,
    session_id: u64,
    operation: &'static str,
) -> RuntimeResult<()> {
    let runtime_id = host_session_id(binding, operation)?;
    let status = unsafe { destack_host_android_midi_event_close(runtime_id, session_id) };

    host_status_result(status, operation, "close event subscription")
}

/// Close one Android output session.
pub(crate) fn close_output_session(
    binding: &BindingCallContext,
    session_id: u64,
    operation: &'static str,
) -> RuntimeResult<()> {
    let runtime_id = host_session_id(binding, operation)?;
    let status = unsafe { destack_host_android_midi_output_port_close(runtime_id, session_id) };

    host_status_result(status, operation, "close output port")
}

/// Grow one descriptor scratch-buffer pair from one host-reported requirement.
fn grow_descriptor_buffers(
    headers: &mut Vec<AndroidHostMidiPortDescriptorHeader>,
    required_headers: usize,
    string_bytes: &mut Vec<u8>,
    required_string_bytes: usize,
    operation: &'static str,
) -> RuntimeResult<()> {
    let required_headers = required_headers.max(headers.len() + 1);
    if required_headers > MAX_DESCRIPTOR_CAPACITY {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi reported one descriptor count above the supported maximum"
        )))
        .boxed());
    }

    let required_string_bytes = required_string_bytes.max(string_bytes.len() + 1);
    if required_string_bytes > MAX_DESCRIPTOR_STRING_CAPACITY {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi reported one descriptor payload above the supported maximum"
        )))
        .boxed());
    }

    headers.resize(
        required_headers,
        AndroidHostMidiPortDescriptorHeader::default(),
    );
    string_bytes.resize(required_string_bytes, 0);

    Ok(())
}

/// Grow one record scratch-buffer pair from one host-reported requirement.
fn grow_record_buffers(
    headers: &mut Vec<AndroidHostMidiInputRecordHeader>,
    required_headers: usize,
    blob_bytes: &mut Vec<u8>,
    required_blob_bytes: usize,
    operation: &'static str,
) -> RuntimeResult<()> {
    let required_headers = required_headers.max(headers.len() + 1);
    if required_headers > MAX_INPUT_RECORD_CAPACITY {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi reported one record count above the supported maximum"
        )))
        .boxed());
    }

    let required_blob_bytes = required_blob_bytes.max(blob_bytes.len() + 1);
    if required_blob_bytes > MAX_RECORD_BLOB_CAPACITY {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi reported one record payload above the supported maximum"
        )))
        .boxed());
    }

    headers.resize(
        required_headers,
        AndroidHostMidiInputRecordHeader::default(),
    );
    blob_bytes.resize(required_blob_bytes, 0);

    Ok(())
}

/// Grow one native-event scratch-buffer pair from one host-reported requirement.
fn grow_event_buffers(
    headers: &mut Vec<AndroidHostMidiEventHeader>,
    required_headers: usize,
    string_bytes: &mut Vec<u8>,
    required_string_bytes: usize,
    operation: &'static str,
) -> RuntimeResult<()> {
    let required_headers = required_headers.max(headers.len() + 1);
    if required_headers > MAX_DESCRIPTOR_CAPACITY {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi reported one event count above the supported maximum"
        )))
        .boxed());
    }

    let required_string_bytes = required_string_bytes.max(string_bytes.len() + 1);
    if required_string_bytes > MAX_DESCRIPTOR_STRING_CAPACITY {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi reported one event payload above the supported maximum"
        )))
        .boxed());
    }

    headers.resize(required_headers, AndroidHostMidiEventHeader::default());
    string_bytes.resize(required_string_bytes, 0);

    Ok(())
}

/// Grow one string payload scratch buffer from one host-reported requirement.
fn grow_string_buffer(
    string_bytes: &mut Vec<u8>,
    required_string_bytes: usize,
    operation: &'static str,
) -> RuntimeResult<()> {
    let required_string_bytes = required_string_bytes.max(string_bytes.len() + 1);
    if required_string_bytes > MAX_DESCRIPTOR_STRING_CAPACITY {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi reported one descriptor payload above the supported maximum"
        )))
        .boxed());
    }

    string_bytes.resize(required_string_bytes, 0);

    Ok(())
}

/// Convert one native length to the Android host ABI width.
fn checked_u32_length(
    length: usize,
    operation: &'static str,
    field: &'static str,
) -> RuntimeResult<u32> {
    u32::try_from(length).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: {field} length exceeds the Android host ABI width"
        )))
        .boxed()
    })
}

#[cfg(test)]
mod tests {
    use crate::host::HostStatus;
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::tests::platform::error_code_from_result;

    use super::host_status_result;

    /// Map Android MIDI permission failures to ioPermissionDenied.
    #[test]
    fn test_host_status_result_maps_permission_denied_to_io_permission_denied() {
        let code = error_code_from_result(host_status_result(
            HostStatus::PermissionDenied.code(),
            "destack.device.midi.test",
            "permission test",
        ))
        .expect("permission-denied status should surface one platform error code");

        assert_eq!(code, PlatformErrorCode::IoPermissionDenied);
    }

    /// Map Android MIDI would-block failures to ioWouldBlock.
    #[test]
    fn test_host_status_result_maps_would_block_to_io_would_block() {
        let code = error_code_from_result(host_status_result(
            HostStatus::WouldBlock.code(),
            "destack.device.midi.test",
            "would-block test",
        ))
        .expect("would-block status should surface one platform error code");

        assert_eq!(code, PlatformErrorCode::IoWouldBlock);
    }

    /// Keep Android MIDI unsupported failures as notSupported.
    #[test]
    fn test_host_status_result_maps_not_supported_to_not_supported() {
        let code = error_code_from_result(host_status_result(
            HostStatus::NotSupported.code(),
            "destack.device.midi.test",
            "unsupported test",
        ))
        .expect("unsupported status should surface one platform error code");

        assert_eq!(code, PlatformErrorCode::NotSupported);
    }
}
