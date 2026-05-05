use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::core::call_out;
use crate::platform::tty::{
    PtyPairVm, TtyModeVm, TtySizeVm, TtyTermiosAttributes, TtyTermiosAttributesVm,
    TtyTermiosFlowAction, TtyTermiosQueue, TtyTermiosSetAction,
};
use crate::platform::{VmSlice, process, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

use super::host as host_tty;

/// Build one native byte slice from one mutable vec.
fn native_bytes_from_vec(bytes: &mut Vec<u8>) -> NativeSlice<u8> {
    NativeSlice {
        data: bytes.as_mut_ptr(),
        len: bytes.len() as u32,
    }
}

/// Convert one native termios payload into one VM termios payload.
fn termios_attributes_to_vm(
    context: &mut vm::BindingContext<'_>,
    attributes: TtyTermiosAttributes,
) -> RuntimeResult<TtyTermiosAttributesVm> {
    // decode native control characters and move them into VM memory
    let control_characters = unsafe { attributes.control_characters.as_slice()? };
    let mut write = context.write();
    let control_characters = VmSlice::from_bytes(&mut write, control_characters)?;

    // return one projected VM termios payload
    Ok(TtyTermiosAttributesVm {
        input_flags: attributes.input_flags,
        output_flags: attributes.output_flags,
        control_flags: attributes.control_flags,
        local_flags: attributes.local_flags,
        control_characters,
        input_speed_code: attributes.input_speed_code,
        output_speed_code: attributes.output_speed_code,
    })
}

/// Convert one VM termios payload into one native termios payload.
fn termios_attributes_from_vm(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    attributes: TtyTermiosAttributesVm,
) -> RuntimeResult<TtyTermiosAttributes> {
    // copy VM control characters into binding-owned native memory
    let read = context.read();
    let control_characters = attributes.control_characters.read_bytes(&read)?;
    let control_characters = binding.store_slice(control_characters);

    // return one projected native termios payload
    Ok(TtyTermiosAttributes {
        input_flags: attributes.input_flags,
        output_flags: attributes.output_flags,
        control_flags: attributes.control_flags,
        local_flags: attributes.local_flags,
        control_characters,
        input_speed_code: attributes.input_speed_code,
        output_speed_code: attributes.output_speed_code,
    })
}

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
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_close(binding, handle) }
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
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_tty::destack_tty_is_terminal_file(binding, out, handle) })
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
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::TtyHandle> {
    call_out(|out| unsafe { host_tty::destack_tty_stdio_stderr(binding, out) })
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
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::TtyHandle> {
    call_out(|out| unsafe { host_tty::destack_tty_stdio_stdin(binding, out) })
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
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::TtyHandle> {
    call_out(|out| unsafe { host_tty::destack_tty_stdio_stdout(binding, out) })
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
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // copy one vm buffer into mutable host memory for the read call
    let read = context.read();
    let mut bytes = buffer.read_bytes(&read)?;

    // forward to host implementation and capture written count
    let read = call_out(|out| {
        let native_buffer = native_bytes_from_vec(&mut bytes);
        unsafe { host_tty::destack_tty_read(binding, out, handle, native_buffer) }
    })?;

    // write host memory back into the vm buffer
    let mut write = context.write();
    buffer.write_bytes(&mut write, &bytes)?;

    Ok(read)
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
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // copy one vm buffer into host memory for the write call
    let read = context.read();
    let mut bytes = buffer.read_bytes(&read)?;

    // forward to host implementation
    let written = call_out(|out| {
        let native_buffer = native_bytes_from_vec(&mut bytes);
        unsafe { host_tty::destack_tty_write(binding, out, handle, native_buffer) }
    })?;

    Ok(written)
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
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<TtyModeVm> {
    call_out(|out| unsafe { host_tty::destack_tty_get_mode(binding, out, handle) })
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
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    mode: TtyModeVm,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_set_mode(binding, handle, mode) }
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
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_set_raw_mode(binding, handle, enabled) }
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
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::PtyHandle,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_pty_close(binding, handle) }
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
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    rows: u32,
    columns: u32,
    flags: u32,
) -> RuntimeResult<PtyPairVm> {
    call_out(|out| unsafe { host_tty::destack_tty_pty_open(binding, out, rows, columns, flags) })
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
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<TtySizeVm> {
    call_out(|out| unsafe { host_tty::destack_tty_get_size(binding, out, handle) })
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
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    size: TtySizeVm,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_set_size(binding, handle, size) }
}

/// Wait for pending output to drain on one terminal handle.
pub(crate) fn destack_tty_termios_drain(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_termios_drain(binding, handle) }
}

/// Apply terminal flow-control action.
pub(crate) fn destack_tty_termios_flow(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    action: TtyTermiosFlowAction,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_termios_flow(binding, handle, action) }
}

/// Flush one terminal queue.
pub(crate) fn destack_tty_termios_flush(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    queue: TtyTermiosQueue,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_termios_flush(binding, handle, queue) }
}

/// Read full termios attributes for one terminal handle.
pub(crate) fn destack_tty_termios_get_attributes(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<TtyTermiosAttributesVm> {
    // query one native termios payload from host binding
    let attributes = call_out(|out| unsafe {
        host_tty::destack_tty_termios_get_attributes(binding, out, handle)
    })?;

    // project one native payload into VM value lanes
    termios_attributes_to_vm(context, attributes)
}

/// Read controlling-terminal process-group id.
pub(crate) fn destack_tty_termios_get_process_group(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<process::ProcessId> {
    call_out(|out| unsafe { host_tty::destack_tty_termios_get_process_group(binding, out, handle) })
}

/// Send one terminal break condition.
pub(crate) fn destack_tty_termios_send_break(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    duration: u32,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_termios_send_break(binding, handle, duration) }
}

/// Apply full termios attributes to one terminal handle.
pub(crate) fn destack_tty_termios_set_attributes(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    attributes: TtyTermiosAttributesVm,
    action: TtyTermiosSetAction,
) -> RuntimeResult<()> {
    // project VM termios payload into native value lanes
    let attributes = termios_attributes_from_vm(binding, context, attributes)?;

    // apply one native termios update through the host backend
    unsafe { host_tty::destack_tty_termios_set_attributes(binding, handle, attributes, action) }
}

/// Set controlling-terminal process-group id.
pub(crate) fn destack_tty_termios_set_process_group(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    processgroupid: process::ProcessId,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_termios_set_process_group(binding, handle, processgroupid) }
}
