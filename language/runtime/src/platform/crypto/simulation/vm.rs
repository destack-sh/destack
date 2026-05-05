#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequestVm, CryptoArgon2idRequestVm,
    CryptoAsymmetricEncryptionParametersVm, CryptoCertificateDescriptorVm, CryptoCertificateFormat,
    CryptoCertificateListPageVm, CryptoCertificateQueryVm, CryptoCertificateVerifyRequestVm,
    CryptoCertificateVerifyResultVm, CryptoCipherAlgorithm, CryptoCipherDirection,
    CryptoCipherOutputVm, CryptoCipherParametersVm, CryptoDigestAlgorithm, CryptoHkdfRequestVm,
    CryptoKdfAlgorithm, CryptoKeyAgreementAlgorithm, CryptoKeyAlgorithm, CryptoKeyDescriptorVm,
    CryptoKeyFormat, CryptoKeyGenerationRequestVm, CryptoKeyImportRequestVm, CryptoKeyListPageVm,
    CryptoKeyPairVm, CryptoKeyQueryVm, CryptoKeyResidency, CryptoKeyWrapAlgorithm,
    CryptoKeyWrapParametersVm, CryptoMacAlgorithm, CryptoMacParametersVm, CryptoNamedCurve,
    CryptoPbkdf2RequestVm, CryptoPrivateKeyExportRequestVm, CryptoScryptRequestVm,
    CryptoSignatureAlgorithm, CryptoSignatureParametersVm, CryptoStoreCapabilityVm,
    CryptoStoreKind, CryptoStoreOptionsVm, CryptoStoreProvider,
};
use crate::platform::{PlatformError, VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Derive one symmetric key from one local private key and one peer public key.
///
/// This operation performs key agreement and an explicit KDF stage.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL key-agreement and KDF primitives for software providers, and host key APIs for host-managed keys: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.agree`, `crypto.kdf`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_agreement_derive_key(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    request: CryptoAgreementDeriveKeyRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (privatekey, peerpublickey, request);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.agreement.deriveKey",
    ))
    .boxed())
}

/// Derive one shared secret from one local private key and one peer public key.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL key-agreement primitives for software providers, and host key APIs for host-managed keys: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.agree`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_agreement_derive_shared_secret(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    algorithm: CryptoKeyAgreementAlgorithm,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (privatekey, peerpublickey, algorithm);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.agreement.deriveSharedSecret",
    ))
    .boxed())
}

/// Delete one certificate from one store when allowed.
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
pub(crate) fn destack_crypto_certificate_delete(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_crypto_certificate_descriptor(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<CryptoCertificateDescriptorVm> {
    let _ = handle;
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
pub(crate) fn destack_crypto_certificate_export(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoCertificateHandle,
    format: CryptoCertificateFormat,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, format);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.export",
    ))
    .boxed())
}

/// Import one certificate into one store.
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
pub(crate) fn destack_crypto_certificate_import(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    store: resource::CryptoStoreHandle,
    format: CryptoCertificateFormat,
    certificate: VmSlice<u8>,
) -> RuntimeResult<resource::CryptoCertificateHandle> {
    let _ = (store, format, certificate);
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
pub(crate) fn destack_crypto_certificate_verify(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    request: CryptoCertificateVerifyRequestVm,
) -> RuntimeResult<CryptoCertificateVerifyResultVm> {
    let _ = request;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.verify",
    ))
    .boxed())
}

/// Close one streaming cipher context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_cipher_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoCipherHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.close")).boxed())
}

/// Decrypt one payload in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_cipher_decrypt(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParametersVm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (key, parameters, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.cipher.decrypt",
    ))
    .boxed())
}

/// Encrypt one payload in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_cipher_encrypt(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParametersVm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<CryptoCipherOutputVm> {
    let _ = (key, parameters, argument_payload);
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
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_cipher_finish(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoCipherHandle,
    finalpayload: VmSlice<u8>,
) -> RuntimeResult<CryptoCipherOutputVm> {
    let _ = (handle, finalpayload);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.finish")).boxed())
}

/// Open one streaming cipher context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_cipher_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    key: resource::CryptoKeyHandle,
    direction: CryptoCipherDirection,
    parameters: CryptoCipherParametersVm,
) -> RuntimeResult<resource::CryptoCipherHandle> {
    let _ = (key, direction, parameters);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.open")).boxed())
}

/// Reset one streaming cipher context with new parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_cipher_reset(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoCipherHandle,
    parameters: CryptoCipherParametersVm,
) -> RuntimeResult<()> {
    let _ = (handle, parameters);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.reset")).boxed())
}

/// Update one streaming cipher context with one payload chunk.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_cipher_update(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoCipherHandle,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.update")).boxed())
}

