mod core;
mod target;
#[cfg(any(test, feature = "execution"))]
pub(crate) mod tests;
#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

pub(crate) use core::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::ClipboardBinaryFormat;
use crate::platform::{NativeAbiCodec, NativeSlice, NativeStringRef, PlatformError};
use crate::runtime::BindingCallContext;

/// Clear clipboard payload.
pub(crate) unsafe fn destack_input_clipboard_clear(
    _binding: &BindingCallContext,
) -> RuntimeResult<()> {
    clear()
}

/// Query whether text clipboard payload exists.
pub(crate) unsafe fn destack_input_clipboard_has_text(
    _binding: &BindingCallContext,
    out: *mut bool,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let value = has_text()?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read binary clipboard payload.
pub(crate) unsafe fn destack_input_clipboard_read_bytes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    format: ClipboardBinaryFormat,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let value = read_bytes(format)?;
    let value = binding.store_slice(value);

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read text clipboard payload.
pub(crate) unsafe fn destack_input_clipboard_read_text(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let value = read_text()?;
    let value = binding.store_string(&value);

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read clipboard sequence number.
pub(crate) unsafe fn destack_input_clipboard_sequence(
    _binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let value = sequence()?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Write binary clipboard payload.
pub(crate) unsafe fn destack_input_clipboard_write_bytes(
    _binding: &BindingCallContext,
    format: ClipboardBinaryFormat,
    argument_bytes: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let bytes = unsafe { argument_bytes.as_slice()? };

    write_bytes(format, bytes)
}

/// Write text clipboard payload.
pub(crate) unsafe fn destack_input_clipboard_write_text(
    _binding: &BindingCallContext,
    text: NativeStringRef,
) -> RuntimeResult<()> {
    let text = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(text)? };

    write_text(text.as_str())
}
