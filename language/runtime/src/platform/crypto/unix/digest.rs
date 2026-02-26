use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{CryptoDigestAlgorithm, core as crypto_core};
use crate::platform::{NativeSlice, resource};
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::{decode_bytes, write_out_bytes, write_out_value};

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
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    algorithm: CryptoDigestAlgorithm,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let digest = crypto_core::digest_compute(algorithm, &payload)?;
    unsafe { write_out_bytes(context, out, digest) }
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
    context: &BindingCallContext,
    out: *mut resource::CryptoDigestHandle,
    algorithm: CryptoDigestAlgorithm,
) -> RuntimeResult<()> {
    let handle = crypto_core::digest_open(context, algorithm)?;
    unsafe { write_out_value(out, handle) }
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
    context: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    crypto_core::digest_update(context, handle, &payload)
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
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    let digest = crypto_core::digest_finish(context, handle)?;
    unsafe { write_out_bytes(context, out, digest) }
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
    context: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    crypto_core::digest_reset(context, handle)
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
    context: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    crypto_core::digest_close(context, handle)
}
