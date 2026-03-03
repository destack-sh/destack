#[cfg(target_os = "android")]
use super::android;
use super::core;
#[cfg(target_os = "ios")]
use super::ios;
#[cfg(target_os = "macos")]
use super::macos;
#[cfg(target_os = "linux")]
use super::wayland;
#[cfg(target_os = "linux")]
use super::x11;
use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayBackend, DisplayColorState, DisplayDescriptor, DisplayGammaRamp, DisplayHdrMode,
    DisplayMode, DisplayMonitorEvent, DisplayMonitorEventOpenOptions, DisplayMonitorListRequest,
    DisplayMonitorOpenOptions, WindowAspectRatio, WindowAttentionLevel, WindowChromeKind,
    WindowCursorIcon, WindowCursorMode, WindowDescriptor, WindowEvent, WindowEventOpenOptions,
    WindowIconSet, WindowLogicalSize, WindowModeOptions, WindowOptions, WindowPhysicalSize,
    WindowPosition, WindowResizeEdge, WindowSizeConstraints, WindowState, WindowVisibility,
    unsupported as display_unsupported,
};
use crate::platform::{NativeArray, resource};
use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef};

macro_rules! dispatch_backend {
    ($backend:expr, $operation:literal, $function:ident($context:expr $(, $arg:expr)* $(,)?)) => {{
        match $backend {
            #[cfg(target_os = "linux")]
            DisplayBackend::Wayland => unsafe { wayland::$function($context $(, $arg)*) },
            #[cfg(target_os = "linux")]
            DisplayBackend::X11 => unsafe { x11::$function($context $(, $arg)*) },
            #[cfg(target_os = "macos")]
            DisplayBackend::AppKit => unsafe { macos::$function($context $(, $arg)*) },
            #[cfg(target_os = "android")]
            DisplayBackend::Android => unsafe { android::$function($context $(, $arg)*) },
            #[cfg(target_os = "ios")]
            DisplayBackend::UIKit => unsafe { ios::$function($context $(, $arg)*) },
            _ => Err(core::backend_not_supported($operation, $backend)),
        }
    }};
}

/// Close one display endpoint.
pub(crate) unsafe fn destack_display_monitor_close(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.monitor.close")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.close",
        monitor_close(context, handle)
    )
}

/// Resolve one requested mode to the closest supported mode.
pub(crate) unsafe fn destack_display_monitor_closest_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
    requested: DisplayMode,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.monitor.closestMode")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.closestMode",
        monitor_closest_mode(context, out, handle, requested)
    )
}

/// Read the current mode for one opened display.
pub(crate) unsafe fn destack_display_monitor_current_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.monitor.currentMode")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.currentMode",
        monitor_current_mode(context, out, handle)
    )
}

/// Read descriptor metadata for one opened display.
pub(crate) unsafe fn destack_display_monitor_descriptor(
    context: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.monitor.descriptor")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.descriptor",
        monitor_descriptor(context, out, handle)
    )
}

/// Read the desktop-preferred mode for one opened display.
pub(crate) unsafe fn destack_display_monitor_desktop_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.monitor.desktopMode")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.desktopMode",
        monitor_desktop_mode(context, out, handle)
    )
}

/// List available displays.
pub(crate) unsafe fn destack_display_monitor_list(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    let backend = core::resolve_backend(
        request.backend,
        request.backend_policy,
        "destack.display.monitor.list",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.list",
        monitor_list(context, out, request)
    )
}

/// Read available display modes.
pub(crate) unsafe fn destack_display_monitor_modes(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.monitor.modes")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.modes",
        monitor_modes(context, out, handle)
    )
}

/// Open one display endpoint.
pub(crate) unsafe fn destack_display_monitor_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
    options: DisplayMonitorOpenOptions,
) -> RuntimeResult<()> {
    let backend = core::resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.display.monitor.open",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.open",
        monitor_open(context, out, id, options)
    )
}

/// Read the current primary display handle.
pub(crate) unsafe fn destack_display_monitor_primary(
    context: &BindingCallContext,
    out: *mut Option<resource::DisplayHandle>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    let backend = core::resolve_backend(
        request.backend,
        request.backend_policy,
        "destack.display.monitor.primary",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.primary",
        monitor_primary(context, out, request)
    )
}

