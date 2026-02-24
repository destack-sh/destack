use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::crypto::{CryptoCipherOutput, core as crypto_core};
use crate::platform::{NativeSlice, PlatformError};
use crate::runtime::BindingCallContext;

/// Write one value into one output pointer.
pub(crate) unsafe fn write_out_value<T>(out: *mut T, value: T) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    unsafe {
        *out = value;
    }

    Ok(())
}

/// Write one byte-slice output into one output pointer.
pub(crate) unsafe fn write_out_bytes(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    value: Vec<u8>,
) -> RuntimeResult<()> {
    unsafe { write_out_value(out, context.store_slice(value)) }
}

/// Build one cipher output payload from bytes and tag values.
pub(crate) fn cipher_output(
    context: &BindingCallContext,
    bytes: Vec<u8>,
    tag: Vec<u8>,
) -> CryptoCipherOutput {
    CryptoCipherOutput {
        bytes: context.store_slice(bytes),
        tag: context.store_slice(tag),
    }
}

/// Decode one native byte slice argument.
pub(crate) fn decode_bytes(slice: NativeSlice<u8>, field: &str) -> RuntimeResult<Vec<u8>> {
    crypto_core::decode_native_bytes(slice, field)
}

/// Decode one native mutable byte slice argument.
pub(crate) fn decode_mut_bytes<'a>(
    slice: NativeSlice<u8>,
    field: &str,
) -> RuntimeResult<&'a mut [u8]> {
    crypto_core::decode_native_mut_bytes(slice, field)
}
