#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::tty::{
    PtyPairVm, TtyModeVm, TtySizeVm, TtyTermiosAttributesVm, TtyTermiosFlowAction, TtyTermiosQueue,
    TtyTermiosSetAction,
};
use crate::platform::{PlatformError, VmSlice, process, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Close one terminal handle.
///
/// Close one terminal endpoint and release runtime ownership.
/// Follow-up operations on the closed handle fail with invalid-handle errors.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Unix and CloseHandle-style finalization on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.handle.close")).boxed())
}

/// Return whether one file handle is attached to a terminal.
///
/// Query one file handle and return true when it targets a terminal endpoint.
/// This can be used before converting process stdio streams into tty workflows.
///
/// # Platform
/// Unix and Windows.
/// Uses isatty(3) on Unix and GetConsoleMode on Windows console handles.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_is_terminal_file(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
) -> RuntimeResult<bool> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.handle.isTerminalFile",
    ))
    .boxed())
}

/// Open one standard error terminal handle.
///
/// Open one terminal handle for the current process standard error stream.
/// The returned handle can be used with tty write, mode, and size operations.
///
/// # Platform
/// Unix and Windows.
/// Uses dup(2) from descriptor 2 on Unix and DuplicateHandle from GetStdHandle(STD_ERROR_HANDLE) on Windows.
/// Fails when the standard stream is not attached to a terminal.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_stdio_stderr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::TtyHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.handle.stdioStderr",
    ))
    .boxed())
}

/// Open one standard input terminal handle.
///
/// Open one terminal handle for the current process standard input stream.
/// The returned handle can be used with tty read, mode, and size operations.
///
/// # Platform
/// Unix and Windows.
/// Uses dup(2) from descriptor 0 on Unix and DuplicateHandle from GetStdHandle(STD_INPUT_HANDLE) on Windows.
/// Fails when the standard stream is not attached to a terminal.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_stdio_stdin(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::TtyHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.handle.stdioStdin",
    ))
    .boxed())
}

/// Open one standard output terminal handle.
///
/// Open one terminal handle for the current process standard output stream.
/// The returned handle can be used with tty write, mode, and size operations.
///
/// # Platform
/// Unix and Windows.
/// Uses dup(2) from descriptor 1 on Unix and DuplicateHandle from GetStdHandle(STD_OUTPUT_HANDLE) on Windows.
/// Fails when the standard stream is not attached to a terminal.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_stdio_stdout(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::TtyHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.handle.stdioStdout",
    ))
    .boxed())
}

/// Read bytes from a terminal.
///
/// Read one byte sequence from one terminal handle into caller memory.
/// Read mode and canonical processing depend on active terminal mode settings.
///
/// # Platform
/// Unix and Windows.
/// Uses read(2) on Unix terminals and ReadConsole or ReadFile on Windows consoles.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.io.read")).boxed())
}

/// Write bytes to a terminal.
///
/// Write one byte sequence from caller memory to one terminal handle.
/// Encoding and newline translation follow host terminal API behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses write(2) on Unix terminals and WriteConsole or WriteFile on Windows consoles.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_write(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.io.write")).boxed())
}

/// Read terminal mode flags.
///
/// Read one terminal mode snapshot for one terminal handle.
/// Mode fields are projected from host terminal APIs.
/// Field-level behavior is host-specific, especially for non-POSIX backends.
///
/// # Platform
/// Unix and Windows.
/// Uses termios get attributes on Unix and GetConsoleMode on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.mode`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_get_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<TtyModeVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.mode.getMode")).boxed())
}

/// Apply terminal mode flags.
///
/// Apply one terminal mode snapshot to one terminal handle.
/// Mode transition timing and unsupported bits follow host API behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses termios set attributes on Unix and SetConsoleMode on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.mode`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_set_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    mode: TtyModeVm,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.mode.setMode")).boxed())
}

/// Enable or disable raw terminal mode.
///
/// Apply one host-defined raw-mode profile for one terminal handle.
/// This maps to cfmakeraw-style behavior on Unix and console-mode toggles on Windows.
///
/// # Platform
/// Unix and Windows.
/// Uses cfmakeraw plus tcsetattr on Unix and SetConsoleMode profile updates on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.mode`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_set_raw_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.mode.setRawMode")).boxed())
}

/// Close one pseudo-terminal controller.
///
/// Close one pseudo-terminal controller endpoint.
/// Worker endpoint behavior after close follows host pseudo-terminal semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses host pseudo-terminal handle close APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.pty`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_pty_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::PtyHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.pty.close")).boxed())
}

/// Open one pseudo-terminal pair.
///
/// Create one controller and worker terminal endpoint pair.
/// Endpoint ownership and inheritance follow host pseudo-terminal semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses posix_openpt and openpty on Unix and ConPTY on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.pty`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_pty_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    rows: u32,
    columns: u32,
    flags: u32,
) -> RuntimeResult<PtyPairVm> {
    let _ = (rows, columns, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.pty.open")).boxed())
}

/// Read terminal size.
///
/// Read one terminal size snapshot for one terminal handle.
/// Pixel fields may be zero when host APIs do not provide pixel metrics.
///
/// # Platform
/// Unix and Windows.
/// Uses TIOCGWINSZ on Unix and console buffer APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.size`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_get_size(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<TtySizeVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.size.getSize")).boxed())
}

/// Apply terminal size.
///
/// Apply one terminal size to one terminal handle.
/// Resize propagation to attached sessions follows host terminal semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses TIOCSWINSZ on Unix and console size APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `tty.size`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_tty_set_size(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    size: TtySizeVm,
) -> RuntimeResult<()> {
    let _ = (handle, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.size.setSize")).boxed())
}

/// Wait for pending output to drain on one terminal handle.
pub(crate) fn destack_tty_termios_drain(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.termios.drain")).boxed())
}

/// Apply terminal flow-control action.
pub(crate) fn destack_tty_termios_flow(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    action: TtyTermiosFlowAction,
) -> RuntimeResult<()> {
    let _ = (handle, action);
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.termios.flow")).boxed())
}

/// Flush one terminal queue.
pub(crate) fn destack_tty_termios_flush(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    queue: TtyTermiosQueue,
) -> RuntimeResult<()> {
    let _ = (handle, queue);
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.termios.flush")).boxed())
}

/// Read full termios attributes for one terminal handle.
pub(crate) fn destack_tty_termios_get_attributes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<TtyTermiosAttributesVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.termios.getAttributes",
    ))
    .boxed())
}

/// Read controlling-terminal process-group id.
pub(crate) fn destack_tty_termios_get_process_group(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<process::ProcessId> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.termios.getProcessGroup",
    ))
    .boxed())
}

/// Send one terminal break condition.
pub(crate) fn destack_tty_termios_send_break(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    duration: u32,
) -> RuntimeResult<()> {
    let _ = (handle, duration);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.termios.sendBreak",
    ))
    .boxed())
}

/// Apply full termios attributes to one terminal handle.
pub(crate) fn destack_tty_termios_set_attributes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    attributes: TtyTermiosAttributesVm,
    action: TtyTermiosSetAction,
) -> RuntimeResult<()> {
    let _ = (handle, attributes, action);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.termios.setAttributes",
    ))
    .boxed())
}

/// Set controlling-terminal process-group id.
pub(crate) fn destack_tty_termios_set_process_group(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    processgroupid: process::ProcessId,
) -> RuntimeResult<()> {
    let _ = (handle, processgroupid);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.termios.setProcessGroup",
    ))
    .boxed())
}
