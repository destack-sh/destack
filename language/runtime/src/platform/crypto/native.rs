use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequest, CryptoArgon2idRequest, CryptoAsymmetricEncryptionParameters,
    CryptoCertificateDescriptor, CryptoCertificateFormat, CryptoCertificateListPage,
    CryptoCertificateQuery, CryptoCertificateVerifyRequest, CryptoCertificateVerifyResult,
    CryptoCipherAlgorithm, CryptoCipherDirection, CryptoCipherOutput, CryptoCipherParameters,
    CryptoDigestAlgorithm, CryptoHkdfRequest, CryptoKdfAlgorithm, CryptoKeyAgreementAlgorithm,
    CryptoKeyAlgorithm, CryptoKeyDescriptor, CryptoKeyFormat, CryptoKeyGenerationRequest,
    CryptoKeyImportRequest, CryptoKeyListPage, CryptoKeyPair, CryptoKeyQuery, CryptoKeyResidency,
    CryptoKeyWrapAlgorithm, CryptoKeyWrapParameters, CryptoMacAlgorithm, CryptoMacParameters,
    CryptoNamedCurve, CryptoPbkdf2Request, CryptoPrivateKeyExportRequest, CryptoScryptRequest,
    CryptoSignatureAlgorithm, CryptoSignatureParameters, CryptoStoreCapability, CryptoStoreKind,
    CryptoStoreOptions, CryptoStoreProvider, host as host_crypto,
};
use crate::platform::{NativeArray, resource};
use crate::runtime::BindingCallContext;

/// Derive one symmetric key from one local private key and one peer public key.
pub(crate) unsafe fn destack_crypto_agreement_derive_key(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    request: CryptoAgreementDeriveKeyRequest,
) -> RuntimeResult<()> {
    unsafe {
        host_crypto::destack_crypto_agreement_derive_key(
            binding,
            out,
            privatekey,
            peerpublickey,
            request,
        )
    }
}

/// Derive one shared secret from one local private key and one peer public key.
pub(crate) unsafe fn destack_crypto_agreement_derive_shared_secret(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    algorithm: CryptoKeyAgreementAlgorithm,
) -> RuntimeResult<()> {
    unsafe {
        host_crypto::destack_crypto_agreement_derive_shared_secret(
            binding,
            out,
            privatekey,
            peerpublickey,
            algorithm,
        )
    }
}

/// Delete one certificate from one store when allowed.
pub(crate) unsafe fn destack_crypto_certificate_delete(
    binding: &BindingCallContext,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_certificate_delete(binding, handle) }
}

/// Return one certificate descriptor.
pub(crate) unsafe fn destack_crypto_certificate_descriptor(
    binding: &BindingCallContext,
    out: *mut CryptoCertificateDescriptor,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_certificate_descriptor(binding, out, handle) }
}

/// Export one certificate from one handle.
pub(crate) unsafe fn destack_crypto_certificate_export(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoCertificateHandle,
    format: CryptoCertificateFormat,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_certificate_export(binding, out, handle, format) }
}

/// Import one certificate into one store.
pub(crate) unsafe fn destack_crypto_certificate_import(
    binding: &BindingCallContext,
    out: *mut resource::CryptoCertificateHandle,
    store: resource::CryptoStoreHandle,
    format: CryptoCertificateFormat,
    certificate: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe {
        host_crypto::destack_crypto_certificate_import(binding, out, store, format, certificate)
    }
}

/// Verify one certificate chain against one trust policy.
pub(crate) unsafe fn destack_crypto_certificate_verify(
    binding: &BindingCallContext,
    out: *mut CryptoCertificateVerifyResult,
    request: CryptoCertificateVerifyRequest,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_certificate_verify(binding, out, request) }
}

/// Close one streaming cipher context.
pub(crate) unsafe fn destack_crypto_cipher_close(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_cipher_close(binding, handle) }
}

