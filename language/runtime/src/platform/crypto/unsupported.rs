#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::crypto::bindings_generated as bindings;
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};

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
    CryptoKeyUsageMask, CryptoMacAlgorithm, CryptoMacParameters, CryptoNamedCurve,
    CryptoPbkdf2Request, CryptoScryptRequest, CryptoSignatureAlgorithm, CryptoSignatureParameters,
    CryptoStoreKind, CryptoStoreOptions,
};
use crate::platform::resource;

/// Derive one symmetric key from one local private key and one peer public key.
///
/// This operation performs key agreement and an explicit KDF stage.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-agreement and KDF provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.agree`, `crypto.kdf`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_agreement_derive_key(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    request: CryptoAgreementDeriveKeyRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, privatekey, peerpublickey, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.agreement.deriveKey",
    ))
    .boxed())
}

/// Derive one shared secret from one local private key and one peer public key.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-agreement provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.agree`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_agreement_derive_shared_secret(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    algorithm: CryptoKeyAgreementAlgorithm,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, privatekey, peerpublickey, algorithm);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.agreement.deriveSharedSecret",
    ))
    .boxed())
}

/// Delete one certificate from one provider store when allowed.
///
/// Remove one certificate object and invalidate the handle.
/// Deletion permissions and persistence are enforced by runtime provider policies.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime certificate parsing, store, and chain-verification provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.certificate.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_certificate_delete(
    context: &BindingCallContext,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.delete",
    ))
    .boxed())
}

/// Return one certificate descriptor.
///
/// Query one certificate handle and return normalized identity and validity metadata.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime certificate parsing, store, and chain-verification provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.certificate.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_certificate_descriptor(
    context: &BindingCallContext,
    out: *mut CryptoCertificateDescriptor,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.descriptor",
    ))
    .boxed())
}

/// Export one certificate from one handle.
///
/// Serialize one certificate handle into the requested encoding format.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime certificate parsing, store, and chain-verification provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.certificate.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_certificate_export(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoCertificateHandle,
    format: CryptoCertificateFormat,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, format);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.export",
    ))
    .boxed())
}

/// Import one certificate into one provider store.
///
/// Parse and import one certificate blob into one store and return one certificate handle.
/// Import visibility and persistence are enforced by runtime provider policies.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime certificate parsing, store, and chain-verification provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.certificate.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_certificate_import(
    context: &BindingCallContext,
    out: *mut resource::CryptoCertificateHandle,
    store: resource::CryptoStoreHandle,
    format: CryptoCertificateFormat,
    certificate: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, store, format, certificate);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.import",
    ))
    .boxed())
}

/// Verify one certificate chain against one trust policy.
///
/// Build and verify one certificate path for the requested purpose and verification time.
/// Chain building and policy evaluation follow runtime provider trust engine behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime certificate parsing, store, and chain-verification provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.certificate.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_certificate_verify(
    context: &BindingCallContext,
    out: *mut CryptoCertificateVerifyResult,
    request: CryptoCertificateVerifyRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.verify",
    ))
    .boxed())
}

/// Close one streaming cipher context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime symmetric cipher provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_cipher_close(
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.close")).boxed())
}

/// Decrypt one payload in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime symmetric cipher provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_cipher_decrypt(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, key, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.cipher.decrypt",
    ))
    .boxed())
}

/// Encrypt one payload in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime symmetric cipher provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_cipher_encrypt(
    context: &BindingCallContext,
    out: *mut CryptoCipherOutput,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, key, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.cipher.encrypt",
    ))
    .boxed())
}

/// Finalize one streaming cipher context.
///
/// Provide one final payload chunk.
/// Return output bytes and one authentication tag when applicable.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime symmetric cipher provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_cipher_finish(
    context: &BindingCallContext,
    out: *mut CryptoCipherOutput,
    handle: resource::CryptoCipherHandle,
    finalpayload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, finalpayload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.finish")).boxed())
}