/// Apply one display mode.
pub(crate) unsafe fn destack_display_monitor_set_mode(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.monitor.setMode")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.setMode",
        monitor_set_mode(context, handle, mode)
    )
}

/// Read display color state.
pub(crate) unsafe fn destack_display_monitor_color_state(
    context: &BindingCallContext,
    out: *mut DisplayColorState,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.colorState")?;
    unsafe { display_unsupported::destack_display_monitor_color_state(context, out, handle) }
}

/// Read display HDR mode.
pub(crate) unsafe fn destack_display_monitor_hdr_mode(
    context: &BindingCallContext,
    out: *mut DisplayHdrMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.hdrMode")?;
    unsafe { display_unsupported::destack_display_monitor_hdr_mode(context, out, handle) }
}

/// Set display HDR mode.
pub(crate) unsafe fn destack_display_monitor_set_hdr_mode(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayHdrMode,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.setHdrMode")?;
    unsafe { display_unsupported::destack_display_monitor_set_hdr_mode(context, handle, mode) }
}

/// Read display gamma ramp.
pub(crate) unsafe fn destack_display_monitor_gamma_ramp(
    context: &BindingCallContext,
    out: *mut DisplayGammaRamp,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.gammaRamp")?;
    unsafe { display_unsupported::destack_display_monitor_gamma_ramp(context, out, handle) }
}

/// Set display gamma ramp.
pub(crate) unsafe fn destack_display_monitor_set_gamma_ramp(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRamp,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.monitor.setGammaRamp")?;
    unsafe { display_unsupported::destack_display_monitor_set_gamma_ramp(context, handle, ramp) }
}

/// Close one global monitor-event stream.
pub(crate) unsafe fn destack_display_monitor_event_close(
    context: &BindingCallContext,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.monitor.eventClose")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.eventClose",
        monitor_event_close(context, handle)
    )
}

/// Open one global monitor-event stream.
pub(crate) unsafe fn destack_display_monitor_event_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayEventHandle,
    options: DisplayMonitorEventOpenOptions,
) -> RuntimeResult<()> {
    let backend = core::resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.display.monitor.eventOpen",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.eventOpen",
        monitor_event_open(context, out, options)
    )
}

/// Wait for one monitor event.
pub(crate) unsafe fn destack_display_monitor_event_read(
    context: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.monitor.eventRead")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.eventRead",
        monitor_event_read(context, out, handle, timeout_ns)
    )
}

/// Wait for one batch of monitor events.
pub(crate) unsafe fn destack_display_monitor_event_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.monitor.eventReadBatch")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.eventReadBatch",
        monitor_event_read_batch(context, out, handle, max_events, timeout_ns)
    )
}

/// Poll one monitor event without blocking.
pub(crate) unsafe fn destack_display_monitor_event_try_read(
    context: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.monitor.eventTryRead")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.eventTryRead",
        monitor_event_try_read(context, out, handle)
    )
}

/// Poll one batch of monitor events without blocking.
pub(crate) unsafe fn destack_display_monitor_event_try_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    max_events: u32,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.monitor.eventTryReadBatch")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.eventTryReadBatch",
        monitor_event_try_read_batch(context, out, handle, max_events)
    )
}

/// Close one window.
pub(crate) unsafe fn destack_display_window_close(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.close")?;
    dispatch_backend!(
        backend,
        "destack.display.window.close",
        window_close(context, window)
    )
}

/// Read descriptor metadata for one window.
pub(crate) unsafe fn destack_display_window_descriptor(
    context: &BindingCallContext,
    out: *mut WindowDescriptor,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.descriptor")?;
    dispatch_backend!(
        backend,
        "destack.display.window.descriptor",
        window_descriptor(context, out, window)
    )
}

/// Open one window.
pub(crate) unsafe fn destack_display_window_open(
    context: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    let backend = core::resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.display.window.open",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.open",
        window_open(context, out, options)
    )
}

/// Request user attention for one window.
pub(crate) unsafe fn destack_display_window_request_attention(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.requestAttention")?;
    dispatch_backend!(
        backend,
        "destack.display.window.requestAttention",
        window_request_attention(context, window, level)
    )
}

/// Request one redraw for one window.
pub(crate) unsafe fn destack_display_window_request_refresh(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.requestRefresh")?;
    dispatch_backend!(
        backend,
        "destack.display.window.requestRefresh",
        window_request_refresh(context, window)
    )
}