/// Decrypt one payload in one shot.
pub(crate) unsafe fn destack_crypto_cipher_decrypt(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe {
        host_crypto::destack_crypto_cipher_decrypt(binding, out, key, parameters, argument_payload)
    }
}

/// Encrypt one payload in one shot.
pub(crate) unsafe fn destack_crypto_cipher_encrypt(
    binding: &BindingCallContext,
    out: *mut CryptoCipherOutput,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe {
        host_crypto::destack_crypto_cipher_encrypt(binding, out, key, parameters, argument_payload)
    }
}

/// Finalize one streaming cipher context.
pub(crate) unsafe fn destack_crypto_cipher_finish(
    binding: &BindingCallContext,
    out: *mut CryptoCipherOutput,
    handle: resource::CryptoCipherHandle,
    finalpayload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_cipher_finish(binding, out, handle, finalpayload) }
}

/// Open one streaming cipher context.
pub(crate) unsafe fn destack_crypto_cipher_open(
    binding: &BindingCallContext,
    out: *mut resource::CryptoCipherHandle,
    key: resource::CryptoKeyHandle,
    direction: CryptoCipherDirection,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_cipher_open(binding, out, key, direction, parameters) }
}

/// Reset one streaming cipher context with new parameters.
pub(crate) unsafe fn destack_crypto_cipher_reset(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_cipher_reset(binding, handle, parameters) }
}

/// Update one streaming cipher context with one payload chunk.
pub(crate) unsafe fn destack_crypto_cipher_update(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoCipherHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_cipher_update(binding, out, handle, argument_payload) }
}

/// Update additional authenticated data for one streaming cipher context.
pub(crate) unsafe fn destack_crypto_cipher_update_additional_data(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    additionaldata: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe {
        host_crypto::destack_crypto_cipher_update_additional_data(binding, handle, additionaldata)
    }
}

/// Close one streaming digest context.
pub(crate) unsafe fn destack_crypto_digest_close(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_digest_close(binding, handle) }
}

/// Compute one digest in one shot.
pub(crate) unsafe fn destack_crypto_digest_compute(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    algorithm: CryptoDigestAlgorithm,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_digest_compute(binding, out, algorithm, argument_payload) }
}

/// Finalize one streaming digest context and return one digest output.
pub(crate) unsafe fn destack_crypto_digest_finish(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_digest_finish(binding, out, handle) }
}

/// Open one streaming digest context.
pub(crate) unsafe fn destack_crypto_digest_open(
    binding: &BindingCallContext,
    out: *mut resource::CryptoDigestHandle,
    algorithm: CryptoDigestAlgorithm,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_digest_open(binding, out, algorithm) }
}

/// Reset one streaming digest context to its initial state.
pub(crate) unsafe fn destack_crypto_digest_reset(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_digest_reset(binding, handle) }
}

/// Update one streaming digest context.
pub(crate) unsafe fn destack_crypto_digest_update(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_digest_update(binding, handle, argument_payload) }
}

/// Derive one key with Argon2id.
pub(crate) unsafe fn destack_crypto_kdf_argon2id(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoArgon2idRequest,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_kdf_argon2id(binding, out, request) }
}

/// Derive one key with HKDF.
pub(crate) unsafe fn destack_crypto_kdf_hkdf(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoHkdfRequest,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_kdf_hkdf(binding, out, request) }
}

/// Derive one key with PBKDF2.
pub(crate) unsafe fn destack_crypto_kdf_pbkdf2(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoPbkdf2Request,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_kdf_pbkdf2(binding, out, request) }
}

/// Derive one key with scrypt.
pub(crate) unsafe fn destack_crypto_kdf_scrypt(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoScryptRequest,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_kdf_scrypt(binding, out, request) }
}

/// Decrypt one payload with one asymmetric key.
pub(crate) unsafe fn destack_crypto_key_decrypt(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe {
        host_crypto::destack_crypto_key_decrypt(binding, out, handle, parameters, argument_payload)
    }
}

