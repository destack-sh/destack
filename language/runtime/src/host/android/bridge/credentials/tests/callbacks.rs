use crate::host::android::abi::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
};
use crate::host::android::bridge::credentials::{
    AndroidHostCredentialsCallbacks, destack_host_android_credentials_authenticate,
    destack_host_android_credentials_contains, destack_host_android_credentials_delete,
    destack_host_android_credentials_read, destack_host_android_credentials_write,
};
use crate::host::android::tests::{
    callback_test_lock, register_android_bindings_credentials, register_android_runtime,
};
use crate::runtime::{NativeSlice, NativeStringRef};

/// Mechanism code for one biometric host authentication result.
const TEST_AUTHENTICATION_MECHANISM_BIOMETRIC: u32 = 2;
/// Access-group value expected by contains callback tests.
const TEST_ACCESS_GROUP: &str = "group.identifier";

/// Report one positive contains result in callback tests.
unsafe extern "C" fn test_contains(
    _runtime_id: u64,
    _service: NativeStringRef,
    _account: NativeStringRef,
    access_group: NativeStringRef,
    is_present: *mut bool,
) -> u32 {
    if is_present.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    let access_group = match unsafe { access_group.as_str() } {
        Ok(value) => value,
        Err(_) => return HOST_STATUS_INVALID_ARGUMENT,
    };
    if access_group != TEST_ACCESS_GROUP {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    unsafe {
        *is_present = true;
    }

    HOST_STATUS_OK
}

/// Report one successful delete when the expected access-group is forwarded.
unsafe extern "C" fn test_delete(
    _runtime_id: u64,
    _service: NativeStringRef,
    _account: NativeStringRef,
    access_group: NativeStringRef,
) -> u32 {
    let access_group = match unsafe { access_group.as_str() } {
        Ok(value) => value,
        Err(_) => return HOST_STATUS_INVALID_ARGUMENT,
    };
    if access_group != TEST_ACCESS_GROUP {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    HOST_STATUS_OK
}

/// Report one successful authentication result in callback tests.
unsafe extern "C" fn test_authenticate(
    _runtime_id: u64,
    _title: NativeStringRef,
    _subtitle: NativeStringRef,
    _message: NativeStringRef,
    _requirement: u32,
    authenticated: *mut bool,
    mechanism: *mut u32,
) -> u32 {
    if authenticated.is_null() || mechanism.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    unsafe {
        *authenticated = true;
        *mechanism = TEST_AUTHENTICATION_MECHANISM_BIOMETRIC;
    }

    HOST_STATUS_OK
}

/// Report one successful write in callback tests.
unsafe extern "C" fn test_write(
    _runtime_id: u64,
    _service: NativeStringRef,
    _account: NativeStringRef,
    _access_group: NativeStringRef,
    payload: NativeSlice<u8>,
    _accessibility: u32,
    _authentication_policy: u32,
    _replace_existing: bool,
) -> u32 {
    if payload.data.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    HOST_STATUS_OK
}

/// Return one deterministic read payload in callback tests.
unsafe extern "C" fn test_read(
    _runtime_id: u64,
    _service: NativeStringRef,
    _account: NativeStringRef,
    _access_group: NativeStringRef,
    _require_authentication: bool,
    output: NativeSlice<u8>,
    output_written: *mut u32,
    created_unix_ns: *mut u64,
    modified_unix_ns: *mut u64,
) -> u32 {
    let payload = [1u8, 2, 3, 4];

    if output_written.is_null() || created_unix_ns.is_null() || modified_unix_ns.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    if output.data.is_null() || output.len < payload.len() as u32 {
        unsafe {
            *output_written = payload.len() as u32;
        }
        return HOST_STATUS_BUFFER_TOO_SMALL;
    }

    let output_bytes = unsafe { std::slice::from_raw_parts_mut(output.data, output.len as usize) };
    output_bytes[..payload.len()].copy_from_slice(&payload);

    unsafe {
        *output_written = payload.len() as u32;
        *created_unix_ns = 11;
        *modified_unix_ns = 22;
    }

    HOST_STATUS_OK
}

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
    assert_eq!(status, HOST_STATUS_NOT_SUPPORTED);
}

#[test]
fn test_register_bindings_rejects_unknown_runtime() {
    let _lock = callback_test_lock().lock().unwrap();

    let callbacks = AndroidHostCredentialsCallbacks::default();
    let status = register_android_bindings_credentials(0, callbacks);

    assert_eq!(status, HOST_STATUS_NOT_FOUND);
}

#[test]
fn test_register_bindings_routes_calls() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let callbacks = AndroidHostCredentialsCallbacks {
        read: Some(test_read),
        write: Some(test_write),
        delete: Some(test_delete),
        contains: Some(test_contains),
        authenticate: Some(test_authenticate),
    };
    let status = register_android_bindings_credentials(runtime_id, callbacks);
    assert_eq!(status, HOST_STATUS_OK);

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
    assert_eq!(status, HOST_STATUS_OK);
    assert!(is_present);

    let status = unsafe {
        destack_host_android_credentials_delete(
            runtime_id,
            empty_string,
            empty_string,
            access_group,
        )
    };
    assert_eq!(status, HOST_STATUS_OK);

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
    assert_eq!(status, HOST_STATUS_OK);

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
    assert_eq!(status, HOST_STATUS_OK);
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
    assert_eq!(status, HOST_STATUS_BUFFER_TOO_SMALL);
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
    assert_eq!(status, HOST_STATUS_OK);
    assert_eq!(output_written, 4);
    assert_eq!(&output[..4], &[1, 2, 3, 4]);
    assert_eq!(created_unix_ns, 11);
    assert_eq!(modified_unix_ns, 22);
}

#[test]
fn test_register_bindings_rejects_duplicate_registration() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let first_callbacks = AndroidHostCredentialsCallbacks {
        contains: Some(test_contains),
        ..AndroidHostCredentialsCallbacks::default()
    };
    let first_status = register_android_bindings_credentials(runtime_id, first_callbacks);
    assert_eq!(first_status, HOST_STATUS_OK);

    let second_callbacks = AndroidHostCredentialsCallbacks {
        contains: Some(test_contains),
        ..AndroidHostCredentialsCallbacks::default()
    };
    let second_status = register_android_bindings_credentials(runtime_id, second_callbacks);
    assert_eq!(second_status, HOST_STATUS_FAILED);
}

#[test]
fn test_register_bindings_missing_callback_reports_not_supported() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let callbacks = AndroidHostCredentialsCallbacks {
        read: Some(test_read),
        ..AndroidHostCredentialsCallbacks::default()
    };
    let register_status = register_android_bindings_credentials(runtime_id, callbacks);
    assert_eq!(register_status, HOST_STATUS_OK);

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

    assert_eq!(status, HOST_STATUS_NOT_SUPPORTED);
}

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

    assert_eq!(status, HOST_STATUS_INVALID_ARGUMENT);
}

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

    assert_eq!(status, HOST_STATUS_INVALID_ARGUMENT);
}

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

    assert_eq!(status, HOST_STATUS_INVALID_ARGUMENT);
}
