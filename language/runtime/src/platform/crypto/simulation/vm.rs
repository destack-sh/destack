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
pub(crate) fn destack_crypto_cipher_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoCipherHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.cipher.close")).boxed())
}

/// Decrypt one payload in one shot.
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
pub(crate) fn destack_crypto_digest_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.close")).boxed())
}

/// Compute one digest in one shot.
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
pub(crate) fn destack_crypto_digest_finish(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.finish")).boxed())
}

/// Open one streaming digest context.
pub(crate) fn destack_crypto_digest_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    algorithm: CryptoDigestAlgorithm,
) -> RuntimeResult<resource::CryptoDigestHandle> {
    let _ = algorithm;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.open")).boxed())
}

/// Reset one streaming digest context to its initial state.
pub(crate) fn destack_crypto_digest_reset(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.digest.reset")).boxed())
}

/// Update one streaming digest context.
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
pub(crate) fn destack_crypto_kdf_argon2id(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    request: CryptoArgon2idRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = request;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.argon2id")).boxed())
}

/// Derive one key with HKDF.
pub(crate) fn destack_crypto_kdf_hkdf(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    request: CryptoHkdfRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = request;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.hkdf")).boxed())
}

/// Derive one key with PBKDF2.
pub(crate) fn destack_crypto_kdf_pbkdf2(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    request: CryptoPbkdf2RequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = request;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.pbkdf2")).boxed())
}

/// Derive one key with scrypt.
pub(crate) fn destack_crypto_kdf_scrypt(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    request: CryptoScryptRequestVm,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = request;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.kdf.scrypt")).boxed())
}

/// Decrypt one payload with one asymmetric key.
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
pub(crate) fn destack_crypto_key_delete(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.delete")).boxed())
}

/// Return one key descriptor.
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
pub(crate) fn destack_crypto_mac_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.close")).boxed())
}

/// Compute one message authentication code in one shot.
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
pub(crate) fn destack_crypto_mac_finish(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.finish")).boxed())
}

/// Open one streaming MAC context.
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
pub(crate) fn destack_crypto_mac_reset(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.mac.reset")).boxed())
}

/// Update one streaming MAC context.
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
pub(crate) fn destack_crypto_random_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    length: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = length;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.random.bytes")).boxed())
}

/// Fill one mutable byte slice with cryptographically secure random bytes.
pub(crate) fn destack_crypto_random_fill(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = buffer;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.random.fill")).boxed())
}

/// Close one crypto store.
pub(crate) fn destack_crypto_store_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::CryptoStoreHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.store.close")).boxed())
}

/// List certificates from one store.
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
pub(crate) fn destack_crypto_store_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    options: CryptoStoreOptionsVm,
) -> RuntimeResult<resource::CryptoStoreHandle> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.store.open")).boxed())
}