/// Delete one key object.
pub(crate) unsafe fn destack_crypto_key_delete(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_key_delete(binding, handle) }
}

/// Return one key descriptor.
pub(crate) unsafe fn destack_crypto_key_descriptor(
    binding: &BindingCallContext,
    out: *mut CryptoKeyDescriptor,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_key_descriptor(binding, out, handle) }
}

/// Encrypt one payload with one asymmetric key.
pub(crate) unsafe fn destack_crypto_key_encrypt(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe {
        host_crypto::destack_crypto_key_encrypt(binding, out, handle, parameters, argument_payload)
    }
}

/// Export one private key.
pub(crate) unsafe fn destack_crypto_key_export_private(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    request: CryptoPrivateKeyExportRequest,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_key_export_private(binding, out, handle, request) }
}

/// Export one public key.
pub(crate) unsafe fn destack_crypto_key_export_public(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_key_export_public(binding, out, handle, format) }
}

/// Export one secret key.
pub(crate) unsafe fn destack_crypto_key_export_secret(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_key_export_secret(binding, out, handle, format) }
}

/// Generate one asymmetric key pair.
pub(crate) unsafe fn destack_crypto_key_generate_pair(
    binding: &BindingCallContext,
    out: *mut CryptoKeyPair,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_key_generate_pair(binding, out, store, request) }
}

/// Generate one symmetric key.
pub(crate) unsafe fn destack_crypto_key_generate_secret(
    binding: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_key_generate_secret(binding, out, store, request) }
}

/// Import one key object.
pub(crate) unsafe fn destack_crypto_key_import(
    binding: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_key_import(binding, out, store, request) }
}

/// Sign one payload.
pub(crate) unsafe fn destack_crypto_key_sign(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe {
        host_crypto::destack_crypto_key_sign(binding, out, handle, parameters, argument_payload)
    }
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
    unsafe {
        host_crypto::destack_crypto_key_unwrap(
            binding,
            out,
            store,
            wrappingkey,
            wrappedkey,
            parameters,
            request,
        )
    }
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
    unsafe {
        host_crypto::destack_crypto_key_verify(
            binding,
            out,
            handle,
            parameters,
            argument_payload,
            signature,
        )
    }
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
    unsafe {
        host_crypto::destack_crypto_key_wrap(
            binding,
            out,
            wrappingkey,
            keytowrap,
            format,
            parameters,
        )
    }
}

/// Close one streaming MAC context.
pub(crate) unsafe fn destack_crypto_mac_close(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_mac_close(binding, handle) }
}

/// Compute one message authentication code in one shot.
pub(crate) unsafe fn destack_crypto_mac_compute(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe {
        host_crypto::destack_crypto_mac_compute(binding, out, key, parameters, argument_payload)
    }
}

/// Finalize one streaming MAC context and return one tag.
pub(crate) unsafe fn destack_crypto_mac_finish(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_mac_finish(binding, out, handle) }
}

/// Open one streaming MAC context.
pub(crate) unsafe fn destack_crypto_mac_open(
    binding: &BindingCallContext,
    out: *mut resource::CryptoMacHandle,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_mac_open(binding, out, key, parameters) }
}

/// Reset one streaming MAC context to its initial state.
pub(crate) unsafe fn destack_crypto_mac_reset(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_mac_reset(binding, handle) }
}

/// Update one streaming MAC context.
pub(crate) unsafe fn destack_crypto_mac_update(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_mac_update(binding, handle, argument_payload) }
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
    unsafe {
        host_crypto::destack_crypto_mac_verify(binding, out, key, parameters, argument_payload, tag)
    }
}

/// List supported key-agreement algorithms.
pub(crate) unsafe fn destack_crypto_probe_agreement_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyAgreementAlgorithm>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_probe_agreement_algorithms(binding, out) }
}

/// List supported cipher algorithms.
pub(crate) unsafe fn destack_crypto_probe_cipher_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoCipherAlgorithm>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_probe_cipher_algorithms(binding, out) }
}