/// Set always-on-top state.
pub(crate) unsafe fn destack_display_window_set_always_on_top(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setAlwaysOnTop")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setAlwaysOnTop",
        window_set_always_on_top(context, window, alwaysontop)
    )
}

/// Set cursor icon for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_icon(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setCursorIcon")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setCursorIcon",
        window_set_cursor_icon(context, window, icon)
    )
}

/// Set cursor interaction mode for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setCursorMode")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setCursorMode",
        window_set_cursor_mode(context, window, mode)
    )
}

/// Set cursor position for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setCursorPosition")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setCursorPosition",
        window_set_cursor_position(context, window, position)
    )
}

/// Set cursor visibility for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_visible(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setCursorVisible")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setCursorVisible",
        window_set_cursor_visible(context, window, visible)
    )
}

/// Set window decoration state.
pub(crate) unsafe fn destack_display_window_set_decorated(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setDecorated")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setDecorated",
        window_set_decorated(context, window, decorated)
    )
}

/// Set one window mode.
pub(crate) unsafe fn destack_display_window_set_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setMode")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setMode",
        window_set_mode(context, window, mode)
    )
}

/// Set one window aspect-ratio lock.
pub(crate) unsafe fn destack_display_window_set_aspect_ratio(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    aspectratio: Option<WindowAspectRatio>,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setAspectRatio")?;
    unsafe {
        display_unsupported::destack_display_window_set_aspect_ratio(context, window, aspectratio)
    }
}

/// Set one window chrome kind.
pub(crate) unsafe fn destack_display_window_set_chrome(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setChrome")?;
    unsafe { display_unsupported::destack_display_window_set_chrome(context, window, chrome) }
}

/// Set one window position.
pub(crate) unsafe fn destack_display_window_set_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setPosition")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setPosition",
        window_set_position(context, window, position)
    )
}

/// Set window resizable state.
pub(crate) unsafe fn destack_display_window_set_resizable(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setResizable")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setResizable",
        window_set_resizable(context, window, resizable)
    )
}

/// Set logical size constraints.
pub(crate) unsafe fn destack_display_window_set_size_constraints(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setSizeConstraints")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setSizeConstraints",
        window_set_size_constraints(context, window, constraints)
    )
}

/// Set one logical window size.
pub(crate) unsafe fn destack_display_window_set_size_logical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setSizeLogical")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setSizeLogical",
        window_set_size_logical(context, window, size)
    )
}

/// Set one physical window size.
pub(crate) unsafe fn destack_display_window_set_size_physical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setSizePhysical")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setSizePhysical",
        window_set_size_physical(context, window, size)
    )
}

/// Set one window title string.
pub(crate) unsafe fn destack_display_window_set_title(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setTitle")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setTitle",
        window_set_title(context, window, title)
    )
}

/// Set one window icon set.
pub(crate) unsafe fn destack_display_window_set_icons(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setIcons")?;
    unsafe { display_unsupported::destack_display_window_set_icons(context, window, icons) }
}

/// Set one window modal state.
pub(crate) unsafe fn destack_display_window_set_modal(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setModal")?;
    unsafe { display_unsupported::destack_display_window_set_modal(context, window, modal) }
}

/// Set one window mouse passthrough state.
pub(crate) unsafe fn destack_display_window_set_mouse_passthrough(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setMousePassthrough")?;
    unsafe {
        display_unsupported::destack_display_window_set_mouse_passthrough(
            context,
            window,
            passthrough,
        )
    }
}

/// Set one window opacity.
pub(crate) unsafe fn destack_display_window_set_opacity(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setOpacity")?;
    unsafe { display_unsupported::destack_display_window_set_opacity(context, window, opacity) }
}

/// Read one window opacity.
pub(crate) unsafe fn destack_display_window_opacity(
    context: &BindingCallContext,
    out: *mut f64,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.opacity")?;
    unsafe { display_unsupported::destack_display_window_opacity(context, out, window) }
}

/// Focus one window.
pub(crate) unsafe fn destack_display_window_focus(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.focus")?;
    unsafe { display_unsupported::destack_display_window_focus(context, window) }
}

/// Raise one window.
pub(crate) unsafe fn destack_display_window_raise(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.raise")?;
    unsafe { display_unsupported::destack_display_window_raise(context, window) }
}

