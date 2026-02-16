#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::bindings_generated as bindings;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::display::{DisplayInfo, DisplayMode, WindowEvent, WindowOptions};
use crate::platform::resource;

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
pub(crate) unsafe fn destack_display_close(
    context: &RuntimeCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_MONITOR_CLOSE)?;
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
pub(crate) unsafe fn destack_display_list(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<DisplayInfo>,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_MONITOR_LIST)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = out;

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
pub(crate) unsafe fn destack_display_modes(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_MONITOR_MODES)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, handle);

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
pub(crate) unsafe fn destack_display_open(
    context: &RuntimeCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_MONITOR_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, id);

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
pub(crate) unsafe fn destack_display_set_mode(
    context: &RuntimeCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_MONITOR_SET_MODE)?;
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
pub(crate) unsafe fn destack_display_window_close(
    context: &RuntimeCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_WINDOW_CLOSE)?;
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
pub(crate) unsafe fn destack_display_window_event(
    context: &RuntimeCallContext,
    out: *mut WindowEvent,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_WINDOW_EVENT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, window);

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
pub(crate) unsafe fn destack_display_window_open(
    context: &RuntimeCallContext,
    out: *mut resource::WindowHandle,
    display: resource::DisplayHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_WINDOW_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, display, options);

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
pub(crate) unsafe fn destack_display_window_set_title(
    context: &RuntimeCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_WINDOW_SET_TITLE)?;
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
pub(crate) unsafe fn destack_display_window_try_event(
    context: &RuntimeCallContext,
    out: *mut WindowEvent,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_WINDOW_TRY_EVENT)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, window);

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
pub(crate) unsafe fn destack_display_window_vsync_wait(
    context: &RuntimeCallContext,
    window: resource::WindowHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    context.check_policy(DISPLAY_WINDOW_VSYNC_WAIT)?;
    let _ = (window, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.vsyncWait",
    ))
    .boxed())
}
