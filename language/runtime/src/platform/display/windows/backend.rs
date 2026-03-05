use super::{core, win32};
use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayColorState, DisplayDescriptor,
    DisplayGammaRamp, DisplayHdrMode, DisplayMode, DisplayMonitorEvent,
    DisplayMonitorEventOpenOptions, DisplayMonitorListRequest, DisplayMonitorOpenOptions,
    WindowAspectRatio, WindowAttentionLevel, WindowChromeKind, WindowCursorIcon, WindowCursorMode,
    WindowDescriptor, WindowEvent, WindowEventOpenOptions, WindowIconSet, WindowLogicalSize,
    WindowModeOptions, WindowOptions, WindowPhysicalSize, WindowPosition, WindowResizeEdge,
    WindowSizeConstraints, WindowState, WindowVisibility,
};
use crate::platform::{NativeArray, core as core_platform, resource};
use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef};

/// Close one display endpoint.
pub(crate) unsafe fn destack_display_monitor_close(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.close")?;
    unsafe { win32::monitor_close(binding, handle) }
}

/// Resolve one requested mode to the closest supported mode.
pub(crate) unsafe fn destack_display_monitor_closest_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
    requested: DisplayMode,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.closestMode")?;
    unsafe { win32::monitor_closest_mode(binding, out, handle, requested) }
}

/// Read the current mode for one opened display.
pub(crate) unsafe fn destack_display_monitor_current_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.currentMode")?;
    unsafe { win32::monitor_current_mode(binding, out, handle) }
}

/// Read descriptor metadata for one opened display.
pub(crate) unsafe fn destack_display_monitor_descriptor(
    binding: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.descriptor")?;
    unsafe { win32::monitor_descriptor(binding, out, handle) }
}

/// Read the desktop-preferred mode for one opened display.
pub(crate) unsafe fn destack_display_monitor_desktop_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.desktopMode")?;
    unsafe { win32::monitor_desktop_mode(binding, out, handle) }
}

/// List available displays.
pub(crate) unsafe fn destack_display_monitor_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    let _backend = core::resolve_backend(
        request.backend,
        request.backend_policy,
        "destack.display.monitor.list",
    )?;
    unsafe { win32::monitor_list(binding, out, request) }
}

/// Read available display modes.
pub(crate) unsafe fn destack_display_monitor_modes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.modes")?;
    unsafe { win32::monitor_modes(binding, out, handle) }
}

/// Open one display endpoint.
pub(crate) unsafe fn destack_display_monitor_open(
    binding: &BindingCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
    options: DisplayMonitorOpenOptions,
) -> RuntimeResult<()> {
    let _backend = core::resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.display.monitor.open",
    )?;
    unsafe { win32::monitor_open(binding, out, id, options) }
}

/// Read the current primary display handle.
pub(crate) unsafe fn destack_display_monitor_primary(
    binding: &BindingCallContext,
    out: *mut Option<resource::DisplayHandle>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    let _backend = core::resolve_backend(
        request.backend,
        request.backend_policy,
        "destack.display.monitor.primary",
    )?;
    unsafe { win32::monitor_primary(binding, out, request) }
}

/// Apply one display mode.
pub(crate) unsafe fn destack_display_monitor_set_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.setMode")?;
    unsafe { win32::monitor_set_mode(binding, handle, mode) }
}

/// Read display color state.
pub(crate) unsafe fn destack_display_monitor_color_state(
    binding: &BindingCallContext,
    out: *mut DisplayColorState,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.colorState")?;
    unsafe { win32::monitor_color_state(binding, out, handle) }
}

/// Read display HDR mode.
pub(crate) unsafe fn destack_display_monitor_hdr_mode(
    binding: &BindingCallContext,
    out: *mut DisplayHdrMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.hdrMode")?;
    unsafe { win32::monitor_hdr_mode(binding, out, handle) }
}

/// Set display HDR mode.
pub(crate) unsafe fn destack_display_monitor_set_hdr_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayHdrMode,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.setHdrMode")?;
    unsafe { win32::monitor_set_hdr_mode(binding, handle, mode) }
}

/// Read display gamma ramp.
pub(crate) unsafe fn destack_display_monitor_gamma_ramp(
    binding: &BindingCallContext,
    out: *mut DisplayGammaRamp,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.gammaRamp")?;
    unsafe { win32::monitor_gamma_ramp(binding, out, handle) }
}

/// Set display gamma ramp.
pub(crate) unsafe fn destack_display_monitor_set_gamma_ramp(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRamp,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.setGammaRamp")?;
    unsafe { win32::monitor_set_gamma_ramp(binding, handle, ramp) }
}

/// Close one global monitor-event stream.
pub(crate) unsafe fn destack_display_monitor_event_close(
    binding: &BindingCallContext,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.eventClose")?;
    unsafe { win32::monitor_event_close(binding, handle) }
}

/// Open one global monitor-event stream.
pub(crate) unsafe fn destack_display_monitor_event_open(
    binding: &BindingCallContext,
    out: *mut resource::DisplayEventHandle,
    options: DisplayMonitorEventOpenOptions,
) -> RuntimeResult<()> {
    let _backend = core::resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.display.monitor.eventOpen",
    )?;
    unsafe { win32::monitor_event_open(binding, out, options) }
}

