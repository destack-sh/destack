use super::tests::{
    TEST_ADAPTER_ID, TEST_DEVICE_ID, TEST_DEVICE_NAME, TEST_SCAN_SESSION_ID, TEST_STATUS_NOT_FOUND,
    TEST_STATUS_NOT_SUPPORTED, lock_test_callbacks, native_string_ref, recorded_test_state,
    register_test_callbacks,
};
use crate::host::abi::HostStatus;
use crate::host::android::bluetooth::ffi::{
    destack_host_android_bluetooth_adapter_list, destack_host_android_bluetooth_close,
    destack_host_android_bluetooth_open, destack_host_android_bluetooth_scan_close,
    destack_host_android_bluetooth_scan_open,
};
use crate::host::android::bluetooth::types::{
    AndroidHostBluetoothAdapterDescriptorHeader, AndroidHostBluetoothCallbacks,
    AndroidHostBluetoothDeviceDescriptorHeader,
};
use crate::host::android::tests::{register_android_bindings_bluetooth, register_android_runtime};
use crate::runtime::NativeSlice;

/// Return not-supported when Android bluetooth callbacks are not registered.
#[test]
fn test_default_callbacks_return_not_supported() {
    let _lock = lock_test_callbacks();
    let (_queue, _registration, runtime_id) = register_android_runtime();
    let mut header_count_written = 0u32;
    let mut string_bytes_written = 0u32;

    let status = unsafe {
        destack_host_android_bluetooth_adapter_list(
            runtime_id,
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

    assert_eq!(status, TEST_STATUS_NOT_SUPPORTED);
}

/// Reject Android bluetooth callback registration for one unknown runtime.
#[test]
fn test_register_bindings_rejects_unknown_runtime() {
    let _lock = lock_test_callbacks();

    let status = register_android_bindings_bluetooth(0, AndroidHostBluetoothCallbacks::default());

    assert_eq!(status, TEST_STATUS_NOT_FOUND);
}

/// Route adapter listing through the registered Android bluetooth callback table.
#[test]
fn test_register_bindings_routes_adapter_list() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut header = AndroidHostBluetoothAdapterDescriptorHeader::default();
    let mut header_count_written = 0u32;
    let mut string_bytes_written = 0u32;

    // verify adapter enumeration honors the two-pass buffer contract
    let status = unsafe {
        destack_host_android_bluetooth_adapter_list(
            callbacks.runtime_id,
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
    assert_eq!(status, HostStatus::BufferTooSmall.code());
    assert_eq!(header_count_written, 1);
    assert!(string_bytes_written > 0);

    let mut string_bytes = vec![0u8; string_bytes_written as usize];
    let status = unsafe {
        destack_host_android_bluetooth_adapter_list(
            callbacks.runtime_id,
            NativeSlice {
                data: &mut header,
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
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(header_count_written, 1);

    // verify the returned row decodes to the deterministic adapter fixture
    let name_start = header.name_offset as usize;
    let name_end = name_start + header.name_len as usize;
    assert_eq!(
        std::str::from_utf8(&string_bytes[name_start..name_end]).unwrap(),
        "Android Adapter"
    );
}

/// Route scan open and device open through the registered callback table.
#[test]
fn test_register_bindings_routes_scan_and_device_lifecycle() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut scan_session_id = 0u64;
    let mut device_session_id = 0u64;
    let mut descriptor = AndroidHostBluetoothDeviceDescriptorHeader::default();
    let mut string_bytes_written = 0u32;
    let mut string_bytes = vec![0u8; 128];

    // verify scan open forwards the adapter id and filter flags
    let status = unsafe {
        destack_host_android_bluetooth_scan_open(
            callbacks.runtime_id,
            native_string_ref(TEST_ADAPTER_ID),
            0x33,
            &mut scan_session_id,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(scan_session_id, TEST_SCAN_SESSION_ID);

    let state = recorded_test_state();
    let scan_open = state.scan_open.expect("scan open should be recorded");
    assert_eq!(scan_open.adapter_id, TEST_ADAPTER_ID);
    assert_eq!(scan_open.filter_flags, 0x33);

    // verify device open returns the deterministic descriptor payload
    let status = unsafe {
        destack_host_android_bluetooth_open(
            callbacks.runtime_id,
            native_string_ref(TEST_ADAPTER_ID),
            native_string_ref(TEST_DEVICE_ID),
            &mut device_session_id,
            &mut descriptor,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(device_session_id, 202);

    let name_start = descriptor.name_offset as usize;
    let name_end = name_start + descriptor.name_len as usize;
    assert_eq!(
        std::str::from_utf8(&string_bytes[name_start..name_end]).unwrap(),
        TEST_DEVICE_NAME
    );

    let state = recorded_test_state();
    let device_open = state.device_open.expect("device open should be recorded");
    assert_eq!(device_open.adapter_id, TEST_ADAPTER_ID);
    assert_eq!(device_open.device_id, TEST_DEVICE_ID);

    // verify close calls route through the registered callbacks
    let status =
        unsafe { destack_host_android_bluetooth_scan_close(callbacks.runtime_id, scan_session_id) };
    assert_eq!(status, HostStatus::Ok.code());
    let status =
        unsafe { destack_host_android_bluetooth_close(callbacks.runtime_id, device_session_id) };
    assert_eq!(status, HostStatus::Ok.code());

    let state = recorded_test_state();
    assert_eq!(state.closed_scan_sessions, vec![TEST_SCAN_SESSION_ID]);
    assert_eq!(state.closed_device_sessions, vec![202]);
}
