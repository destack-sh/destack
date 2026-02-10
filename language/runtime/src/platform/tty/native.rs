#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::tty::bindings_generated as bindings;
use crate::platform::{NativeSlice, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::resource;
use crate::platform::tty::{PtyPair, TtyMode, TtySize};

/// Stub for destack.tty.io.read.
pub unsafe fn destack_tty_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::TtyHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(TTY_IO_READ)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.io.read")).boxed())
}

/// Stub for destack.tty.io.write.
pub unsafe fn destack_tty_write(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::TtyHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    context.check_policy(TTY_IO_WRITE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.io.write")).boxed())
}

/// Stub for destack.tty.mode.getMode.
pub unsafe fn destack_tty_get_mode(
    context: &RuntimeCallContext,
    out: *mut TtyMode,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    context.check_policy(TTY_MODE_GET_MODE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.mode.getMode")).boxed())
}

/// Stub for destack.tty.mode.setMode.
pub unsafe fn destack_tty_set_mode(
    context: &RuntimeCallContext,
    handle: resource::TtyHandle,
    mode: TtyMode,
) -> RuntimeResult<()> {
    context.check_policy(TTY_MODE_SET_MODE)?;
    let _ = (handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.mode.setMode")).boxed())
}

/// Stub for destack.tty.pty.close.
pub unsafe fn destack_tty_pty_close(
    context: &RuntimeCallContext,
    handle: resource::PtyHandle,
) -> RuntimeResult<()> {
    context.check_policy(TTY_PTY_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.pty.close")).boxed())
}

/// Stub for destack.tty.pty.open.
pub unsafe fn destack_tty_pty_open(
    context: &RuntimeCallContext,
    out: *mut PtyPair,
    rows: u32,
    columns: u32,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(TTY_PTY_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, rows, columns, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.pty.open")).boxed())
}

/// Stub for destack.tty.size.getSize.
pub unsafe fn destack_tty_get_size(
    context: &RuntimeCallContext,
    out: *mut TtySize,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    context.check_policy(TTY_SIZE_GET_SIZE)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.size.getSize")).boxed())
}

/// Stub for destack.tty.size.setSize.
pub unsafe fn destack_tty_set_size(
    context: &RuntimeCallContext,
    handle: resource::TtyHandle,
    size: TtySize,
) -> RuntimeResult<()> {
    context.check_policy(TTY_SIZE_SET_SIZE)?;
    let _ = (handle, size);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.size.setSize")).boxed())
}
