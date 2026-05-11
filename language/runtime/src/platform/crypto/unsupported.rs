#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
#[cfg(any(unix, windows))]
use openssl::pkey::{PKey, Private};
#[cfg(any(unix, windows))]
use openssl::x509::X509;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::crypto::bindings_generated as bindings;
use crate::platform::{NativeArray, PlatformError};

use crate::runtime::BindingCallContext;
use bindings::*;

use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequest, CryptoArgon2idRequest, CryptoAsymmetricEncryptionAlgorithm,
    CryptoAsymmetricEncryptionParameters, CryptoCertificateDescriptor, CryptoCertificateFormat,
    CryptoCertificateListEntry, CryptoCertificateListPage, CryptoCertificatePurpose,
    CryptoCertificateQuery, CryptoCertificateRevocationMode, CryptoCertificateValidity,
    CryptoCertificateVerifyRequest, CryptoCertificateVerifyResult, CryptoCipherAlgorithm,
    CryptoCipherDirection, CryptoCipherOutput, CryptoCipherParameters, CryptoDigestAlgorithm,
    CryptoHkdfRequest, CryptoKdfAlgorithm, CryptoKeyAgreementAlgorithm, CryptoKeyAlgorithm,
    CryptoKeyDescriptor, CryptoKeyFormat, CryptoKeyGenerationRequest, CryptoKeyImportRequest,
    CryptoKeyKind, CryptoKeyListEntry, CryptoKeyListPage, CryptoKeyPair, CryptoKeyQuery,
    CryptoKeyResidency, CryptoKeyUsageMask, CryptoKeyWrapAlgorithm, CryptoKeyWrapParameters,
    CryptoMacAlgorithm, CryptoMacParameters, CryptoNamedCurve, CryptoPbkdf2Request,
    CryptoPrivateKeyExportRequest, CryptoScryptRequest, CryptoSignatureAlgorithm,
    CryptoSignatureParameters, CryptoStoreCapability, CryptoStoreKind, CryptoStoreOptions,
    CryptoStoreProvider,
};
#[cfg(any(unix, windows))]
use crate::platform::crypto::{HostKeyMaterial, core as crypto_core};
use crate::platform::resource;

/// Return whether one host-lane store supports persistent key writes.
#[cfg(any(unix, windows))]
pub(crate) fn host_store_supports_key_persistence(kind: CryptoStoreKind) -> bool {
    let _ = kind;

    false
}

/// Return whether one host lane has a writable persistent-key backend.
#[cfg(any(unix, windows))]
pub(crate) fn host_store_persistence_backend_is_available(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = (binding, kind);

    false
}

/// Return whether one host store lane is currently available.
#[cfg(any(unix, windows))]
pub(crate) fn host_store_lane_is_available(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = (binding, kind);

    false
}

/// Return whether one host store lane supports hardware-backed keys.
#[cfg(any(unix, windows))]
pub(crate) fn host_store_supports_hardware_backed_key(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = (binding, kind);

    false
}

/// Return whether one host store lane supports one hardware-backed pair algorithm.
#[cfg(any(unix, windows))]
pub(crate) fn host_store_supports_hardware_backed_pair_algorithm(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    let _ = (binding, kind, algorithm);

    false
}

/// Return whether one host store lane supports hardware-backed secret keys.
#[cfg(any(unix, windows))]
pub(crate) fn host_store_supports_hardware_backed_secret_key(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    let _ = (binding, kind, algorithm);

    false
}

/// Return whether one host store lane supports persistent non-extractable pair generation.
#[cfg(any(unix, windows))]
pub(crate) fn host_store_supports_nonextractable_pair_algorithm(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    let _ = (binding, kind, algorithm);

    false
}

/// Return whether one host store lane supports persistent non-extractable private-key import.
#[cfg(any(unix, windows))]
pub(crate) fn host_store_supports_nonextractable_private_import_algorithm(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    let _ = (binding, kind, algorithm);

    false
}

