use openssl::pkey::{PKey, Private};

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::core::{HostGeneratedKeyPair, HostKeyBackend, HostKeyMaterial};
use crate::platform::crypto::host::unix::core as unix_core;
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionParameters, CryptoCipherParameters, CryptoDigestAlgorithm,
    CryptoKeyAlgorithm, CryptoKeyUsageMask, CryptoMacParameters, CryptoNamedCurve,
    CryptoSignatureParameters, CryptoStoreKind,
};
use crate::runtime::BindingCallContext;

use super::core::not_supported;

/// Supported software host-key backends for POSIX key operations.
const POSIX_SOFTWARE_BACKENDS: [HostKeyBackend; 2] = [
    HostKeyBackend::PosixSoftwareKeyStorageRsa,
    HostKeyBackend::PosixSoftwareKeyStorageEc,
];

/// Return whether one store lane supports POSIX host key persistence.
fn supports_persistent_host_keys(kind: CryptoStoreKind) -> bool {
    unix_core::host_store_supports_key_persistence(kind)
}

/// Return whether one host store lane supports hardware-backed keys.
pub(crate) fn host_store_supports_hardware_backed_key(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = (context, kind);

    false
}

/// Generate one host-backed hardware key pair.
pub(crate) fn host_generate_hardware_backed_key_pair(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<HostGeneratedKeyPair> {
    let _ = (context, kind, algorithm, named_curve, persistent_key_label);

    Err(not_supported(operation))
}

/// Generate one host-backed hardware secret key.
pub(crate) fn host_generate_hardware_backed_secret_key(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    digest: CryptoDigestAlgorithm,
    size_bits: u32,
    usage_mask: CryptoKeyUsageMask,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<HostKeyMaterial> {
    let _ = (
        context,
        kind,
        algorithm,
        digest,
        size_bits,
        usage_mask,
        persistent_key_label,
    );

    Err(not_supported(operation))
}

/// Generate one host-managed persistent key pair when available.
pub(crate) fn host_generate_persistent_key_pair(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    modulus_bits: u32,
    public_exponent: u32,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostGeneratedKeyPair>> {
    let _ = (context, usage_mask);

    // enforce store-kind persistence policy for POSIX host lanes
    if !supports_persistent_host_keys(kind) {
        return Ok(None);
    }

    unix_core::generate_software_persistent_key_pair(
        kind,
        algorithm,
        named_curve,
        modulus_bits,
        public_exponent,
        persistent_key_label,
        true,
        HostKeyBackend::PosixSoftwareKeyStorageRsa,
        HostKeyBackend::PosixSoftwareKeyStorageEc,
        operation,
    )
}

/// Import one persistent host-managed private key when available.
pub(crate) fn host_import_persistent_private_key(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    private_key: &PKey<Private>,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostKeyMaterial>> {
    let _ = (context, usage_mask);

    // enforce store-kind persistence policy for POSIX host lanes
    if !supports_persistent_host_keys(kind) {
        return Ok(None);
    }

    unix_core::import_software_persistent_private_key(
        kind,
        algorithm,
        named_curve,
        private_key,
        persistent_key_label,
        true,
        true,
        HostKeyBackend::PosixSoftwareKeyStorageRsa,
        HostKeyBackend::PosixSoftwareKeyStorageEc,
        operation,
    )
}

/// Sign one payload with one host-managed key.
pub(crate) fn host_key_sign(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoSignatureParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (context, store_kind);

    unix_core::sign_with_software_host_key(
        key,
        &POSIX_SOFTWARE_BACKENDS,
        algorithm,
        parameters,
        payload,
        operation,
    )
}

/// Decrypt one payload with one host-managed key.
pub(crate) fn host_key_decrypt(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (context, store_kind);

    unix_core::decrypt_with_software_host_key(
        key,
        &POSIX_SOFTWARE_BACKENDS,
        algorithm,
        parameters,
        payload,
        operation,
    )
}

/// Delete one host-managed key.
pub(crate) fn host_key_delete(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (context, store_kind);

    // delete is fully managed by snapshot persistence for POSIX host keys
    unix_core::delete_software_host_key(key, &POSIX_SOFTWARE_BACKENDS, operation)
}

/// Derive one shared secret with one host-managed private key.
pub(crate) fn host_key_derive_shared_secret(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    peer_public_spki_der: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (context, store_kind);

    // enforce one explicit curve mismatch error for POSIX software lanes
    unix_core::derive_shared_secret_with_software_host_key(
        key,
        &POSIX_SOFTWARE_BACKENDS,
        algorithm,
        named_curve,
        peer_public_spki_der,
        true,
        operation,
    )
}

/// Encrypt one payload with one host-managed secret key.
pub(crate) fn host_key_cipher_encrypt(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoCipherParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
    let _ = (context, key, store_kind, algorithm, parameters, payload);

    Err(not_supported(operation))
}

/// Decrypt one payload with one host-managed secret key.
pub(crate) fn host_key_cipher_decrypt(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoCipherParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (context, key, store_kind, algorithm, parameters, payload);

    Err(not_supported(operation))
}

/// Compute one MAC with one host-managed secret key.
pub(crate) fn host_key_mac_compute(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoMacParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (context, key, store_kind, algorithm, parameters, payload);

    Err(not_supported(operation))
}
