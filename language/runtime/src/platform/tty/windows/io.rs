use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile};

use super::core::{io_error, tty_binding, tty_handle, validate_buffer_length};

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

    // resolve one tty resolved_binding and caller buffer
    let resolved_binding = tty_binding(binding, handle, "destack.tty.io.read")?;
    let host_handle = match resolved_binding {
        Some(resolved_binding) => resolved_binding.read_handle,
        None => tty_handle(binding, handle, "destack.tty.io.read")?,
    };
    let buffer = unsafe { buffer.as_mut_slice()? };
    let length = validate_buffer_length(buffer.len(), "buffer")?;

    // issue one host read
    let mut bytes_read = 0u32;
    let status = unsafe {
        ReadFile(
            host_handle,
            buffer.as_mut_ptr(),
            length,
            &mut bytes_read,
            std::ptr::null_mut(),
        )
    };
    if status == 0 {
        return Err(io_error(
            "destack.tty.io.read",
            "ReadFile",
            "failed to read from tty handle",
        ));
    }

    // write read count to the output pointer
    unsafe {
        out.write(u64::from(bytes_read));
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

    // resolve one tty resolved_binding and caller buffer
    let resolved_binding = tty_binding(binding, handle, "destack.tty.io.write")?;
    let host_handle = match resolved_binding {
        Some(resolved_binding) => resolved_binding.write_handle,
        None => tty_handle(binding, handle, "destack.tty.io.write")?,
    };
    let buffer = unsafe { buffer.as_slice()? };
    let length = validate_buffer_length(buffer.len(), "buffer")?;

    // issue one host write
    let mut bytes_written = 0u32;
    let status = unsafe {
        WriteFile(
            host_handle,
            buffer.as_ptr(),
            length,
            &mut bytes_written,
            std::ptr::null_mut(),
        )
    };
    if status == 0 {
        return Err(io_error(
            "destack.tty.io.write",
            "WriteFile",
            "failed to write to tty handle",
        ));
    }

    // write write count to the output pointer
    unsafe {
        out.write(u64::from(bytes_written));
    }

    Ok(())
}
