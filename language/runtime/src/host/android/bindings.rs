use super::abi::HOST_STATUS_NOT_SUPPORTED;
use super::credentials::AndroidHostCredentialsCallbacks;
use super::crypto::AndroidHostCryptoCallbacks;
use super::midi::AndroidHostMidiCallbacks;
use super::registry::{register_android_bindings, resolve_android_bindings};

/// Android host bindings container for callback-backed lanes.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AndroidHostBindings {
    /// Credentials host callbacks.
    pub credentials: AndroidHostCredentialsCallbacks,
    /// Crypto host callbacks.
    pub crypto: AndroidHostCryptoCallbacks,
    /// MIDI host callbacks.
    pub midi: AndroidHostMidiCallbacks,
}

/// Register one callback table for Android host interop through one C ABI entrypoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_register_bindings(
    runtime_id: u64,
    bindings: AndroidHostBindings,
) -> u32 {
    register_android_bindings(runtime_id, bindings)
}

/// Resolve one callback from one Android host bindings lane.
pub(super) fn resolve_android_binding_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostBindings) -> Option<T>,
) -> Result<T, u32> {
    // resolve one runtime-scoped bindings snapshot
    let bindings = resolve_android_bindings(runtime_id)?;

    // resolve one callback and report unsupported lanes explicitly
    resolve(&bindings).ok_or(HOST_STATUS_NOT_SUPPORTED)
}

/// Resolve and invoke one Android host callback.
pub(super) fn invoke_android_binding_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostBindings) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    // route one callback and return status for unsupported or unknown runtimes
    let callback = match resolve_android_binding_callback(runtime_id, resolve) {
        Ok(callback) => callback,
        Err(status) => return status,
    };

    // invoke one resolved callback
    invoke(callback)
}
