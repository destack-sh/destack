use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoCipherDirection, CryptoCipherOutput, CryptoCipherParameters, core as crypto_core,
};
use crate::platform::{NativeSlice, resource};
use crate::runtime::BindingCallContext;

use super::core::{cipher_output, decode_bytes, write_out_value};

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
    context: &BindingCallContext,
    out: *mut CryptoCipherOutput,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let (bytes, tag) = crypto_core::cipher_encrypt(context, key, parameters, &payload)?;
    let output = cipher_output(context, bytes, tag);
    unsafe { write_out_value(out, output) }
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
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let output = crypto_core::cipher_decrypt(context, key, parameters, &payload)?;
    unsafe { write_out_value(out, context.store_slice(output)) }
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
    context: &BindingCallContext,
    out: *mut resource::CryptoCipherHandle,
    key: resource::CryptoKeyHandle,
    direction: CryptoCipherDirection,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    let handle = crypto_core::cipher_open(context, key, direction, parameters)?;
    unsafe { write_out_value(out, handle) }
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
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    additionaldata: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let additional_data = decode_bytes(additionaldata, "additionalData")?;
    crypto_core::cipher_update_additional_data(context, handle, &additional_data)
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
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoCipherHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let output = crypto_core::cipher_update(context, handle, &payload)?;
    unsafe { write_out_value(out, context.store_slice(output)) }
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
    context: &BindingCallContext,
    out: *mut CryptoCipherOutput,
    handle: resource::CryptoCipherHandle,
    finalpayload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(finalpayload, "finalPayload")?;
    let (bytes, tag) = crypto_core::cipher_finish(context, handle, &payload)?;
    let output = cipher_output(context, bytes, tag);
    unsafe { write_out_value(out, output) }
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
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    crypto_core::cipher_reset(context, handle, parameters)
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
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
) -> RuntimeResult<()> {
    crypto_core::cipher_close(context, handle)
}
