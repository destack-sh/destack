use crate::host::android::abi::{HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_OK};
use crate::host::android::bridge::midi::ffi::{
    destack_host_android_midi_event_close, destack_host_android_midi_event_open,
    destack_host_android_midi_event_read,
};
use crate::host::android::bridge::midi::tests::core::{
    TEST_DESCRIPTOR_ID, TEST_DESCRIPTOR_NAME, TEST_EVENT_SESSION_ID, lock_test_callbacks,
    recorded_test_state, register_test_callbacks,
};
use crate::host::android::bridge::midi::types::AndroidHostMidiEventHeader;
use crate::runtime::NativeSlice;

/// Route Android MIDI event callbacks through the registered callback table.
#[test]
fn test_register_bindings_routes_event_callbacks() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut session_id = 0u64;
    let mut header = AndroidHostMidiEventHeader::default();
    let mut event_count_written = 0u32;
    let mut string_bytes_written = 0u32;

    // verify native event open returns the deterministic session id
    let status = unsafe {
        destack_host_android_midi_event_open(callbacks.runtime_id, 0x12, 0x03, &mut session_id)
    };
    assert_eq!(status, HOST_STATUS_OK);
    assert_eq!(session_id, TEST_EVENT_SESSION_ID);

    let state = recorded_test_state();
    let event_open = state
        .event_open
        .expect("event open should record one forwarded request");
    assert_eq!(event_open.flags, 0x12);
    assert_eq!(event_open.direction_mask, 0x03);

    // verify event reads honor the two-pass buffer contract
    let status = unsafe {
        destack_host_android_midi_event_read(
            callbacks.runtime_id,
            TEST_EVENT_SESSION_ID,
            1,
            0,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut event_count_written,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HOST_STATUS_BUFFER_TOO_SMALL);
    assert_eq!(event_count_written, 1);
    assert!(string_bytes_written > 0);

    let mut string_bytes = vec![0u8; string_bytes_written as usize];
    let status = unsafe {
        destack_host_android_midi_event_read(
            callbacks.runtime_id,
            TEST_EVENT_SESSION_ID,
            1,
            0,
            NativeSlice {
                data: &mut header,
                len: 1,
            },
            &mut event_count_written,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HOST_STATUS_OK);
    assert_eq!(event_count_written, 1);
    assert_eq!(header.kind, 1);
    assert_eq!(header.direction, 1);
    assert_eq!(header.timestamp_ns, 88);

    let id_start = header.descriptor.id_offset as usize;
    let id_end = id_start + header.descriptor.id_len as usize;
    let name_start = header.descriptor.name_offset as usize;
    let name_end = name_start + header.descriptor.name_len as usize;
    assert_eq!(
        std::str::from_utf8(&string_bytes[id_start..id_end]).unwrap(),
        TEST_DESCRIPTOR_ID
    );
    assert_eq!(
        std::str::from_utf8(&string_bytes[name_start..name_end]).unwrap(),
        TEST_DESCRIPTOR_NAME
    );

    // verify event close routes through the registered callback
    let status = unsafe {
        destack_host_android_midi_event_close(callbacks.runtime_id, TEST_EVENT_SESSION_ID)
    };
    assert_eq!(status, HOST_STATUS_OK);

    let state = recorded_test_state();
    assert_eq!(state.closed_event_sessions, vec![TEST_EVENT_SESSION_ID]);
}
