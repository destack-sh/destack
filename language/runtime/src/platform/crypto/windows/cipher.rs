use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::crypto::{
    CryptoCipherDirection, CryptoCipherOutput, CryptoCipherParameters, core as crypto_core,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::{cipher_output, decode_bytes, write_out_value};

/// Encrypt one payload in one shot.
pub(crate) unsafe fn destack_crypto_cipher_encrypt(
    binding: &BindingCallContext,
    out: *mut CryptoCipherOutput,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let (bytes, tag) = crypto_core::cipher_encrypt(binding, key, parameters, &payload)?;
    let output = cipher_output(binding, bytes, tag);
    unsafe { write_out_value(out, output) }
}

/// Decrypt one payload in one shot.
pub(crate) unsafe fn destack_crypto_cipher_decrypt(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    key: resource::CryptoKeyHandle,
    parameters: CryptoCipherParameters,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let output = crypto_core::cipher_decrypt(binding, key, parameters, &payload)?;
    unsafe { write_out_value(out, binding.store_slice(output)) }
}

/// Open one streaming cipher context.
pub(crate) unsafe fn destack_crypto_cipher_open(
    binding: &BindingCallContext,
    out: *mut resource::CryptoCipherHandle,
    key: resource::CryptoKeyHandle,
    direction: CryptoCipherDirection,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    let handle = crypto_core::cipher_open(binding, key, direction, parameters)?;
    unsafe { write_out_value(out, handle) }
}

/// Update additional authenticated data for one streaming cipher context.
pub(crate) unsafe fn destack_crypto_cipher_update_additional_data(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    additionaldata: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let additional_data = decode_bytes(additionaldata, "additionalData")?;
    crypto_core::cipher_update_additional_data(binding, handle, &additional_data)
}

/// Update one streaming cipher context with one payload chunk.
pub(crate) unsafe fn destack_crypto_cipher_update(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoCipherHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(payload, "payload")?;
    let output = crypto_core::cipher_update(binding, handle, &payload)?;
    unsafe { write_out_value(out, binding.store_slice(output)) }
}

/// Finalize one streaming cipher context.
pub(crate) unsafe fn destack_crypto_cipher_finish(
    binding: &BindingCallContext,
    out: *mut CryptoCipherOutput,
    handle: resource::CryptoCipherHandle,
    finalpayload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let payload = decode_bytes(finalpayload, "finalPayload")?;
    let (bytes, tag) = crypto_core::cipher_finish(binding, handle, &payload)?;
    let output = cipher_output(binding, bytes, tag);
    unsafe { write_out_value(out, output) }
}

/// Reset one streaming cipher context with new parameters.
pub(crate) unsafe fn destack_crypto_cipher_reset(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    parameters: CryptoCipherParameters,
) -> RuntimeResult<()> {
    crypto_core::cipher_reset(binding, handle, parameters)
}

/// Close one streaming cipher context.
pub(crate) unsafe fn destack_crypto_cipher_close(
    binding: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
) -> RuntimeResult<()> {
    crypto_core::cipher_close(binding, handle)
}
