use windows_sys::Win32::Networking::WinSock::{recv, send};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::SocketHandle;
use crate::platform::{NativeSlice, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Read from a socket into a buffer.
pub(crate) unsafe fn destack_net_read(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let socket = socket_descriptor(_context, handle)?;
    let buffer = unsafe { buffer.as_mut_slice()? };
    let buffer_len = i32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large",
        ))
        .boxed()
    })?;
    let rc = unsafe { recv(socket, buffer.as_mut_ptr() as *mut _, buffer_len, 0) };
    if rc < 0 {
        return Err(last_net_error("recv"));
    }
    unsafe {
        *out = rc as u64;
    }
    Ok(())
}

/// Write to a socket from a buffer.
pub(crate) unsafe fn destack_net_write(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let socket = socket_descriptor(_context, handle)?;
    let buffer = unsafe { buffer.as_slice()? };
    let buffer_len = i32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large",
        ))
        .boxed()
    })?;
    let rc = unsafe { send(socket, buffer.as_ptr() as *const _, buffer_len, 0) };
    if rc < 0 {
        return Err(last_net_error("send"));
    }
    unsafe {
        *out = rc as u64;
    }
    Ok(())
}
