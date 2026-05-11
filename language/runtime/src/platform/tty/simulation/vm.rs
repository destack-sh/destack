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
pub(crate) fn destack_tty_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.handle.close")).boxed())
}

/// Return whether one file handle is attached to a terminal.
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
pub(crate) fn destack_tty_get_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<TtyModeVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.mode.getMode")).boxed())
}

/// Apply terminal mode flags.
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
pub(crate) fn destack_tty_pty_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::PtyHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.pty.close")).boxed())
}

/// Open one pseudo-terminal pair.
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
pub(crate) fn destack_tty_get_size(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<TtySizeVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.tty.size.getSize")).boxed())
}

/// Apply terminal size.
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
