use parking_lot::RwLock;
use std::sync::OnceLock;

use super::abi::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND,
    HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
};
use crate::platform::{NativeSlice, NativeStringRef};
use crate::runtime::host::HostPlatform;
use crate::runtime::host::core::host_bridge_for_runtime;

/// ABI version for the Android host-crypto callback table.
pub(super) const ANDROID_HOST_CRYPTO_CALLBACKS_ABI_VERSION: u32 = 1;

/// Host callback for probing one hardware-backed key lane.
pub type AndroidHostSupportsHardwareKeyCallback =
    unsafe extern "C" fn(runtime_id: u64, store_kind: u32) -> u32;
/// Host callback for generating one hardware-backed key pair.
pub type AndroidHostGenerateHardwareKeyPairCallback = unsafe extern "C" fn(
    runtime_id: u64,
    store_kind: u32,
    key_algorithm: u32,
    named_curve: u32,
    modulus_bits: u32,
    public_exponent: u32,
    key_label: NativeStringRef,
) -> u32;
/// Host callback for exporting one hardware-backed public key.
pub type AndroidHostExportHardwarePublicKeyCallback = unsafe extern "C" fn(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;
/// Host callback for signing one payload with one hardware-backed key.
pub type AndroidHostSignHardwareKeyCallback = unsafe extern "C" fn(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    signature_algorithm: u32,
    digest_algorithm: u32,
    salt_length_bytes: u32,
    payload: NativeSlice<u8>,
    output_signature: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;
/// Host callback for decrypting one payload with one hardware-backed key.
pub type AndroidHostDecryptHardwareKeyCallback = unsafe extern "C" fn(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    encryption_algorithm: u32,
    digest_algorithm: u32,
    label: NativeSlice<u8>,
    payload: NativeSlice<u8>,
    output_plaintext: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;
/// Host callback for deriving one shared secret with one hardware-backed key.
pub type AndroidHostDeriveHardwareSharedSecretCallback = unsafe extern "C" fn(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    named_curve: u32,
    peer_public_spki: NativeSlice<u8>,
    output_shared_secret: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;
/// Host callback for deleting one hardware-backed key.
pub type AndroidHostDeleteHardwareKeyCallback =
    unsafe extern "C" fn(runtime_id: u64, key_algorithm: u32, key_label: NativeStringRef) -> u32;

/// Callback table for Android host crypto interop.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AndroidHostCryptoCallbacks {
    /// ABI version for this callback table.
    pub abi_version: u32,
    /// Probe callback for one hardware-backed lane.
    pub supports_hardware_key: Option<AndroidHostSupportsHardwareKeyCallback>,
    /// Generate callback for one hardware-backed key pair.
    pub generate_hardware_key_pair: Option<AndroidHostGenerateHardwareKeyPairCallback>,
    /// Export callback for one hardware-backed public key.
    pub export_hardware_public_key: Option<AndroidHostExportHardwarePublicKeyCallback>,
    /// Sign callback for one hardware-backed key.
    pub sign_hardware_key: Option<AndroidHostSignHardwareKeyCallback>,
    /// Decrypt callback for one hardware-backed key.
    pub decrypt_hardware_key: Option<AndroidHostDecryptHardwareKeyCallback>,
    /// Derive callback for one hardware-backed key.
    pub derive_hardware_shared_secret: Option<AndroidHostDeriveHardwareSharedSecretCallback>,
    /// Delete callback for one hardware-backed key.
    pub delete_hardware_key: Option<AndroidHostDeleteHardwareKeyCallback>,
}

impl Default for AndroidHostCryptoCallbacks {
    /// Build one callback table with no handlers.
    fn default() -> Self {
        Self {
            abi_version: ANDROID_HOST_CRYPTO_CALLBACKS_ABI_VERSION,
            supports_hardware_key: None,
            generate_hardware_key_pair: None,
            export_hardware_public_key: None,
            sign_hardware_key: None,
            decrypt_hardware_key: None,
            derive_hardware_shared_secret: None,
            delete_hardware_key: None,
        }
    }
}

/// Return the shared Android host-crypto callback registry.
fn android_host_crypto_callbacks() -> &'static RwLock<AndroidHostCryptoCallbacks> {
    static CALLBACKS: OnceLock<RwLock<AndroidHostCryptoCallbacks>> = OnceLock::new();

    CALLBACKS.get_or_init(|| RwLock::new(AndroidHostCryptoCallbacks::default()))
}

/// Return whether one callback table uses the expected ABI version.
fn callbacks_abi_is_supported(callbacks: &AndroidHostCryptoCallbacks) -> bool {
    callbacks.abi_version == ANDROID_HOST_CRYPTO_CALLBACKS_ABI_VERSION
}

/// Return whether one runtime identifier resolves to one live Android host bridge.
fn runtime_id_is_registered(runtime_id: u64) -> bool {
    host_bridge_for_runtime(runtime_id, HostPlatform::Android).is_ok()
}

/// Return the callback-table ABI version for Android host crypto interop.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_callbacks_abi_version() -> u32 {
    ANDROID_HOST_CRYPTO_CALLBACKS_ABI_VERSION
}

/// Set one callback table for Android host crypto interop.
pub fn set_android_host_crypto_callbacks(callbacks: AndroidHostCryptoCallbacks) {
    // replace the entire callback table atomically
    let mut stored_callbacks = android_host_crypto_callbacks().write();
    *stored_callbacks = callbacks;
}

/// Clear Android host crypto callbacks for one test reset.
#[cfg(test)]
fn clear_android_host_crypto_callbacks() {
    set_android_host_crypto_callbacks(AndroidHostCryptoCallbacks::default());
}

/// Set one callback table for Android host crypto interop through one C ABI entrypoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_set_callbacks(
    callbacks: AndroidHostCryptoCallbacks,
) -> u32 {
    // reject callback tables built against one incompatible ABI version
    if !callbacks_abi_is_supported(&callbacks) {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    set_android_host_crypto_callbacks(callbacks);

    HOST_STATUS_OK
}

/// Probe one Android host lane for hardware-backed key support.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_supports_hardware_key(
    runtime_id: u64,
    store_kind: u32,
) -> u32 {
    // reject unknown runtime identifiers
    if !runtime_id_is_registered(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    // route one callback when available
    let callbacks = android_host_crypto_callbacks().read();
    let Some(callback) = callbacks.supports_hardware_key else {
        return HOST_STATUS_NOT_SUPPORTED;
    };

    unsafe { callback(runtime_id, store_kind) }
}

/// Generate one Android host hardware-backed key pair.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_generate_hardware_key_pair(
    runtime_id: u64,
    store_kind: u32,
    key_algorithm: u32,
    named_curve: u32,
    modulus_bits: u32,
    public_exponent: u32,
    key_label: NativeStringRef,
) -> u32 {
    // reject unknown runtime identifiers
    if !runtime_id_is_registered(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    // route one callback when available
    let callbacks = android_host_crypto_callbacks().read();
    let Some(callback) = callbacks.generate_hardware_key_pair else {
        return HOST_STATUS_NOT_SUPPORTED;
    };

    unsafe {
        callback(
            runtime_id,
            store_kind,
            key_algorithm,
            named_curve,
            modulus_bits,
            public_exponent,
            key_label,
        )
    }
}

/// Export one Android host hardware-backed public key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_export_hardware_public_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    // reject unknown runtime identifiers
    if !runtime_id_is_registered(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    // route one callback when available
    let callbacks = android_host_crypto_callbacks().read();
    let Some(callback) = callbacks.export_hardware_public_key else {
        return HOST_STATUS_NOT_SUPPORTED;
    };

    unsafe { callback(runtime_id, key_algorithm, key_label, output, output_written) }
}

/// Sign one payload with one Android host hardware-backed key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_sign_hardware_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    signature_algorithm: u32,
    digest_algorithm: u32,
    salt_length_bytes: u32,
    payload: NativeSlice<u8>,
    output_signature: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    // reject unknown runtime identifiers
    if !runtime_id_is_registered(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    // route one callback when available
    let callbacks = android_host_crypto_callbacks().read();
    let Some(callback) = callbacks.sign_hardware_key else {
        return HOST_STATUS_NOT_SUPPORTED;
    };

    unsafe {
        callback(
            runtime_id,
            key_algorithm,
            key_label,
            signature_algorithm,
            digest_algorithm,
            salt_length_bytes,
            payload,
            output_signature,
            output_written,
        )
    }
}

/// Decrypt one payload with one Android host hardware-backed key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_decrypt_hardware_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    encryption_algorithm: u32,
    digest_algorithm: u32,
    label: NativeSlice<u8>,
    payload: NativeSlice<u8>,
    output_plaintext: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    // reject unknown runtime identifiers
    if !runtime_id_is_registered(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    // route one callback when available
    let callbacks = android_host_crypto_callbacks().read();
    let Some(callback) = callbacks.decrypt_hardware_key else {
        return HOST_STATUS_NOT_SUPPORTED;
    };

    unsafe {
        callback(
            runtime_id,
            key_algorithm,
            key_label,
            encryption_algorithm,
            digest_algorithm,
            label,
            payload,
            output_plaintext,
            output_written,
        )
    }
}

/// Derive one shared secret with one Android host hardware-backed key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_derive_hardware_shared_secret(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    named_curve: u32,
    peer_public_spki: NativeSlice<u8>,
    output_shared_secret: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    // reject unknown runtime identifiers
    if !runtime_id_is_registered(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    // route one callback when available
    let callbacks = android_host_crypto_callbacks().read();
    let Some(callback) = callbacks.derive_hardware_shared_secret else {
        return HOST_STATUS_NOT_SUPPORTED;
    };

    unsafe {
        callback(
            runtime_id,
            key_algorithm,
            key_label,
            named_curve,
            peer_public_spki,
            output_shared_secret,
            output_written,
        )
    }
}

/// Delete one Android host hardware-backed key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_delete_hardware_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
) -> u32 {
    // reject unknown runtime identifiers
    if !runtime_id_is_registered(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    // route one callback when available
    let callbacks = android_host_crypto_callbacks().read();
    let Some(callback) = callbacks.delete_hardware_key else {
        return HOST_STATUS_NOT_SUPPORTED;
    };

    unsafe { callback(runtime_id, key_algorithm, key_label) }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, OnceLock};

    use super::{
        ANDROID_HOST_CRYPTO_CALLBACKS_ABI_VERSION, AndroidHostCryptoCallbacks,
        HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND,
        HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK, clear_android_host_crypto_callbacks,
        destack_runtime_host_android_crypto_export_hardware_public_key,
        destack_runtime_host_android_crypto_set_callbacks,
        destack_runtime_host_android_crypto_supports_hardware_key,
    };
    use crate::platform::{NativeSlice, NativeStringRef};
    use crate::runtime::host::HostPlatform;
    use crate::runtime::host::core::{HostBridge, HostStateStore, register_host_bridge};

    /// Return the shared test lock for callback-registry mutations.
    fn callback_test_lock() -> &'static Mutex<()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    /// Report one supported hardware lane in callback tests.
    unsafe extern "C" fn test_supports_hardware_key(_runtime_id: u64, _store_kind: u32) -> u32 {
        HOST_STATUS_OK
    }

    /// Export one fixed payload for callback tests.
    unsafe extern "C" fn test_export_hardware_public_key(
        _runtime_id: u64,
        _key_algorithm: u32,
        _key_label: NativeStringRef,
        output: NativeSlice<u8>,
        output_written: *mut u32,
    ) -> u32 {
        let payload = [1u8, 2, 3, 4];
        if output_written.is_null() {
            return HOST_STATUS_NOT_SUPPORTED;
        }

        // report required size when the caller has no writable buffer
        if output.data.is_null() || output.len < payload.len() as u32 {
            unsafe {
                *output_written = payload.len() as u32;
            }

            return HOST_STATUS_BUFFER_TOO_SMALL;
        }

        // copy the payload into the output buffer
        let output_bytes =
            unsafe { std::slice::from_raw_parts_mut(output.data, output.len as usize) };
        output_bytes[..payload.len()].copy_from_slice(&payload);

        // publish final output length
        unsafe {
            *output_written = payload.len() as u32;
        }

        HOST_STATUS_OK
    }

    #[test]
    fn test_default_callbacks_return_not_supported() {
        let _lock = callback_test_lock().lock().unwrap();
        clear_android_host_crypto_callbacks();

        let support_status =
            unsafe { destack_runtime_host_android_crypto_supports_hardware_key(1, 2) };
        assert_eq!(support_status, HOST_STATUS_NOT_FOUND);
    }

    #[test]
    fn test_set_callbacks_routes_calls() {
        let _lock = callback_test_lock().lock().unwrap();
        clear_android_host_crypto_callbacks();

        // register one temporary android bridge for runtime-id validation
        let state_store = Arc::new(HostStateStore::new());
        let bridge = Arc::new(HostBridge::new(state_store));
        let registration = register_host_bridge(HostPlatform::Android, &bridge);
        let runtime_id = registration.runtime_id();

        // set one callback table with support and export handlers
        let callbacks = AndroidHostCryptoCallbacks {
            abi_version: ANDROID_HOST_CRYPTO_CALLBACKS_ABI_VERSION,
            supports_hardware_key: Some(test_supports_hardware_key),
            export_hardware_public_key: Some(test_export_hardware_public_key),
            ..AndroidHostCryptoCallbacks::default()
        };
        let set_status = unsafe { destack_runtime_host_android_crypto_set_callbacks(callbacks) };
        assert_eq!(set_status, HOST_STATUS_OK);

        // verify that support probing now routes into the callback table
        let support_status =
            unsafe { destack_runtime_host_android_crypto_supports_hardware_key(runtime_id, 2) };
        assert_eq!(support_status, HOST_STATUS_OK);

        // verify two-pass export flow with one null first pass
        let mut required = 0u32;
        let first_status = unsafe {
            destack_runtime_host_android_crypto_export_hardware_public_key(
                runtime_id,
                2,
                NativeStringRef::from("key"),
                NativeSlice {
                    data: std::ptr::null_mut(),
                    len: 0,
                },
                &mut required as *mut u32,
            )
        };
        assert_eq!(first_status, HOST_STATUS_BUFFER_TOO_SMALL);
        assert_eq!(required, 4);

        // verify second pass writes the exported bytes
        let mut output = vec![0u8; required as usize];
        let mut written = output.len() as u32;
        let second_status = unsafe {
            destack_runtime_host_android_crypto_export_hardware_public_key(
                runtime_id,
                2,
                NativeStringRef::from("key"),
                NativeSlice {
                    data: output.as_mut_ptr(),
                    len: output.len() as u32,
                },
                &mut written as *mut u32,
            )
        };
        assert_eq!(second_status, HOST_STATUS_OK);
        assert_eq!(written, 4);
        assert_eq!(output, vec![1u8, 2, 3, 4]);
    }

    #[test]
    fn test_set_callbacks_rejects_unknown_abi_version() {
        let _lock = callback_test_lock().lock().unwrap();
        clear_android_host_crypto_callbacks();

        let callbacks = AndroidHostCryptoCallbacks {
            abi_version: ANDROID_HOST_CRYPTO_CALLBACKS_ABI_VERSION + 1,
            ..AndroidHostCryptoCallbacks::default()
        };
        let status = unsafe { destack_runtime_host_android_crypto_set_callbacks(callbacks) };

        assert_eq!(status, HOST_STATUS_INVALID_ARGUMENT);
    }
}
