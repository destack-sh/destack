use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionParameters, CryptoKeyDescriptor, CryptoKeyFormat,
    CryptoKeyGenerationRequest, CryptoKeyImportRequest, CryptoKeyPair, CryptoKeyWrapParameters,
    CryptoPrivateKeyExportRequest, CryptoSignatureParameters, core as crypto_core,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::{decode_bytes, write_out_bytes, write_out_value};

/// Generate one symmetric key.
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
pub(crate) unsafe fn destack_crypto_key_descriptor(
    binding: &BindingCallContext,
    out: *mut CryptoKeyDescriptor,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    let descriptor = crypto_core::key_descriptor(binding, handle)?;
    unsafe { write_out_value(out, descriptor) }
}

/// Sign one payload.
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
pub(crate) unsafe fn destack_crypto_key_delete(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    crypto_core::key_delete(binding, handle)
}
