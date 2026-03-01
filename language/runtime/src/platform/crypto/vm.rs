use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequest, CryptoAgreementDeriveKeyRequestVm, CryptoArgon2idRequest,
    CryptoArgon2idRequestVm, CryptoAsymmetricEncryptionParameters,
    CryptoAsymmetricEncryptionParametersVm, CryptoCertificateDescriptor,
    CryptoCertificateDescriptorVm, CryptoCertificateFormat, CryptoCertificateListEntryVm,
    CryptoCertificateListPage, CryptoCertificateListPageVm, CryptoCertificateQuery,
    CryptoCertificateQueryVm, CryptoCertificateValidityVm, CryptoCertificateVerifyIdentity,
    CryptoCertificateVerifyRequest, CryptoCertificateVerifyRequestVm,
    CryptoCertificateVerifyResult, CryptoCertificateVerifyResultVm, CryptoCipherAlgorithm,
    CryptoCipherDirection, CryptoCipherOutput, CryptoCipherOutputVm, CryptoCipherParameters,
    CryptoCipherParametersVm, CryptoDigestAlgorithm, CryptoHkdfRequest, CryptoHkdfRequestVm,
    CryptoKdfAlgorithm, CryptoKeyAgreementAlgorithm, CryptoKeyAlgorithm, CryptoKeyDescriptor,
    CryptoKeyDescriptorAesVm, CryptoKeyDescriptorChaCha20Vm, CryptoKeyDescriptorEcVm,
    CryptoKeyDescriptorEd448Vm, CryptoKeyDescriptorEd25519Vm, CryptoKeyDescriptorHmacVm,
    CryptoKeyDescriptorRsaVm, CryptoKeyDescriptorVm, CryptoKeyDescriptorX448Vm,
    CryptoKeyDescriptorX25519Vm, CryptoKeyFormat, CryptoKeyGenerationRequest,
    CryptoKeyGenerationRequestAes, CryptoKeyGenerationRequestChaCha20,
    CryptoKeyGenerationRequestEc, CryptoKeyGenerationRequestEd448,
    CryptoKeyGenerationRequestEd25519, CryptoKeyGenerationRequestHmac,
    CryptoKeyGenerationRequestRsa, CryptoKeyGenerationRequestVm, CryptoKeyGenerationRequestX448,
    CryptoKeyGenerationRequestX25519, CryptoKeyImportRequest, CryptoKeyImportRequestAes,
    CryptoKeyImportRequestChaCha20, CryptoKeyImportRequestEc, CryptoKeyImportRequestEd448,
    CryptoKeyImportRequestEd25519, CryptoKeyImportRequestHmac, CryptoKeyImportRequestRsa,
    CryptoKeyImportRequestVm, CryptoKeyImportRequestX448, CryptoKeyImportRequestX25519,
    CryptoKeyListEntryVm, CryptoKeyListPage, CryptoKeyListPageVm, CryptoKeyPair, CryptoKeyPairVm,
    CryptoKeyQuery, CryptoKeyQueryVm, CryptoKeyResidency, CryptoKeyWrapAlgorithm,
    CryptoKeyWrapParameters, CryptoKeyWrapParametersVm, CryptoMacAlgorithm, CryptoMacParametersVm,
    CryptoNamedCurve, CryptoPbkdf2Request, CryptoPbkdf2RequestVm, CryptoPrivateKeyExportRequest,
    CryptoPrivateKeyExportRequestVm, CryptoScryptRequest, CryptoScryptRequestVm,
    CryptoSignatureAlgorithm, CryptoSignatureParametersVm,
    CryptoStoreAsymmetricEncryptionCapabilityVm, CryptoStoreCapability, CryptoStoreCapabilityVm,
    CryptoStoreCertificateCapabilityVm, CryptoStoreCipherCapabilityVm, CryptoStoreIdentityVm,
    CryptoStoreKeyCapabilityVm, CryptoStoreKeyWrapCapabilityVm, CryptoStoreKind,
    CryptoStoreMacCapabilityVm, CryptoStoreOptions, CryptoStoreOptionsVm, CryptoStoreProvenance,
    CryptoStoreProvenanceVm, CryptoStoreProvider, CryptoStoreSignatureCapabilityVm,
    host as host_crypto,
};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, VmAggregateCodec, VmArray, VmSlice, resource,
};
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    request: CryptoAgreementDeriveKeyRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let request = agreement_request_from_vm(runtime, context, request)?;
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_agreement_derive_key(
            runtime,
            out,
            privatekey,
            peerpublickey,
            request,
        )
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    algorithm: CryptoKeyAgreementAlgorithm,
) -> RuntimeResult<VmSlice<u8>> {
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_agreement_derive_shared_secret(
            runtime,
            out,
            privatekey,
            peerpublickey,
            algorithm,
        )
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_certificate_delete(runtime, handle) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<CryptoCertificateDescriptorVm> {
    let descriptor = call_out(|out| unsafe {
        host_crypto::destack_crypto_certificate_descriptor(runtime, out, handle)
    })?;

    certificate_descriptor_to_vm(context, descriptor)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoCertificateHandle,
    format: CryptoCertificateFormat,
) -> RuntimeResult<VmSlice<u8>> {
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_certificate_export(runtime, out, handle, format)
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    store: resource::CryptoStoreHandle,
    format: CryptoCertificateFormat,
    certificate: VmSlice<u8>,
) -> RuntimeResult<resource::CryptoCertificateHandle> {
    let certificate = bytes_from_vm(runtime, context, certificate)?;
    call_out(|out| unsafe {
        host_crypto::destack_crypto_certificate_import(runtime, out, store, format, certificate)
    })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoCertificateVerifyRequestVm,
) -> RuntimeResult<CryptoCertificateVerifyResultVm> {
    let request = certificate_verify_request_from_vm(runtime, context, request)?;
    let result = call_out(|out| unsafe {
        host_crypto::destack_crypto_certificate_verify(runtime, out, request)
    })?;
    certificate_verify_result_to_vm(context, result)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoCipherHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_cipher_close(runtime, handle) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParametersVm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let parameters = cipher_parameters_from_vm(runtime, context, parameters)?;
    let payload = bytes_from_vm(runtime, context, argument_payload)?;
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_cipher_decrypt(runtime, out, key, parameters, payload)
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParametersVm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<CryptoCipherOutputVm> {
    let parameters = cipher_parameters_from_vm(runtime, context, parameters)?;
    let payload = bytes_from_vm(runtime, context, argument_payload)?;
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_cipher_encrypt(runtime, out, key, parameters, payload)
    })?;

    cipher_output_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoCipherHandle,
    finalpayload: VmSlice<u8>,
) -> RuntimeResult<CryptoCipherOutputVm> {
    let payload = bytes_from_vm(runtime, context, finalpayload)?;
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_cipher_finish(runtime, out, handle, payload)
    })?;

    cipher_output_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    key: resource::CryptoKeyHandle,
    direction: CryptoCipherDirection,
    parameters: CryptoCipherParametersVm,
) -> RuntimeResult<resource::CryptoCipherHandle> {
    let parameters = cipher_parameters_from_vm(runtime, context, parameters)?;
    call_out(|out| unsafe {
        host_crypto::destack_crypto_cipher_open(runtime, out, key, direction, parameters)
    })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoCipherHandle,
    parameters: CryptoCipherParametersVm,
) -> RuntimeResult<()> {
    let parameters = cipher_parameters_from_vm(runtime, context, parameters)?;
    unsafe { host_crypto::destack_crypto_cipher_reset(runtime, handle, parameters) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoCipherHandle,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let payload = bytes_from_vm(runtime, context, argument_payload)?;
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_cipher_update(runtime, out, handle, payload)
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoCipherHandle,
    additionaldata: VmSlice<u8>,
) -> RuntimeResult<()> {
    let additional_data = bytes_from_vm(runtime, context, additionaldata)?;
    unsafe {
        host_crypto::destack_crypto_cipher_update_additional_data(runtime, handle, additional_data)
    }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_digest_close(runtime, handle) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    algorithm: CryptoDigestAlgorithm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let payload = bytes_from_vm(runtime, context, argument_payload)?;
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_digest_compute(runtime, out, algorithm, payload)
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<VmSlice<u8>> {
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_digest_finish(runtime, out, handle) })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    algorithm: CryptoDigestAlgorithm,
) -> RuntimeResult<resource::CryptoDigestHandle> {
    call_out(|out| unsafe { host_crypto::destack_crypto_digest_open(runtime, out, algorithm) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_digest_reset(runtime, handle) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoDigestHandle,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<()> {
    let payload = bytes_from_vm(runtime, context, argument_payload)?;
    unsafe { host_crypto::destack_crypto_digest_update(runtime, handle, payload) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoArgon2idRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let request = argon2id_request_from_vm(runtime, context, request)?;
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_kdf_argon2id(runtime, out, request) })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoHkdfRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let request = hkdf_request_from_vm(runtime, context, request)?;
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_kdf_hkdf(runtime, out, request) })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoPbkdf2RequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let request = pbkdf2_request_from_vm(runtime, context, request)?;
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_kdf_pbkdf2(runtime, out, request) })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoScryptRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let request = scrypt_request_from_vm(runtime, context, request)?;
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_kdf_scrypt(runtime, out, request) })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParametersVm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let parameters = asymmetric_encryption_parameters_from_vm(runtime, context, parameters)?;
    let payload = bytes_from_vm(runtime, context, argument_payload)?;
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_key_decrypt(runtime, out, handle, parameters, payload)
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_key_delete(runtime, handle) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<CryptoKeyDescriptorVm> {
    let descriptor = call_out(|out| unsafe {
        host_crypto::destack_crypto_key_descriptor(runtime, out, handle)
    })?;

    key_descriptor_to_vm(context, descriptor)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParametersVm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let parameters = asymmetric_encryption_parameters_from_vm(runtime, context, parameters)?;
    let payload = bytes_from_vm(runtime, context, argument_payload)?;
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_key_encrypt(runtime, out, handle, parameters, payload)
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
    request: CryptoPrivateKeyExportRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let request = private_key_export_request_from_vm(runtime, context, request)?;
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_key_export_private(runtime, out, handle, request)
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<VmSlice<u8>> {
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_key_export_public(runtime, out, handle, format)
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<VmSlice<u8>> {
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_key_export_secret(runtime, out, handle, format)
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequestVm,
) -> RuntimeResult<CryptoKeyPairVm> {
    let request = key_generation_request_from_vm(runtime, context, request)?;
    let pair = call_out(|out| unsafe {
        host_crypto::destack_crypto_key_generate_pair(runtime, out, store, request)
    })?;

    Ok(key_pair_to_vm(pair))
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequestVm,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    let request = key_generation_request_from_vm(runtime, context, request)?;
    call_out(|out| unsafe {
        host_crypto::destack_crypto_key_generate_secret(runtime, out, store, request)
    })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyImportRequestVm,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    let request = key_import_request_from_vm(runtime, context, request)?;
    call_out(|out| unsafe { host_crypto::destack_crypto_key_import(runtime, out, store, request) })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParametersVm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let payload = bytes_from_vm(runtime, context, argument_payload)?;
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_key_sign(runtime, out, handle, parameters, payload)
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    store: resource::CryptoStoreHandle,
    wrappingkey: resource::CryptoKeyHandle,
    wrappedkey: VmSlice<u8>,
    parameters: CryptoKeyWrapParametersVm,
    request: CryptoKeyImportRequestVm,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    let wrapped_key = bytes_from_vm(runtime, context, wrappedkey)?;
    let parameters = key_wrap_parameters_from_vm(runtime, context, parameters)?;
    let request = key_import_request_from_vm(runtime, context, request)?;
    call_out(|out| unsafe {
        host_crypto::destack_crypto_key_unwrap(
            runtime,
            out,
            store,
            wrappingkey,
            wrapped_key,
            parameters,
            request,
        )
    })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParametersVm,
    argument_payload: VmSlice<u8>,
    signature: VmSlice<u8>,
) -> RuntimeResult<bool> {
    let payload = bytes_from_vm(runtime, context, argument_payload)?;
    let signature = bytes_from_vm(runtime, context, signature)?;
    call_out(|out| unsafe {
        host_crypto::destack_crypto_key_verify(runtime, out, handle, parameters, payload, signature)
    })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    wrappingkey: resource::CryptoKeyHandle,
    keytowrap: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
    parameters: CryptoKeyWrapParametersVm,
) -> RuntimeResult<VmSlice<u8>> {
    let parameters = key_wrap_parameters_from_vm(runtime, context, parameters)?;
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_key_wrap(
            runtime,
            out,
            wrappingkey,
            keytowrap,
            format,
            parameters,
        )
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_mac_close(runtime, handle) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParametersVm,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let payload = bytes_from_vm(runtime, context, argument_payload)?;
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_mac_compute(runtime, out, key, parameters, payload)
    })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<VmSlice<u8>> {
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_mac_finish(runtime, out, handle) })?;

    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParametersVm,
) -> RuntimeResult<resource::CryptoMacHandle> {
    call_out(|out| unsafe { host_crypto::destack_crypto_mac_open(runtime, out, key, parameters) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_mac_reset(runtime, handle) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoMacHandle,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<()> {
    let payload = bytes_from_vm(runtime, context, argument_payload)?;
    unsafe { host_crypto::destack_crypto_mac_update(runtime, handle, payload) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParametersVm,
    argument_payload: VmSlice<u8>,
    tag: VmSlice<u8>,
) -> RuntimeResult<bool> {
    let payload = bytes_from_vm(runtime, context, argument_payload)?;
    let tag = bytes_from_vm(runtime, context, tag)?;
    call_out(|out| unsafe {
        host_crypto::destack_crypto_mac_verify(runtime, out, key, parameters, payload, tag)
    })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<CryptoKeyAgreementAlgorithm>> {
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_probe_agreement_algorithms(runtime, out)
    })?;
    slice_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<CryptoCipherAlgorithm>> {
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_probe_cipher_algorithms(runtime, out)
    })?;
    slice_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<CryptoDigestAlgorithm>> {
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_probe_digest_algorithms(runtime, out)
    })?;
    slice_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<CryptoKdfAlgorithm>> {
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_probe_kdf_algorithms(runtime, out) })?;
    slice_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<CryptoKeyAlgorithm>> {
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_probe_key_algorithms(runtime, out) })?;
    slice_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<CryptoKeyWrapAlgorithm>> {
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_probe_key_wrap_algorithms(runtime, out)
    })?;
    slice_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<CryptoKeyFormat>> {
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_probe_key_formats(runtime, out) })?;
    slice_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<CryptoKeyResidency>> {
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_probe_key_residencies(runtime, out) })?;
    slice_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<CryptoMacAlgorithm>> {
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_probe_mac_algorithms(runtime, out) })?;
    slice_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<CryptoNamedCurve>> {
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_probe_named_curves(runtime, out) })?;
    slice_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<CryptoSignatureAlgorithm>> {
    let output = call_out(|out| unsafe {
        host_crypto::destack_crypto_probe_signature_algorithms(runtime, out)
    })?;
    slice_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    length: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let output =
        call_out(|out| unsafe { host_crypto::destack_crypto_random_bytes(runtime, out, length) })?;
    bytes_to_vm(context, output)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    let native_buffer = runtime.store_slice(vec![0_u8; buffer.len as usize]);
    unsafe { host_crypto::destack_crypto_random_fill(runtime, native_buffer) }?;
    copy_native_bytes_into_vm(context, buffer, native_buffer)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoStoreHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_store_close(runtime, handle) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoStoreHandle,
    query: CryptoCertificateQueryVm,
) -> RuntimeResult<CryptoCertificateListPageVm> {
    let query = certificate_query_from_vm(runtime, context, query)?;
    let page = call_out(|out| unsafe {
        host_crypto::destack_crypto_store_list_certificates(runtime, out, handle, query)
    })?;

    certificate_list_page_to_vm(context, page)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoStoreHandle,
    query: CryptoKeyQueryVm,
) -> RuntimeResult<CryptoKeyListPageVm> {
    let query = key_query_from_vm(runtime, context, query)?;
    let page = call_out(|out| unsafe {
        host_crypto::destack_crypto_store_list_keys(runtime, out, handle, query)
    })?;

    key_list_page_to_vm(context, page)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    kind: CryptoStoreKind,
    provider: CryptoStoreProvider,
) -> RuntimeResult<CryptoStoreCapabilityVm> {
    let capability = call_out(|out| unsafe {
        host_crypto::destack_crypto_store_probe_capability(runtime, out, kind, provider)
    })?;

    store_capability_to_vm(context, capability)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<CryptoStoreKind>> {
    let kinds =
        call_out(|out| unsafe { host_crypto::destack_crypto_store_probe_kinds(runtime, out) })?;
    array_to_vm(context, kinds)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: CryptoStoreOptionsVm,
) -> RuntimeResult<resource::CryptoStoreHandle> {
    let options = store_options_from_vm(runtime, context, options)?;
    call_out(|out| unsafe { host_crypto::destack_crypto_store_open(runtime, out, options) })
}

fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    let mut value = std::mem::MaybeUninit::<T>::uninit();
    call(value.as_mut_ptr())?;
    Ok(unsafe { value.assume_init() })
}

