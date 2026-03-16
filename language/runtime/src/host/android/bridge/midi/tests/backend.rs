use crate::host::android::abi::HOST_STATUS_OK;
use crate::host::android::bridge::midi::ffi::destack_host_android_midi_describe_backend;
use crate::host::android::bridge::midi::tests::core::{
    TEST_CAPABILITY_FLAGS, TEST_DATA_FORMATS, TEST_PROTOCOLS, TEST_STATUS_NOT_FOUND,
    TEST_STATUS_NOT_SUPPORTED, lock_test_callbacks, register_test_callbacks,
};
use crate::host::android::bridge::midi::types::AndroidHostMidiCallbacks;
use crate::host::android::tests::{register_android_bindings_midi, register_android_runtime};

/// Return not-supported when Android MIDI callbacks are not registered.
#[test]
fn test_default_callbacks_return_not_supported() {
    let _lock = lock_test_callbacks();
    let (_queue, _registration, runtime_id) = register_android_runtime();
    let mut capability_flags = 0u64;
    let mut data_formats = 0u32;
    let mut protocols = 0u32;

    // verify describe reports not-supported when callbacks are unset
    let status = unsafe {
        destack_host_android_midi_describe_backend(
            runtime_id,
            &mut capability_flags,
            &mut data_formats,
            &mut protocols,
        )
    };

    assert_eq!(status, TEST_STATUS_NOT_SUPPORTED);
}

/// Reject Android MIDI callback registration for one unknown runtime.
#[test]
fn test_register_bindings_rejects_unknown_runtime() {
    let _lock = lock_test_callbacks();

    let status = register_android_bindings_midi(0, AndroidHostMidiCallbacks::default());

    assert_eq!(status, TEST_STATUS_NOT_FOUND);
}

/// Route backend description through the registered Android MIDI callback table.
#[test]
fn test_register_bindings_routes_backend_describe() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut capability_flags = 0u64;
    let mut data_formats = 0u32;
    let mut protocols = 0u32;

    // verify backend probing routes through the registered callback
    let status = unsafe {
        destack_host_android_midi_describe_backend(
            callbacks.runtime_id,
            &mut capability_flags,
            &mut data_formats,
            &mut protocols,
        )
    };

    assert_eq!(status, HOST_STATUS_OK);
    assert_eq!(capability_flags, TEST_CAPABILITY_FLAGS);
    assert_eq!(data_formats, TEST_DATA_FORMATS);
    assert_eq!(protocols, TEST_PROTOCOLS);
}
