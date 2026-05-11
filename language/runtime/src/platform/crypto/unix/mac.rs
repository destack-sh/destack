use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::crypto::{CryptoMacParameters, core as crypto_core};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::{decode_bytes, write_out_bytes, write_out_value};

/// Compute one message authentication code in one shot.
pub(crate) unsafe fn destack_crypto_mac_compute(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let tag = crypto_core::mac_compute(binding, key, parameters, &payload)?;
    unsafe { write_out_bytes(binding, out, tag) }
}

/// Verify one message authentication code in one shot.
pub(crate) unsafe fn destack_crypto_mac_verify(
    binding: &BindingCallContext,
    out: *mut bool,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
    payload: NativeSlice<u8>,
    tag: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let tag = decode_bytes(tag, "tag")?;
    let is_valid = crypto_core::mac_verify(binding, key, parameters, &payload, &tag)?;
    unsafe { write_out_value(out, is_valid) }
}

/// Open one streaming MAC context.
pub(crate) unsafe fn destack_crypto_mac_open(
    binding: &BindingCallContext,
    out: *mut resource::CryptoMacHandle,
    key: resource::CryptoKeyHandle,
    parameters: CryptoMacParameters,
) -> RuntimeResult<()> {
    let handle = crypto_core::mac_open(binding, key, parameters)?;
    unsafe { write_out_value(out, handle) }
}

/// Update one streaming MAC context.
pub(crate) unsafe fn destack_crypto_mac_update(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    crypto_core::mac_update(binding, handle, &payload)
}

/// Finalize one streaming MAC context and return one tag.
pub(crate) unsafe fn destack_crypto_mac_finish(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    let tag = crypto_core::mac_finish(binding, handle)?;
    unsafe { write_out_bytes(binding, out, tag) }
}

/// Reset one streaming MAC context to its initial state.
pub(crate) unsafe fn destack_crypto_mac_reset(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    crypto_core::mac_reset(binding, handle)
}

/// Close one streaming MAC context.
pub(crate) unsafe fn destack_crypto_mac_close(
    binding: &BindingCallContext,
    handle: resource::CryptoMacHandle,
) -> RuntimeResult<()> {
    crypto_core::mac_close(binding, handle)
}
