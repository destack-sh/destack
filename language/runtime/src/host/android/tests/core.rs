use std::sync::{Arc, Mutex, OnceLock};

use crate::host::android::unregister_android_bindings;
use crate::host::core::registry::HostRegistrationGuard;
use crate::host::core::{HostQueue, HostQueueRegistry};
use crate::host::{
    AndroidHostBindings, AndroidHostCredentialsCallbacks, AndroidHostCryptoCallbacks, Platform,
    destack_host_android_register_bindings,
};
use crate::runtime::world::RuntimeId;

/// Return the shared test lock for Android bindings registration.
pub(crate) fn callback_test_lock() -> &'static Mutex<()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    TEST_LOCK.get_or_init(|| Mutex::new(()))
}

/// Register one temporary Android host queue and keep registration state alive.
pub(crate) fn register_android_runtime() -> (Arc<HostQueue>, HostRegistrationGuard, u64) {
    let queue = Arc::new(HostQueue::new());
    let registration = HostQueueRegistry::shared().write().register(
        Platform::Android,
        RuntimeId(1),
        Arc::downgrade(&queue),
        Some(unregister_android_bindings),
    );
    let runtime_id = registration.runtime_id().0;

    (queue, registration, runtime_id)
}

/// Register one runtime-scoped Android host bindings payload.
pub(crate) fn register_android_bindings(runtime_id: u64, bindings: AndroidHostBindings) -> u32 {
    unsafe { destack_host_android_register_bindings(runtime_id, bindings) }
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
