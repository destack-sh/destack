use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionParameters, CryptoKeyDescriptor, CryptoKeyFormat,
    CryptoKeyGenerationRequest, CryptoKeyImportRequest, CryptoKeyPair, CryptoSignatureParameters,
    core as crypto_core,
};
use crate::platform::{NativeSlice, resource};
use crate::runtime::BindingCallContext;

use super::core::{decode_bytes, write_out_bytes, write_out_value};

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
    let handle = crypto_core::key_generate_secret(context, store, request)?;
    unsafe { write_out_value(out, handle) }
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
    let pair = crypto_core::key_generate_pair(context, store, request)?;
    unsafe { write_out_value(out, pair) }
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
    let handle = crypto_core::key_import(context, store, request)?;
    unsafe { write_out_value(out, handle) }
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
    let bytes = crypto_core::key_export_public(context, handle, format)?;
    unsafe { write_out_bytes(context, out, bytes) }
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
    let bytes = crypto_core::key_export_private(context, handle, format)?;
    unsafe { write_out_bytes(context, out, bytes) }
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
    let bytes = crypto_core::key_export_secret(context, handle, format)?;
    unsafe { write_out_bytes(context, out, bytes) }
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
    let descriptor = crypto_core::key_descriptor(context, handle)?;
    unsafe { write_out_value(out, descriptor) }
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
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let signature = crypto_core::key_sign(context, handle, parameters, &payload)?;
    unsafe { write_out_bytes(context, out, signature) }
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
    payload: NativeSlice<u8>,
    signature: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let signature = decode_bytes(signature, "signature")?;
    let is_valid = crypto_core::key_verify(context, handle, parameters, &payload, &signature)?;
    unsafe { write_out_value(out, is_valid) }
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
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let ciphertext = crypto_core::key_encrypt(context, handle, parameters, &payload)?;
    unsafe { write_out_bytes(context, out, ciphertext) }
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
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let plaintext = crypto_core::key_decrypt(context, handle, parameters, &payload)?;
    unsafe { write_out_bytes(context, out, plaintext) }
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
    let wrapped = crypto_core::key_wrap(context, wrappingkey, keytowrap, format, parameters)?;
    unsafe { write_out_bytes(context, out, wrapped) }
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
    let wrapped_key = decode_bytes(wrappedkey, "wrappedKey")?;
    let handle = crypto_core::key_unwrap(
        context,
        store,
        wrappingkey,
        &wrapped_key,
        parameters,
        request,
    )?;
    unsafe { write_out_value(out, handle) }
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
    crypto_core::key_delete(context, handle)
}