/// Update additional authenticated data for one streaming cipher context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP symmetric-cipher primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.cipher`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_cipher_update_additional_data(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoCipherHandle,
    additionaldata: VmSlice<u8>,
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
pub(crate) fn destack_crypto_digest_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_crypto_digest_compute(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    algorithm: CryptoDigestAlgorithm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (algorithm, argument_payload);
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
pub(crate) fn destack_crypto_digest_finish(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = handle;
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
pub(crate) fn destack_crypto_digest_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    algorithm: CryptoDigestAlgorithm,
) -> RuntimeResult<resource::CryptoDigestHandle> {
    let _ = algorithm;
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
pub(crate) fn destack_crypto_digest_reset(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_crypto_digest_update(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoDigestHandle,
    argument_payload: VmSlice<u8>,
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
pub(crate) fn destack_crypto_kdf_argon2id(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    request: CryptoArgon2idRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = request;
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
pub(crate) fn destack_crypto_kdf_hkdf(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    request: CryptoHkdfRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = request;
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
pub(crate) fn destack_crypto_kdf_pbkdf2(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    request: CryptoPbkdf2RequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = request;
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
pub(crate) fn destack_crypto_kdf_scrypt(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    request: CryptoScryptRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = request;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.scrypt")).boxed())
}

/// Decrypt one payload with one asymmetric key.
///
/// Decrypt one payload using one store-backed private key.
/// Padding and label semantics are controlled by encryption parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.decrypt`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_decrypt(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParametersVm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, parameters, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.decrypt")).boxed())
}

/// Delete one key object.
///
/// Delete one store-backed key object and invalidate this handle.
/// Deletion permissions and persistence policies are enforced by runtime store policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_delete(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_descriptor(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<CryptoKeyDescriptorVm> {
    let _ = handle;
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
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.encrypt`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_encrypt(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParametersVm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, parameters, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.encrypt")).boxed())
}

/// Export one private key.
///
/// Export one private key representation in the requested encoding format.
/// Output format and passphrase are provided by `CryptoPrivateKeyExportRequest`.
/// Encrypted PKCS#8 output requires one non-empty passphrase.
/// The operation fails when store policy marks this key as non-exportable.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_export_private(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoKeyHandle,
    request: CryptoPrivateKeyExportRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, request);
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
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_export_public(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, format);
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
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_export_secret(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, format);
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
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.generate`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_generate_pair(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequestVm,
) -> RuntimeResult<CryptoKeyPairVm> {
    let _ = (store, request);
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
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
/// Hardware-backed secret-key generation is available when the selected host store exposes symmetric hardware-key callbacks.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.generate`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_generate_secret(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequestVm,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    let _ = (store, request);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.generateSecret",
    ))
    .boxed())
}

/// Import one key object.
///
/// Parse and import one key blob into one store.
/// Key visibility and persistence follow runtime store policies.
/// Encrypted PKCS#8 inputs require one non-empty `request.passphrase`.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_import(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyImportRequestVm,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    let _ = (store, request);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.import")).boxed())
}

/// Sign one payload.
///
/// Produce one signature over one payload using one store-backed private key.
/// Payload hashing behavior is controlled by signature parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.sign`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_sign(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParametersVm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, parameters, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.sign")).boxed())
}

/// Unwrap one key.
///
/// Decrypt and import one wrapped key object into one store.
/// Import semantics follow runtime store policy and the import request.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.unwrap`, `crypto.store.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_unwrap(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    store: resource::CryptoStoreHandle,
    wrappingkey: resource::CryptoKeyHandle,
    wrappedkey: VmSlice<u8>,
    parameters: CryptoKeyWrapParametersVm,
    request: CryptoKeyImportRequestVm,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    let _ = (store, wrappingkey, wrappedkey, parameters, request);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.unwrap")).boxed())
}

/// Verify one signature.
///
/// Verify one signature over one payload using one store-backed public key.
/// Payload hashing behavior is controlled by signature parameters.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.verify`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_verify(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParametersVm,
    argument_payload: VmSlice<u8>,
    signature: VmSlice<u8>,
) -> RuntimeResult<bool> {
    let _ = (handle, parameters, argument_payload, signature);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.verify")).boxed())
}

/// Wrap one key.
///
/// Export and encrypt one key object under one wrapping key.
/// Wrapping semantics are selected by `CryptoKeyWrapParameters`.
/// `RsaOaep` uses asymmetric OAEP wrapping and `AesKw` or `AesKwp` use RFC 3394 or RFC 5649 key-wrap semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software key-management primitives and host key stores: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when configured for hardware-backed storage.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.wrap`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_wrap(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    wrappingkey: resource::CryptoKeyHandle,
    keytowrap: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
    parameters: CryptoKeyWrapParametersVm,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (wrappingkey, keytowrap, format, parameters);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.wrap")).boxed())
}

/// Close one streaming MAC context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_mac_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.close")).boxed())
}

/// Compute one message authentication code in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_mac_compute(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParametersVm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (key, parameters, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.compute")).boxed())
}

/// Finalize one streaming MAC context and return one tag.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_mac_finish(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.finish")).boxed())
}

/// Open one streaming MAC context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_mac_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParametersVm,
) -> RuntimeResult<resource::CryptoMacHandle> {
    let _ = (key, parameters);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.open")).boxed())
}

/// Reset one streaming MAC context to its initial state.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_mac_reset(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.reset")).boxed())
}

