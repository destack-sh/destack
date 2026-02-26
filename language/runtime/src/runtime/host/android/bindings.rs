use std::sync::Arc;

use super::abi::{
    HOST_STATUS_FAILED, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
};
use super::credentials::AndroidHostCredentialsCallbacks;
use super::crypto::AndroidHostCryptoCallbacks;
use crate::runtime::host::HostPlatform;
use crate::runtime::host::core::{HostBridge, host_bridge_for_runtime};

/// Android host bindings container for callback-backed lanes.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AndroidHostBindings {
    /// Credentials host callbacks.
    pub credentials: AndroidHostCredentialsCallbacks,
    /// Crypto host callbacks.
    pub crypto: AndroidHostCryptoCallbacks,
}

/// Resolve one Android host bridge from one runtime id.
fn android_host_bridge(runtime_id: u64) -> Result<Arc<HostBridge>, u32> {
    // resolve one runtime-scoped bridge for android host routing
    match host_bridge_for_runtime(runtime_id, HostPlatform::Android) {
        Ok(bridge) => Ok(bridge),
        Err(_) => Err(HOST_STATUS_NOT_FOUND),
    }
}

/// Register one Android host bindings payload.
fn register_android_host_bindings(runtime_id: u64, bindings: AndroidHostBindings) -> u32 {
    // resolve one runtime-scoped host bridge
    let bridge = match android_host_bridge(runtime_id) {
        Ok(bridge) => bridge,
        Err(status) => return status,
    };

    // register one runtime-scoped Android bindings payload
    if !bridge.register_android_bindings(bindings) {
        return HOST_STATUS_FAILED;
    }

    HOST_STATUS_OK
}

/// Register one callback table for Android host interop through one C ABI entrypoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_register_bindings(
    runtime_id: u64,
    bindings: AndroidHostBindings,
) -> u32 {
    register_android_host_bindings(runtime_id, bindings)
}

/// Return one Android host crypto callback snapshot.
pub fn android_host_crypto_callbacks_snapshot(
    runtime_id: u64,
) -> Option<AndroidHostCryptoCallbacks> {
    // resolve one runtime-scoped host bridge
    let bridge = android_host_bridge(runtime_id).ok()?;

    // copy one stable callback table snapshot for one runtime-scoped lane
    Some(bridge.android_bindings()?.crypto)
}

/// Resolve one callback from one Android host bindings lane.
pub(super) fn route_android_binding_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostBindings) -> Option<T>,
) -> Result<T, u32> {
    // resolve one runtime-scoped host bridge
    let bridge = android_host_bridge(runtime_id)?;
    let Some(bindings) = bridge.android_bindings() else {
        return Err(HOST_STATUS_NOT_SUPPORTED);
    };

    // resolve one callback and report unsupported lanes explicitly
    resolve(&bindings).ok_or(HOST_STATUS_NOT_SUPPORTED)
}

/// Resolve and invoke one Android host callback.
pub(super) fn call_android_binding_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostBindings) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    // route one callback and return status for unsupported or unknown runtimes
    let callback = match route_android_binding_callback(runtime_id, resolve) {
        Ok(callback) => callback,
        Err(status) => return status,
    };

    // invoke one resolved callback
    invoke(callback)
}
