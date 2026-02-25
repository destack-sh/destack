#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::BindingCallContext;

use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequest, CryptoArgon2idRequest, CryptoAsymmetricEncryptionParameters,
    CryptoCertificateDescriptor, CryptoCertificateFormat, CryptoCertificateListPage,
    CryptoCertificateQuery, CryptoCertificateVerifyRequest, CryptoCertificateVerifyResult,
    CryptoCipherAlgorithm, CryptoCipherDirection, CryptoCipherOutput, CryptoCipherParameters,
    CryptoDigestAlgorithm, CryptoHkdfRequest, CryptoKdfAlgorithm, CryptoKeyAgreementAlgorithm,
    CryptoKeyAlgorithm, CryptoKeyDescriptor, CryptoKeyFormat, CryptoKeyGenerationRequest,
    CryptoKeyImportRequest, CryptoKeyListPage, CryptoKeyPair, CryptoKeyQuery, CryptoMacAlgorithm,
    CryptoMacParameters, CryptoNamedCurve, CryptoPbkdf2Request, CryptoScryptRequest,
    CryptoSignatureAlgorithm, CryptoSignatureParameters, CryptoStoreCapability, CryptoStoreKind,
    CryptoStoreOptions, CryptoStoreProvider,
};
use crate::platform::resource;

/// Derive one symmetric key from one local private key and one peer public key.
///
/// This operation performs key agreement and an explicit KDF stage.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL key-agreement and KDF primitives for software lanes, and host key APIs for host-managed lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when registered.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    request: CryptoAgreementDeriveKeyRequest,
) -> RuntimeResult<()> {
    let _ = (out, privatekey, peerpublickey, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.agreement.deriveKey",
    ))
    .boxed())
}

/// Derive one shared secret from one local private key and one peer public key.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL key-agreement primitives for software lanes, and host key APIs for host-managed lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when registered.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    algorithm: CryptoKeyAgreementAlgorithm,
) -> RuntimeResult<()> {
    let _ = (out, privatekey, peerpublickey, algorithm);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.agreement.deriveSharedSecret",
    ))
    .boxed())
}

/// Delete one certificate from one store lane when allowed.
///
/// Remove one certificate object and invalidate the handle.
/// Deletion permissions and persistence are enforced by runtime store policies.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software certificate parsing and verification, plus host trust stores: Security.framework keychain and trust settings on Apple, and Crypt32 or CNG stores on Windows.
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
    _context: &BindingCallContext,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    let _ = handle;

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
/// Uses OpenSSL software certificate parsing and verification, plus host trust stores: Security.framework keychain and trust settings on Apple, and Crypt32 or CNG stores on Windows.
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
    _context: &BindingCallContext,
    out: *mut CryptoCertificateDescriptor,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

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
/// Uses OpenSSL software certificate parsing and verification, plus host trust stores: Security.framework keychain and trust settings on Apple, and Crypt32 or CNG stores on Windows.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoCertificateHandle,
    format: CryptoCertificateFormat,
) -> RuntimeResult<()> {
    let _ = (out, handle, format);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.export",
    ))
    .boxed())
}

/// Import one certificate into one store lane.
///
/// Parse and import one certificate blob into one store and return one certificate handle.
/// Import visibility and persistence are enforced by runtime store policies.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software certificate parsing and verification, plus host trust stores: Security.framework keychain and trust settings on Apple, and Crypt32 or CNG stores on Windows.
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
    _context: &BindingCallContext,
    out: *mut resource::CryptoCertificateHandle,
    store: resource::CryptoStoreHandle,
    format: CryptoCertificateFormat,
    certificate: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, store, format, certificate);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.import",
    ))
    .boxed())
}

/// Verify one certificate chain against one trust policy.
///
/// Build and verify one certificate path for the requested purpose and verification time.
/// Chain building and policy evaluation follow runtime trust engine behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software certificate parsing and verification, plus host trust stores: Security.framework keychain and trust settings on Apple, and Crypt32 or CNG stores on Windows.
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
    _context: &BindingCallContext,
    out: *mut CryptoCertificateVerifyResult,
    request: CryptoCertificateVerifyRequest,
) -> RuntimeResult<()> {
    let _ = (out, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.verify",
    ))
    .boxed())
}