/// List supported digest algorithms.
pub(crate) unsafe fn destack_crypto_probe_digest_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoDigestAlgorithm>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_probe_digest_algorithms(binding, out) }
}

/// List supported KDF algorithms.
pub(crate) unsafe fn destack_crypto_probe_kdf_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKdfAlgorithm>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_probe_kdf_algorithms(binding, out) }
}

/// List supported key algorithm families.
pub(crate) unsafe fn destack_crypto_probe_key_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyAlgorithm>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_probe_key_algorithms(binding, out) }
}

/// List supported key-wrap algorithms.
pub(crate) unsafe fn destack_crypto_probe_key_wrap_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyWrapAlgorithm>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_probe_key_wrap_algorithms(binding, out) }
}

/// List supported key formats.
pub(crate) unsafe fn destack_crypto_probe_key_formats(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyFormat>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_probe_key_formats(binding, out) }
}

/// List supported key residencies.
pub(crate) unsafe fn destack_crypto_probe_key_residencies(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyResidency>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_probe_key_residencies(binding, out) }
}

/// List supported MAC algorithms.
pub(crate) unsafe fn destack_crypto_probe_mac_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoMacAlgorithm>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_probe_mac_algorithms(binding, out) }
}

/// List supported named curves.
pub(crate) unsafe fn destack_crypto_probe_named_curves(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoNamedCurve>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_probe_named_curves(binding, out) }
}

/// List supported signature algorithms.
pub(crate) unsafe fn destack_crypto_probe_signature_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoSignatureAlgorithm>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_probe_signature_algorithms(binding, out) }
}

/// Allocate one random byte vector with the requested length.
pub(crate) unsafe fn destack_crypto_random_bytes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    length: u32,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_random_bytes(binding, out, length) }
}

/// Fill one mutable byte slice with cryptographically secure random bytes.
pub(crate) unsafe fn destack_crypto_random_fill(
    binding: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_random_fill(binding, buffer) }
}

/// Close one crypto store.
pub(crate) unsafe fn destack_crypto_store_close(
    binding: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_store_close(binding, handle) }
}

/// List certificates from one store.
pub(crate) unsafe fn destack_crypto_store_list_certificates(
    binding: &BindingCallContext,
    out: *mut CryptoCertificateListPage,
    handle: resource::CryptoStoreHandle,
    query: CryptoCertificateQuery,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_store_list_certificates(binding, out, handle, query) }
}

/// List keys from one store.
pub(crate) unsafe fn destack_crypto_store_list_keys(
    binding: &BindingCallContext,
    out: *mut CryptoKeyListPage,
    handle: resource::CryptoStoreHandle,
    query: CryptoKeyQuery,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_store_list_keys(binding, out, handle, query) }
}

/// Return capabilities for one store backend identity.
pub(crate) unsafe fn destack_crypto_store_probe_capability(
    binding: &BindingCallContext,
    out: *mut CryptoStoreCapability,
    kind: CryptoStoreKind,
    provider: Option<CryptoStoreProvider>,
) -> RuntimeResult<()> {
    // normalize one absent provider to the default software provider
    let provider = provider.unwrap_or(CryptoStoreProvider::OpenSsl);

    unsafe {
        host_crypto::destack_crypto_store_probe_capability(binding, out, kind, Some(provider))
    }
}

/// List store backend kinds that are currently available.
pub(crate) unsafe fn destack_crypto_store_probe_kinds(
    binding: &BindingCallContext,
    out: *mut NativeArray<CryptoStoreKind>,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_store_probe_kinds(binding, out) }
}

/// Open one crypto store.
pub(crate) unsafe fn destack_crypto_store_open(
    binding: &BindingCallContext,
    out: *mut resource::CryptoStoreHandle,
    options: CryptoStoreOptions,
) -> RuntimeResult<()> {
    unsafe { host_crypto::destack_crypto_store_open(binding, out, options) }
}