fn string_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    let value = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    Ok(runtime.store_string(value.as_str()))
}

fn string_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeStringRef,
) -> RuntimeResult<vm::StringHandle> {
    let value = unsafe { value.as_str()? };
    Ok(vm::StringHandle::new(context.intern_string(value)))
}

fn bytes_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: VmSlice<u8>,
) -> RuntimeResult<NativeSlice<u8>> {
    let bytes = value.read_bytes(context)?;
    Ok(runtime.store_slice(bytes))
}

fn slice_from_vm<T: Copy + VmAggregateCodec + 'static>(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: VmSlice<T>,
) -> RuntimeResult<NativeSlice<T>> {
    let values = value.read_values(context)?;
    Ok(runtime.store_slice(values))
}

fn bytes_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let value = unsafe { value.as_slice()? };
    Ok(VmSlice::from_bytes(context, value))
}

fn slice_to_vm<T: Copy + VmAggregateCodec>(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeSlice<T>,
) -> RuntimeResult<VmSlice<T>> {
    let value = unsafe { value.as_slice()? };
    VmSlice::from_values(context, value)
}

fn array_to_vm<T: Copy + VmAggregateCodec>(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeArray<T>,
) -> RuntimeResult<VmArray<T>> {
    let value = unsafe { value.as_slice()? };
    VmArray::from_values(context, value)
}

