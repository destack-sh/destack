use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::core as crypto_core;
use crate::runtime::{BindingCallContext, NativeSlice};

use crate::platform::crypto::core::{decode_mut_bytes, write_out_bytes};

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
pub(crate) unsafe fn destack_crypto_random_fill(
    _binding: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let buffer = decode_mut_bytes(buffer, "buffer")?;
    crypto_core::random_fill(buffer)
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
pub(crate) unsafe fn destack_crypto_random_bytes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    length: u32,
) -> RuntimeResult<()> {
    let bytes = crypto_core::random_bytes(length)?;
    unsafe { write_out_bytes(binding, out, bytes) }
}
