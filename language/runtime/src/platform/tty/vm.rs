use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::tty::{PtyPairVm, TtyModeVm, TtySizeVm};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.tty.io.read.
pub(super) fn destack_tty_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TtyHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.io.read is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.tty.io.write.
pub(super) fn destack_tty_write(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TtyHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.io.write is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.tty.mode.getMode.
pub(super) fn destack_tty_get_mode(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<TtyModeVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.mode.getMode is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.tty.mode.setMode.
pub(super) fn destack_tty_set_mode(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TtyHandle,
    mode: TtyModeVm,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.mode.setMode is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.tty.pty.close.
pub(super) fn destack_tty_pty_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::PtyHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.pty.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.tty.pty.open.
pub(super) fn destack_tty_pty_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    rows: u32,
    columns: u32,
    flags: u32,
) -> RuntimeResult<PtyPairVm> {
    let _ = (rows, columns, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.pty.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.tty.size.getSize.
pub(super) fn destack_tty_get_size(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TtyHandle,
) -> RuntimeResult<TtySizeVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.size.getSize is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.tty.size.setSize.
pub(super) fn destack_tty_set_size(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TtyHandle,
    size: TtySizeVm,
) -> RuntimeResult<()> {
    let _ = (handle, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.tty.size.setSize is not available in the VM yet",
    ))
    .boxed())
}