fn copy_native_bytes_into_vm(
    context: &mut vm::ExternalCallContext<'_>,
    output: VmSlice<u8>,
    value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let value = unsafe { value.as_slice()? };
    output.write_bytes(context, value)
}

fn agreement_request_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoAgreementDeriveKeyRequestVm,
) -> RuntimeResult<CryptoAgreementDeriveKeyRequest> {
    Ok(CryptoAgreementDeriveKeyRequest {
        algorithm: request.algorithm,
        digest: request.digest,
        salt: bytes_from_vm(runtime, context, request.salt)?,
        info: bytes_from_vm(runtime, context, request.info)?,
        output_length: request.output_length,
    })
}

fn certificate_verify_request_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoCertificateVerifyRequestVm,
) -> RuntimeResult<CryptoCertificateVerifyRequest> {
    Ok(CryptoCertificateVerifyRequest {
        leaf: request.leaf,
        intermediates: slice_from_vm(runtime, context, request.intermediates)?,
        trust_anchors: slice_from_vm(runtime, context, request.trust_anchors)?,
        use_system_trust_anchors: request.use_system_trust_anchors,
        purpose: request.purpose,
        identity: CryptoCertificateVerifyIdentity {
            kind: request.identity.kind,
            value: string_from_vm(runtime, context, request.identity.value)?,
        },
        verification_unix_seconds: request.verification_unix_seconds,
        revocation_mode: request.revocation_mode,
    })
}

