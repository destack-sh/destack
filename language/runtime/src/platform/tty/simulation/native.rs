#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::abi::NativeSlice;

use crate::runtime::BindingCallContext;

use crate::platform::tty::{
    PtyPair, TtyMode, TtySize, TtyTermiosAttributes, TtyTermiosFlowAction, TtyTermiosQueue,
    TtyTermiosSetAction,
};
use crate::platform::{process, resource};

/// Close one terminal handle.
pub(crate) unsafe fn destack_tty_close(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.handle.close")).boxed())
}

/// Return whether one file handle is attached to a terminal.
pub(crate) unsafe fn destack_tty_is_terminal_file(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: resource::FileHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.handle.isTerminalFile",
    ))
    .boxed())
}

/// Open one standard error terminal handle.
pub(crate) unsafe fn destack_tty_stdio_stderr(
    binding: &BindingCallContext,
    out: *mut resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.handle.stdioStderr",
    ))
    .boxed())
}

/// Open one standard input terminal handle.
pub(crate) unsafe fn destack_tty_stdio_stdin(
    binding: &BindingCallContext,
    out: *mut resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.handle.stdioStdin",
    ))
    .boxed())
}

/// Open one standard output terminal handle.
pub(crate) unsafe fn destack_tty_stdio_stdout(
    binding: &BindingCallContext,
    out: *mut resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.handle.stdioStdout",
    ))
    .boxed())
}

/// Read bytes from a terminal.
pub(crate) unsafe fn destack_tty_read(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::TtyHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.io.read")).boxed())
}

/// Write bytes to a terminal.
pub(crate) unsafe fn destack_tty_write(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::TtyHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.io.write")).boxed())
}

/// Read terminal mode flags.
pub(crate) unsafe fn destack_tty_get_mode(
    binding: &BindingCallContext,
    out: *mut TtyMode,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.mode.getMode")).boxed())
}

/// Apply terminal mode flags.
pub(crate) unsafe fn destack_tty_set_mode(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    mode: TtyMode,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.mode.setMode")).boxed())
}

/// Enable or disable raw terminal mode.
pub(crate) unsafe fn destack_tty_set_raw_mode(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.mode.setRawMode")).boxed())
}

/// Close one pseudo-terminal controller.
pub(crate) unsafe fn destack_tty_pty_close(
    binding: &BindingCallContext,
    handle: resource::PtyHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.pty.close")).boxed())
}

/// Open one pseudo-terminal pair.
pub(crate) unsafe fn destack_tty_pty_open(
    binding: &BindingCallContext,
    out: *mut PtyPair,
    rows: u32,
    columns: u32,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, rows, columns, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.pty.open")).boxed())
}

/// Read terminal size.
pub(crate) unsafe fn destack_tty_get_size(
    binding: &BindingCallContext,
    out: *mut TtySize,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.size.getSize")).boxed())
}

/// Apply terminal size.
pub(crate) unsafe fn destack_tty_set_size(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    size: TtySize,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (handle, size);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.size.setSize")).boxed())
}

/// Wait for pending output to drain on one terminal handle.
pub(crate) unsafe fn destack_tty_termios_drain(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.termios.drain")).boxed())
}

/// Apply terminal flow-control action.
pub(crate) unsafe fn destack_tty_termios_flow(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    action: TtyTermiosFlowAction,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (handle, action);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.termios.flow")).boxed())
}

/// Flush one terminal queue.
pub(crate) unsafe fn destack_tty_termios_flush(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    queue: TtyTermiosQueue,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (handle, queue);

    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.termios.flush")).boxed())
}

/// Read full termios attributes for one terminal handle.
pub(crate) unsafe fn destack_tty_termios_get_attributes(
    binding: &BindingCallContext,
    out: *mut TtyTermiosAttributes,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.termios.getAttributes",
    ))
    .boxed())
}

/// Read controlling-terminal process-group id.
pub(crate) unsafe fn destack_tty_termios_get_process_group(
    binding: &BindingCallContext,
    out: *mut process::ProcessId,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = binding;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.termios.getProcessGroup",
    ))
    .boxed())
}

/// Send one terminal break condition.
pub(crate) unsafe fn destack_tty_termios_send_break(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    duration: u32,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (handle, duration);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.termios.sendBreak",
    ))
    .boxed())
}

/// Apply full termios attributes to one terminal handle.
pub(crate) unsafe fn destack_tty_termios_set_attributes(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    attributes: TtyTermiosAttributes,
    action: TtyTermiosSetAction,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (handle, attributes, action);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.termios.setAttributes",
    ))
    .boxed())
}

/// Set controlling-terminal process-group id.
pub(crate) unsafe fn destack_tty_termios_set_process_group(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    processgroupid: process::ProcessId,
) -> RuntimeResult<()> {
    let _ = binding;
    let _ = (handle, processgroupid);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.termios.setProcessGroup",
    ))
    .boxed())
}
