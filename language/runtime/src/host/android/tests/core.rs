use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::host::Platform;
use crate::host::android::bluetooth::types::AndroidHostBluetoothCallbacks;
use crate::host::android::bridge::bindings::{
    AndroidHostBindings, destack_host_android_register_bindings,
};
use crate::host::android::bridge::credentials::AndroidHostCredentialsCallbacks;
use crate::host::android::bridge::crypto::AndroidHostCryptoCallbacks;
use crate::host::android::bridge::midi::types::AndroidHostMidiCallbacks;
use crate::host::android::camera::types::AndroidHostCameraCallbacks;
use crate::host::android::unregister_android_bindings;
use crate::host::core::registry::HostRegistrationGuard;
use crate::host::core::{HostQueue, HostRuntimeRegistry};
use crate::runtime::world::RuntimeId;

/// Shared runtime-id allocator for Android host tests.
static TEST_RUNTIME_ID_NEXT: AtomicU64 = AtomicU64::new(u64::MAX - 20_480);

/// Return the shared test lock for Android bindings registration.
pub(crate) fn callback_test_lock() -> &'static Mutex<()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    TEST_LOCK.get_or_init(|| Mutex::new(()))
}

/// Allocate one unique runtime id for this test process.
fn next_test_runtime_id() -> RuntimeId {
    RuntimeId(TEST_RUNTIME_ID_NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Register one temporary Android host queue and keep registration state alive.
pub(crate) fn register_android_runtime() -> (Arc<HostQueue>, HostRegistrationGuard, u64) {
    // allocate one host queue and register it under one unique Android runtime id
    let runtime_id = next_test_runtime_id();
    let queue = Arc::new(HostQueue::new(runtime_id));
    let registration = HostRuntimeRegistry::register_queue(
        Platform::Android,
        runtime_id,
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
    // install one credentials-only callback lane
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
    // install one crypto-only callback lane
    register_android_bindings(
        runtime_id,
        AndroidHostBindings {
            crypto: callbacks,
            ..AndroidHostBindings::default()
        },
    )
}

/// Register one runtime-scoped Android MIDI callback payload.
pub(crate) fn register_android_bindings_midi(
    runtime_id: u64,
    callbacks: AndroidHostMidiCallbacks,
) -> u32 {
    // install one midi-only callback lane
    register_android_bindings(
        runtime_id,
        AndroidHostBindings {
            midi: callbacks,
            ..AndroidHostBindings::default()
        },
    )
}

/// Register one runtime-scoped Android Bluetooth callback payload.
pub(crate) fn register_android_bindings_bluetooth(
    runtime_id: u64,
    callbacks: AndroidHostBluetoothCallbacks,
) -> u32 {
    // install one bluetooth-only callback lane
    register_android_bindings(
        runtime_id,
        AndroidHostBindings {
            bluetooth: callbacks,
            ..AndroidHostBindings::default()
        },
    )
}

/// Register one runtime-scoped Android camera callback payload.
pub(crate) fn register_android_bindings_camera(
    runtime_id: u64,
    callbacks: AndroidHostCameraCallbacks,
) -> u32 {
    // install one camera-only callback lane
    register_android_bindings(
        runtime_id,
        AndroidHostBindings {
            camera: callbacks,
            ..AndroidHostBindings::default()
        },
    )
}