/// Wait for one monitor event.
pub(crate) unsafe fn destack_display_monitor_event_read(
    binding: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.eventRead")?;
    unsafe { win32::monitor_event_read(binding, out, handle, timeout_ns) }
}

/// Wait for one batch of monitor events.
pub(crate) unsafe fn destack_display_monitor_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.eventReadBatch")?;
    unsafe { win32::monitor_event_read_batch(binding, out, handle, max_events, timeout_ns) }
}

/// Poll one monitor event without blocking.
pub(crate) unsafe fn destack_display_monitor_event_try_read(
    binding: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.eventTryRead")?;
    unsafe { win32::monitor_event_try_read(binding, out, handle) }
}

/// Poll one batch of monitor events without blocking.
pub(crate) unsafe fn destack_display_monitor_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    max_events: u32,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.eventTryReadBatch")?;
    unsafe { win32::monitor_event_try_read_batch(binding, out, handle, max_events) }
}

/// Close one window.
pub(crate) unsafe fn destack_display_window_close(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.close")?;
    unsafe { win32::window_close(binding, window) }
}

/// Read descriptor metadata for one window.
pub(crate) unsafe fn destack_display_window_descriptor(
    binding: &BindingCallContext,
    out: *mut WindowDescriptor,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.descriptor")?;
    unsafe { win32::window_descriptor(binding, out, window) }
}

/// Open one window.
pub(crate) unsafe fn destack_display_window_open(
    binding: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    let _backend = core::resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.display.window.open",
    )?;
    unsafe { win32::window_open(binding, out, options) }
}

/// Request user attention for one window.
pub(crate) unsafe fn destack_display_window_request_attention(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.requestAttention")?;
    unsafe { win32::window_request_attention(binding, window, level) }
}

/// Request one redraw for one window.
pub(crate) unsafe fn destack_display_window_request_refresh(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.requestRefresh")?;
    unsafe { win32::window_request_refresh(binding, window) }
}

/// Set always-on-top state.
pub(crate) unsafe fn destack_display_window_set_always_on_top(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setAlwaysOnTop")?;
    unsafe { win32::window_set_always_on_top(binding, window, alwaysontop) }
}

/// Set cursor icon for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_icon(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setCursorIcon")?;
    unsafe { win32::window_set_cursor_icon(binding, window, icon) }
}

/// Set cursor interaction mode for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_mode(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setCursorMode")?;
    unsafe { win32::window_set_cursor_mode(binding, window, mode) }
}

/// Set cursor position for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_position(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setCursorPosition")?;
    unsafe { win32::window_set_cursor_position(binding, window, position) }
}

/// Set cursor visibility for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_visible(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setCursorVisible")?;
    unsafe { win32::window_set_cursor_visible(binding, window, visible) }
}

/// Set window decoration state.
pub(crate) unsafe fn destack_display_window_set_decorated(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setDecorated")?;
    unsafe { win32::window_set_decorated(binding, window, decorated) }
}

/// Set one window mode.
pub(crate) unsafe fn destack_display_window_set_mode(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setMode")?;
    unsafe { win32::window_set_mode(binding, window, mode) }
}

/// Set one window aspect-ratio lock.
pub(crate) unsafe fn destack_display_window_set_aspect_ratio(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    aspectratio: Option<WindowAspectRatio>,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setAspectRatio")?;
    unsafe { win32::window_set_aspect_ratio(binding, window, aspectratio) }
}

/// Set one window chrome kind.
pub(crate) unsafe fn destack_display_window_set_chrome(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setChrome")?;
    unsafe { win32::window_set_chrome(binding, window, chrome) }
}

/// Set one window position.
pub(crate) unsafe fn destack_display_window_set_position(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setPosition")?;
    unsafe { win32::window_set_position(binding, window, position) }
}

/// Set window resizable state.
pub(crate) unsafe fn destack_display_window_set_resizable(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setResizable")?;
    unsafe { win32::window_set_resizable(binding, window, resizable) }
}

/// Set logical size constraints.
pub(crate) unsafe fn destack_display_window_set_size_constraints(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setSizeConstraints")?;
    unsafe { win32::window_set_size_constraints(binding, window, constraints) }
}

/// Set one logical window size.
pub(crate) unsafe fn destack_display_window_set_size_logical(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setSizeLogical")?;
    unsafe { win32::window_set_size_logical(binding, window, size) }
}

/// Set one physical window size.
pub(crate) unsafe fn destack_display_window_set_size_physical(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setSizePhysical")?;
    unsafe { win32::window_set_size_physical(binding, window, size) }
}

/// Set one window title string.
pub(crate) unsafe fn destack_display_window_set_title(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setTitle")?;
    unsafe { win32::window_set_title(binding, window, title) }
}

/// Set one window icon set.
pub(crate) unsafe fn destack_display_window_set_icons(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setIcons")?;
    unsafe { win32::window_set_icons(binding, window, icons) }
}