/// Close one streaming cipher context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.close")).boxed())
}

/// Decrypt one payload in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, key, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.cipher.decrypt",
    ))
    .boxed())
}

/// Encrypt one payload in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    out: *mut CryptoCipherOutput,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, key, parameters, argument_payload);

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
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    out: *mut CryptoCipherOutput,
    handle: resource::CryptoCipherHandle,
    finalpayload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, finalpayload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.finish")).boxed())
}

/// Open one streaming cipher context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    out: *mut resource::CryptoCipherHandle,
    key: resource::CryptoKeyHandle,
    direction: CryptoCipherDirection,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    let _ = (out, key, direction, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.open")).boxed())
}

/// Reset one streaming cipher context with new parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    let _ = (handle, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.reset")).boxed())
}

/// Update one streaming cipher context with one payload chunk.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoCipherHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.update")).boxed())
}

/// Update additional authenticated data for one streaming cipher context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    additionaldata: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, additionaldata);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.cipher.updateAdditionalData",
    ))
    .boxed())
}

/// Close one streaming digest context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP digest primitives on Unix and Windows.
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
    _context: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.close")).boxed())
}

/// Compute one digest in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP digest primitives on Unix and Windows.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    algorithm: CryptoDigestAlgorithm,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, algorithm, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.digest.compute",
    ))
    .boxed())
}

/// Finalize one streaming digest context and return one digest output.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP digest primitives on Unix and Windows.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.finish")).boxed())
}

/// Open one streaming digest context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP digest primitives on Unix and Windows.
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
    _context: &BindingCallContext,
    out: *mut resource::CryptoDigestHandle,
    algorithm: CryptoDigestAlgorithm,
) -> RuntimeResult<()> {
    let _ = (out, algorithm);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.open")).boxed())
}

/// Reset one streaming digest context to its initial state.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP digest primitives on Unix and Windows.
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
    _context: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.reset")).boxed())
}

/// Update one streaming digest context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP digest primitives on Unix and Windows.
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
    _context: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.update")).boxed())
}

/// Derive one key with Argon2id.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL KDF primitives on Unix and Windows.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoArgon2idRequest,
) -> RuntimeResult<()> {
    let _ = (out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.argon2id")).boxed())
}

/// Derive one key with HKDF.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL KDF primitives on Unix and Windows.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoHkdfRequest,
) -> RuntimeResult<()> {
    let _ = (out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.hkdf")).boxed())
}

/// Derive one key with PBKDF2.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL KDF primitives on Unix and Windows.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoPbkdf2Request,
) -> RuntimeResult<()> {
    let _ = (out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.pbkdf2")).boxed())
}

/// Derive one key with scrypt.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL KDF primitives on Unix and Windows.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoScryptRequest,
) -> RuntimeResult<()> {
    let _ = (out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.scrypt")).boxed())
}

/// Decrypt one payload with one asymmetric key.
///
/// Decrypt one payload using one store-backed private key.
/// Padding and label semantics are controlled by encryption parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.decrypt")).boxed())
}

/// Delete one key object.
///
/// Delete one store-backed key object and invalidate this handle.
/// Deletion permissions and persistence policies are enforced by runtime store policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.delete")).boxed())
}

/// Return one key descriptor.
///
/// Query one key object and return normalized metadata fields.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    out: *mut CryptoKeyDescriptor,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.descriptor",
    ))
    .boxed())
}

/// Encrypt one payload with one asymmetric key.
///
/// Encrypt one payload using one store-backed public key.
/// Padding and label semantics are controlled by encryption parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.encrypt")).boxed())
}

/// Export one private key.
///
/// Export one private key representation in the requested encoding format.
/// The operation fails when store policy marks this key as non-exportable.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<()> {
    let _ = (out, handle, format);

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
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<()> {
    let _ = (out, handle, format);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.exportPublic",
    ))
    .boxed())
}

/// Export one secret key.
///
/// Export one symmetric or raw-secret key representation in the requested encoding format.
/// The operation fails when store policy marks this key as non-exportable.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<()> {
    let _ = (out, handle, format);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.exportSecret",
    ))
    .boxed())
}

