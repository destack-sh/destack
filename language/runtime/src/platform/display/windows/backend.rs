use super::{core, win32};
use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayDescriptor, DisplayEvent, DisplayMode, DisplayMonitorEventOpenOptions,
    DisplayMonitorListRequest, DisplayMonitorOpenOptions, WindowAttentionLevel, WindowCursorIcon,
    WindowCursorMode, WindowDescriptor, WindowEvent, WindowEventOpenOptions, WindowLogicalSize,
    WindowModeOptions, WindowOptions, WindowPhysicalSize, WindowPosition, WindowSizeConstraints,
    WindowState, WindowVisibility,
};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, resource};
use crate::runtime::BindingCallContext;

/// Close one display endpoint.
pub(crate) unsafe fn destack_display_monitor_close(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.close")?;
    unsafe { win32::monitor_close(context, handle) }
}

/// Resolve one requested mode to the closest supported mode.
pub(crate) unsafe fn destack_display_monitor_closest_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
    requested: DisplayMode,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.closestMode")?;
    unsafe { win32::monitor_closest_mode(context, out, handle, requested) }
}

/// Read the current mode for one opened display.
pub(crate) unsafe fn destack_display_monitor_current_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.currentMode")?;
    unsafe { win32::monitor_current_mode(context, out, handle) }
}

/// Read descriptor metadata for one opened display.
pub(crate) unsafe fn destack_display_monitor_descriptor(
    context: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.descriptor")?;
    unsafe { win32::monitor_descriptor(context, out, handle) }
}

/// Read the desktop-preferred mode for one opened display.
pub(crate) unsafe fn destack_display_monitor_desktop_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.desktopMode")?;
    unsafe { win32::monitor_desktop_mode(context, out, handle) }
}

/// List available displays.
pub(crate) unsafe fn destack_display_monitor_list(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    let _backend = core::resolve_backend(
        request.backend,
        request.backend_policy,
        "destack.display.monitor.list",
    )?;
    unsafe { win32::monitor_list(context, out, request) }
}

/// Read available display modes.
pub(crate) unsafe fn destack_display_monitor_modes(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.modes")?;
    unsafe { win32::monitor_modes(context, out, handle) }
}

/// Open one display endpoint.
pub(crate) unsafe fn destack_display_monitor_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
    options: DisplayMonitorOpenOptions,
) -> RuntimeResult<()> {
    let _backend = core::resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.display.monitor.open",
    )?;
    unsafe { win32::monitor_open(context, out, id, options) }
}

/// Read the current primary display handle.
pub(crate) unsafe fn destack_display_monitor_primary(
    context: &BindingCallContext,
    out: *mut Option<resource::DisplayHandle>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    let _backend = core::resolve_backend(
        request.backend,
        request.backend_policy,
        "destack.display.monitor.primary",
    )?;
    unsafe { win32::monitor_primary(context, out, request) }
}

/// Apply one display mode.
pub(crate) unsafe fn destack_display_monitor_set_mode(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.setMode")?;
    unsafe { win32::monitor_set_mode(context, handle, mode) }
}

/// Close one global monitor-event stream.
pub(crate) unsafe fn destack_display_monitor_event_close(
    context: &BindingCallContext,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.eventClose")?;
    unsafe { win32::monitor_event_close(context, handle) }
}

/// Open one global monitor-event stream.
pub(crate) unsafe fn destack_display_monitor_event_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayEventHandle,
    options: DisplayMonitorEventOpenOptions,
) -> RuntimeResult<()> {
    let _backend = core::resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.display.monitor.eventOpen",
    )?;
    unsafe { win32::monitor_event_open(context, out, options) }
}

/// Wait for one monitor event.
pub(crate) unsafe fn destack_display_monitor_event_read(
    context: &BindingCallContext,
    out: *mut DisplayEvent,
    handle: resource::DisplayEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.eventRead")?;
    unsafe { win32::monitor_event_read(context, out, handle, timeout_ns) }
}

/// Wait for one batch of monitor events.
pub(crate) unsafe fn destack_display_monitor_event_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<DisplayEvent>,
    handle: resource::DisplayEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.eventReadBatch")?;
    unsafe { win32::monitor_event_read_batch(context, out, handle, max_events, timeout_ns) }
}

/// Poll one monitor event without blocking.
pub(crate) unsafe fn destack_display_monitor_event_try_read(
    context: &BindingCallContext,
    out: *mut DisplayEvent,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.eventTryRead")?;
    unsafe { win32::monitor_event_try_read(context, out, handle) }
}

/// Poll one batch of monitor events without blocking.
pub(crate) unsafe fn destack_display_monitor_event_try_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<DisplayEvent>,
    handle: resource::DisplayEventHandle,
    max_events: u32,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.eventTryReadBatch")?;
    unsafe { win32::monitor_event_try_read_batch(context, out, handle, max_events) }
}

/// Close one window.
pub(crate) unsafe fn destack_display_window_close(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.close")?;
    unsafe { win32::window_close(context, window) }
}

