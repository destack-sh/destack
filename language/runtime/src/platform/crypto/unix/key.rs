use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionParameters, CryptoKeyDescriptor, CryptoKeyFormat,
    CryptoKeyGenerationRequest, CryptoKeyImportRequest, CryptoKeyPair, CryptoKeyWrapParameters,
    CryptoPrivateKeyExportRequest, CryptoSignatureParameters, core as crypto_core,
};
use crate::platform::resource;
use crate::runtime::{BindingCallContext, NativeSlice};

use crate::platform::crypto::core::{decode_bytes, write_out_bytes, write_out_value};

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
pub(crate) unsafe fn destack_crypto_key_generate_secret(
    binding: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<()> {
    let handle = crypto_core::key_generate_secret(binding, store, request)?;
    unsafe { write_out_value(out, handle) }
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
pub(crate) unsafe fn destack_crypto_key_generate_pair(
    binding: &BindingCallContext,
    out: *mut CryptoKeyPair,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<()> {
    let pair = crypto_core::key_generate_pair(binding, store, request)?;
    unsafe { write_out_value(out, pair) }
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
pub(crate) unsafe fn destack_crypto_key_import(
    binding: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<()> {
    let handle = crypto_core::key_import(binding, store, request)?;
    unsafe { write_out_value(out, handle) }
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
pub(crate) unsafe fn destack_crypto_key_export_public(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<()> {
    let bytes = crypto_core::key_export_public(binding, handle, format)?;
    unsafe { write_out_bytes(binding, out, bytes) }
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
pub(crate) unsafe fn destack_crypto_key_export_private(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    request: CryptoPrivateKeyExportRequest,
) -> RuntimeResult<()> {
    let bytes = crypto_core::key_export_private(binding, handle, request)?;
    unsafe { write_out_bytes(binding, out, bytes) }
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
pub(crate) unsafe fn destack_crypto_key_export_secret(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<()> {
    let bytes = crypto_core::key_export_secret(binding, handle, format)?;
    unsafe { write_out_bytes(binding, out, bytes) }
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
pub(crate) unsafe fn destack_crypto_key_descriptor(
    binding: &BindingCallContext,
    out: *mut CryptoKeyDescriptor,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    let descriptor = crypto_core::key_descriptor(binding, handle)?;
    unsafe { write_out_value(out, descriptor) }
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
pub(crate) unsafe fn destack_crypto_key_sign(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParameters,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let signature = crypto_core::key_sign(binding, handle, parameters, &payload)?;
    unsafe { write_out_bytes(binding, out, signature) }
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
pub(crate) unsafe fn destack_crypto_key_verify(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParameters,
    payload: NativeSlice<u8>,
    signature: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let signature = decode_bytes(signature, "signature")?;
    let is_valid = crypto_core::key_verify(binding, handle, parameters, &payload, &signature)?;
    unsafe { write_out_value(out, is_valid) }
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
pub(crate) unsafe fn destack_crypto_key_encrypt(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let ciphertext = crypto_core::key_encrypt(binding, handle, parameters, &payload)?;
    unsafe { write_out_bytes(binding, out, ciphertext) }
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
pub(crate) unsafe fn destack_crypto_key_decrypt(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let plaintext = crypto_core::key_decrypt(binding, handle, parameters, &payload)?;
    unsafe { write_out_bytes(binding, out, plaintext) }
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
pub(crate) unsafe fn destack_crypto_key_wrap(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    wrappingkey: resource::CryptoKeyHandle,
    keytowrap: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
    parameters: CryptoKeyWrapParameters,
) -> RuntimeResult<()> {
    let wrapped = crypto_core::key_wrap(binding, wrappingkey, keytowrap, format, parameters)?;
    unsafe { write_out_bytes(binding, out, wrapped) }
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
pub(crate) unsafe fn destack_crypto_key_unwrap(
    binding: &BindingCallContext,
    out: *mut resource::CryptoKeyHandle,
    store: resource::CryptoStoreHandle,
    wrappingkey: resource::CryptoKeyHandle,
    wrappedkey: NativeSlice<u8>,
    parameters: CryptoKeyWrapParameters,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<()> {
    let wrapped_key = decode_bytes(wrappedkey, "wrappedKey")?;
    let handle = crypto_core::key_unwrap(
        binding,
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
pub(crate) unsafe fn destack_crypto_key_delete(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    crypto_core::key_delete(binding, handle)
}