/// Generate one asymmetric key pair.
///
/// Create one store-backed asymmetric key pair and return public and private handles.
/// Generation policy and persistence semantics follow runtime store behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    out: *mut CryptoKeyPair,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<()> {
    let _ = (out, store, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.generatePair",
    ))
    .boxed())
}

/// Generate one symmetric key.
///
/// Create one store-backed secret key object.
/// Generation policy and persistence semantics follow runtime store behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
/// Hardware-backed secret-key generation is available when the selected host lane exposes symmetric hardware-key callbacks.
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
    _context: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<()> {
    let _ = (out, store, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.generateSecret",
    ))
    .boxed())
}

/// Import one key object.
///
/// Parse and import one key blob into one store lane.
/// Key visibility and persistence follow runtime store policies.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<()> {
    let _ = (out, store, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.import")).boxed())
}

/// Sign one payload.
///
/// Produce one signature over one payload using one store-backed private key.
/// Payload hashing behavior is controlled by signature parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.sign")).boxed())
}

/// Unwrap one key.
///
/// Decrypt and import one wrapped key object into one store lane.
/// Import semantics follow runtime store policy and the import request.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    wrappingkey: resource::CryptoKeyHandle,
    wrappedkey: NativeSlice<u8>,
    parameters: CryptoAsymmetricEncryptionParameters,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<()> {
    let _ = (out, store, wrappingkey, wrappedkey, parameters, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.unwrap")).boxed())
}

/// Verify one signature.
///
/// Verify one signature over one payload using one store-backed public key.
/// Payload hashing behavior is controlled by signature parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    out: *mut bool,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParameters,
    argument_payload: NativeSlice<u8>,
    signature: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, parameters, argument_payload, signature);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.verify")).boxed())
}

/// Wrap one key.
///
/// Export and encrypt one key object under one wrapping key.
/// Wrapping format and encryption semantics follow runtime store behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key-store lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed lanes.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    wrappingkey: resource::CryptoKeyHandle,
    keytowrap: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
    parameters: CryptoAsymmetricEncryptionParameters,
) -> RuntimeResult<()> {
    let _ = (out, wrappingkey, keytowrap, format, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.wrap")).boxed())
}

/// Close one streaming MAC context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.close")).boxed())
}

/// Compute one message authentication code in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, key, parameters, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.compute")).boxed())
}

/// Finalize one streaming MAC context and return one tag.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.finish")).boxed())
}

/// Open one streaming MAC context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    out: *mut resource::CryptoMacHandle,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
) -> RuntimeResult<()> {
    let _ = (out, key, parameters);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.open")).boxed())
}

/// Reset one streaming MAC context to its initial state.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.reset")).boxed())
}

/// Update one streaming MAC context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    handle: resource::CryptoMacHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.update")).boxed())
}

/// Verify one message authentication code in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret-key lanes when available.
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
    _context: &BindingCallContext,
    out: *mut bool,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    argument_payload: NativeSlice<u8>,
    tag: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, key, parameters, argument_payload, tag);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.verify")).boxed())
}

/// List supported key-agreement algorithms.
///
/// Return key-agreement algorithms available through active host provider implementations.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto capability introspection over OpenSSL software lanes and host key-store lanes: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyAgreementAlgorithm>,
) -> RuntimeResult<()> {
    let _ = out;

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
/// Uses runtime crypto capability introspection over OpenSSL software lanes and host key-store lanes: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<CryptoCipherAlgorithm>,
) -> RuntimeResult<()> {
    let _ = out;

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
/// Uses runtime crypto capability introspection over OpenSSL software lanes and host key-store lanes: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<CryptoDigestAlgorithm>,
) -> RuntimeResult<()> {
    let _ = out;

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
/// Uses runtime crypto capability introspection over OpenSSL software lanes and host key-store lanes: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<CryptoKdfAlgorithm>,
) -> RuntimeResult<()> {
    let _ = out;

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
/// Uses runtime crypto capability introspection over OpenSSL software lanes and host key-store lanes: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyAlgorithm>,
) -> RuntimeResult<()> {
    let _ = out;

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
/// Uses runtime crypto capability introspection over OpenSSL software lanes and host key-store lanes: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyFormat>,
) -> RuntimeResult<()> {
    let _ = out;

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
/// Uses runtime crypto capability introspection over OpenSSL software lanes and host key-store lanes: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<CryptoMacAlgorithm>,
) -> RuntimeResult<()> {
    let _ = out;

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
/// Uses runtime crypto capability introspection over OpenSSL software lanes and host key-store lanes: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<CryptoNamedCurve>,
) -> RuntimeResult<()> {
    let _ = out;

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
/// Uses runtime crypto capability introspection over OpenSSL software lanes and host key-store lanes: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<CryptoSignatureAlgorithm>,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.signatureAlgorithms",
    ))
    .boxed())
}