/// Read descriptor metadata for one window.
pub(crate) unsafe fn destack_display_window_descriptor(
    context: &BindingCallContext,
    out: *mut WindowDescriptor,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.descriptor")?;
    unsafe { win32::window_descriptor(context, out, window) }
}

/// Open one window.
pub(crate) unsafe fn destack_display_window_open(
    context: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    let _backend = core::resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.display.window.open",
    )?;
    unsafe { win32::window_open(context, out, options) }
}

/// Request user attention for one window.
pub(crate) unsafe fn destack_display_window_request_attention(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.requestAttention")?;
    unsafe { win32::window_request_attention(context, window, level) }
}

/// Request one redraw for one window.
pub(crate) unsafe fn destack_display_window_request_refresh(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.requestRefresh")?;
    unsafe { win32::window_request_refresh(context, window) }
}

/// Set always-on-top state.
pub(crate) unsafe fn destack_display_window_set_always_on_top(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setAlwaysOnTop")?;
    unsafe { win32::window_set_always_on_top(context, window, alwaysontop) }
}

/// Set cursor icon for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_icon(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setCursorIcon")?;
    unsafe { win32::window_set_cursor_icon(context, window, icon) }
}

/// Set cursor interaction mode for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setCursorMode")?;
    unsafe { win32::window_set_cursor_mode(context, window, mode) }
}

/// Set cursor position for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setCursorPosition")?;
    unsafe { win32::window_set_cursor_position(context, window, position) }
}

/// Set cursor visibility for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_visible(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setCursorVisible")?;
    unsafe { win32::window_set_cursor_visible(context, window, visible) }
}

/// Set window decoration state.
pub(crate) unsafe fn destack_display_window_set_decorated(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setDecorated")?;
    unsafe { win32::window_set_decorated(context, window, decorated) }
}

/// Set one window mode.
pub(crate) unsafe fn destack_display_window_set_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setMode")?;
    unsafe { win32::window_set_mode(context, window, mode) }
}

/// Set one window position.
pub(crate) unsafe fn destack_display_window_set_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setPosition")?;
    unsafe { win32::window_set_position(context, window, position) }
}

/// Set window resizable state.
pub(crate) unsafe fn destack_display_window_set_resizable(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setResizable")?;
    unsafe { win32::window_set_resizable(context, window, resizable) }
}

/// Set logical size constraints.
pub(crate) unsafe fn destack_display_window_set_size_constraints(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setSizeConstraints")?;
    unsafe { win32::window_set_size_constraints(context, window, constraints) }
}

/// Set one logical window size.
pub(crate) unsafe fn destack_display_window_set_size_logical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setSizeLogical")?;
    unsafe { win32::window_set_size_logical(context, window, size) }
}

/// Set one physical window size.
pub(crate) unsafe fn destack_display_window_set_size_physical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setSizePhysical")?;
    unsafe { win32::window_set_size_physical(context, window, size) }
}

/// Set one window title string.
pub(crate) unsafe fn destack_display_window_set_title(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setTitle")?;
    unsafe { win32::window_set_title(context, window, title) }
}

/// Set one window visibility state.
pub(crate) unsafe fn destack_display_window_set_visibility(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setVisibility")?;
    unsafe { win32::window_set_visibility(context, window, visibility) }
}

/// Read one window state snapshot.
pub(crate) unsafe fn destack_display_window_state(
    context: &BindingCallContext,
    out: *mut WindowState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.state")?;
    unsafe { win32::window_state(context, out, window) }
}

/// Present one frame interval marker.
pub(crate) unsafe fn destack_display_window_vsync_wait(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.vsyncWait")?;
    unsafe { win32::window_vsync_wait(context, window, timeoutns) }
}

/// Close one global window-event stream.
pub(crate) unsafe fn destack_display_window_event_close(
    context: &BindingCallContext,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.eventClose")?;
    unsafe { win32::window_event_close(context, handle) }
}

/// Open one global window-event stream.
pub(crate) unsafe fn destack_display_window_event_open(
    context: &BindingCallContext,
    out: *mut resource::WindowEventHandle,
    options: WindowEventOpenOptions,
) -> RuntimeResult<()> {
    let _backend = core::resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.display.window.eventOpen",
    )?;
    unsafe { win32::window_event_open(context, out, options) }
}

/// Wait for one window event.
pub(crate) unsafe fn destack_display_window_event_read(
    context: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.eventRead")?;
    unsafe { win32::window_event_read(context, out, handle, timeoutns) }
}

/// Wait for one batch of window events.
pub(crate) unsafe fn destack_display_window_event_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.eventReadBatch")?;
    unsafe { win32::window_event_read_batch(context, out, handle, maxevents, timeoutns) }
}

/// Poll one window event without blocking.
pub(crate) unsafe fn destack_display_window_event_try_read(
    context: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.eventTryRead")?;
    unsafe { win32::window_event_try_read(context, out, handle) }
}

/// Poll one batch of window events without blocking.
pub(crate) unsafe fn destack_display_window_event_try_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.eventTryReadBatch")?;
    unsafe { win32::window_event_try_read_batch(context, out, handle, maxevents) }
}
