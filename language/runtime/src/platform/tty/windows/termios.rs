use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::tty::core::ensure_out;
use crate::platform::tty::{
    TtyTermiosAttributes, TtyTermiosFlowAction, TtyTermiosQueue, TtyTermiosSetAction,
};
use crate::platform::{PlatformError, process, resource};
use crate::runtime::BindingCallContext;

/// Return one not-supported error for termios operations on Windows.
fn termios_not_supported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Wait for pending output to drain on one terminal handle.
pub(crate) unsafe fn destack_tty_termios_drain(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(termios_not_supported("destack.tty.termios.drain"))
}

/// Apply terminal flow-control action.
pub(crate) unsafe fn destack_tty_termios_flow(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    action: TtyTermiosFlowAction,
) -> RuntimeResult<()> {
    let _ = (handle, action);
    Err(termios_not_supported("destack.tty.termios.flow"))
}

/// Flush one terminal queue.
pub(crate) unsafe fn destack_tty_termios_flush(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    queue: TtyTermiosQueue,
) -> RuntimeResult<()> {
    let _ = (handle, queue);
    Err(termios_not_supported("destack.tty.termios.flush"))
}

/// Read full termios attributes for one terminal handle.
pub(crate) unsafe fn destack_tty_termios_get_attributes(
    binding: &BindingCallContext,
    out: *mut TtyTermiosAttributes,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    let _ = handle;
    Err(termios_not_supported("destack.tty.termios.getAttributes"))
}

/// Read controlling-terminal process-group id.
pub(crate) unsafe fn destack_tty_termios_get_process_group(
    binding: &BindingCallContext,
    out: *mut process::ProcessId,
    handle: resource::TtyHandle,
) -> RuntimeResult<()> {
    ensure_out(out, "out")?;
    let _ = handle;
    Err(termios_not_supported("destack.tty.termios.getProcessGroup"))
}

/// Send one terminal break condition.
pub(crate) unsafe fn destack_tty_termios_send_break(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    duration: u32,
) -> RuntimeResult<()> {
    let _ = (handle, duration);
    Err(termios_not_supported("destack.tty.termios.sendBreak"))
}

/// Apply full termios attributes to one terminal handle.
pub(crate) unsafe fn destack_tty_termios_set_attributes(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    attributes: TtyTermiosAttributes,
    action: TtyTermiosSetAction,
) -> RuntimeResult<()> {
    let _ = (handle, attributes, action);
    Err(termios_not_supported("destack.tty.termios.setAttributes"))
}

/// Set controlling-terminal process-group id.
pub(crate) unsafe fn destack_tty_termios_set_process_group(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    processgroupid: process::ProcessId,
) -> RuntimeResult<()> {
    let _ = (handle, processgroupid);
    Err(termios_not_supported("destack.tty.termios.setProcessGroup"))
}