/// Open one streaming cipher context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime symmetric cipher provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_cipher_open(
    context: &BindingCallContext,
    out: *mut resource::CryptoCipherHandle,
    key: resource::CryptoKeyHandle,
    direction: CryptoCipherDirection,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, key, direction, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.open")).boxed())
}

/// Reset one streaming cipher context with new parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime symmetric cipher provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_cipher_reset(
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    let _ = (context, handle, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.reset")).boxed())
}

/// Update one streaming cipher context with one payload chunk.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime symmetric cipher provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_cipher_update(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoCipherHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.update")).boxed())
}

/// Update additional authenticated data for one streaming cipher context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime symmetric cipher provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_cipher_update_additional_data(
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    additionaldata: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, handle, additionaldata);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.cipher.updateAdditionalData",
    ))
    .boxed())
}

/// Close one streaming digest context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime digest provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `crypto.digest`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_digest_close(
    context: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.close")).boxed())
}

/// Compute one digest in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime digest provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.digest`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_digest_compute(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    algorithm: CryptoDigestAlgorithm,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, algorithm, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.digest.compute",
    ))
    .boxed())
}

/// Finalize one streaming digest context and return one digest output.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime digest provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.digest`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_digest_finish(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.finish")).boxed())
}

/// Open one streaming digest context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime digest provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.digest`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_digest_open(
    context: &BindingCallContext,
    out: *mut resource::CryptoDigestHandle,
    algorithm: CryptoDigestAlgorithm,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, algorithm);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.open")).boxed())
}

/// Reset one streaming digest context to its initial state.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime digest provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.digest`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_digest_reset(
    context: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.reset")).boxed())
}

/// Update one streaming digest context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime digest provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.digest`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_digest_update(
    context: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, handle, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.update")).boxed())
}

/// Derive one key with Argon2id.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime KDF provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.kdf`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_kdf_argon2id(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoArgon2idRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.argon2id")).boxed())
}

/// Derive one key with HKDF.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime KDF provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.kdf`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_kdf_hkdf(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoHkdfRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.hkdf")).boxed())
}

/// Derive one key with PBKDF2.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime KDF provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.kdf`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_kdf_pbkdf2(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoPbkdf2Request,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.pbkdf2")).boxed())
}

/// Derive one key with scrypt.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime KDF provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.kdf`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_kdf_scrypt(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoScryptRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.scrypt")).boxed())
}

/// Decrypt one payload with one asymmetric key.
///
/// Decrypt one payload using one provider-backed private key.
/// Padding and label semantics are controlled by encryption parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.decrypt`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_decrypt(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.decrypt")).boxed())
}

/// Delete one key object.
///
/// Delete one provider-backed key object and invalidate this handle.
/// Deletion permissions and persistence policies are enforced by the runtime provider.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_delete(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.delete")).boxed())
}

/// Return one key descriptor.
///
/// Query one provider key object and return normalized metadata fields.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_descriptor(
    context: &BindingCallContext,
    out: *mut CryptoKeyDescriptor,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.descriptor",
    ))
    .boxed())
}

/// Encrypt one payload with one asymmetric key.
///
/// Encrypt one payload using one provider-backed public key.
/// Padding and label semantics are controlled by encryption parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.encrypt`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_encrypt(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.encrypt")).boxed())
}

/// Export one private key.
///
/// Export one private key representation in the requested encoding format.
/// The operation fails when provider policy marks this key as non-exportable.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_export_private(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, format);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.exportPrivate",
    ))
    .boxed())
}

/// Export one public key.
///
/// Export one public key representation in the requested encoding format.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_export_public(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, format);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.exportPublic",
    ))
    .boxed())
}

/// Export one secret key.
///
/// Export one symmetric or raw-secret key representation in the requested encoding format.
/// The operation fails when provider policy marks this key as non-exportable.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_export_secret(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, format);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.exportSecret",
    ))
    .boxed())
}

