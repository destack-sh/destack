use super::super::registry::android_bindings_contains_runtime_id;
use super::core::{callback_test_lock, register_android_bindings, register_android_runtime};
use crate::host::{AndroidHostBindings, HOST_STATUS_OK};

#[test]
fn test_drop_runtime_bridge_unregisters_android_bindings_eagerly() {
    let _lock = callback_test_lock().lock().unwrap();
    let (_bridge, registration, runtime_id) = register_android_runtime();

    // register one android bindings payload for this runtime id
    let status = register_android_bindings(runtime_id, AndroidHostBindings::default());
    assert_eq!(status, HOST_STATUS_OK);
    assert!(android_bindings_contains_runtime_id(runtime_id));

    // drop runtime registration and assert eager registry cleanup
    drop(registration);
    assert!(!android_bindings_contains_runtime_id(runtime_id));
}
