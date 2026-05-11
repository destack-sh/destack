use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::crypto::{CryptoDigestAlgorithm, core as crypto_core};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::{decode_bytes, write_out_bytes, write_out_value};

/// Compute one digest in one shot.
pub(crate) unsafe fn destack_crypto_digest_compute(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    algorithm: CryptoDigestAlgorithm,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let digest = crypto_core::digest_compute(algorithm, &payload)?;
    unsafe { write_out_bytes(binding, out, digest) }
}

/// Open one streaming digest context.
pub(crate) unsafe fn destack_crypto_digest_open(
    binding: &BindingCallContext,
    out: *mut resource::CryptoDigestHandle,
    algorithm: CryptoDigestAlgorithm,
) -> RuntimeResult<()> {
    let handle = crypto_core::digest_open(binding, algorithm)?;
    unsafe { write_out_value(out, handle) }
}

/// Update one streaming digest context.
pub(crate) unsafe fn destack_crypto_digest_update(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    crypto_core::digest_update(binding, handle, &payload)
}

/// Finalize one streaming digest context and return one digest output.
pub(crate) unsafe fn destack_crypto_digest_finish(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    let digest = crypto_core::digest_finish(binding, handle)?;
    unsafe { write_out_bytes(binding, out, digest) }
}

/// Reset one streaming digest context to its initial state.
pub(crate) unsafe fn destack_crypto_digest_reset(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    crypto_core::digest_reset(binding, handle)
}

/// Close one streaming digest context.
pub(crate) unsafe fn destack_crypto_digest_close(
    binding: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
) -> RuntimeResult<()> {
    crypto_core::digest_close(binding, handle)
}
