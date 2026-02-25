use std::ffi::c_void;
use std::sync::OnceLock;

use crate::platform::{NativeSlice, NativeStringRef};

/// Host callback status code for one successful call.
pub(super) const HOST_STATUS_OK: u32 = 0;
/// Host callback status code for one unsupported call.
pub(super) const HOST_STATUS_NOT_SUPPORTED: u32 = 1;
/// Host callback status code for one invalid argument.
pub(super) const HOST_STATUS_INVALID_ARGUMENT: u32 = 2;
/// Host callback status code for one missing key or object.
pub(super) const HOST_STATUS_NOT_FOUND: u32 = 3;
/// Host callback status code for one permission error.
pub(super) const HOST_STATUS_PERMISSION_DENIED: u32 = 4;
/// Host callback status code for one output buffer that is too small.
pub(super) const HOST_STATUS_BUFFER_TOO_SMALL: u32 = 5;
/// Host callback status code for one generic failure.
pub(super) const HOST_STATUS_FAILED: u32 = 6;

/// Host callback for probing hardware-backed-key support.
pub(super) type HostSupportsHardwareKeyFn =
    unsafe extern "C" fn(runtime_id: u64, store_kind: u32) -> u32;
/// Host callback for generating one hardware-backed key pair.
pub(super) type HostGenerateHardwareKeyPairFn = unsafe extern "C" fn(
    runtime_id: u64,
    store_kind: u32,
    key_algorithm: u32,
    named_curve: u32,
    modulus_bits: u32,
    public_exponent: u32,
    key_label: NativeStringRef,
) -> u32;
/// Host callback for exporting one hardware-backed public key.
pub(super) type HostExportHardwarePublicKeyFn = unsafe extern "C" fn(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;
/// Host callback for signing one payload with one hardware-backed key.
pub(super) type HostSignHardwareKeyFn = unsafe extern "C" fn(
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
pub(super) type HostDecryptHardwareKeyFn = unsafe extern "C" fn(
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
pub(super) type HostDeriveHardwareSharedSecretFn = unsafe extern "C" fn(
    runtime_id: u64,
    key_algorithm: u32,
    key_label: NativeStringRef,
    named_curve: u32,
    peer_public_spki: NativeSlice<u8>,
    output_shared_secret: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;
/// Host callback for deleting one hardware-backed key.
pub(super) type HostDeleteHardwareKeyFn =
    unsafe extern "C" fn(runtime_id: u64, key_algorithm: u32, key_label: NativeStringRef) -> u32;

/// Resolved Android host crypto callbacks from the process symbol table.
pub(super) struct AndroidHostCryptoApi {
    /// Probe for one lane hardware-backed support.
    pub(super) supports_hardware_key: Option<HostSupportsHardwareKeyFn>,
    /// Generate one hardware-backed key pair.
    pub(super) generate_hardware_key_pair: Option<HostGenerateHardwareKeyPairFn>,
    /// Export one hardware-backed public key payload.
    pub(super) export_hardware_public_key: Option<HostExportHardwarePublicKeyFn>,
    /// Sign one payload with one hardware-backed key.
    pub(super) sign_hardware_key: Option<HostSignHardwareKeyFn>,
    /// Decrypt one payload with one hardware-backed key.
    pub(super) decrypt_hardware_key: Option<HostDecryptHardwareKeyFn>,
    /// Derive one shared secret with one hardware-backed key.
    pub(super) derive_hardware_shared_secret: Option<HostDeriveHardwareSharedSecretFn>,
    /// Delete one hardware-backed key.
    pub(super) delete_hardware_key: Option<HostDeleteHardwareKeyFn>,
}

/// Resolve and cache one Android host crypto callback table.
pub(super) fn android_host_crypto_api() -> &'static AndroidHostCryptoApi {
    static HOST_CRYPTO_API: OnceLock<AndroidHostCryptoApi> = OnceLock::new();

    HOST_CRYPTO_API.get_or_init(|| AndroidHostCryptoApi {
        supports_hardware_key: load_symbol(
            b"destack_runtime_host_android_crypto_supports_hardware_key\0",
        ),
        generate_hardware_key_pair: load_symbol(
            b"destack_runtime_host_android_crypto_generate_hardware_key_pair\0",
        ),
        export_hardware_public_key: load_symbol(
            b"destack_runtime_host_android_crypto_export_hardware_public_key\0",
        ),
        sign_hardware_key: load_symbol(b"destack_runtime_host_android_crypto_sign_hardware_key\0"),
        decrypt_hardware_key: load_symbol(
            b"destack_runtime_host_android_crypto_decrypt_hardware_key\0",
        ),
        derive_hardware_shared_secret: load_symbol(
            b"destack_runtime_host_android_crypto_derive_hardware_shared_secret\0",
        ),
        delete_hardware_key: load_symbol(
            b"destack_runtime_host_android_crypto_delete_hardware_key\0",
        ),
    })
}

/// Resolve one typed symbol from one process-global symbol table.
fn load_symbol<T>(name: &[u8]) -> Option<T>
where
    T: Copy,
{
    if name.is_empty() || *name.last()? != 0 {
        return None;
    }

    let symbol = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr().cast()) };
    if symbol.is_null() {
        return None;
    }

    // cast one raw symbol pointer into one typed function pointer
    union SymbolCast<T: Copy> {
        pointer: *mut c_void,
        symbol: T,
    }
    let value = unsafe { SymbolCast { pointer: symbol }.symbol };

    Some(value)
}
