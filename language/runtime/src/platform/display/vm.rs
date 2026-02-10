use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::{DisplayInfoVm, DisplayModeVm, WindowEventVm, WindowOptionsVm};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.display.monitor.close.
pub(super) fn destack_display_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.display.monitor.list.
pub(super) fn destack_display_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<DisplayInfoVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.list is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.display.monitor.modes.
pub(super) fn destack_display_modes(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<VmSlice<DisplayModeVm>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.modes is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.display.monitor.open.
pub(super) fn destack_display_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::DisplayHandle> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.display.monitor.setMode.
pub(super) fn destack_display_set_mode(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
    mode: DisplayModeVm,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.setMode is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.display.window.close.
pub(super) fn destack_display_window_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.display.window.event.
pub(super) fn destack_display_window_event(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowEventVm> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.event is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.display.window.open.
pub(super) fn destack_display_window_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    display: resource::DisplayHandle,
    options: WindowOptionsVm,
) -> RuntimeResult<resource::WindowHandle> {
    let _ = (display, options);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.display.window.setTitle.
pub(super) fn destack_display_window_set_title(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    title: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (window, title);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setTitle is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.display.window.tryEvent.
pub(super) fn destack_display_window_try_event(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowEventVm> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.tryEvent is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.display.window.vsyncWait.
pub(super) fn destack_display_window_vsync_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (window, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.vsyncWait is not available in the VM yet",
    ))
    .boxed())
}