/// Allocate one random byte vector with the requested length.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL RAND primitives backed by host entropy sources on Unix and Windows.
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
    _context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    length: u32,
) -> RuntimeResult<()> {
    let _ = (out, length);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.random.bytes")).boxed())
}

/// Fill one mutable byte slice with cryptographically secure random bytes.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL RAND primitives backed by host entropy sources on Unix and Windows.
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
    _context: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = buffer;

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.random.fill")).boxed())
}

/// Close one crypto store.
///
/// Release one runtime crypto store handle.
/// Open key and certificate handles remain valid according to runtime store lifetime rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store primitives over OpenSSL software lanes and host store lanes: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software lanes plus keystore lanes when host callbacks are configured.
/// Operations may return `notSupported` when host store lanes are unavailable.
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
    _context: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
) -> RuntimeResult<()> {
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.store.close")).boxed())
}

/// List certificates from one store.
///
/// Enumerate certificate entries that match one query selector.
/// Result ordering and visibility follow runtime store policies and caller permissions.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store primitives over OpenSSL software lanes and host store lanes: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software lanes plus keystore lanes when host callbacks are configured.
/// Operations may return `notSupported` when host store lanes are unavailable.
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
    _context: &BindingCallContext,
    out: *mut CryptoCertificateListPage,
    handle: resource::CryptoStoreHandle,
    query: CryptoCertificateQuery,
) -> RuntimeResult<()> {
    let _ = (out, handle, query);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.listCertificates",
    ))
    .boxed())
}

/// List keys from one store.
///
/// Enumerate key entries that match one query selector.
/// Result ordering and visibility follow runtime store policies and caller permissions.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store primitives over OpenSSL software lanes and host store lanes: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software lanes plus keystore lanes when host callbacks are configured.
/// Operations may return `notSupported` when host store lanes are unavailable.
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
    _context: &BindingCallContext,
    out: *mut CryptoKeyListPage,
    handle: resource::CryptoStoreHandle,
    query: CryptoKeyQuery,
) -> RuntimeResult<()> {
    let _ = (out, handle, query);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.listKeys",
    ))
    .boxed())
}

/// Return capabilities for one store backend lane.
///
/// Query one store kind and optional provider and return effective capability policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store capability introspection over OpenSSL software lanes and host store lanes: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software lanes plus keystore lanes when host callbacks are configured.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_store_probe_capability(
    _context: &BindingCallContext,
    out: *mut CryptoStoreCapability,
    kind: CryptoStoreKind,
    provider: CryptoStoreProvider,
) -> RuntimeResult<()> {
    let _ = (out, kind, provider);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.probeCapability",
    ))
    .boxed())
}

/// List store backend kinds that are currently available.
///
/// Return one runtime capability snapshot for store backends that can be opened.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store capability introspection over OpenSSL software lanes and host store lanes: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software lanes plus keystore lanes when host callbacks are configured.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_store_probe_kinds(
    _context: &BindingCallContext,
    out: *mut NativeArray<CryptoStoreKind>,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.probeKinds",
    ))
    .boxed())
}

/// Open one crypto store.
///
/// Create one runtime crypto store handle for key and certificate operations.
/// Provider selection and access scope follow runtime crypto store semantics.
/// `Ephemeral` and `Provider` store support is required.
/// Host-backed `System`, `User`, and `Machine` support is host dependent.
/// Host-backed lanes may expose certificate reads while rejecting key or certificate writes.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store primitives over OpenSSL software lanes and host store lanes: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software lanes plus keystore lanes when host callbacks are configured.
/// Operations may return `notSupported` when host store lanes are unavailable.
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
    _context: &BindingCallContext,
    out: *mut resource::CryptoStoreHandle,
    options: CryptoStoreOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.store.open")).boxed())
}