/// Minimize one window.
pub(crate) unsafe fn destack_display_window_minimize(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.minimize")?;
    unsafe { display_unsupported::destack_display_window_minimize(context, window) }
}

/// Maximize one window.
pub(crate) unsafe fn destack_display_window_maximize(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.maximize")?;
    unsafe { display_unsupported::destack_display_window_maximize(context, window) }
}

/// Restore one window.
pub(crate) unsafe fn destack_display_window_restore(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.restore")?;
    unsafe { display_unsupported::destack_display_window_restore(context, window) }
}

/// Set one window parent relationship.
pub(crate) unsafe fn destack_display_window_set_parent(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    parent: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setParent")?;
    unsafe { display_unsupported::destack_display_window_set_parent(context, window, parent) }
}

/// Set one window transient relationship.
pub(crate) unsafe fn destack_display_window_set_transient_for(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    transientfor: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setTransientFor")?;
    unsafe {
        display_unsupported::destack_display_window_set_transient_for(context, window, transientfor)
    }
}

/// Set one window taskbar visibility state.
pub(crate) unsafe fn destack_display_window_set_taskbar_visible(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.setTaskbarVisible")?;
    unsafe {
        display_unsupported::destack_display_window_set_taskbar_visible(context, window, visible)
    }
}

/// Begin one native window move drag.
pub(crate) unsafe fn destack_display_window_begin_move_drag(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.beginMoveDrag")?;
    unsafe { display_unsupported::destack_display_window_begin_move_drag(context, window) }
}

/// Begin one native window resize drag.
pub(crate) unsafe fn destack_display_window_begin_resize_drag(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    edge: WindowResizeEdge,
) -> RuntimeResult<()> {
    let _backend = core::resolve_default_backend("destack.display.window.beginResizeDrag")?;
    unsafe { display_unsupported::destack_display_window_begin_resize_drag(context, window, edge) }
}

/// Set one window visibility state.
pub(crate) unsafe fn destack_display_window_set_visibility(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.setVisibility")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setVisibility",
        window_set_visibility(context, window, visibility)
    )
}

/// Read one window state snapshot.
pub(crate) unsafe fn destack_display_window_state(
    context: &BindingCallContext,
    out: *mut WindowState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.state")?;
    dispatch_backend!(
        backend,
        "destack.display.window.state",
        window_state(context, out, window)
    )
}

/// Close one global window-event stream.
pub(crate) unsafe fn destack_display_window_event_close(
    context: &BindingCallContext,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.eventClose")?;
    dispatch_backend!(
        backend,
        "destack.display.window.eventClose",
        window_event_close(context, handle)
    )
}

/// Open one global window-event stream.
pub(crate) unsafe fn destack_display_window_event_open(
    context: &BindingCallContext,
    out: *mut resource::WindowEventHandle,
    options: WindowEventOpenOptions,
) -> RuntimeResult<()> {
    let backend = core::resolve_backend(
        options.backend,
        options.backend_policy,
        "destack.display.window.eventOpen",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.eventOpen",
        window_event_open(context, out, options)
    )
}

/// Wait for one window event.
pub(crate) unsafe fn destack_display_window_event_read(
    context: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.eventRead")?;
    dispatch_backend!(
        backend,
        "destack.display.window.eventRead",
        window_event_read(context, out, handle, timeoutns)
    )
}

/// Wait for one batch of window events.
pub(crate) unsafe fn destack_display_window_event_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.eventReadBatch")?;
    dispatch_backend!(
        backend,
        "destack.display.window.eventReadBatch",
        window_event_read_batch(context, out, handle, maxevents, timeoutns)
    )
}

/// Poll one window event without blocking.
pub(crate) unsafe fn destack_display_window_event_try_read(
    context: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.eventTryRead")?;
    dispatch_backend!(
        backend,
        "destack.display.window.eventTryRead",
        window_event_try_read(context, out, handle)
    )
}

/// Poll one batch of window events without blocking.
pub(crate) unsafe fn destack_display_window_event_try_read_batch(
    context: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    let backend = core::resolve_default_backend("destack.display.window.eventTryReadBatch")?;
    dispatch_backend!(
        backend,
        "destack.display.window.eventTryReadBatch",
        window_event_try_read_batch(context, out, handle, maxevents)
    )
}
