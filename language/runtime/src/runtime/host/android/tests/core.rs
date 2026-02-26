use std::sync::{Arc, Mutex, OnceLock};

use crate::runtime::host::core::{
    HostBridge, HostBridgeRegistration, HostStateStore, register_host_bridge,
};
use crate::runtime::host::{
    AndroidHostBindings, AndroidHostCredentialsCallbacks, AndroidHostCryptoCallbacks, HostPlatform,
    destack_runtime_host_android_register_bindings,
};

/// Return the shared test lock for Android bindings registration.
pub(crate) fn callback_test_lock() -> &'static Mutex<()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    TEST_LOCK.get_or_init(|| Mutex::new(()))
}

/// Register one temporary Android bridge and keep registration state alive.
pub(crate) fn register_android_runtime() -> (Arc<HostBridge>, HostBridgeRegistration, u64) {
    let state_store = Arc::new(HostStateStore::new());
    let bridge = Arc::new(HostBridge::new(state_store));
    let registration = register_host_bridge(HostPlatform::Android, &bridge);
    let runtime_id = registration.runtime_id();

    (bridge, registration, runtime_id)
}

/// Register one runtime-scoped Android host bindings payload.
pub(crate) fn register_android_bindings(runtime_id: u64, bindings: AndroidHostBindings) -> u32 {
    unsafe { destack_runtime_host_android_register_bindings(runtime_id, bindings) }
}

/// Register one runtime-scoped Android credentials callback payload.
pub(crate) fn register_android_bindings_credentials(
    runtime_id: u64,
    callbacks: AndroidHostCredentialsCallbacks,
) -> u32 {
    register_android_bindings(
        runtime_id,
        AndroidHostBindings {
            credentials: callbacks,
            ..AndroidHostBindings::default()
        },
    )
}

/// Register one runtime-scoped Android crypto callback payload.
pub(crate) fn register_android_bindings_crypto(
    runtime_id: u64,
    callbacks: AndroidHostCryptoCallbacks,
) -> u32 {
    register_android_bindings(
        runtime_id,
        AndroidHostBindings {
            crypto: callbacks,
            ..AndroidHostBindings::default()
        },
    )
}
