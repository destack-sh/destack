use crate::host::android::abi::midi::tests::core::{
    RecordedEventOpenCall, RecordedInputOpenCall, RecordedInputVirtualCreateCall,
    RecordedOutputOpenCall, RecordedOutputVirtualCreateCall, RecordedOutputWriteCall,
    RecordedOutputWriteRecord, TEST_CAPABILITY_FLAGS, TEST_DATA_FORMATS, TEST_DESCRIPTOR_ID,
    TEST_DESCRIPTOR_NAME, TEST_EVENT_SESSION_ID, TEST_INPUT_SESSION_ID, TEST_MANUFACTURER,
    TEST_MODEL, TEST_OUTPUT_SESSION_ID, TEST_PROTOCOLS, TEST_SOURCE_ID, TEST_VERSION,
    append_string, decode_native_string, test_state,
};
use crate::host::android::abi::midi::types::{
    AndroidHostMidiCallbacks, AndroidHostMidiEventHeader, AndroidHostMidiInputRecordHeader,
    AndroidHostMidiOpenedPortHeader, AndroidHostMidiOutputRecordHeader,
    AndroidHostMidiPortDescriptorHeader,
};
use crate::host::{HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_OK};
use crate::runtime::{NativeSlice, NativeStringRef};

/// Build one deterministic descriptor header and append its strings.
fn build_test_descriptor_header(buffer: &mut Vec<u8>) -> AndroidHostMidiPortDescriptorHeader {
    let (id_offset, id_len) = append_string(buffer, TEST_DESCRIPTOR_ID);
    let (name_offset, name_len) = append_string(buffer, TEST_DESCRIPTOR_NAME);
    let (manufacturer_offset, manufacturer_len) = append_string(buffer, TEST_MANUFACTURER);
    let (model_offset, model_len) = append_string(buffer, TEST_MODEL);
    let (version_offset, version_len) = append_string(buffer, TEST_VERSION);

    AndroidHostMidiPortDescriptorHeader {
        id_offset,
        id_len,
        group_id_offset: 0,
        group_id_len: 0,
        backend_id_offset: 0,
        backend_id_len: 0,
        name_offset,
        name_len,
        group_name_offset: 0,
        group_name_len: 0,
        manufacturer_offset,
        manufacturer_len,
        model_offset,
        model_len,
        version_offset,
        version_len,
        supported_data_formats: TEST_DATA_FORMATS,
        default_data_format: 1,
        supported_protocols: TEST_PROTOCOLS,
        default_protocol: 1,
        is_virtual: 0,
        is_connected: 1,
    }
}

/// Copy one descriptor header and string payload into caller buffers.
unsafe fn write_descriptor_payload(
    headers: NativeSlice<AndroidHostMidiPortDescriptorHeader>,
    header_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    let mut buffer = Vec::new();
    let header = build_test_descriptor_header(&mut buffer);

    // report required output on short buffers
    if headers.data.is_null()
        || headers.len == 0
        || string_bytes.data.is_null()
        || string_bytes.len < buffer.len() as u32
    {
        unsafe {
            *header_count_written = 1;
            *string_bytes_written = buffer.len() as u32;
        }

        return HOST_STATUS_BUFFER_TOO_SMALL;
    }

    // write one descriptor row and payload bytes
    unsafe {
        *headers.data = header;
        *header_count_written = 1;
        *string_bytes_written = buffer.len() as u32;
    }

    let output =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output[..buffer.len()].copy_from_slice(&buffer);

    HOST_STATUS_OK
}