/// Update one streaming MAC context.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_mac_update(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoMacHandle,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.update")).boxed())
}

/// Verify one message authentication code in one shot.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL EVP MAC primitives for software keys, and host key APIs for host-managed secret keys when available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.mac`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_mac_verify(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParametersVm,
    argument_payload: VmSlice<u8>,
    tag: VmSlice<u8>,
) -> RuntimeResult<bool> {
    let _ = (key, parameters, argument_payload, tag);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.verify")).boxed())
}

/// List supported key-agreement algorithms.
///
/// Return key-agreement algorithms available through active host provider implementations.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto capability introspection over OpenSSL software providers and host key stores: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_probe_agreement_algorithms(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<CryptoKeyAgreementAlgorithm>> {
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
/// Uses runtime crypto capability introspection over OpenSSL software providers and host key stores: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_probe_cipher_algorithms(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<CryptoCipherAlgorithm>> {
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
/// Uses runtime crypto capability introspection over OpenSSL software providers and host key stores: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_probe_digest_algorithms(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<CryptoDigestAlgorithm>> {
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
/// Uses runtime crypto capability introspection over OpenSSL software providers and host key stores: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_probe_kdf_algorithms(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<CryptoKdfAlgorithm>> {
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
/// Uses runtime crypto capability introspection over OpenSSL software providers and host key stores: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_probe_key_algorithms(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<CryptoKeyAlgorithm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.keyAlgorithms",
    ))
    .boxed())
}

/// List supported key-wrap algorithms.
///
/// Return key-wrap algorithms available through active host provider implementations.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto capability introspection over OpenSSL software providers and host key stores: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_probe_key_wrap_algorithms(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<CryptoKeyWrapAlgorithm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.keyWrapAlgorithms",
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
/// Uses runtime crypto capability introspection over OpenSSL software providers and host key stores: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_probe_key_formats(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<CryptoKeyFormat>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.keyFormats",
    ))
    .boxed())
}

/// List supported key residencies.
///
/// Return key residencies available through active host provider implementations.
/// Results are capability snapshots and may vary across hosts and runtime builds.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto capability introspection over OpenSSL software providers and host key stores: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_probe_key_residencies(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<CryptoKeyResidency>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.probe.keyResidencies",
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
/// Uses runtime crypto capability introspection over OpenSSL software providers and host key stores: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_probe_mac_algorithms(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<CryptoMacAlgorithm>> {
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
/// Uses runtime crypto capability introspection over OpenSSL software providers and host key stores: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_probe_named_curves(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<CryptoNamedCurve>> {
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
/// Uses runtime crypto capability introspection over OpenSSL software providers and host key stores: Security.framework on Apple, CNG or Crypt32 on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_probe_signature_algorithms(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<CryptoSignatureAlgorithm>> {
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
pub(crate) fn destack_crypto_random_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    length: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = length;
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
pub(crate) fn destack_crypto_random_fill(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    buffer: VmSlice<u8>,
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
/// Uses runtime crypto store primitives over OpenSSL software providers and host stores: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software providers plus keystore callbacks when host callbacks are configured.
/// Operations may return `notSupported` when host stores are unavailable.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_store_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
/// Uses runtime crypto store primitives over OpenSSL software providers and host stores: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software providers plus keystore callbacks when host callbacks are configured.
/// Operations may return `notSupported` when host stores are unavailable.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_store_list_certificates(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoStoreHandle,
    query: CryptoCertificateQueryVm,
) -> RuntimeResult<CryptoCertificateListPageVm> {
    let _ = (handle, query);
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
/// Uses runtime crypto store primitives over OpenSSL software providers and host stores: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software providers plus keystore callbacks when host callbacks are configured.
/// Operations may return `notSupported` when host stores are unavailable.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_store_list_keys(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoStoreHandle,
    query: CryptoKeyQueryVm,
) -> RuntimeResult<CryptoKeyListPageVm> {
    let _ = (handle, query);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.listKeys",
    ))
    .boxed())
}

/// Return capabilities for one store backend identity.
///
/// Query one store kind and optional provider and return effective capability policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store capability introspection over OpenSSL software providers and host stores: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software providers plus keystore callbacks when host callbacks are configured.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_store_probe_capability(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    kind: CryptoStoreKind,
    provider: Option<CryptoStoreProvider>,
) -> RuntimeResult<CryptoStoreCapabilityVm> {
    let _ = (kind, provider);
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
/// Uses runtime crypto store capability introspection over OpenSSL software providers and host stores: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software providers plus keystore callbacks when host callbacks are configured.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_crypto_store_probe_kinds(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmArray<CryptoStoreKind>> {
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
/// Host-backed stores may expose certificate reads while rejecting key or certificate writes.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store primitives over OpenSSL software providers and host stores: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software providers plus keystore callbacks when host callbacks are configured.
/// Operations may return `notSupported` when host stores are unavailable.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_store_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    options: CryptoStoreOptionsVm,
) -> RuntimeResult<resource::CryptoStoreHandle> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.store.open")).boxed())
}