fn cipher_parameters_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    parameters: CryptoCipherParametersVm,
) -> RuntimeResult<CryptoCipherParameters> {
    Ok(CryptoCipherParameters {
        algorithm: parameters.algorithm,
        nonce: bytes_from_vm(runtime, context, parameters.nonce)?,
        additional_data: bytes_from_vm(runtime, context, parameters.additional_data)?,
        tag: bytes_from_vm(runtime, context, parameters.tag)?,
        tag_length_bytes: parameters.tag_length_bytes,
    })
}

fn hkdf_request_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoHkdfRequestVm,
) -> RuntimeResult<CryptoHkdfRequest> {
    Ok(CryptoHkdfRequest {
        digest: request.digest,
        input_key_material: bytes_from_vm(runtime, context, request.input_key_material)?,
        salt: bytes_from_vm(runtime, context, request.salt)?,
        info: bytes_from_vm(runtime, context, request.info)?,
        length: request.length,
    })
}

fn pbkdf2_request_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoPbkdf2RequestVm,
) -> RuntimeResult<CryptoPbkdf2Request> {
    Ok(CryptoPbkdf2Request {
        digest: request.digest,
        password: bytes_from_vm(runtime, context, request.password)?,
        salt: bytes_from_vm(runtime, context, request.salt)?,
        iterations: request.iterations,
        length: request.length,
    })
}

fn scrypt_request_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoScryptRequestVm,
) -> RuntimeResult<CryptoScryptRequest> {
    Ok(CryptoScryptRequest {
        password: bytes_from_vm(runtime, context, request.password)?,
        salt: bytes_from_vm(runtime, context, request.salt)?,
        cost: request.cost,
        block_size: request.block_size,
        parallelization: request.parallelization,
        max_memory_bytes: request.max_memory_bytes,
        length: request.length,
    })
}

fn argon2id_request_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoArgon2idRequestVm,
) -> RuntimeResult<CryptoArgon2idRequest> {
    Ok(CryptoArgon2idRequest {
        password: bytes_from_vm(runtime, context, request.password)?,
        salt: bytes_from_vm(runtime, context, request.salt)?,
        associated_data: bytes_from_vm(runtime, context, request.associated_data)?,
        secret: bytes_from_vm(runtime, context, request.secret)?,
        iterations: request.iterations,
        memory_ki_b: request.memory_ki_b,
        parallelism: request.parallelism,
        length: request.length,
    })
}

fn asymmetric_encryption_parameters_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    parameters: CryptoAsymmetricEncryptionParametersVm,
) -> RuntimeResult<CryptoAsymmetricEncryptionParameters> {
    Ok(CryptoAsymmetricEncryptionParameters {
        algorithm: parameters.algorithm,
        digest: parameters.digest,
        label: bytes_from_vm(runtime, context, parameters.label)?,
    })
}

fn key_generation_request_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoKeyGenerationRequestVm,
) -> RuntimeResult<CryptoKeyGenerationRequest> {
    match request {
        CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestAes(request) => {
            Ok(CryptoKeyGenerationRequest::CryptoKeyGenerationRequestAes(
                CryptoKeyGenerationRequestAes {
                    algorithm: runtime.store_string("aes"),
                    size_bits: request.size_bits,
                    usage_mask: request.usage_mask,
                    label: string_from_vm(runtime, context, request.label)?,
                    extractable: request.extractable,
                    residency: request.residency,
                    hardware_backed: request.hardware_backed,
                    persistent: request.persistent,
                },
            ))
        }
        CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestChaCha20(request) => Ok(
            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestChaCha20(
                CryptoKeyGenerationRequestChaCha20 {
                    algorithm: runtime.store_string("chacha20"),
                    size_bits: request.size_bits,
                    usage_mask: request.usage_mask,
                    label: string_from_vm(runtime, context, request.label)?,
                    extractable: request.extractable,
                    residency: request.residency,
                    hardware_backed: request.hardware_backed,
                    persistent: request.persistent,
                },
            ),
        ),
        CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestEc(request) => {
            Ok(CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEc(
                CryptoKeyGenerationRequestEc {
                    algorithm: runtime.store_string("ec"),
                    named_curve: request.named_curve,
                    usage_mask: request.usage_mask,
                    label: string_from_vm(runtime, context, request.label)?,
                    extractable: request.extractable,
                    residency: request.residency,
                    hardware_backed: request.hardware_backed,
                    persistent: request.persistent,
                },
            ))
        }
        CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestEd25519(request) => Ok(
            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEd25519(
                CryptoKeyGenerationRequestEd25519 {
                    algorithm: runtime.store_string("ed25519"),
                    usage_mask: request.usage_mask,
                    label: string_from_vm(runtime, context, request.label)?,
                    extractable: request.extractable,
                    residency: request.residency,
                    hardware_backed: request.hardware_backed,
                    persistent: request.persistent,
                },
            ),
        ),
        CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestEd448(request) => {
            Ok(CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEd448(
                CryptoKeyGenerationRequestEd448 {
                    algorithm: runtime.store_string("ed448"),
                    usage_mask: request.usage_mask,
                    label: string_from_vm(runtime, context, request.label)?,
                    extractable: request.extractable,
                    residency: request.residency,
                    hardware_backed: request.hardware_backed,
                    persistent: request.persistent,
                },
            ))
        }
        CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestHmac(request) => {
            Ok(CryptoKeyGenerationRequest::CryptoKeyGenerationRequestHmac(
                CryptoKeyGenerationRequestHmac {
                    algorithm: runtime.store_string("hmac"),
                    size_bits: request.size_bits,
                    digest: request.digest,
                    usage_mask: request.usage_mask,
                    label: string_from_vm(runtime, context, request.label)?,
                    extractable: request.extractable,
                    residency: request.residency,
                    hardware_backed: request.hardware_backed,
                    persistent: request.persistent,
                },
            ))
        }
        CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestRsa(request) => {
            Ok(CryptoKeyGenerationRequest::CryptoKeyGenerationRequestRsa(
                CryptoKeyGenerationRequestRsa {
                    algorithm: runtime.store_string("rsa"),
                    modulus_bits: request.modulus_bits,
                    public_exponent: request.public_exponent,
                    digest: request.digest,
                    usage_mask: request.usage_mask,
                    label: string_from_vm(runtime, context, request.label)?,
                    extractable: request.extractable,
                    residency: request.residency,
                    hardware_backed: request.hardware_backed,
                    persistent: request.persistent,
                },
            ))
        }
        CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestX25519(request) => Ok(
            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestX25519(
                CryptoKeyGenerationRequestX25519 {
                    algorithm: runtime.store_string("x25519"),
                    usage_mask: request.usage_mask,
                    label: string_from_vm(runtime, context, request.label)?,
                    extractable: request.extractable,
                    residency: request.residency,
                    hardware_backed: request.hardware_backed,
                    persistent: request.persistent,
                },
            ),
        ),
        CryptoKeyGenerationRequestVm::CryptoKeyGenerationRequestX448(request) => {
            Ok(CryptoKeyGenerationRequest::CryptoKeyGenerationRequestX448(
                CryptoKeyGenerationRequestX448 {
                    algorithm: runtime.store_string("x448"),
                    usage_mask: request.usage_mask,
                    label: string_from_vm(runtime, context, request.label)?,
                    extractable: request.extractable,
                    residency: request.residency,
                    hardware_backed: request.hardware_backed,
                    persistent: request.persistent,
                },
            ))
        }
    }
}