/// Write one deterministic opened-port payload.
unsafe fn write_opened_port_payload(
    session_id: u64,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    let mut buffer = Vec::new();
    let descriptor = build_test_descriptor_header(&mut buffer);

    // report required payload size on short buffers
    if string_bytes.data.is_null() || string_bytes.len < buffer.len() as u32 {
        unsafe {
            *string_bytes_written = buffer.len() as u32;
        }

        return HOST_STATUS_BUFFER_TOO_SMALL;
    }

    // write the opened-port payload and descriptor bytes
    unsafe {
        *opened_port = AndroidHostMidiOpenedPortHeader {
            session_id,
            descriptor,
        };
        *string_bytes_written = buffer.len() as u32;
    }

    let output =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output[..buffer.len()].copy_from_slice(&buffer);

    HOST_STATUS_OK
}

/// Build one complete test callback table.
pub(super) fn test_callbacks() -> AndroidHostMidiCallbacks {
    AndroidHostMidiCallbacks {
        describe_backend: Some(test_describe_backend),
        input_port_list: Some(test_input_port_list),
        output_port_list: Some(test_output_port_list),
        input_port_open: Some(test_input_port_open),
        output_port_open: Some(test_output_port_open),
        input_virtual_create: Some(test_input_virtual_create),
        output_virtual_create: Some(test_output_virtual_create),
        input_port_close: Some(test_input_port_close),
        output_port_close: Some(test_output_port_close),
        input_read: Some(test_input_read),
        event_open: Some(test_event_open),
        event_read: Some(test_event_read),
        event_close: Some(test_event_close),
        output_write: Some(test_output_write),
    }
}

