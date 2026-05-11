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
pub(crate) fn destack_tty_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_close(binding, handle) }
}

/// Return whether one file handle is attached to a terminal.
pub(crate) fn destack_tty_is_terminal_file(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_tty::destack_tty_is_terminal_file(binding, out, handle) })
}

/// Open one standard error terminal handle.
pub(crate) fn destack_tty_stdio_stderr(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::TtyHandle> {
    call_out(|out| unsafe { host_tty::destack_tty_stdio_stderr(binding, out) })
}

/// Open one standard input terminal handle.
pub(crate) fn destack_tty_stdio_stdin(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::TtyHandle> {
    call_out(|out| unsafe { host_tty::destack_tty_stdio_stdin(binding, out) })
}

/// Open one standard output terminal handle.
pub(crate) fn destack_tty_stdio_stdout(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::TtyHandle> {
    call_out(|out| unsafe { host_tty::destack_tty_stdio_stdout(binding, out) })
}

/// Read bytes from a terminal.
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
pub(crate) fn destack_tty_get_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<TtyModeVm> {
    call_out(|out| unsafe { host_tty::destack_tty_get_mode(binding, out, handle) })
}

/// Apply terminal mode flags.
pub(crate) fn destack_tty_set_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    mode: TtyModeVm,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_set_mode(binding, handle, mode) }
}

/// Enable or disable raw terminal mode.
pub(crate) fn destack_tty_set_raw_mode(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_set_raw_mode(binding, handle, enabled) }
}

/// Close one pseudo-terminal controller.
pub(crate) fn destack_tty_pty_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::PtyHandle,
) -> RuntimeResult<()> {
    unsafe { host_tty::destack_tty_pty_close(binding, handle) }
}

/// Open one pseudo-terminal pair.
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
pub(crate) fn destack_tty_get_size(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<TtySizeVm> {
    call_out(|out| unsafe { host_tty::destack_tty_get_size(binding, out, handle) })
}

/// Apply terminal size.
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