fn key_import_request_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoKeyImportRequestVm,
) -> RuntimeResult<CryptoKeyImportRequest> {
    match request {
        CryptoKeyImportRequestVm::CryptoKeyImportRequestAes(request) => Ok(
            CryptoKeyImportRequest::CryptoKeyImportRequestAes(CryptoKeyImportRequestAes {
                algorithm: runtime.store_string("aes"),
                format: request.format,
                bytes: bytes_from_vm(runtime, context, request.bytes)?,
                usage_mask: request.usage_mask,
                label: string_from_vm(runtime, context, request.label)?,
                extractable: request.extractable,
                residency: request.residency,
                passphrase: bytes_from_vm(runtime, context, request.passphrase)?,
                persistent: request.persistent,
            }),
        ),
        CryptoKeyImportRequestVm::CryptoKeyImportRequestChaCha20(request) => {
            Ok(CryptoKeyImportRequest::CryptoKeyImportRequestChaCha20(
                CryptoKeyImportRequestChaCha20 {
                    algorithm: runtime.store_string("chacha20"),
                    format: request.format,
                    bytes: bytes_from_vm(runtime, context, request.bytes)?,
                    usage_mask: request.usage_mask,
                    label: string_from_vm(runtime, context, request.label)?,
                    extractable: request.extractable,
                    residency: request.residency,
                    passphrase: bytes_from_vm(runtime, context, request.passphrase)?,
                    persistent: request.persistent,
                },
            ))
        }
        CryptoKeyImportRequestVm::CryptoKeyImportRequestEc(request) => Ok(
            CryptoKeyImportRequest::CryptoKeyImportRequestEc(CryptoKeyImportRequestEc {
                algorithm: runtime.store_string("ec"),
                format: request.format,
                bytes: bytes_from_vm(runtime, context, request.bytes)?,
                named_curve: request.named_curve,
                usage_mask: request.usage_mask,
                label: string_from_vm(runtime, context, request.label)?,
                extractable: request.extractable,
                residency: request.residency,
                passphrase: bytes_from_vm(runtime, context, request.passphrase)?,
                persistent: request.persistent,
            }),
        ),
        CryptoKeyImportRequestVm::CryptoKeyImportRequestEd25519(request) => Ok(
            CryptoKeyImportRequest::CryptoKeyImportRequestEd25519(CryptoKeyImportRequestEd25519 {
                algorithm: runtime.store_string("ed25519"),
                format: request.format,
                bytes: bytes_from_vm(runtime, context, request.bytes)?,
                usage_mask: request.usage_mask,
                label: string_from_vm(runtime, context, request.label)?,
                extractable: request.extractable,
                residency: request.residency,
                passphrase: bytes_from_vm(runtime, context, request.passphrase)?,
                persistent: request.persistent,
            }),
        ),
        CryptoKeyImportRequestVm::CryptoKeyImportRequestEd448(request) => Ok(
            CryptoKeyImportRequest::CryptoKeyImportRequestEd448(CryptoKeyImportRequestEd448 {
                algorithm: runtime.store_string("ed448"),
                format: request.format,
                bytes: bytes_from_vm(runtime, context, request.bytes)?,
                usage_mask: request.usage_mask,
                label: string_from_vm(runtime, context, request.label)?,
                extractable: request.extractable,
                residency: request.residency,
                passphrase: bytes_from_vm(runtime, context, request.passphrase)?,
                persistent: request.persistent,
            }),
        ),
        CryptoKeyImportRequestVm::CryptoKeyImportRequestHmac(request) => Ok(
            CryptoKeyImportRequest::CryptoKeyImportRequestHmac(CryptoKeyImportRequestHmac {
                algorithm: runtime.store_string("hmac"),
                format: request.format,
                bytes: bytes_from_vm(runtime, context, request.bytes)?,
                digest: request.digest,
                usage_mask: request.usage_mask,
                label: string_from_vm(runtime, context, request.label)?,
                extractable: request.extractable,
                residency: request.residency,
                passphrase: bytes_from_vm(runtime, context, request.passphrase)?,
                persistent: request.persistent,
            }),
        ),
        CryptoKeyImportRequestVm::CryptoKeyImportRequestRsa(request) => Ok(
            CryptoKeyImportRequest::CryptoKeyImportRequestRsa(CryptoKeyImportRequestRsa {
                algorithm: runtime.store_string("rsa"),
                format: request.format,
                bytes: bytes_from_vm(runtime, context, request.bytes)?,
                digest: request.digest,
                usage_mask: request.usage_mask,
                label: string_from_vm(runtime, context, request.label)?,
                extractable: request.extractable,
                residency: request.residency,
                passphrase: bytes_from_vm(runtime, context, request.passphrase)?,
                persistent: request.persistent,
            }),
        ),
        CryptoKeyImportRequestVm::CryptoKeyImportRequestX25519(request) => Ok(
            CryptoKeyImportRequest::CryptoKeyImportRequestX25519(CryptoKeyImportRequestX25519 {
                algorithm: runtime.store_string("x25519"),
                format: request.format,
                bytes: bytes_from_vm(runtime, context, request.bytes)?,
                usage_mask: request.usage_mask,
                label: string_from_vm(runtime, context, request.label)?,
                extractable: request.extractable,
                residency: request.residency,
                passphrase: bytes_from_vm(runtime, context, request.passphrase)?,
                persistent: request.persistent,
            }),
        ),
        CryptoKeyImportRequestVm::CryptoKeyImportRequestX448(request) => Ok(
            CryptoKeyImportRequest::CryptoKeyImportRequestX448(CryptoKeyImportRequestX448 {
                algorithm: runtime.store_string("x448"),
                format: request.format,
                bytes: bytes_from_vm(runtime, context, request.bytes)?,
                usage_mask: request.usage_mask,
                label: string_from_vm(runtime, context, request.label)?,
                extractable: request.extractable,
                residency: request.residency,
                passphrase: bytes_from_vm(runtime, context, request.passphrase)?,
                persistent: request.persistent,
            }),
        ),
    }
}