/// Generate one asymmetric key pair.
///
/// Create one provider-backed asymmetric key pair and return public and private handles.
/// Generation policy and persistence semantics follow runtime provider behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.generate`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_generate_pair(
    context: &BindingCallContext,
    out: *mut CryptoKeyPair,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, store, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.generatePair",
    ))
    .boxed())
}

/// Generate one symmetric key.
///
/// Create one provider-backed secret key object.
/// Generation policy and persistence semantics follow runtime provider behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.generate`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_generate_secret(
    context: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, store, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.generateSecret",
    ))
    .boxed())
}

/// Import one key object.
///
/// Parse and import one key blob into one provider store.
/// Key visibility and persistence follow runtime provider policies.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_import(
    context: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, store, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.import")).boxed())
}

/// Sign one payload.
///
/// Produce one signature over one payload using one provider-backed private key.
/// Payload hashing behavior is controlled by signature parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.sign`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_sign(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.sign")).boxed())
}

/// Unwrap one key.
///
/// Decrypt and import one wrapped key object into one provider store.
/// Import semantics follow runtime provider policy and the import request.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.unwrap`, `crypto.store.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_unwrap(
    context: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    wrappingkey: resource::CryptoKeyHandle,
    wrappedkey: NativeSlice<u8>,
    parameters: CryptoAsymmetricEncryptionParameters,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (
        context,
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
///
/// Verify one signature over one payload using one provider-backed public key.
/// Payload hashing behavior is controlled by signature parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.verify`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_verify(
    context: &BindingCallContext,
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
        context,
        out,
        handle,
        parameters,
        argument_payload,
        signature,
    );

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.verify")).boxed())
}

/// Wrap one key.
///
/// Export and encrypt one key object under one wrapping key.
/// Wrapping format and encryption semantics follow runtime provider behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime key-management and cryptographic provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.wrap`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_key_wrap(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    wrappingkey: resource::CryptoKeyHandle,
    keytowrap: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
    parameters: CryptoAsymmetricEncryptionParameters,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, wrappingkey, keytowrap, format, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.wrap")).boxed())
}

/// Close one streaming MAC context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime MAC provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_mac_close(
    context: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.close")).boxed())
}

/// Compute one message authentication code in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime MAC provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_mac_compute(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, key, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.compute")).boxed())
}

/// Finalize one streaming MAC context and return one tag.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime MAC provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_mac_finish(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.finish")).boxed())
}

/// Open one streaming MAC context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime MAC provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_mac_open(
    context: &BindingCallContext,
    out: *mut resource::CryptoMacHandle,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, key, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.open")).boxed())
}

/// Reset one streaming MAC context to its initial state.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime MAC provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_mac_reset(
    context: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.reset")).boxed())
}

/// Update one streaming MAC context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime MAC provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_mac_update(
    context: &BindingCallContext,
    handle: resource::CryptoMacHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, handle, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.update")).boxed())
}

/// Verify one message authentication code in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime MAC provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_mac_verify(
    context: &BindingCallContext,
    out: *mut bool,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    argument_payload: NativeSlice<u8>,
    tag: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, key, parameters, argument_payload, tag);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.verify")).boxed())
}

/// List supported key-agreement algorithms.
///
/// Return key-agreement algorithms available through active host provider implementations.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime provider capability introspection over host crypto implementations.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_probe_agreement_algorithms(
    context: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyAgreementAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.agreementAlgorithms",
    ))
    .boxed())
}

/// List supported cipher algorithms.
///
/// Return cipher algorithms available through active host provider implementations.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime provider capability introspection over host crypto implementations.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_probe_cipher_algorithms(
    context: &BindingCallContext,
    out: *mut NativeSlice<CryptoCipherAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.cipherAlgorithms",
    ))
    .boxed())
}

/// List supported digest algorithms.
///
/// Return digest algorithms available through active host provider implementations.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime provider capability introspection over host crypto implementations.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_probe_digest_algorithms(
    context: &BindingCallContext,
    out: *mut NativeSlice<CryptoDigestAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.digestAlgorithms",
    ))
    .boxed())
}

