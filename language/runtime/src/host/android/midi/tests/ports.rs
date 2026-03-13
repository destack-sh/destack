use super::core::{
    TEST_DESCRIPTOR_NAME, TEST_INPUT_SESSION_ID, TEST_MANUFACTURER, TEST_MODEL,
    TEST_OUTPUT_SESSION_ID, TEST_VERSION, lock_test_callbacks, native_string_ref,
    register_test_callbacks,
};
use crate::host::android::abi::{HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_OK};
use crate::host::android::midi::{
    AndroidHostMidiOpenedPortHeader, AndroidHostMidiPortDescriptorHeader,
    destack_host_android_midi_input_port_list, destack_host_android_midi_input_port_open,
    destack_host_android_midi_output_port_list, destack_host_android_midi_output_virtual_create,
};
use crate::runtime::NativeSlice;

/// Route Android MIDI port-list callbacks through the registered callback table.
#[test]
fn test_register_bindings_routes_port_list_callbacks() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut descriptor = AndroidHostMidiPortDescriptorHeader::default();
    let mut header_count_written = 0u32;
    let mut string_bytes_written = 0u32;

    // verify list operations honor the two-pass buffer contract
    let status = unsafe {
        destack_host_android_midi_input_port_list(
            callbacks.runtime_id,
            0,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut header_count_written,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HOST_STATUS_BUFFER_TOO_SMALL);
    assert_eq!(header_count_written, 1);
    assert!(string_bytes_written > 0);

    let mut string_bytes = vec![0u8; string_bytes_written as usize];
    let status = unsafe {
        destack_host_android_midi_output_port_list(
            callbacks.runtime_id,
            0,
            NativeSlice {
                data: &mut descriptor,
                len: 1,
            },
            &mut header_count_written,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HOST_STATUS_OK);
    assert_eq!(header_count_written, 1);
}

/// Route Android MIDI open and virtual-create callbacks through the registered callback table.
#[test]
fn test_register_bindings_routes_open_and_virtual_create_callbacks() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut opened_port = AndroidHostMidiOpenedPortHeader::default();
    let mut opened_string_bytes_written = 0u32;
    let name = native_string_ref(TEST_DESCRIPTOR_NAME);
    let manufacturer = native_string_ref(TEST_MANUFACTURER);
    let model = native_string_ref(TEST_MODEL);
    let version = native_string_ref(TEST_VERSION);
    let mut open_bytes = vec![0u8; 128];

    // verify input port open returns the deterministic opened session payload
    let status = unsafe {
        destack_host_android_midi_input_port_open(
            callbacks.runtime_id,
            name,
            1,
            1,
            16,
            &mut opened_port,
            NativeSlice {
                data: open_bytes.as_mut_ptr(),
                len: open_bytes.len() as u32,
            },
            &mut opened_string_bytes_written,
        )
    };
    assert_eq!(status, HOST_STATUS_OK);
    assert_eq!(opened_port.session_id, TEST_INPUT_SESSION_ID);

    // verify virtual output creation routes through the dedicated callback
    let status = unsafe {
        destack_host_android_midi_output_virtual_create(
            callbacks.runtime_id,
            name,
            manufacturer,
            model,
            version,
            1,
            1,
            &mut opened_port,
            NativeSlice {
                data: open_bytes.as_mut_ptr(),
                len: open_bytes.len() as u32,
            },
            &mut opened_string_bytes_written,
        )
    };
    assert_eq!(status, HOST_STATUS_OK);
    assert_eq!(opened_port.session_id, TEST_OUTPUT_SESSION_ID);
}