fn private_key_export_request_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: CryptoPrivateKeyExportRequestVm,
) -> RuntimeResult<CryptoPrivateKeyExportRequest> {
    Ok(CryptoPrivateKeyExportRequest {
        format: request.format,
        passphrase: bytes_from_vm(runtime, context, request.passphrase)?,
    })
}

fn key_wrap_parameters_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    parameters: CryptoKeyWrapParametersVm,
) -> RuntimeResult<CryptoKeyWrapParameters> {
    Ok(CryptoKeyWrapParameters {
        algorithm: parameters.algorithm,
        digest: parameters.digest,
        label: bytes_from_vm(runtime, context, parameters.label)?,
    })
}

fn key_query_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    query: CryptoKeyQueryVm,
) -> RuntimeResult<CryptoKeyQuery> {
    Ok(CryptoKeyQuery {
        label_prefix: string_from_vm(runtime, context, query.label_prefix)?,
        algorithm: query.algorithm,
        usage_mask: query.usage_mask,
        cursor: string_from_vm(runtime, context, query.cursor)?,
        limit: query.limit,
    })
}

fn certificate_query_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    query: CryptoCertificateQueryVm,
) -> RuntimeResult<CryptoCertificateQuery> {
    Ok(CryptoCertificateQuery {
        subject_contains: string_from_vm(runtime, context, query.subject_contains)?,
        issuer_contains: string_from_vm(runtime, context, query.issuer_contains)?,
        subject_alternative_name: string_from_vm(runtime, context, query.subject_alternative_name)?,
        cursor: string_from_vm(runtime, context, query.cursor)?,
        limit: query.limit,
    })
}

fn store_options_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: CryptoStoreOptionsVm,
) -> RuntimeResult<CryptoStoreOptions> {
    Ok(CryptoStoreOptions {
        kind: options.kind,
        provider: options.provider,
        namespace: string_from_vm(runtime, context, options.namespace)?,
    })
}

fn key_pair_to_vm(value: CryptoKeyPair) -> CryptoKeyPairVm {
    CryptoKeyPairVm {
        public_key: value.public_key,
        private_key: value.private_key,
    }
}

fn cipher_output_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: CryptoCipherOutput,
) -> RuntimeResult<CryptoCipherOutputVm> {
    Ok(CryptoCipherOutputVm {
        bytes: bytes_to_vm(context, value.bytes)?,
        tag: bytes_to_vm(context, value.tag)?,
    })
}