/// List supported KDF algorithms.
///
/// Return key-derivation algorithms available through active host provider implementations.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime provider capability introspection over host crypto implementations.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_probe_kdf_algorithms(
    context: &BindingCallContext,
    out: *mut NativeSlice<CryptoKdfAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.kdfAlgorithms",
    ))
    .boxed())
}

/// List supported key algorithm families.
///
/// Return the key algorithm families available through the active host provider set.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime provider capability introspection over host crypto implementations.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_probe_key_algorithms(
    context: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.keyAlgorithms",
    ))
    .boxed())
}

/// List supported key formats.
///
/// Return the key encoding formats supported by active host provider implementations.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime provider capability introspection over host crypto implementations.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_probe_key_formats(
    context: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyFormat>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.keyFormats",
    ))
    .boxed())
}

/// List supported MAC algorithms.
///
/// Return message-authentication algorithms available through active host providers.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime provider capability introspection over host crypto implementations.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_probe_mac_algorithms(
    context: &BindingCallContext,
    out: *mut NativeSlice<CryptoMacAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.macAlgorithms",
    ))
    .boxed())
}

/// List supported named curves.
///
/// Return elliptic-curve families available through active host provider implementations.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime provider capability introspection over host crypto implementations.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_probe_named_curves(
    context: &BindingCallContext,
    out: *mut NativeSlice<CryptoNamedCurve>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.namedCurves",
    ))
    .boxed())
}

/// List supported signature algorithms.
///
/// Return signature algorithms available through active host provider implementations.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime provider capability introspection over host crypto implementations.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_probe_signature_algorithms(
    context: &BindingCallContext,
    out: *mut NativeSlice<CryptoSignatureAlgorithm>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.signatureAlgorithms",
    ))
    .boxed())
}

/// Allocate one random byte vector with the requested length.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime CSPRNG provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.random`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_random_bytes(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    length: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, length);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.random.bytes")).boxed())
}

/// Fill one mutable byte slice with cryptographically secure random bytes.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime CSPRNG provider primitives.
///
/// # Errors
/// Returns invalidArgument, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.random`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_random_fill(
    context: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.random.fill")).boxed())
}

/// Close one crypto store.
///
/// Release one runtime provider store handle.
/// Open key and certificate handles remain valid according to runtime provider lifetime rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store provider primitives, and may return `notSupported` when host store backends are unavailable.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_store_close(
    context: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.store.close")).boxed())
}

/// List certificates from one store.
///
/// Enumerate certificate entries that match one query selector.
/// Result ordering and visibility follow runtime provider policies and caller permissions.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store provider primitives, and may return `notSupported` when host store backends are unavailable.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_store_list_certificates(
    context: &BindingCallContext,
    out: *mut CryptoCertificateListPage,
    handle: resource::CryptoStoreHandle,
    query: CryptoCertificateQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, query);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.listCertificates",
    ))
    .boxed())
}

/// List keys from one store.
///
/// Enumerate key entries that match one query selector.
/// Result ordering and visibility follow runtime provider policies and caller permissions.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store provider primitives, and may return `notSupported` when host store backends are unavailable.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_store_list_keys(
    context: &BindingCallContext,
    out: *mut CryptoKeyListPage,
    handle: resource::CryptoStoreHandle,
    query: CryptoKeyQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, query);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.listKeys",
    ))
    .boxed())
}

/// Open one crypto store.
///
/// Create one runtime provider store handle for key and certificate operations.
/// Provider selection and access scope follow runtime crypto store semantics.
/// `Ephemeral` store support is required.
/// Other kinds may return notSupported until host store providers are implemented.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store provider primitives, and may return `notSupported` when host store backends are unavailable.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_store_open(
    context: &BindingCallContext,
    out: *mut resource::CryptoStoreHandle,
    options: CryptoStoreOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.store.open")).boxed())
}