/// Generate one host-backed hardware key pair.
#[cfg(any(unix, windows))]
pub(crate) fn host_generate_hardware_backed_key_pair(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    modulus_bits: u32,
    public_exponent: u32,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<crypto_core::HostGeneratedKeyPair> {
    let _ = (
        binding,
        kind,
        algorithm,
        named_curve,
        usage_mask,
        modulus_bits,
        public_exponent,
        persistent_key_label,
    );

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Generate one host-backed hardware secret key.
#[cfg(any(unix, windows))]
pub(crate) fn host_generate_hardware_backed_secret_key(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    digest: CryptoDigestAlgorithm,
    size_bits: u32,
    usage_mask: CryptoKeyUsageMask,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<HostKeyMaterial> {
    let _ = (
        binding,
        kind,
        algorithm,
        digest,
        size_bits,
        usage_mask,
        persistent_key_label,
    );

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Generate one host-managed persistent key pair when available.
#[cfg(any(unix, windows))]
pub(crate) fn host_generate_persistent_key_pair(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    modulus_bits: u32,
    public_exponent: u32,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<crypto_core::HostGeneratedKeyPair>> {
    let _ = (
        binding,
        kind,
        algorithm,
        named_curve,
        usage_mask,
        modulus_bits,
        public_exponent,
        persistent_key_label,
        operation,
    );

    Ok(None)
}

/// Import one persistent host-managed private key when available.
#[cfg(any(unix, windows))]
pub(crate) fn host_import_persistent_private_key(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    private_key: &PKey<Private>,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostKeyMaterial>> {
    let _ = (
        binding,
        kind,
        algorithm,
        named_curve,
        usage_mask,
        private_key,
        persistent_key_label,
        operation,
    );

    Ok(None)
}

/// Sign one payload with one host-managed key.
#[cfg(any(unix, windows))]
pub(crate) fn host_key_sign(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoSignatureParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (binding, key, kind, algorithm, parameters, payload);

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Decrypt one payload with one host-managed key.
#[cfg(any(unix, windows))]
pub(crate) fn host_key_decrypt(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (binding, key, kind, algorithm, parameters, payload);

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Delete one host-managed key.
#[cfg(any(unix, windows))]
pub(crate) fn host_key_delete(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (binding, key, kind);

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Derive one shared secret with one host-managed private key.
#[cfg(any(unix, windows))]
pub(crate) fn host_key_derive_shared_secret(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    peer_public_spki_der: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (
        binding,
        key,
        kind,
        algorithm,
        named_curve,
        peer_public_spki_der,
    );

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Encrypt one payload with one host-managed secret key.
#[cfg(any(unix, windows))]
pub(crate) fn host_key_cipher_encrypt(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoCipherParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
    let _ = (binding, key, kind, algorithm, parameters, payload);

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Decrypt one payload with one host-managed secret key.
#[cfg(any(unix, windows))]
pub(crate) fn host_key_cipher_decrypt(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoCipherParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (binding, key, kind, algorithm, parameters, payload);

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Compute one MAC with one host-managed secret key.
#[cfg(any(unix, windows))]
pub(crate) fn host_key_mac_compute(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoMacParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = (binding, key, kind, algorithm, parameters, payload);

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Open one host store lane and return certificate snapshots.
#[cfg(any(unix, windows))]
pub(crate) fn open_host_store_certificates(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> RuntimeResult<Vec<X509>> {
    let _ = (binding, kind);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.store.open")).boxed())
}

/// Load one backend host-key snapshot payload for one store lane.
#[cfg(any(unix, windows))]
pub(crate) fn load_host_key_snapshot_bytes(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<Option<Vec<u8>>> {
    let _ = (binding, kind, operation);

    Ok(None)
}

/// Store one backend host-key snapshot payload for one store lane.
#[cfg(any(unix, windows))]
pub(crate) fn store_host_key_snapshot_bytes(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    snapshot_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (binding, kind, snapshot_bytes);

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Return whether one host store lane supports certificate write operations.
#[cfg(any(unix, windows))]
pub(crate) fn host_store_supports_certificate_write(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = (binding, kind);

    false
}

/// Import one certificate into one host store lane.
#[cfg(any(unix, windows))]
pub(crate) fn host_store_import_certificate(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (binding, kind, certificate);

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Delete one certificate from one host store lane.
#[cfg(any(unix, windows))]
pub(crate) fn host_store_delete_certificate(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    certificate: &X509,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = (binding, kind, certificate);

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Derive one symmetric key from one local private key and one peer public key.
pub(crate) unsafe fn destack_crypto_agreement_derive_key(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    request: CryptoAgreementDeriveKeyRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, privatekey, peerpublickey, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.agreement.deriveKey",
    ))
    .boxed())
}

/// Derive one shared secret from one local private key and one peer public key.
pub(crate) unsafe fn destack_crypto_agreement_derive_shared_secret(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    algorithm: CryptoKeyAgreementAlgorithm,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, privatekey, peerpublickey, algorithm);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.agreement.deriveSharedSecret",
    ))
    .boxed())
}

/// Delete one certificate from one store when allowed.
pub(crate) unsafe fn destack_crypto_certificate_delete(
    binding: &BindingCallContext,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.delete",
    ))
    .boxed())
}

/// Return one certificate descriptor.
pub(crate) unsafe fn destack_crypto_certificate_descriptor(
    binding: &BindingCallContext,
    out: *mut CryptoCertificateDescriptor,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.descriptor",
    ))
    .boxed())
}

/// Export one certificate from one handle.
pub(crate) unsafe fn destack_crypto_certificate_export(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoCertificateHandle,
    format: CryptoCertificateFormat,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, format);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.export",
    ))
    .boxed())
}

/// Import one certificate into one store.
pub(crate) unsafe fn destack_crypto_certificate_import(
    binding: &BindingCallContext,
    out: *mut resource::CryptoCertificateHandle,
    store: resource::CryptoStoreHandle,
    format: CryptoCertificateFormat,
    certificate: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, store, format, certificate);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.import",
    ))
    .boxed())
}

