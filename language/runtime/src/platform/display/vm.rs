#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::{
    DisplayDescriptorVm, DisplayModeVm, WindowDescriptorVm, WindowEventKind, WindowEventPayloadVm,
    WindowEventVm, WindowFocusPayloadVm, WindowMode, WindowOcclusionPayloadVm, WindowOptionsVm,
    WindowPositionPayloadVm, WindowScaleFactorPayloadVm, WindowSizePayloadVm, WindowStateVm,
    WindowVisibility, WindowVisibilityPayloadVm,
};
use crate::platform::{PlatformError, VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.close is not available in the VM yet",
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmSlice<DisplayDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.list is not available in the VM yet",
    ))
    .boxed())
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<VmSlice<DisplayModeVm>> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.modes is not available in the VM yet",
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<resource::DisplayHandle> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.monitor.open is not available in the VM yet",
    ))
    .boxed())
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
    _runtime: &BindingCallContext,
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

/// Close one window.
///
/// Close one host window and release associated compositor or window-system resources.
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.close is not available in the VM yet",
    ))
    .boxed())
}

/// Read descriptor metadata for one window.
///
/// Read one normalized descriptor snapshot for one opened host window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific window metadata queries.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_descriptor(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowDescriptorVm> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.descriptor is not available in the VM yet",
    ))
    .boxed())
}

/// Close one global window-event stream.
///
/// Close one opened window-event stream and release host event routing resources.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific event-stream close operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window.events`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_event_close(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventClose is not available in the VM yet",
    ))
    .boxed())
}

/// Open one global window-event stream.
///
/// Open one host window-event stream for all windows in this runtime.
/// Event ordering follows host event-loop delivery behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses one host event-loop stream model aligned with winit and SDL style window id routing.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window.events`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_event_open(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<resource::WindowEventHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Wait for one window event.
///
/// Wait for one event from one opened host window-event stream.
///
/// # Platform
/// Unix and Windows.
/// Uses host event queue wait operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window.events`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_event_read(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<WindowEventVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventRead is not available in the VM yet",
    ))
    .boxed())
}

/// Wait for one batch of window events.
///
/// Wait for pending events from one opened host window-event stream and return up to `maxEvents` events.
///
/// # Platform
/// Unix and Windows.
/// Uses host event queue batch wait operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInterrupted, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window.events`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_event_read_batch(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<VmArray<WindowEventVm>> {
    let _ = (handle, maxevents, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventReadBatch is not available in the VM yet",
    ))
    .boxed())
}

/// Poll one window event without blocking.
///
/// Poll one pending event from one opened host window-event stream without waiting.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking host event queue polling.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window.events`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_event_try_read(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<WindowEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventTryRead is not available in the VM yet",
    ))
    .boxed())
}

/// Poll one batch of window events without blocking.
///
/// Poll pending events from one opened host window-event stream and return up to `maxEvents` events.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking host event queue batch polling.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window.events`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_event_try_read_batch(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<VmArray<WindowEventVm>> {
    let _ = (handle, maxevents);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.eventTryReadBatch is not available in the VM yet",
    ))
    .boxed())
}

/// Open one window on a display.
///
/// Create one host window bound to the specified display endpoint.
/// Window lifecycle and compositor integration follow host window-system semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses winit or SDL class host backends over Wayland or X11 or Win32 windowing APIs.
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
    _runtime: &BindingCallContext,
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

/// Request one redraw for one window.
///
/// Enqueue one host redraw request for one opened window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend redraw request operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_request_refresh(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.requestRefresh is not available in the VM yet",
    ))
    .boxed())
}

/// Set one window mode.
///
/// Apply one host window mode transition for one opened window.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific fullscreen and borderless and windowed mode operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_set_mode(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    mode: WindowMode,
) -> RuntimeResult<()> {
    let _ = (window, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setMode is not available in the VM yet",
    ))
    .boxed())
}

/// Set one window position.
///
/// Apply one host window position in physical pixels.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific window move operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_set_position(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    x: i32,
    y: i32,
) -> RuntimeResult<()> {
    let _ = (window, x, y);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setPosition is not available in the VM yet",
    ))
    .boxed())
}

/// Set one window size.
///
/// Apply one host window size in physical pixels.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific window resize operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_set_size(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    width: u32,
    height: u32,
) -> RuntimeResult<()> {
    let _ = (window, width, height);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setSize is not available in the VM yet",
    ))
    .boxed())
}

/// Set one window title string.
///
/// Update one host window title using host window-system APIs.
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
    _runtime: &BindingCallContext,
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

/// Set one window visibility state.
///
/// Apply one window visibility state transition.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific show and hide and minimize and maximize operations.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_set_visibility(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    let _ = (window, visibility);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.setVisibility is not available in the VM yet",
    ))
    .boxed())
}

/// Read one window state snapshot.
///
/// Read one point-in-time host window state snapshot.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific window state queries.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `display.window`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_display_window_state(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    window: resource::WindowHandle,
) -> RuntimeResult<WindowStateVm> {
    let _ = window;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.display.window.state is not available in the VM yet",
    ))
    .boxed())
}

/// Present one frame interval marker.
///
/// Block until the next present interval for one window when host backends support vsync synchronization.
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
    _runtime: &BindingCallContext,
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