fn key_descriptor_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: CryptoKeyDescriptor,
) -> RuntimeResult<CryptoKeyDescriptorVm> {
    match value {
        CryptoKeyDescriptor::CryptoKeyDescriptorAes(value) => Ok(
            CryptoKeyDescriptorVm::CryptoKeyDescriptorAes(CryptoKeyDescriptorAesVm {
                algorithm: string_to_vm(context, value.algorithm)?,
                key_kind: value.key_kind,
                size_bits: value.size_bits,
                usage_mask: value.usage_mask,
                label: string_to_vm(context, value.label)?,
                extractable: value.extractable,
                residency: value.residency,
                hardware_backed: value.hardware_backed,
                persistent: value.persistent,
                store_provenance: store_provenance_to_vm(context, value.store_provenance)?,
            }),
        ),
        CryptoKeyDescriptor::CryptoKeyDescriptorChaCha20(value) => Ok(
            CryptoKeyDescriptorVm::CryptoKeyDescriptorChaCha20(CryptoKeyDescriptorChaCha20Vm {
                algorithm: string_to_vm(context, value.algorithm)?,
                key_kind: value.key_kind,
                size_bits: value.size_bits,
                usage_mask: value.usage_mask,
                label: string_to_vm(context, value.label)?,
                extractable: value.extractable,
                residency: value.residency,
                hardware_backed: value.hardware_backed,
                persistent: value.persistent,
                store_provenance: store_provenance_to_vm(context, value.store_provenance)?,
            }),
        ),
        CryptoKeyDescriptor::CryptoKeyDescriptorEc(value) => Ok(
            CryptoKeyDescriptorVm::CryptoKeyDescriptorEc(CryptoKeyDescriptorEcVm {
                algorithm: string_to_vm(context, value.algorithm)?,
                key_kind: value.key_kind,
                named_curve: value.named_curve,
                usage_mask: value.usage_mask,
                label: string_to_vm(context, value.label)?,
                extractable: value.extractable,
                residency: value.residency,
                hardware_backed: value.hardware_backed,
                persistent: value.persistent,
                store_provenance: store_provenance_to_vm(context, value.store_provenance)?,
            }),
        ),
        CryptoKeyDescriptor::CryptoKeyDescriptorEd25519(value) => Ok(
            CryptoKeyDescriptorVm::CryptoKeyDescriptorEd25519(CryptoKeyDescriptorEd25519Vm {
                algorithm: string_to_vm(context, value.algorithm)?,
                key_kind: value.key_kind,
                usage_mask: value.usage_mask,
                label: string_to_vm(context, value.label)?,
                extractable: value.extractable,
                residency: value.residency,
                hardware_backed: value.hardware_backed,
                persistent: value.persistent,
                store_provenance: store_provenance_to_vm(context, value.store_provenance)?,
            }),
        ),
        CryptoKeyDescriptor::CryptoKeyDescriptorEd448(value) => Ok(
            CryptoKeyDescriptorVm::CryptoKeyDescriptorEd448(CryptoKeyDescriptorEd448Vm {
                algorithm: string_to_vm(context, value.algorithm)?,
                key_kind: value.key_kind,
                usage_mask: value.usage_mask,
                label: string_to_vm(context, value.label)?,
                extractable: value.extractable,
                residency: value.residency,
                hardware_backed: value.hardware_backed,
                persistent: value.persistent,
                store_provenance: store_provenance_to_vm(context, value.store_provenance)?,
            }),
        ),
        CryptoKeyDescriptor::CryptoKeyDescriptorHmac(value) => Ok(
            CryptoKeyDescriptorVm::CryptoKeyDescriptorHmac(CryptoKeyDescriptorHmacVm {
                algorithm: string_to_vm(context, value.algorithm)?,
                key_kind: value.key_kind,
                size_bits: value.size_bits,
                digest: value.digest,
                usage_mask: value.usage_mask,
                label: string_to_vm(context, value.label)?,
                extractable: value.extractable,
                residency: value.residency,
                hardware_backed: value.hardware_backed,
                persistent: value.persistent,
                store_provenance: store_provenance_to_vm(context, value.store_provenance)?,
            }),
        ),
        CryptoKeyDescriptor::CryptoKeyDescriptorRsa(value) => Ok(
            CryptoKeyDescriptorVm::CryptoKeyDescriptorRsa(CryptoKeyDescriptorRsaVm {
                algorithm: string_to_vm(context, value.algorithm)?,
                key_kind: value.key_kind,
                modulus_bits: value.modulus_bits,
                public_exponent: value.public_exponent,
                digest: value.digest,
                usage_mask: value.usage_mask,
                label: string_to_vm(context, value.label)?,
                extractable: value.extractable,
                residency: value.residency,
                hardware_backed: value.hardware_backed,
                persistent: value.persistent,
                store_provenance: store_provenance_to_vm(context, value.store_provenance)?,
            }),
        ),
        CryptoKeyDescriptor::CryptoKeyDescriptorX25519(value) => Ok(
            CryptoKeyDescriptorVm::CryptoKeyDescriptorX25519(CryptoKeyDescriptorX25519Vm {
                algorithm: string_to_vm(context, value.algorithm)?,
                key_kind: value.key_kind,
                usage_mask: value.usage_mask,
                label: string_to_vm(context, value.label)?,
                extractable: value.extractable,
                residency: value.residency,
                hardware_backed: value.hardware_backed,
                persistent: value.persistent,
                store_provenance: store_provenance_to_vm(context, value.store_provenance)?,
            }),
        ),
        CryptoKeyDescriptor::CryptoKeyDescriptorX448(value) => Ok(
            CryptoKeyDescriptorVm::CryptoKeyDescriptorX448(CryptoKeyDescriptorX448Vm {
                algorithm: string_to_vm(context, value.algorithm)?,
                key_kind: value.key_kind,
                usage_mask: value.usage_mask,
                label: string_to_vm(context, value.label)?,
                extractable: value.extractable,
                residency: value.residency,
                hardware_backed: value.hardware_backed,
                persistent: value.persistent,
                store_provenance: store_provenance_to_vm(context, value.store_provenance)?,
            }),
        ),
    }
}

fn certificate_descriptor_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: CryptoCertificateDescriptor,
) -> RuntimeResult<CryptoCertificateDescriptorVm> {
    let names_native = unsafe { value.subject_alternative_names.as_slice()? };
    let mut names_vm = Vec::with_capacity(names_native.len());
    for value in names_native {
        names_vm.push(string_to_vm(context, *value)?);
    }

    let validity = CryptoCertificateValidityVm {
        not_before_unix_seconds: value.validity.not_before_unix_seconds,
        not_after_unix_seconds: value.validity.not_after_unix_seconds,
    };

    Ok(CryptoCertificateDescriptorVm {
        subject: string_to_vm(context, value.subject)?,
        issuer: string_to_vm(context, value.issuer)?,
        serial_number: string_to_vm(context, value.serial_number)?,
        subject_alternative_names: VmArray::from_values(context, &names_vm)?,
        fingerprint_sha256: bytes_to_vm(context, value.fingerprint_sha256)?,
        validity,
        is_certificate_authority: value.is_certificate_authority,
        key_usage_mask: value.key_usage_mask,
        store_provenance: store_provenance_to_vm(context, value.store_provenance)?,
    })
}

fn store_provenance_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: CryptoStoreProvenance,
) -> RuntimeResult<CryptoStoreProvenanceVm> {
    Ok(CryptoStoreProvenanceVm {
        identity: CryptoStoreIdentityVm {
            kind: value.identity.kind,
            provider: value.identity.provider,
            namespace: string_to_vm(context, value.identity.namespace)?,
        },
    })
}

