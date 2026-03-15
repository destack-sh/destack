use super::tests::{
    TEST_ACCESS_GROUP, TEST_AUTHENTICATION_MECHANISM_BIOMETRIC, test_callbacks, test_contains,
    test_read,
};
use crate::host::android::bridge::credentials::{
    AndroidHostCredentialsCallbacks, destack_host_android_credentials_authenticate,
    destack_host_android_credentials_contains, destack_host_android_credentials_delete,
    destack_host_android_credentials_read, destack_host_android_credentials_write,
};
use crate::host::abi::HostStatus;
use crate::host::android::tests::{
    callback_test_lock, register_android_bindings_credentials, register_android_runtime,
};
use crate::runtime::{NativeSlice, NativeStringRef};

/// Return not-supported when Android credentials callbacks are not registered.
#[test]
fn test_default_callbacks_return_not_supported() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();
    let mut is_present = false;

    // verify contains reports not-supported when runtime-id is valid but callbacks are unset
    let status = unsafe {
        destack_host_android_credentials_contains(
            runtime_id,
            NativeStringRef {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeStringRef {
                data: std::ptr::null_mut(),
                len: 0,
            },
            NativeStringRef {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut is_present,
        )
    };
    assert_eq!(status, HostStatus::NotSupported.code());
}

/// Reject Android credentials callback registration for one unknown runtime.
#[test]
fn test_register_bindings_rejects_unknown_runtime() {
    let _lock = callback_test_lock().lock().unwrap();

    let callbacks = AndroidHostCredentialsCallbacks::default();
    let status = register_android_bindings_credentials(0, callbacks);

    assert_eq!(status, HostStatus::NotFound.code());
}

/// Route Android credentials calls through the registered callback table.
#[test]
fn test_register_bindings_routes_calls() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let callbacks = test_callbacks();
    let status = register_android_bindings_credentials(runtime_id, callbacks);
    assert_eq!(status, HostStatus::Ok.code());

    let empty_string = NativeStringRef {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let access_group_string = TEST_ACCESS_GROUP.to_string();
    let access_group = NativeStringRef {
        data: access_group_string.as_ptr().cast_mut(),
        len: access_group_string.len() as u32,
    };

    let mut is_present = false;
    let status = unsafe {
        destack_host_android_credentials_contains(
            runtime_id,
            empty_string,
            empty_string,
            access_group,
            &mut is_present,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert!(is_present);

    let status = unsafe {
        destack_host_android_credentials_delete(
            runtime_id,
            empty_string,
            empty_string,
            access_group,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let payload = [7u8, 8, 9];
    let status = unsafe {
        destack_host_android_credentials_write(
            runtime_id,
            empty_string,
            empty_string,
            empty_string,
            NativeSlice {
                data: payload.as_ptr() as *mut u8,
                len: payload.len() as u32,
            },
            1,
            1,
            true,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let mut authenticated = false;
    let mut mechanism = 0;
    let status = unsafe {
        destack_host_android_credentials_authenticate(
            runtime_id,
            empty_string,
            empty_string,
            empty_string,
            1,
            &mut authenticated,
            &mut mechanism,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert!(authenticated);
    assert_eq!(mechanism, TEST_AUTHENTICATION_MECHANISM_BIOMETRIC);

    let empty_string = NativeStringRef {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let mut output_small = [0u8; 2];
    let mut output_written = 0u32;
    let mut created_unix_ns = 0u64;
    let mut modified_unix_ns = 0u64;
    let status = unsafe {
        destack_host_android_credentials_read(
            runtime_id,
            empty_string,
            empty_string,
            empty_string,
            false,
            NativeSlice {
                data: output_small.as_mut_ptr(),
                len: output_small.len() as u32,
            },
            &mut output_written,
            &mut created_unix_ns,
            &mut modified_unix_ns,
        )
    };
    assert_eq!(status, HostStatus::BufferTooSmall.code());
    assert_eq!(output_written, 4);

    let mut output = [0u8; 8];
    let mut output_written = 0u32;
    let mut created_unix_ns = 0u64;
    let mut modified_unix_ns = 0u64;
    let status = unsafe {
        destack_host_android_credentials_read(
            runtime_id,
            empty_string,
            empty_string,
            empty_string,
            false,
            NativeSlice {
                data: output.as_mut_ptr(),
                len: output.len() as u32,
            },
            &mut output_written,
            &mut created_unix_ns,
            &mut modified_unix_ns,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(output_written, 4);
    assert_eq!(&output[..4], &[1, 2, 3, 4]);
    assert_eq!(created_unix_ns, 11);
    assert_eq!(modified_unix_ns, 22);
}

/// Reject duplicate Android credentials callback registration for one runtime.
#[test]
fn test_register_bindings_rejects_duplicate_registration() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let first_callbacks = AndroidHostCredentialsCallbacks {
        contains: Some(test_contains),
        ..AndroidHostCredentialsCallbacks::default()
    };
    let first_status = register_android_bindings_credentials(runtime_id, first_callbacks);
    assert_eq!(first_status, HostStatus::Ok.code());

    let second_callbacks = AndroidHostCredentialsCallbacks {
        contains: Some(test_contains),
        ..AndroidHostCredentialsCallbacks::default()
    };
    let second_status = register_android_bindings_credentials(runtime_id, second_callbacks);
    assert_eq!(second_status, HostStatus::Failed.code());
}

/// Return not-supported when one requested Android credentials lane is missing.
#[test]
fn test_register_bindings_missing_callback_reports_not_supported() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let callbacks = AndroidHostCredentialsCallbacks {
        read: Some(test_read),
        ..AndroidHostCredentialsCallbacks::default()
    };
    let register_status = register_android_bindings_credentials(runtime_id, callbacks);
    assert_eq!(register_status, HostStatus::Ok.code());

    let empty_string = NativeStringRef {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let mut authenticated = false;
    let mut mechanism = 0;
    let status = unsafe {
        destack_host_android_credentials_authenticate(
            runtime_id,
            empty_string,
            empty_string,
            empty_string,
            1,
            &mut authenticated,
            &mut mechanism,
        )
    };

    assert_eq!(status, HostStatus::NotSupported.code());
}

/// Reject Android credentials reads with null output pointers.
#[test]
fn test_read_rejects_null_output_pointers() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let empty_string = NativeStringRef {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let status = unsafe {
        destack_host_android_credentials_read(
            runtime_id,
            empty_string,
            empty_string,
            empty_string,
            false,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    assert_eq!(status, HostStatus::InvalidArgument.code());
}

/// Reject Android credentials contains calls with one null output pointer.
#[test]
fn test_contains_rejects_null_output_pointer() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let empty_string = NativeStringRef {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let status = unsafe {
        destack_host_android_credentials_contains(
            runtime_id,
            empty_string,
            empty_string,
            empty_string,
            std::ptr::null_mut(),
        )
    };

    assert_eq!(status, HostStatus::InvalidArgument.code());
}

/// Reject Android credentials authentication with null output pointers.
#[test]
fn test_authenticate_rejects_null_output_pointers() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let empty_string = NativeStringRef {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let status = unsafe {
        destack_host_android_credentials_authenticate(
            runtime_id,
            empty_string,
            empty_string,
            empty_string,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    assert_eq!(status, HostStatus::InvalidArgument.code());
}