/// Report one deterministic backend description.
pub(super) unsafe extern "C" fn test_describe_backend(
    _runtime_id: u64,
    capability_flags: *mut u64,
    supported_data_formats: *mut u32,
    supported_protocols: *mut u32,
) -> u32 {
    if capability_flags.is_null()
        || supported_data_formats.is_null()
        || supported_protocols.is_null()
    {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    unsafe {
        *capability_flags = TEST_CAPABILITY_FLAGS;
        *supported_data_formats = TEST_DATA_FORMATS;
        *supported_protocols = TEST_PROTOCOLS;
    }

    HOST_STATUS_OK
}

/// Return one deterministic input-port list row.
pub(super) unsafe extern "C" fn test_input_port_list(
    _runtime_id: u64,
    _flags: u32,
    headers: NativeSlice<AndroidHostMidiPortDescriptorHeader>,
    header_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if header_count_written.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    unsafe {
        write_descriptor_payload(
            headers,
            header_count_written,
            string_bytes,
            string_bytes_written,
        )
    }
}

/// Return one deterministic output-port list row.
pub(super) unsafe extern "C" fn test_output_port_list(
    _runtime_id: u64,
    _flags: u32,
    headers: NativeSlice<AndroidHostMidiPortDescriptorHeader>,
    header_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if header_count_written.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    unsafe {
        write_descriptor_payload(
            headers,
            header_count_written,
            string_bytes,
            string_bytes_written,
        )
    }
}

/// Return one deterministic opened input-port payload.
pub(super) unsafe extern "C" fn test_input_port_open(
    _runtime_id: u64,
    id: NativeStringRef,
    data_format: u32,
    protocol: u32,
    queue_capacity: u32,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if opened_port.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    let mut state = test_state().lock().unwrap();
    state.input_port_open = Some(RecordedInputOpenCall {
        id: decode_native_string(id),
        data_format,
        protocol,
        queue_capacity,
    });
    drop(state);

    unsafe {
        write_opened_port_payload(
            TEST_INPUT_SESSION_ID,
            opened_port,
            string_bytes,
            string_bytes_written,
        )
    }
}

/// Return one deterministic opened output-port payload.
pub(super) unsafe extern "C" fn test_output_port_open(
    _runtime_id: u64,
    id: NativeStringRef,
    data_format: u32,
    protocol: u32,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if opened_port.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    let mut state = test_state().lock().unwrap();
    state.output_port_open = Some(RecordedOutputOpenCall {
        id: decode_native_string(id),
        data_format,
        protocol,
    });
    drop(state);

    unsafe {
        write_opened_port_payload(
            TEST_OUTPUT_SESSION_ID,
            opened_port,
            string_bytes,
            string_bytes_written,
        )
    }
}

/// Return one deterministic virtual input-port payload.
pub(super) unsafe extern "C" fn test_input_virtual_create(
    _runtime_id: u64,
    name: NativeStringRef,
    manufacturer: NativeStringRef,
    model: NativeStringRef,
    version: NativeStringRef,
    data_format: u32,
    protocol: u32,
    queue_capacity: u32,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if opened_port.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    let mut state = test_state().lock().unwrap();
    state.input_virtual_create = Some(RecordedInputVirtualCreateCall {
        name: decode_native_string(name),
        manufacturer: decode_native_string(manufacturer),
        model: decode_native_string(model),
        version: decode_native_string(version),
        data_format,
        protocol,
        queue_capacity,
    });
    drop(state);

    unsafe {
        write_opened_port_payload(
            TEST_INPUT_SESSION_ID,
            opened_port,
            string_bytes,
            string_bytes_written,
        )
    }
}

/// Return one deterministic virtual output-port payload.
pub(super) unsafe extern "C" fn test_output_virtual_create(
    _runtime_id: u64,
    name: NativeStringRef,
    manufacturer: NativeStringRef,
    model: NativeStringRef,
    version: NativeStringRef,
    data_format: u32,
    protocol: u32,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if opened_port.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    let mut state = test_state().lock().unwrap();
    state.output_virtual_create = Some(RecordedOutputVirtualCreateCall {
        name: decode_native_string(name),
        manufacturer: decode_native_string(manufacturer),
        model: decode_native_string(model),
        version: decode_native_string(version),
        data_format,
        protocol,
    });
    drop(state);

    unsafe {
        write_opened_port_payload(
            TEST_OUTPUT_SESSION_ID,
            opened_port,
            string_bytes,
            string_bytes_written,
        )
    }
}

/// Report one successful input-port close.
pub(super) unsafe extern "C" fn test_input_port_close(_runtime_id: u64, _session_id: u64) -> u32 {
    test_state()
        .lock()
        .unwrap()
        .closed_input_sessions
        .push(_session_id);

    HOST_STATUS_OK
}

/// Report one successful event-subscription open.
pub(super) unsafe extern "C" fn test_event_open(
    _runtime_id: u64,
    flags: u32,
    direction_mask: u32,
    session_id: *mut u64,
) -> u32 {
    if session_id.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    let mut state = test_state().lock().unwrap();
    state.event_open = Some(RecordedEventOpenCall {
        flags,
        direction_mask,
    });
    drop(state);

    unsafe {
        *session_id = TEST_EVENT_SESSION_ID;
    }

    HOST_STATUS_OK
}

/// Return one deterministic topology event payload.
pub(super) unsafe extern "C" fn test_event_read(
    _runtime_id: u64,
    _session_id: u64,
    _max_events: u32,
    _timeout_ns: u64,
    headers: NativeSlice<AndroidHostMidiEventHeader>,
    event_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if event_count_written.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    let mut buffer = Vec::new();
    let descriptor = build_test_descriptor_header(&mut buffer);

    if headers.data.is_null()
        || headers.len == 0
        || string_bytes.data.is_null()
        || string_bytes.len < buffer.len() as u32
    {
        unsafe {
            *event_count_written = 1;
            *string_bytes_written = buffer.len() as u32;
        }

        return HOST_STATUS_BUFFER_TOO_SMALL;
    }

    unsafe {
        *headers.data = AndroidHostMidiEventHeader {
            timestamp_ns: 88,
            dropped_count: 0,
            kind: 1,
            direction: 1,
            flags: 0,
            id_offset: 0,
            id_len: 0,
            group_id_offset: 0,
            group_id_len: 0,
            descriptor,
        };
        *event_count_written = 1;
        *string_bytes_written = buffer.len() as u32;
    }

    let output =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output[..buffer.len()].copy_from_slice(&buffer);

    HOST_STATUS_OK
}

/// Report one successful event-subscription close.
pub(super) unsafe extern "C" fn test_event_close(_runtime_id: u64, _session_id: u64) -> u32 {
    test_state()
        .lock()
        .unwrap()
        .closed_event_sessions
        .push(_session_id);

    HOST_STATUS_OK
}

/// Report one successful output-port close.
pub(super) unsafe extern "C" fn test_output_port_close(_runtime_id: u64, _session_id: u64) -> u32 {
    test_state()
        .lock()
        .unwrap()
        .closed_output_sessions
        .push(_session_id);

    HOST_STATUS_OK
}

/// Return one deterministic inbound MIDI record.
pub(super) unsafe extern "C" fn test_input_read(
    _runtime_id: u64,
    _session_id: u64,
    _max_records: u32,
    _timeout_ns: u64,
    headers: NativeSlice<AndroidHostMidiInputRecordHeader>,
    record_count_written: *mut u32,
    blob_bytes: NativeSlice<u8>,
    blob_bytes_written: *mut u32,
) -> u32 {
    let mut blob = Vec::new();
    let (source_id_offset, source_id_len) = append_string(&mut blob, TEST_SOURCE_ID);
    let data_offset = blob.len() as u32;
    let record_bytes = [0x90u8, 0x40, 0x7f];
    blob.extend_from_slice(&record_bytes);

    if record_count_written.is_null() || blob_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    // report the required payload sizes on short buffers
    if headers.data.is_null()
        || headers.len == 0
        || blob_bytes.data.is_null()
        || blob_bytes.len < blob.len() as u32
    {
        unsafe {
            *record_count_written = 1;
            *blob_bytes_written = blob.len() as u32;
        }

        return HOST_STATUS_BUFFER_TOO_SMALL;
    }

    // write one input record and payload blob
    unsafe {
        *headers.data = AndroidHostMidiInputRecordHeader {
            received_at_ns: 77,
            source_id_offset,
            source_id_len,
            data_offset,
            data_len: record_bytes.len() as u32,
            data_format: 1,
            protocol: 1,
            framing: 1,
        };
        *record_count_written = 1;
        *blob_bytes_written = blob.len() as u32;
    }

    let output =
        unsafe { std::slice::from_raw_parts_mut(blob_bytes.data, blob_bytes.len as usize) };
    output[..blob.len()].copy_from_slice(&blob);

    HOST_STATUS_OK
}

/// Report one successful outbound write.
pub(super) unsafe extern "C" fn test_output_write(
    _runtime_id: u64,
    session_id: u64,
    headers: NativeSlice<AndroidHostMidiOutputRecordHeader>,
    record_count: u32,
    blob_bytes: NativeSlice<u8>,
    records_written: *mut u32,
) -> u32 {
    if records_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    let header_slice = if headers.data.is_null() || headers.len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(headers.data, headers.len as usize) }
    };
    let blob_slice = if blob_bytes.data.is_null() || blob_bytes.len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(blob_bytes.data, blob_bytes.len as usize) }
    };

    let mut recorded_records = Vec::with_capacity(header_slice.len());

    for header in header_slice {
        let data_start = header.data_offset as usize;
        let data_end = data_start.saturating_add(header.data_len as usize);
        let data = blob_slice
            .get(data_start..data_end)
            .expect("android midi output write should reference one in-range payload slice")
            .to_vec();

        recorded_records.push(RecordedOutputWriteRecord {
            send_at_ns: header.send_at_ns,
            has_send_at: header.has_send_at,
            data_format: header.data_format,
            protocol: header.protocol,
            framing: header.framing,
            data,
        });
    }

    let mut state = test_state().lock().unwrap();
    state.output_write = Some(RecordedOutputWriteCall {
        session_id,
        records: recorded_records,
    });
    drop(state);

    unsafe {
        *records_written = record_count;
    }

    HOST_STATUS_OK
}
