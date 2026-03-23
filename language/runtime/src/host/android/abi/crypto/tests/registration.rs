use crate::host::android::abi::crypto::ffi::destack_host_android_crypto_supports_hardware_key;
use crate::host::android::abi::crypto::tests::core::{
    default_callbacks, probe_callbacks, support_only_callbacks,
};
use crate::host::android::tests::{
    callback_test_lock, register_android_bindings_crypto, register_android_runtime,
};
use crate::host::{
    HOST_STATUS_FAILED, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
};

/// Return not-supported when Android crypto callbacks are not registered.
#[test]
fn test_default_callbacks_return_not_supported() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let support_status =
        unsafe { destack_host_android_crypto_supports_hardware_key(runtime_id, 2) };
    assert_eq!(support_status, HOST_STATUS_NOT_SUPPORTED);
}

/// Reject Android crypto callback registration for one unknown runtime.
#[test]
fn test_register_bindings_rejects_unknown_runtime() {
    let _lock = callback_test_lock().lock().unwrap();

    let status = register_android_bindings_crypto(0, default_callbacks());

    assert_eq!(status, HOST_STATUS_NOT_FOUND);
}

/// Reject duplicate Android crypto callback registration.
#[test]
fn test_register_bindings_rejects_duplicate_registration() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, _registration, runtime_id) = register_android_runtime();

    let first_status = register_android_bindings_crypto(runtime_id, probe_callbacks());
    assert_eq!(first_status, HOST_STATUS_OK);

    let duplicate_status = register_android_bindings_crypto(runtime_id, support_only_callbacks());
    assert_eq!(duplicate_status, HOST_STATUS_FAILED);
}