/// Verify one certificate chain against one trust policy.
pub(crate) unsafe fn destack_crypto_certificate_verify(
    binding: &BindingCallContext,
    out: *mut CryptoCertificateVerifyResult,
    request: CryptoCertificateVerifyRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.verify",
    ))
    .boxed())
}

/// Close one streaming cipher context.
pub(crate) unsafe fn destack_crypto_cipher_close(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.close")).boxed())
}

/// Decrypt one payload in one shot.
pub(crate) unsafe fn destack_crypto_cipher_decrypt(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, key, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.cipher.decrypt",
    ))
    .boxed())
}

/// Encrypt one payload in one shot.
pub(crate) unsafe fn destack_crypto_cipher_encrypt(
    binding: &BindingCallContext,
    out: *mut CryptoCipherOutput,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, key, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.cipher.encrypt",
    ))
    .boxed())
}

/// Finalize one streaming cipher context.
pub(crate) unsafe fn destack_crypto_cipher_finish(
    binding: &BindingCallContext,
    out: *mut CryptoCipherOutput,
    handle: resource::CryptoCipherHandle,
    finalpayload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, finalpayload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.finish")).boxed())
}

/// Open one streaming cipher context.
pub(crate) unsafe fn destack_crypto_cipher_open(
    binding: &BindingCallContext,
    out: *mut resource::CryptoCipherHandle,
    key: resource::CryptoKeyHandle,
    direction: CryptoCipherDirection,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, key, direction, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.open")).boxed())
}

/// Reset one streaming cipher context with new parameters.
pub(crate) unsafe fn destack_crypto_cipher_reset(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    let _ = (binding, handle, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.reset")).boxed())
}

/// Update one streaming cipher context with one payload chunk.
pub(crate) unsafe fn destack_crypto_cipher_update(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoCipherHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.update")).boxed())
}

/// Update additional authenticated data for one streaming cipher context.
pub(crate) unsafe fn destack_crypto_cipher_update_additional_data(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    additionaldata: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, handle, additionaldata);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.cipher.updateAdditionalData",
    ))
    .boxed())
}

/// Close one streaming digest context.
pub(crate) unsafe fn destack_crypto_digest_close(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.close")).boxed())
}

/// Compute one digest in one shot.
pub(crate) unsafe fn destack_crypto_digest_compute(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    algorithm: CryptoDigestAlgorithm,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, algorithm, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.digest.compute",
    ))
    .boxed())
}

/// Finalize one streaming digest context and return one digest output.
pub(crate) unsafe fn destack_crypto_digest_finish(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.finish")).boxed())
}

/// Open one streaming digest context.
pub(crate) unsafe fn destack_crypto_digest_open(
    binding: &BindingCallContext,
    out: *mut resource::CryptoDigestHandle,
    algorithm: CryptoDigestAlgorithm,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, algorithm);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.open")).boxed())
}

/// Reset one streaming digest context to its initial state.
pub(crate) unsafe fn destack_crypto_digest_reset(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.reset")).boxed())
}