/// Set one window modal state.
pub(crate) unsafe fn destack_display_window_set_modal(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setModal")?;
    unsafe { win32::window_set_modal(binding, window, modal) }
}

/// Set one window mouse passthrough state.
pub(crate) unsafe fn destack_display_window_set_mouse_passthrough(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setMousePassthrough")?;
    unsafe { win32::window_set_mouse_passthrough(binding, window, passthrough) }
}

/// Set one window opacity.
pub(crate) unsafe fn destack_display_window_set_opacity(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setOpacity")?;
    unsafe { win32::window_set_opacity(binding, window, opacity) }
}

/// Read one window opacity.
pub(crate) unsafe fn destack_display_window_opacity(
    binding: &BindingCallContext,
    out: *mut f64,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.opacity")?;
    unsafe { win32::window_opacity(binding, out, window) }
}

/// Focus one window.
pub(crate) unsafe fn destack_display_window_focus(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.focus")?;
    unsafe { win32::window_focus(binding, window) }
}

/// Raise one window.
pub(crate) unsafe fn destack_display_window_raise(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.raise")?;
    unsafe { win32::window_raise(binding, window) }
}

/// Minimize one window.
pub(crate) unsafe fn destack_display_window_minimize(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.minimize")?;
    unsafe { win32::window_minimize(binding, window) }
}

/// Maximize one window.
pub(crate) unsafe fn destack_display_window_maximize(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.maximize")?;
    unsafe { win32::window_maximize(binding, window) }
}

/// Restore one window.
pub(crate) unsafe fn destack_display_window_restore(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.restore")?;
    unsafe { win32::window_restore(binding, window) }
}

/// Set one window parent relationship.
pub(crate) unsafe fn destack_display_window_set_parent(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    parent: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setParent")?;
    unsafe { win32::window_set_parent(binding, window, parent) }
}

/// Set one window transient relationship.
pub(crate) unsafe fn destack_display_window_set_transient_for(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    transientfor: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setTransientFor")?;
    unsafe { win32::window_set_transient_for(binding, window, transientfor) }
}

/// Set one window taskbar visibility state.
pub(crate) unsafe fn destack_display_window_set_taskbar_visible(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setTaskbarVisible")?;
    unsafe { win32::window_set_taskbar_visible(binding, window, visible) }
}

/// Begin one native window move drag.
pub(crate) unsafe fn destack_display_window_begin_move_drag(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.beginMoveDrag")?;
    unsafe { win32::window_begin_move_drag(binding, window) }
}

/// Begin one native window resize drag.
pub(crate) unsafe fn destack_display_window_begin_resize_drag(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    edge: WindowResizeEdge,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.beginResizeDrag")?;
    unsafe { win32::window_begin_resize_drag(binding, window, edge) }
}

/// Set one window visibility state.
pub(crate) unsafe fn destack_display_window_set_visibility(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setVisibility")?;
    unsafe { win32::window_set_visibility(binding, window, visibility) }
}

/// Read one capability mask for one opened window backend.
pub(crate) unsafe fn destack_display_window_capabilities(
    context: &BindingCallContext,
    out: *mut DisplayBackendCapabilityFlags,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let operation = "destack.display.window.capabilities";

    // validate that this runtime window handle resolves to one win32 window binding
    win32::ensure_window_binding_exists(context, window, operation)?;

    let _backend = core::resolve_default_backend(operation)?;
    unsafe {
        *out = core::backend_capabilities(DisplayBackend::Win32);
    }

    Ok(())
}

/// Read one window state snapshot.
pub(crate) unsafe fn destack_display_window_state(
    binding: &BindingCallContext,
    out: *mut WindowState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.state")?;
    unsafe { win32::window_state(binding, out, window) }
}

/// Close one global window-event stream.
pub(crate) unsafe fn destack_display_window_event_close(
    binding: &BindingCallContext,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.eventClose")?;
    unsafe { win32::window_event_close(binding, handle) }
}

/// Open one global window-event stream.
pub(crate) unsafe fn destack_display_window_event_open(
    binding: &BindingCallContext,
    out: *mut resource::WindowEventHandle,
    options: WindowEventOpenOptions,
) -> RuntimeResult<()> {
    let _backend = core::resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.display.window.eventOpen",
    )?;
    unsafe { win32::window_event_open(binding, out, options) }
}

/// Wait for one window event.
pub(crate) unsafe fn destack_display_window_event_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.eventRead")?;
    unsafe { win32::window_event_read(binding, out, handle, timeoutns) }
}

/// Wait for one batch of window events.
pub(crate) unsafe fn destack_display_window_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.eventReadBatch")?;
    unsafe { win32::window_event_read_batch(binding, out, handle, maxevents, timeoutns) }
}

/// Poll one window event without blocking.
pub(crate) unsafe fn destack_display_window_event_try_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.eventTryRead")?;
    unsafe { win32::window_event_try_read(binding, out, handle) }
}

/// Poll one batch of window events without blocking.
pub(crate) unsafe fn destack_display_window_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.eventTryReadBatch")?;
    unsafe { win32::window_event_try_read_batch(binding, out, handle, maxevents) }
}
