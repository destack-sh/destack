#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::crypto::{
    CryptoCertificateFormat, CryptoCertificateMetadataVm, CryptoCertificateQueryVm,
    CryptoCertificateVerifyRequestVm, CryptoCertificateVerifyResultVm, CryptoEncryptionScheme,
    CryptoKeyFormat, CryptoKeyMetadataVm, CryptoKeyQueryVm, CryptoKeySpecVm, CryptoKeyUsageMask,
    CryptoSignatureScheme, CryptoStoreOptionsVm,
};
use crate::platform::{PlatformError, VmArray, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Delete one certificate from one provider store when allowed.
///
/// Remove one certificate object and invalidate the handle.
/// Deletion permissions and persistence are enforced by host provider policies.
///
/// # Platform
/// Unix and Windows.
/// Uses host certificate delete APIs.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.delete",
    ))
    .boxed())
}

/// Export one certificate from one handle.
///
/// Serialize one certificate handle into the requested encoding format.
/// Output bytes are provider-normalized representations of the underlying certificate.
///
/// # Platform
/// Unix and Windows.
/// Uses host certificate export APIs.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoCertificateHandle,
    format: CryptoCertificateFormat,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, format);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.export",
    ))
    .boxed())
}

/// Import one certificate into one provider store.
///
/// Parse and import one certificate blob into one store and return one certificate handle.
/// Import visibility and persistence are enforced by host provider policies.
///
/// # Platform
/// Unix and Windows.
/// Uses host certificate import APIs for keychain and cert store backends.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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

/// Return metadata for one certificate.
///
/// Query one certificate handle and return normalized identity and validity metadata.
/// Metadata extraction follows host parser and provider normalization behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses host certificate query APIs.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.certificate.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_certificate_metadata(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<CryptoCertificateMetadataVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.metadata",
    ))
    .boxed())
}

/// Verify one certificate chain against requested trust anchors.
///
/// Build and verify one certificate path for the requested purpose and verification time.
/// Chain-building and policy evaluation follow host provider trust engine behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses host trust engine APIs for path building and policy evaluation.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    request: CryptoCertificateVerifyRequestVm,
) -> RuntimeResult<CryptoCertificateVerifyResultVm> {
    let _ = request;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.certificate.verify",
    ))
    .boxed())
}

/// Decrypt one payload with one key handle.
///
/// Decrypt one payload using one provider-backed key and one encryption scheme.
/// Padding and nonce requirements are provider-specific and scheme-specific.
///
/// # Platform
/// Unix and Windows.
/// Uses provider decrypt APIs over host key handles.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
    scheme: CryptoEncryptionScheme,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, scheme, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.decrypt")).boxed())
}

/// Delete one key handle and backing key material when allowed.
///
/// Remove one provider-backed key object and invalidate this handle.
/// Deletion permissions and persistence policies are enforced by the host provider.
///
/// # Platform
/// Unix and Windows.
/// Uses provider key delete APIs.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.delete")).boxed())
}

/// Encrypt one payload with one key handle.
///
/// Encrypt one payload using one provider-backed key and one encryption scheme.
/// Padding and nonce requirements are provider-specific and scheme-specific.
///
/// # Platform
/// Unix and Windows.
/// Uses provider encrypt APIs over host key handles.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
    scheme: CryptoEncryptionScheme,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, scheme, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.encrypt")).boxed())
}

/// Export one public key.
///
/// Export one public-key representation for the selected key handle in the requested format.
/// Exported bytes only include public material and are provider-normalized.
///
/// # Platform
/// Unix and Windows.
/// Uses host provider public-key export APIs.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, format);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.key.exportPublic",
    ))
    .boxed())
}

/// Generate one key in one provider store.
///
/// Create one provider-backed key object with the requested algorithm and key policy.
/// Generated key material remains owned by the host provider unless export is explicitly allowed.
///
/// # Platform
/// Unix and Windows.
/// Uses Security.framework, CNG, or provider-backed generate-key APIs.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.generate`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_generate(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    store: resource::CryptoStoreHandle,
    spec: CryptoKeySpecVm,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    let _ = (store, spec);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.generate")).boxed())
}

/// Import one key into one provider store.
///
/// Parse and import one key blob into the selected provider store.
/// Key visibility and persistence follow host provider policies.
///
/// # Platform
/// Unix and Windows.
/// Uses host provider key import APIs.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    store: resource::CryptoStoreHandle,
    format: CryptoKeyFormat,
    argument_bytes: VmSlice<u8>,
    usagemask: CryptoKeyUsageMask,
    label: vm::StringHandle,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    let _ = (store, format, argument_bytes, usagemask, label);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.import")).boxed())
}

/// Return metadata for one key.
///
/// Query one provider key object and return normalized metadata fields.
/// Metadata visibility is subject to host provider permissions.
///
/// # Platform
/// Unix and Windows.
/// Uses host provider key attribute queries.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_crypto_key_metadata(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<CryptoKeyMetadataVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.metadata")).boxed())
}

/// Sign one message digest or payload.
///
/// Produce one signature using one provider-backed key and one signature scheme.
/// Payload interpretation follows provider and scheme requirements.
///
/// # Platform
/// Unix and Windows.
/// Uses provider sign APIs over host key handles.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
    scheme: CryptoSignatureScheme,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, scheme, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.sign")).boxed())
}

/// Verify one signature with one key handle.
///
/// Verify one signature over one payload using one provider-backed key.
/// Verification semantics and required prehashing follow scheme and provider rules.
///
/// # Platform
/// Unix and Windows.
/// Uses provider verify APIs over host key handles.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoKeyHandle,
    scheme: CryptoSignatureScheme,
    argument_payload: VmSlice<u8>,
    signature: VmSlice<u8>,
) -> RuntimeResult<bool> {
    let _ = (handle, scheme, argument_payload, signature);
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.key.verify")).boxed())
}

/// Close one crypto store.
///
/// Release one host provider store handle.
/// Open key and certificate handles remain valid according to host provider lifetime rules.
///
/// # Platform
/// Unix and Windows.
/// Uses provider-specific store teardown semantics.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoStoreHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.store.close")).boxed())
}

/// List certificates from one store.
///
/// Enumerate certificate handles that match one query selector.
/// Result ordering and visibility follow host provider policies and caller permissions.
///
/// # Platform
/// Unix and Windows.
/// Uses host certificate store enumeration APIs.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoStoreHandle,
    query: CryptoCertificateQueryVm,
) -> RuntimeResult<VmArray<resource::CryptoCertificateHandle>> {
    let _ = (handle, query);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.listCertificates",
    ))
    .boxed())
}

/// List keys from one store.
///
/// Enumerate key handles that match one query selector.
/// Result ordering and visibility follow host provider policies and caller permissions.
///
/// # Platform
/// Unix and Windows.
/// Uses host keychain and keystore enumeration APIs.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::CryptoStoreHandle,
    query: CryptoKeyQueryVm,
) -> RuntimeResult<VmArray<resource::CryptoKeyHandle>> {
    let _ = (handle, query);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.crypto.store.listKeys",
    ))
    .boxed())
}

/// Open one crypto store.
///
/// Create one host provider store handle for key and certificate operations.
/// Provider selection and access scope follow host keychain and keystore semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses Security.framework on macos and ios, CNG and cert stores on Windows, and provider backends on Linux.
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
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: CryptoStoreOptionsVm,
) -> RuntimeResult<resource::CryptoStoreHandle> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported("destack.crypto.store.open")).boxed())
}
