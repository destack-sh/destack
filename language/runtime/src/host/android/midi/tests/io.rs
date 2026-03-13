use super::core::{
    TEST_INPUT_SESSION_ID, TEST_OUTPUT_SESSION_ID, lock_test_callbacks, register_test_callbacks,
};
use crate::host::android::abi::{HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_OK};
use crate::host::android::midi::{
    AndroidHostMidiInputRecordHeader, AndroidHostMidiOutputRecordHeader,
    destack_host_android_midi_input_port_close, destack_host_android_midi_input_read,
    destack_host_android_midi_output_flush, destack_host_android_midi_output_port_close,
    destack_host_android_midi_output_write,
};
use crate::runtime::NativeSlice;

/// Route Android MIDI input reads through the registered callback table.
#[test]
fn test_register_bindings_routes_input_read_callback() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut record = AndroidHostMidiInputRecordHeader::default();
    let mut record_count_written = 0u32;
    let mut blob_bytes_written = 0u32;

    // verify reads honor the two-pass buffer contract
    let status = unsafe {
        destack_host_android_midi_input_read(
            callbacks.runtime_id,
            TEST_INPUT_SESSION_ID,
            1,
            0,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut record_count_written,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut blob_bytes_written,
        )
    };
    assert_eq!(status, HOST_STATUS_BUFFER_TOO_SMALL);
    assert_eq!(record_count_written, 1);
    assert!(blob_bytes_written > 0);

    let mut blob_bytes = vec![0u8; blob_bytes_written as usize];
    let status = unsafe {
        destack_host_android_midi_input_read(
            callbacks.runtime_id,
            TEST_INPUT_SESSION_ID,
            1,
            0,
            NativeSlice {
                data: &mut record,
                len: 1,
            },
            &mut record_count_written,
            NativeSlice {
                data: blob_bytes.as_mut_ptr(),
                len: blob_bytes.len() as u32,
            },
            &mut blob_bytes_written,
        )
    };
    assert_eq!(status, HOST_STATUS_OK);
    assert_eq!(record_count_written, 1);
    assert_eq!(record.received_at_ns, 77);
}

/// Route Android MIDI write, flush, and close calls through the registered callback table.
#[test]
fn test_register_bindings_routes_write_flush_and_close_callbacks() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let output_record = AndroidHostMidiOutputRecordHeader {
        send_at_ns: 0,
        has_send_at: 0,
        data_offset: 0,
        data_len: 3,
        data_format: 1,
        protocol: 1,
        framing: 1,
    };
    let output_blob = [0x90u8, 0x40, 0x7f];
    let mut records_written = 0u32;

    // verify writes report the number of written records
    let status = unsafe {
        destack_host_android_midi_output_write(
            callbacks.runtime_id,
            TEST_OUTPUT_SESSION_ID,
            NativeSlice {
                data: (&output_record as *const AndroidHostMidiOutputRecordHeader).cast_mut(),
                len: 1,
            },
            1,
            NativeSlice {
                data: output_blob.as_ptr().cast_mut(),
                len: output_blob.len() as u32,
            },
            &mut records_written,
        )
    };
    assert_eq!(status, HOST_STATUS_OK);
    assert_eq!(records_written, 1);

    // verify flush and close route through the registered callbacks
    let status = unsafe {
        destack_host_android_midi_output_flush(callbacks.runtime_id, TEST_OUTPUT_SESSION_ID)
    };
    assert_eq!(status, HOST_STATUS_OK);

    let status = unsafe {
        destack_host_android_midi_input_port_close(callbacks.runtime_id, TEST_INPUT_SESSION_ID)
    };
    assert_eq!(status, HOST_STATUS_OK);

    let status = unsafe {
        destack_host_android_midi_output_port_close(callbacks.runtime_id, TEST_OUTPUT_SESSION_ID)
    };
    assert_eq!(status, HOST_STATUS_OK);
}
