use std::sync::{Arc, Mutex, OnceLock};

use crate::runtime::host::HostPlatform;
use crate::runtime::host::core::{
    HostBridge, HostBridgeRegistration, HostStateStore, register_host_bridge,
};

/// Return the shared test lock for callback-registry mutations.
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
