use std::sync::{Arc, Mutex, OnceLock};

use crate::host::core::registry::HostSessionRegistrationGuard;
use crate::host::os::android::abi::binding::AndroidHostBindings;
use crate::host::os::android::abi::registry::{
    register_android_bindings as register_android_bindings_payload, unregister_android_bindings,
};
use crate::host::{HostQueue, HostSessionId, HostSessionRegistry, Platform};

/// Return the shared test lock for Android bindings registration.
pub(crate) fn callback_test_lock() -> &'static Mutex<()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    TEST_LOCK.get_or_init(|| Mutex::new(()))
}

/// Register one temporary Android host queue and keep registration state alive.
pub(crate) fn register_android_runtime() -> (Arc<HostQueue>, HostSessionRegistrationGuard, u64) {
    // allocate one host queue and register it under one unique Android runtime id
    let runtime_id = HostSessionRegistry::allocate_session_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostSessionRegistry::register_queue(
        Platform::Android,
        runtime_id,
        Arc::clone(&queue),
        Some(unregister_android_bindings_for_runtime),
    );
    let runtime_id = registration.host_session_id().0;

    (queue, registration, runtime_id)
}

/// Unregister one Android bindings payload for one host runtime id.
fn unregister_android_bindings_for_runtime(runtime_id: HostSessionId) {
    unregister_android_bindings(runtime_id.0);
}

/// Register one runtime-scoped Android host bindings payload.
pub(crate) fn register_android_bindings(runtime_id: u64, bindings: AndroidHostBindings) -> u32 {
    register_android_bindings_payload(runtime_id, bindings)
}
