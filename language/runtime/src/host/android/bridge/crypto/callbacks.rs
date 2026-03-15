use crate::runtime::{NativeSlice, NativeStringRef};

/// Host key algorithm code for rsa.
pub(super) const HOST_KEY_ALGORITHM_RSA: u32 = 1;
/// Host key algorithm code for ec.
pub(super) const HOST_KEY_ALGORITHM_EC: u32 = 2;
/// Host key algorithm code for aes.
pub(super) const HOST_KEY_ALGORITHM_AES: u32 = 3;
/// Host key algorithm code for hmac.
pub(super) const HOST_KEY_ALGORITHM_HMAC: u32 = 4;

/// Host store-kind code for system lane.
pub(super) const HOST_STORE_KIND_SYSTEM: u32 = 1;
/// Host store-kind code for user lane.
pub(super) const HOST_STORE_KIND_USER: u32 = 2;
/// Host store-kind code for machine lane.
pub(super) const HOST_STORE_KIND_MACHINE: u32 = 3;

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
