use crate::host::android::abi::{HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK};
use crate::host::android::bridge::bindings::{
    invoke_android_binding_callback, resolve_android_binding_callback,
};
use crate::host::android::bridge::crypto::callbacks::{
    AndroidHostCryptoCallbacks, HOST_KEY_ALGORITHM_AES, HOST_KEY_ALGORITHM_EC,
    HOST_KEY_ALGORITHM_HMAC, HOST_KEY_ALGORITHM_RSA, HOST_STORE_KIND_MACHINE,
    HOST_STORE_KIND_SYSTEM, HOST_STORE_KIND_USER,
};

/// Resolve one runtime-scoped Android crypto callback table.
pub(super) fn resolve_android_crypto_callbacks(
    runtime_id: u64,
) -> Result<AndroidHostCryptoCallbacks, u32> {
    resolve_android_binding_callback(runtime_id, |bindings| Some(bindings.crypto))
}

/// Return whether one callback table supports one key-pair lane.
pub(super) fn has_hardware_key_pair_lane(
    callbacks: &AndroidHostCryptoCallbacks,
    key_algorithm: u32,
) -> bool {
    let has_shared_pair_callbacks = callbacks.generate_hardware_key_pair.is_some()
        && callbacks.export_hardware_public_key.is_some()
        && callbacks.sign_hardware_key.is_some()
        && callbacks.delete_hardware_key.is_some();
    if !has_shared_pair_callbacks {
        return false;
    }

    match key_algorithm {
        HOST_KEY_ALGORITHM_RSA => callbacks.decrypt_hardware_key.is_some(),
        HOST_KEY_ALGORITHM_EC => callbacks.derive_hardware_shared_secret.is_some(),
        _ => false,
    }
}

/// Return whether one callback table supports one secret-key lane.
pub(super) fn has_hardware_secret_key_lane(
    callbacks: &AndroidHostCryptoCallbacks,
    key_algorithm: u32,
) -> bool {
    let has_shared_secret_callbacks =
        callbacks.generate_hardware_secret_key.is_some() && callbacks.delete_hardware_key.is_some();
    if !has_shared_secret_callbacks {
        return false;
    }

    match key_algorithm {
        HOST_KEY_ALGORITHM_AES => {
            callbacks.encrypt_hardware_secret_key.is_some()
                && callbacks.decrypt_hardware_secret_key.is_some()
        }
        HOST_KEY_ALGORITHM_HMAC => callbacks.compute_hardware_mac.is_some(),
        _ => false,
    }
}

/// Return whether one callback table supports certificate writes.
pub(super) fn supports_certificate_write(
    callbacks: &AndroidHostCryptoCallbacks,
    store_kind: u32,
) -> bool {
    if !matches!(
        store_kind,
        HOST_STORE_KIND_SYSTEM | HOST_STORE_KIND_USER | HOST_STORE_KIND_MACHINE
    ) {
        return false;
    }

    callbacks.import_certificate.is_some() && callbacks.delete_certificate.is_some()
}

/// Resolve and invoke one Android host crypto callback.
pub(super) fn call_android_crypto_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostCryptoCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.crypto), invoke)
}

/// Probe one Android host lane for one explicit hardware-support callback.
pub(super) fn probe_hardware_support(
    runtime_id: u64,
    store_kind: u32,
    callbacks: AndroidHostCryptoCallbacks,
) -> u32 {
    let Some(callback) = callbacks.supports_hardware_key else {
        return HOST_STATUS_OK;
    };

    unsafe { callback(runtime_id, store_kind) }
}

/// Report that one runtime lacks full callback coverage for the requested lane.
pub(super) const fn unsupported_lane_status() -> u32 {
    HOST_STATUS_NOT_SUPPORTED
}
