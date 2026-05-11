use super::core::{io_error, tty_descriptor, validate_buffer_length};

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::resource;
use crate::platform::tty::core::ensure_out;
use crate::runtime::BindingCallContext;

/// Read bytes from a terminal.
pub(crate) unsafe fn destack_tty_read(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::TtyHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // resolve one tty descriptor and caller buffer
    let descriptor = tty_descriptor(binding, handle, "destack.tty.io.read")?;
    let buffer = unsafe { buffer.as_mut_slice()? };
    validate_buffer_length(buffer.len(), "buffer")?;

    // perform one host read
    let status = unsafe {
        libc::read(
            descriptor,
            buffer.as_mut_ptr().cast::<libc::c_void>(),
            buffer.len(),
        )
    };
    if status < 0 {
        return Err(io_error(
            "destack.tty.io.read",
            "read",
            "failed to read from tty descriptor",
        ));
    }

    // write the byte count to the output pointer
    unsafe {
        out.write(status as u64);
    }

    Ok(())
}

/// Write bytes to a terminal.
pub(crate) unsafe fn destack_tty_write(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::TtyHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate the output pointer
    ensure_out(out, "out")?;

    // resolve one tty descriptor and caller buffer
    let descriptor = tty_descriptor(binding, handle, "destack.tty.io.write")?;
    let buffer = unsafe { buffer.as_slice()? };
    validate_buffer_length(buffer.len(), "buffer")?;

    // perform one host write
    let status = unsafe {
        libc::write(
            descriptor,
            buffer.as_ptr().cast::<libc::c_void>(),
            buffer.len(),
        )
    };
    if status < 0 {
        return Err(io_error(
            "destack.tty.io.write",
            "write",
            "failed to write to tty descriptor",
        ));
    }

    // write the byte count to the output pointer
    unsafe {
        out.write(status as u64);
    }

    Ok(())
}
