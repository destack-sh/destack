use crate::host::android::abi::crypto::ffi::{
    destack_host_android_crypto_delete_certificate, destack_host_android_crypto_import_certificate,
    destack_host_android_crypto_supports_certificate_write,
};
use crate::host::android::abi::crypto::tests::core::{
    certificate_callbacks, partial_certificate_callbacks,
};
use crate::host::android::tests::{
    callback_test_lock, register_android_bindings_crypto, register_android_runtime,
};
use crate::host::{HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK};
use crate::runtime::NativeSlice;

/// Route certificate operations through the registered callback table.
#[test]
fn test_register_bindings_routes_certificate_calls() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();
    let register_status = register_android_bindings_crypto(runtime_id, certificate_callbacks());
    assert_eq!(register_status, HOST_STATUS_OK);

    let import_status = unsafe {
        destack_host_android_crypto_import_certificate(
            runtime_id,
            2,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
        )
    };
    assert_eq!(import_status, HOST_STATUS_OK);

    let delete_status = unsafe {
        destack_host_android_crypto_delete_certificate(
            runtime_id,
            2,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
        )
    };
    assert_eq!(delete_status, HOST_STATUS_OK);
}

/// Reject partial certificate callback coverage for support probing.
#[test]
fn test_certificate_write_probe_requires_both_callbacks() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();
    let register_status =
        register_android_bindings_crypto(runtime_id, partial_certificate_callbacks());
    assert_eq!(register_status, HOST_STATUS_OK);

    let partial_status =
        unsafe { destack_host_android_crypto_supports_certificate_write(runtime_id, 2) };
    assert_eq!(partial_status, HOST_STATUS_NOT_SUPPORTED);
}
