use super::bindings::call_android_binding_callback;
use crate::platform::{NativeSlice, NativeStringRef};

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
/// Host callback for generating one hardware-backed secret key.
pub type AndroidHostGenerateHardwareSecretKeyCallback = unsafe extern "C" fn(
    runtime_id: u64,
    store_kind: u32,
    key_algorithm: u32,
    digest_algorithm: u32,
    key_size_bits: u32,
    key_usage_mask: u32,
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
/// Host callback for encrypting one payload with one hardware-backed secret key.
pub type AndroidHostEncryptHardwareSecretKeyCallback = unsafe extern "C" fn(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    cipher_algorithm: u32,
    nonce: NativeSlice<u8>,
    additional_data: NativeSlice<u8>,
    tag_length_bytes: u32,
    payload: NativeSlice<u8>,
    output_ciphertext: NativeSlice<u8>,
    output_tag: NativeSlice<u8>,
    output_ciphertext_written: *mut u32,
    output_tag_written: *mut u32,
) -> u32;
/// Host callback for decrypting one payload with one hardware-backed secret key.
pub type AndroidHostDecryptHardwareSecretKeyCallback = unsafe extern "C" fn(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    cipher_algorithm: u32,
    nonce: NativeSlice<u8>,
    additional_data: NativeSlice<u8>,
    tag: NativeSlice<u8>,
    payload: NativeSlice<u8>,
    output_plaintext: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;
/// Host callback for computing one MAC with one hardware-backed secret key.
pub type AndroidHostComputeHardwareMacCallback = unsafe extern "C" fn(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    mac_algorithm: u32,
    digest_algorithm: u32,
    tag_length_bytes: u32,
    payload: NativeSlice<u8>,
    output_tag: NativeSlice<u8>,
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
/// Host callback for importing one certificate into one host lane.
pub type AndroidHostImportCertificateCallback =
    unsafe extern "C" fn(runtime_id: u64, store_kind: u32, certificate_der: NativeSlice<u8>) -> u32;
/// Host callback for deleting one certificate from one host lane.
pub type AndroidHostDeleteCertificateCallback =
    unsafe extern "C" fn(runtime_id: u64, store_kind: u32, certificate_der: NativeSlice<u8>) -> u32;

/// Callback table for Android host crypto interop.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AndroidHostCryptoCallbacks {
    /// Probe callback for one hardware-backed lane.
    pub supports_hardware_key: Option<AndroidHostSupportsHardwareKeyCallback>,
    /// Generate callback for one hardware-backed key pair.
    pub generate_hardware_key_pair: Option<AndroidHostGenerateHardwareKeyPairCallback>,
    /// Generate callback for one hardware-backed secret key.
    pub generate_hardware_secret_key: Option<AndroidHostGenerateHardwareSecretKeyCallback>,
    /// Export callback for one hardware-backed public key.
    pub export_hardware_public_key: Option<AndroidHostExportHardwarePublicKeyCallback>,
    /// Sign callback for one hardware-backed key.
    pub sign_hardware_key: Option<AndroidHostSignHardwareKeyCallback>,
    /// Decrypt callback for one hardware-backed key.
    pub decrypt_hardware_key: Option<AndroidHostDecryptHardwareKeyCallback>,
    /// Encrypt callback for one hardware-backed secret key.
    pub encrypt_hardware_secret_key: Option<AndroidHostEncryptHardwareSecretKeyCallback>,
    /// Decrypt callback for one hardware-backed secret key.
    pub decrypt_hardware_secret_key: Option<AndroidHostDecryptHardwareSecretKeyCallback>,
    /// Compute callback for one hardware-backed secret-key MAC.
    pub compute_hardware_mac: Option<AndroidHostComputeHardwareMacCallback>,
    /// Derive callback for one hardware-backed key.
    pub derive_hardware_shared_secret: Option<AndroidHostDeriveHardwareSharedSecretCallback>,
    /// Delete callback for one hardware-backed key.
    pub delete_hardware_key: Option<AndroidHostDeleteHardwareKeyCallback>,
    /// Import callback for one certificate into one host lane.
    pub import_certificate: Option<AndroidHostImportCertificateCallback>,
    /// Delete callback for one certificate from one host lane.
    pub delete_certificate: Option<AndroidHostDeleteCertificateCallback>,
}

impl Default for AndroidHostCryptoCallbacks {
    /// Build one callback table with no handlers.
    fn default() -> Self {
        Self {
            supports_hardware_key: None,
            generate_hardware_key_pair: None,
            generate_hardware_secret_key: None,
            export_hardware_public_key: None,
            sign_hardware_key: None,
            decrypt_hardware_key: None,
            encrypt_hardware_secret_key: None,
            decrypt_hardware_secret_key: None,
            compute_hardware_mac: None,
            derive_hardware_shared_secret: None,
            delete_hardware_key: None,
            import_certificate: None,
            delete_certificate: None,
        }
    }
}

/// Resolve and invoke one Android host crypto callback.
fn call_android_crypto_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostCryptoCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    call_android_binding_callback(runtime_id, |bindings| resolve(&bindings.crypto), invoke)
}

/// Probe one Android host lane for hardware-backed key support.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_supports_hardware_key(
    runtime_id: u64,
    store_kind: u32,
) -> u32 {
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.supports_hardware_key,
        |callback| unsafe { callback(runtime_id, store_kind) },
    )
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
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.generate_hardware_key_pair,
        |callback| unsafe {
            callback(
                runtime_id,
                store_kind,
                key_algorithm,
                named_curve,
                modulus_bits,
                public_exponent,
                key_label,
            )
        },
    )
}