fn store_capability_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: CryptoStoreCapability,
) -> RuntimeResult<CryptoStoreCapabilityVm> {
    let key_capabilities_native = unsafe { value.key_capabilities.as_slice()? };
    let mut key_capabilities = Vec::with_capacity(key_capabilities_native.len());
    for row in key_capabilities_native {
        key_capabilities.push(CryptoStoreKeyCapabilityVm {
            algorithm: row.algorithm,
            residency: row.residency,
            supports_generate_secret: row.supports_generate_secret,
            supports_generate_pair: row.supports_generate_pair,
            supports_import: row.supports_import,
            supports_export_public: row.supports_export_public,
            supports_export_private: row.supports_export_private,
            supports_export_secret: row.supports_export_secret,
            supported_usage_mask: row.supported_usage_mask,
            supported_import_formats: slice_to_vm(context, row.supported_import_formats)?,
            supported_export_formats: slice_to_vm(context, row.supported_export_formats)?,
        });
    }

    let signature_capabilities_native = unsafe { value.signature_capabilities.as_slice()? };
    let mut signature_capabilities = Vec::with_capacity(signature_capabilities_native.len());
    for row in signature_capabilities_native {
        signature_capabilities.push(CryptoStoreSignatureCapabilityVm {
            key_algorithm: row.key_algorithm,
            signature_algorithm: row.signature_algorithm,
            supports_sign: row.supports_sign,
            supports_verify: row.supports_verify,
            supported_digests: slice_to_vm(context, row.supported_digests)?,
        });
    }

    let asymmetric_capabilities_native =
        unsafe { value.asymmetric_encryption_capabilities.as_slice()? };
    let mut asymmetric_capabilities = Vec::with_capacity(asymmetric_capabilities_native.len());
    for row in asymmetric_capabilities_native {
        asymmetric_capabilities.push(CryptoStoreAsymmetricEncryptionCapabilityVm {
            key_algorithm: row.key_algorithm,
            algorithm: row.algorithm,
            supports_encrypt: row.supports_encrypt,
            supports_decrypt: row.supports_decrypt,
            supported_digests: slice_to_vm(context, row.supported_digests)?,
        });
    }

    let key_wrap_capabilities_native = unsafe { value.key_wrap_capabilities.as_slice()? };
    let mut key_wrap_capabilities = Vec::with_capacity(key_wrap_capabilities_native.len());
    for row in key_wrap_capabilities_native {
        key_wrap_capabilities.push(CryptoStoreKeyWrapCapabilityVm {
            wrapping_key_algorithm: row.wrapping_key_algorithm,
            algorithm: row.algorithm,
            supports_wrap: row.supports_wrap,
            supports_unwrap: row.supports_unwrap,
            supported_digests: slice_to_vm(context, row.supported_digests)?,
        });
    }

    let cipher_capabilities_native = unsafe { value.cipher_capabilities.as_slice()? };
    let mut cipher_capabilities = Vec::with_capacity(cipher_capabilities_native.len());
    for row in cipher_capabilities_native {
        cipher_capabilities.push(CryptoStoreCipherCapabilityVm {
            key_algorithm: row.key_algorithm,
            algorithm: row.algorithm,
            supports_one_shot: row.supports_one_shot,
            supports_streaming: row.supports_streaming,
            supports_additional_data: row.supports_additional_data,
            supports_detached_tag: row.supports_detached_tag,
            min_tag_length_bytes: row.min_tag_length_bytes,
            max_tag_length_bytes: row.max_tag_length_bytes,
        });
    }

    let mac_capabilities_native = unsafe { value.mac_capabilities.as_slice()? };
    let mut mac_capabilities = Vec::with_capacity(mac_capabilities_native.len());
    for row in mac_capabilities_native {
        mac_capabilities.push(CryptoStoreMacCapabilityVm {
            key_algorithm: row.key_algorithm,
            algorithm: row.algorithm,
            supports_one_shot: row.supports_one_shot,
            supports_streaming: row.supports_streaming,
            supported_digests: slice_to_vm(context, row.supported_digests)?,
            min_tag_length_bytes: row.min_tag_length_bytes,
            max_tag_length_bytes: row.max_tag_length_bytes,
        });
    }

    let agreement_capabilities_native = unsafe { value.agreement_capabilities.as_slice()? };
    let agreement_capabilities = agreement_capabilities_native.to_vec();

    Ok(CryptoStoreCapabilityVm {
        identity: CryptoStoreIdentityVm {
            kind: value.identity.kind,
            provider: value.identity.provider,
            namespace: string_to_vm(context, value.identity.namespace)?,
        },
        is_available: value.is_available,
        supports_hardware_backed: value.supports_hardware_backed,
        supports_persistent: value.supports_persistent,
        supports_key_export: value.supports_key_export,
        supported_key_algorithms: array_to_vm(context, value.supported_key_algorithms)?,
        supported_key_formats: array_to_vm(context, value.supported_key_formats)?,
        supported_key_residencies: array_to_vm(context, value.supported_key_residencies)?,
        key_capabilities: VmArray::from_values(context, &key_capabilities)?,
        signature_capabilities: VmArray::from_values(context, &signature_capabilities)?,
        asymmetric_encryption_capabilities: VmArray::from_values(
            context,
            &asymmetric_capabilities,
        )?,
        key_wrap_capabilities: VmArray::from_values(context, &key_wrap_capabilities)?,
        cipher_capabilities: VmArray::from_values(context, &cipher_capabilities)?,
        mac_capabilities: VmArray::from_values(context, &mac_capabilities)?,
        agreement_capabilities: VmArray::from_values(context, &agreement_capabilities)?,
        certificate_capabilities: CryptoStoreCertificateCapabilityVm {
            supports_import: value.certificate_capabilities.supports_import,
            supports_export: value.certificate_capabilities.supports_export,
            supports_descriptor: value.certificate_capabilities.supports_descriptor,
            supports_verify: value.certificate_capabilities.supports_verify,
            supports_delete: value.certificate_capabilities.supports_delete,
            supports_system_trust_anchors: value
                .certificate_capabilities
                .supports_system_trust_anchors,
        },
    })
}

fn certificate_verify_result_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: CryptoCertificateVerifyResult,
) -> RuntimeResult<CryptoCertificateVerifyResultVm> {
    Ok(CryptoCertificateVerifyResultVm {
        valid: value.valid,
        error: value.error,
        error_code: value.error_code,
        failed_certificate_index: value.failed_certificate_index,
        failed_certificate_subject: string_to_vm(context, value.failed_certificate_subject)?,
        chain_length: value.chain_length,
        used_system_trust_anchor: value.used_system_trust_anchor,
    })
}

fn key_list_page_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: CryptoKeyListPage,
) -> RuntimeResult<CryptoKeyListPageVm> {
    let entries_native = unsafe { value.entries.as_slice()? };
    let mut entries_vm = Vec::with_capacity(entries_native.len());
    for entry in entries_native {
        entries_vm.push(CryptoKeyListEntryVm {
            handle: entry.handle,
            label: string_to_vm(context, entry.label)?,
            algorithm: entry.algorithm,
            usage_mask: entry.usage_mask,
        });
    }

    Ok(CryptoKeyListPageVm {
        entries: VmArray::from_values(context, &entries_vm)?,
        next_cursor: string_to_vm(context, value.next_cursor)?,
    })
}

fn certificate_list_page_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: CryptoCertificateListPage,
) -> RuntimeResult<CryptoCertificateListPageVm> {
    let entries_native = unsafe { value.entries.as_slice()? };
    let mut entries_vm = Vec::with_capacity(entries_native.len());
    for entry in entries_native {
        entries_vm.push(CryptoCertificateListEntryVm {
            handle: entry.handle,
            subject: string_to_vm(context, entry.subject)?,
            issuer: string_to_vm(context, entry.issuer)?,
            serial_number: string_to_vm(context, entry.serial_number)?,
        });
    }

    Ok(CryptoCertificateListPageVm {
        entries: VmArray::from_values(context, &entries_vm)?,
        next_cursor: string_to_vm(context, value.next_cursor)?,
    })
}
