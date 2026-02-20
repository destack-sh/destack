#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::{
    DisplayDescriptorVm, DisplayModeVm, WindowEventVm, WindowOptionsVm,
};
use crate::platform::{PlatformError, VmSlice, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Close one display endpoint.
///
/// Close one opened display endpoint and release host resources.
/// Any outstanding mode-change session state is discarded.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific display close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.close",
    ))
    .boxed())
}

/// List available displays.
///
/// Enumerate host display outputs and return stable identifiers and physical metadata.
/// Output ordering and hotplug visibility follow host compositor or kernel display APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses DRM or Wayland or X11 display enumeration on Unix-like hosts and DXGI display enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<DisplayDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.monitor.list")).boxed())
}

/// Read available display modes.
///
/// Read all host-supported modes for one opened display endpoint.
/// Mode list ordering follows host backend reporting behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses drmModeGetConnector or compositor APIs on Unix-like hosts and DXGI mode queries on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.mode`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_modes(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<VmSlice<DisplayModeVm>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.modes",
    ))
    .boxed())
}

/// Open one display endpoint.
///
/// Open one host display endpoint by identifier for mode queries and updates.
/// Endpoint lifetime semantics follow host display management APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific display open handles.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `display.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::DisplayHandle> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.monitor.open")).boxed())
}

/// Apply one display mode.
///
/// Apply one mode to an opened display endpoint.
/// Mode-set behavior and rollback semantics are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses KMS mode setting on Unix-like hosts and display mode APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.mode`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_display_set_mode(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
    mode: DisplayModeVm,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.setMode",
    ))
    .boxed())
}

/// Close one window.
///
/// Close one host window and release associated compositor or window-system resources.
/// Close behavior follows host event-loop and teardown semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific window close and destroy operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.close")).boxed())
}

/// Wait for one window event.
///
/// Wait for one event from the host window queue and return it as a normalized payload.
/// Event ordering follows host event-loop delivery behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses host event queue wait operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_event(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowEventVm> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.event")).boxed())
}

/// Open one window on a display.
///
/// Create one host window bound to the specified display endpoint.
/// Window lifecycle and compositor integration follow host window-system semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses Wayland or X11 window creation on Unix-like hosts and CreateWindowExW on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    display: resource::DisplayHandle,
    options: WindowOptionsVm,
) -> RuntimeResult<resource::WindowHandle> {
    let _ = (display, options);
    Err(RuntimeError::from(PlatformError::not_supported("destack.display.window.open")).boxed())
}

/// Set one window title string.
///
/// Update one host window title using host window-system APIs.
/// Encoding and truncation semantics follow host platform behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific title update operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_set_title(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    title: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (window, title);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setTitle",
    ))
    .boxed())
}

/// Poll one window event.
///
/// Poll one pending event from the host window event queue without blocking.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking host event queue polling on both platforms.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_try_event(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowEventVm> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.tryEvent",
    ))
    .boxed())
}

/// Present one frame interval marker.
///
/// Block until the next present interval for one window when host backends support vsync synchronization.
/// Wake timing follows host compositor and swap-chain behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses compositor frame callbacks or swap-chain present fences.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.vsync`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_vsync_wait(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _ = (window, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.vsyncWait",
    ))
    .boxed())
}
