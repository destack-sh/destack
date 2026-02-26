use super::super::abi::HOST_STATUS_BUFFER_TOO_SMALL;
use super::super::tests::{callback_test_lock, register_android_runtime};
use super::{
    ANDROID_HOST_CREDENTIALS_CALLBACKS_ABI_VERSION, AndroidHostCredentialsCallbacks,
    HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    clear_android_host_credentials_callbacks,
    destack_runtime_host_android_credentials_authenticate,
    destack_runtime_host_android_credentials_contains,
    destack_runtime_host_android_credentials_read,
    destack_runtime_host_android_credentials_set_callbacks,
};
use crate::platform::{NativeSlice, NativeStringRef};

/// Mechanism code for one biometric host authentication result.
const TEST_AUTHENTICATION_MECHANISM_BIOMETRIC: u32 = 2;

/// Report one positive contains result in callback tests.
unsafe extern "C" fn test_contains(
    _runtime_id: u64,
    _service: NativeStringRef,
    _account: NativeStringRef,
    is_present: *mut bool,
) -> u32 {
    if is_present.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    unsafe {
        *is_present = true;
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
    clear_android_host_credentials_callbacks();
    let (_bridge, _registration, runtime_id) = register_android_runtime();
    let mut is_present = false;

    // verify contains reports not-supported when runtime-id is valid but callbacks are unset
    let status = unsafe {
        destack_runtime_host_android_credentials_contains(
            runtime_id,
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
fn test_set_callbacks_routes_calls() {
    let _lock = callback_test_lock().lock().unwrap();
    clear_android_host_credentials_callbacks();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let callbacks = AndroidHostCredentialsCallbacks {
        abi_version: ANDROID_HOST_CREDENTIALS_CALLBACKS_ABI_VERSION,
        read: Some(test_read),
        contains: Some(test_contains),
        authenticate: Some(test_authenticate),
        ..AndroidHostCredentialsCallbacks::default()
    };
    let status = unsafe { destack_runtime_host_android_credentials_set_callbacks(callbacks) };
    assert_eq!(status, HOST_STATUS_OK);

    let empty_string = NativeStringRef {
        data: std::ptr::null_mut(),
        len: 0,
    };

    let mut is_present = false;
    let status = unsafe {
        destack_runtime_host_android_credentials_contains(
            runtime_id,
            empty_string,
            empty_string,
            &mut is_present,
        )
    };
    assert_eq!(status, HOST_STATUS_OK);
    assert!(is_present);

    let mut authenticated = false;
    let mut mechanism = 0;
    let status = unsafe {
        destack_runtime_host_android_credentials_authenticate(
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
        destack_runtime_host_android_credentials_read(
            runtime_id,
            empty_string,
            empty_string,
            empty_string,
            false,
            crate::platform::NativeSlice {
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
        destack_runtime_host_android_credentials_read(
            runtime_id,
            empty_string,
            empty_string,
            empty_string,
            false,
            crate::platform::NativeSlice {
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

    clear_android_host_credentials_callbacks();
}

#[test]
fn test_read_rejects_null_output_pointers() {
    let _lock = callback_test_lock().lock().unwrap();
    clear_android_host_credentials_callbacks();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let empty_string = NativeStringRef {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let status = unsafe {
        destack_runtime_host_android_credentials_read(
            runtime_id,
            empty_string,
            empty_string,
            empty_string,
            false,
            crate::platform::NativeSlice {
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
fn test_set_callbacks_rejects_unknown_abi_version() {
    let _lock = callback_test_lock().lock().unwrap();
    clear_android_host_credentials_callbacks();

    let callbacks = AndroidHostCredentialsCallbacks {
        abi_version: 99,
        ..AndroidHostCredentialsCallbacks::default()
    };
    let status = unsafe { destack_runtime_host_android_credentials_set_callbacks(callbacks) };
    assert_eq!(status, HOST_STATUS_INVALID_ARGUMENT);
}
