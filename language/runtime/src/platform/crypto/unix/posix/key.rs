use openssl::pkey::{PKey, Private};

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::core::{HostGeneratedKeyPair, HostKeyMaterial};
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionParameters, CryptoKeyAlgorithm, CryptoNamedCurve,
    CryptoSignatureParameters, CryptoStoreKind,
};
use crate::runtime::BindingCallContext;

use super::core::not_supported;

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

/// Generate one host-managed persistent key pair when available.
pub(crate) fn host_generate_persistent_key_pair(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    modulus_bits: u32,
    public_exponent: u32,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostGeneratedKeyPair>> {
    let _ = (
        context,
        kind,
        algorithm,
        named_curve,
        modulus_bits,
        public_exponent,
        persistent_key_label,
        operation,
    );

    Ok(None)
}

/// Import one persistent host-managed private key when available.
pub(crate) fn host_import_persistent_private_key(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    private_key: &PKey<Private>,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostKeyMaterial>> {
    let _ = (
        context,
        kind,
        algorithm,
        named_curve,
        private_key,
        persistent_key_label,
        operation,
    );

    Ok(None)
}

/// Sign one payload with one host-managed key.
pub(crate) fn host_key_sign(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoSignatureParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (context, key, algorithm, parameters, payload);

    Err(not_supported(operation))
}

/// Decrypt one payload with one host-managed key.
pub(crate) fn host_key_decrypt(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (context, key, algorithm, parameters, payload);

    Err(not_supported(operation))
}

/// Delete one host-managed key.
pub(crate) fn host_key_delete(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (context, key);

    Err(not_supported(operation))
}

/// Derive one shared secret with one host-managed private key.
pub(crate) fn host_key_derive_shared_secret(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    peer_public_spki_der: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (context, key, algorithm, named_curve, peer_public_spki_der);

    Err(not_supported(operation))
}