/// Generate one Android host hardware-backed secret key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_generate_hardware_secret_key(
    runtime_id: u64,
    store_kind: u32,
    key_algorithm: u32,
    digest_algorithm: u32,
    key_size_bits: u32,
    key_usage_mask: u32,
    key_label: NativeStringRef,
) -> u32 {
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.generate_hardware_secret_key,
        |callback| unsafe {
            callback(
                runtime_id,
                store_kind,
                key_algorithm,
                digest_algorithm,
                key_size_bits,
                key_usage_mask,
                key_label,
            )
        },
    )
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
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.export_hardware_public_key,
        |callback| unsafe {
            callback(runtime_id, key_algorithm, key_label, output, output_written)
        },
    )
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
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.sign_hardware_key,
        |callback| unsafe {
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
        },
    )
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
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.decrypt_hardware_key,
        |callback| unsafe {
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
        },
    )
}

/// Encrypt one payload with one Android host hardware-backed secret key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_encrypt_hardware_secret_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    cipher_algorithm: u32,
    nonce: NativeSlice<u8>,
    additional_data: NativeSlice<u8>,
    tag_length_bytes: u32,
    payload: NativeSlice<u8>,
    output_ciphertext: NativeSlice<u8>,
    output_tag: NativeSlice<u8>,
    output_ciphertext_written: *mut u32,
    output_tag_written: *mut u32,
) -> u32 {
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.encrypt_hardware_secret_key,
        |callback| unsafe {
            callback(
                runtime_id,
                key_algorithm,
                key_label,
                cipher_algorithm,
                nonce,
                additional_data,
                tag_length_bytes,
                payload,
                output_ciphertext,
                output_tag,
                output_ciphertext_written,
                output_tag_written,
            )
        },
    )
}

/// Decrypt one payload with one Android host hardware-backed secret key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_decrypt_hardware_secret_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    cipher_algorithm: u32,
    nonce: NativeSlice<u8>,
    additional_data: NativeSlice<u8>,
    tag: NativeSlice<u8>,
    payload: NativeSlice<u8>,
    output_plaintext: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.decrypt_hardware_secret_key,
        |callback| unsafe {
            callback(
                runtime_id,
                key_algorithm,
                key_label,
                cipher_algorithm,
                nonce,
                additional_data,
                tag,
                payload,
                output_plaintext,
                output_written,
            )
        },
    )
}

/// Compute one MAC with one Android host hardware-backed secret key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_compute_hardware_mac(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    mac_algorithm: u32,
    digest_algorithm: u32,
    tag_length_bytes: u32,
    payload: NativeSlice<u8>,
    output_tag: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32 {
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.compute_hardware_mac,
        |callback| unsafe {
            callback(
                runtime_id,
                key_algorithm,
                key_label,
                mac_algorithm,
                digest_algorithm,
                tag_length_bytes,
                payload,
                output_tag,
                output_written,
            )
        },
    )
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
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.derive_hardware_shared_secret,
        |callback| unsafe {
            callback(
                runtime_id,
                key_algorithm,
                key_label,
                named_curve,
                peer_public_spki,
                output_shared_secret,
                output_written,
            )
        },
    )
}

/// Delete one Android host hardware-backed key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_delete_hardware_key(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
) -> u32 {
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.delete_hardware_key,
        |callback| unsafe { callback(runtime_id, key_algorithm, key_label) },
    )
}

/// Import one certificate into one Android host lane.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_import_certificate(
    runtime_id: u64,
    store_kind: u32,
    certificate_der: NativeSlice<u8>,
) -> u32 {
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.import_certificate,
        |callback| unsafe { callback(runtime_id, store_kind, certificate_der) },
    )
}

/// Delete one certificate from one Android host lane.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_crypto_delete_certificate(
    runtime_id: u64,
    store_kind: u32,
    certificate_der: NativeSlice<u8>,
) -> u32 {
    // route one callback when available
    call_android_crypto_callback(
        runtime_id,
        |callbacks| callbacks.delete_certificate,
        |callback| unsafe { callback(runtime_id, store_kind, certificate_der) },
    )
}