/// Update one streaming digest context.
pub(crate) unsafe fn destack_crypto_digest_update(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, handle, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.update")).boxed())
}

/// Derive one key with Argon2id.
pub(crate) unsafe fn destack_crypto_kdf_argon2id(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoArgon2idRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.argon2id")).boxed())
}

/// Derive one key with HKDF.
pub(crate) unsafe fn destack_crypto_kdf_hkdf(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoHkdfRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.hkdf")).boxed())
}

/// Derive one key with PBKDF2.
pub(crate) unsafe fn destack_crypto_kdf_pbkdf2(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoPbkdf2Request,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.pbkdf2")).boxed())
}

/// Derive one key with scrypt.
pub(crate) unsafe fn destack_crypto_kdf_scrypt(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoScryptRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.scrypt")).boxed())
}

/// Decrypt one payload with one asymmetric key.
pub(crate) unsafe fn destack_crypto_key_decrypt(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.decrypt")).boxed())
}

/// Delete one key object.
pub(crate) unsafe fn destack_crypto_key_delete(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.delete")).boxed())
}

/// Return one key descriptor.
pub(crate) unsafe fn destack_crypto_key_descriptor(
    binding: &BindingCallContext,
    out: *mut CryptoKeyDescriptor,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.descriptor",
    ))
    .boxed())
}

/// Encrypt one payload with one asymmetric key.
pub(crate) unsafe fn destack_crypto_key_encrypt(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.encrypt")).boxed())
}

/// Export one private key.
pub(crate) unsafe fn destack_crypto_key_export_private(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    request: CryptoPrivateKeyExportRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.exportPrivate",
    ))
    .boxed())
}

/// Export one public key.
pub(crate) unsafe fn destack_crypto_key_export_public(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, format);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.exportPublic",
    ))
    .boxed())
}

/// Export one secret key.
pub(crate) unsafe fn destack_crypto_key_export_secret(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, format);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.exportSecret",
    ))
    .boxed())
}

/// Generate one asymmetric key pair.
pub(crate) unsafe fn destack_crypto_key_generate_pair(
    binding: &BindingCallContext,
    out: *mut CryptoKeyPair,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, store, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.generatePair",
    ))
    .boxed())
}

/// Generate one symmetric key.
pub(crate) unsafe fn destack_crypto_key_generate_secret(
    binding: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, store, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.generateSecret",
    ))
    .boxed())
}

/// Import one key object.
pub(crate) unsafe fn destack_crypto_key_import(
    binding: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, store, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.import")).boxed())
}

/// Sign one payload.
pub(crate) unsafe fn destack_crypto_key_sign(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.sign")).boxed())
}

/// Unwrap one key.
pub(crate) unsafe fn destack_crypto_key_unwrap(
    binding: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    wrappingkey: resource::CryptoKeyHandle,
    wrappedkey: NativeSlice<u8>,
    parameters: CryptoKeyWrapParameters,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (
        binding,
        out,
        store,
        wrappingkey,
        wrappedkey,
        parameters,
        request,
    );

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.unwrap")).boxed())
}

/// Verify one signature.
pub(crate) unsafe fn destack_crypto_key_verify(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParameters,
    argument_payload: NativeSlice<u8>,
    signature: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (
        binding,
        out,
        handle,
        parameters,
        argument_payload,
        signature,
    );

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.verify")).boxed())
}

/// Wrap one key.
pub(crate) unsafe fn destack_crypto_key_wrap(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    wrappingkey: resource::CryptoKeyHandle,
    keytowrap: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
    parameters: CryptoKeyWrapParameters,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, wrappingkey, keytowrap, format, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.wrap")).boxed())
}

/// Close one streaming MAC context.
pub(crate) unsafe fn destack_crypto_mac_close(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.close")).boxed())
}

/// Compute one message authentication code in one shot.
pub(crate) unsafe fn destack_crypto_mac_compute(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, key, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.compute")).boxed())
}

/// Finalize one streaming MAC context and return one tag.
pub(crate) unsafe fn destack_crypto_mac_finish(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.finish")).boxed())
}

/// Open one streaming MAC context.
pub(crate) unsafe fn destack_crypto_mac_open(
    binding: &BindingCallContext,
    out: *mut resource::CryptoMacHandle,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, key, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.open")).boxed())
}

/// Reset one streaming MAC context to its initial state.
pub(crate) unsafe fn destack_crypto_mac_reset(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.reset")).boxed())
}

