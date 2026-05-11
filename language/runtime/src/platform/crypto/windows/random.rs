use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::crypto::core as crypto_core;
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::{decode_mut_bytes, write_out_bytes};

/// Fill one mutable byte slice with cryptographically secure random bytes.
pub(crate) unsafe fn destack_crypto_random_fill(
    _binding: &BindingCallContext,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let buffer = decode_mut_bytes(buffer, "buffer")?;
    crypto_core::random_fill(buffer)
}

/// Allocate one random byte vector with the requested length.
pub(crate) unsafe fn destack_crypto_random_bytes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    length: u32,
) -> RuntimeResult<()> {
    let bytes = crypto_core::random_bytes(length)?;
    unsafe { write_out_bytes(binding, out, bytes) }
}