/// Update one streaming MAC context.
pub(crate) unsafe fn destack_crypto_mac_update(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, handle, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.update")).boxed())
}

/// Verify one message authentication code in one shot.
pub(crate) unsafe fn destack_crypto_mac_verify(
    binding: &BindingCallContext,
    out: *mut bool,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    argument_payload: NativeSlice<u8>,
    tag: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, key, parameters, argument_payload, tag);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.verify")).boxed())
}

/// List supported key-agreement algorithms.
pub(crate) unsafe fn destack_crypto_probe_agreement_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyAgreementAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.agreementAlgorithms",
    ))
    .boxed())
}

/// List supported cipher algorithms.
pub(crate) unsafe fn destack_crypto_probe_cipher_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoCipherAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.cipherAlgorithms",
    ))
    .boxed())
}

/// List supported digest algorithms.
pub(crate) unsafe fn destack_crypto_probe_digest_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoDigestAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.digestAlgorithms",
    ))
    .boxed())
}

/// List supported KDF algorithms.
pub(crate) unsafe fn destack_crypto_probe_kdf_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKdfAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.kdfAlgorithms",
    ))
    .boxed())
}

/// List supported key algorithm families.
pub(crate) unsafe fn destack_crypto_probe_key_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.keyAlgorithms",
    ))
    .boxed())
}

/// List supported key-wrap algorithms.
pub(crate) unsafe fn destack_crypto_probe_key_wrap_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyWrapAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.keyWrapAlgorithms",
    ))
    .boxed())
}

/// List supported key formats.
pub(crate) unsafe fn destack_crypto_probe_key_formats(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyFormat>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.keyFormats",
    ))
    .boxed())
}

/// List supported key residencies.
pub(crate) unsafe fn destack_crypto_probe_key_residencies(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyResidency>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.keyResidencies",
    ))
    .boxed())
}

/// List supported MAC algorithms.
pub(crate) unsafe fn destack_crypto_probe_mac_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoMacAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.macAlgorithms",
    ))
    .boxed())
}

/// List supported named curves.
pub(crate) unsafe fn destack_crypto_probe_named_curves(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoNamedCurve>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.namedCurves",
    ))
    .boxed())
}

/// List supported signature algorithms.
pub(crate) unsafe fn destack_crypto_probe_signature_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoSignatureAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.signatureAlgorithms",
    ))
    .boxed())
}

/// Allocate one random byte vector with the requested length.
pub(crate) unsafe fn destack_crypto_random_bytes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    length: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, length);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.random.bytes")).boxed())
}

/// Fill one mutable byte slice with cryptographically secure random bytes.
pub(crate) unsafe fn destack_crypto_random_fill(
    binding: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.random.fill")).boxed())
}

/// Close one crypto store.
pub(crate) unsafe fn destack_crypto_store_close(
    binding: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.store.close")).boxed())
}

/// List certificates from one store.
pub(crate) unsafe fn destack_crypto_store_list_certificates(
    binding: &BindingCallContext,
    out: *mut CryptoCertificateListPage,
    handle: resource::CryptoStoreHandle,
    query: CryptoCertificateQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, query);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.listCertificates",
    ))
    .boxed())
}

/// List keys from one store.
pub(crate) unsafe fn destack_crypto_store_list_keys(
    binding: &BindingCallContext,
    out: *mut CryptoKeyListPage,
    handle: resource::CryptoStoreHandle,
    query: CryptoKeyQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, handle, query);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.listKeys",
    ))
    .boxed())
}

/// Return capabilities for one store backend identity.
pub(crate) unsafe fn destack_crypto_store_probe_capability(
    binding: &BindingCallContext,
    out: *mut CryptoStoreCapability,
    kind: CryptoStoreKind,
    provider: Option<CryptoStoreProvider>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, kind, provider);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.probeCapability",
    ))
    .boxed())
}

/// List store backend kinds that are currently available.
pub(crate) unsafe fn destack_crypto_store_probe_kinds(
    binding: &BindingCallContext,
    out: *mut NativeArray<CryptoStoreKind>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.probeKinds",
    ))
    .boxed())
}

/// Open one crypto store.
pub(crate) unsafe fn destack_crypto_store_open(
    binding: &BindingCallContext,
    out: *mut resource::CryptoStoreHandle,
    options: CryptoStoreOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (binding, out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.store.open")).boxed())
}
