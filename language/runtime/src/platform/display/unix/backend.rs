#[cfg(target_os = "android")]
use super::android;
#[cfg(target_os = "macos")]
use super::appkit;
use super::core;
#[cfg(target_os = "ios")]
use super::ios;
#[cfg(target_os = "linux")]
use super::wayland;
#[cfg(target_os = "linux")]
use super::x11;
use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayBackendDescriptor, DisplayColorState,
    DisplayDescriptor, DisplayGammaRamp, DisplayHdrMode, DisplayMode, DisplayMonitorEvent,
    DisplayMonitorEventOpenOptions, DisplayMonitorListRequest, DisplayMonitorOpenOptions,
    WindowAspectRatio, WindowAttentionLevel, WindowChromeKind, WindowCursorIcon, WindowCursorMode,
    WindowDescriptor, WindowEvent, WindowEventOpenOptions, WindowIconSet, WindowLogicalSize,
    WindowModeOptions, WindowOptions, WindowPhysicalSize, WindowPosition, WindowResizeEdge,
    WindowSizeConstraints, WindowState, WindowVisibility,
};
use crate::platform::{NativeArray, core as core_platform, resource};
use crate::runtime::BindingCallContext;

macro_rules! dispatch_backend {
    ($backend:expr, $operation:literal, $function:ident($binding:expr $(, $arg:expr)* $(,)?)) => {{
        match $backend {
            #[cfg(target_os = "linux")]
            DisplayBackend::Wayland => unsafe { wayland::$function($binding $(, $arg)*) },
            #[cfg(target_os = "linux")]
            DisplayBackend::X11 => unsafe { x11::$function($binding $(, $arg)*) },
            #[cfg(target_os = "macos")]
            DisplayBackend::AppKit => unsafe { appkit::$function($binding $(, $arg)*) },
            #[cfg(target_os = "android")]
            DisplayBackend::Android => unsafe { android::$function($binding $(, $arg)*) },
            #[cfg(target_os = "ios")]
            DisplayBackend::UIKit => unsafe { ios::$function($binding $(, $arg)*) },
            _ => Err(core_platform::backend_support_error(
                $operation,
                core::backend_name($backend),
                core::backend_support($backend),
            )),
        }
    }};
}

/// List unix display backend descriptors for the active host.
pub(crate) fn display_backend_descriptors(
    binding: &BindingCallContext,
) -> Vec<DisplayBackendDescriptor> {
    core::backend_descriptors(binding)
}

/// Resolve one owning backend for one opened display handle.
fn resolve_display_backend_by_handle(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<DisplayBackend> {
    #[cfg(target_os = "linux")]
    {
        // resolve wayland-owned display handles first
        if wayland::ensure_display_handle_exists(binding, handle, operation).is_ok() {
            return Ok(DisplayBackend::Wayland);
        }

        // resolve x11-owned display handles next
        if x11::ensure_display_handle_exists(binding, handle, operation).is_ok() {
            return Ok(DisplayBackend::X11);
        }

        Err(core_platform::io_not_found(
            operation,
            format!("display handle {} was not found", handle.0.local_id),
        ))
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = binding;
        let _ = handle;
        core::resolve_default_backend(operation)
    }
}

/// Resolve one owning backend for one opened window handle.
fn resolve_window_backend_by_handle(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<DisplayBackend> {
    #[cfg(target_os = "linux")]
    {
        // resolve wayland-owned window handles first
        if wayland::ensure_window_handle_exists(binding, window, operation).is_ok() {
            return Ok(DisplayBackend::Wayland);
        }

        // resolve x11-owned window handles next
        if x11::ensure_window_handle_exists(binding, window, operation).is_ok() {
            return Ok(DisplayBackend::X11);
        }

        Err(core_platform::io_not_found(
            operation,
            format!("window handle {} was not found", window.0.local_id),
        ))
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = binding;
        let _ = window;
        core::resolve_default_backend(operation)
    }
}

/// Resolve one owning backend for one opened monitor-event stream.
fn resolve_monitor_event_backend_by_handle(
    binding: &BindingCallContext,
    handle: resource::DisplayEventHandle,
    operation: &'static str,
) -> RuntimeResult<DisplayBackend> {
    #[cfg(target_os = "linux")]
    {
        // resolve wayland-owned monitor-event streams first
        if wayland::ensure_monitor_event_handle_exists(binding, handle, operation).is_ok() {
            return Ok(DisplayBackend::Wayland);
        }

        // resolve x11-owned monitor-event streams next
        if x11::ensure_monitor_event_handle_exists(binding, handle, operation).is_ok() {
            return Ok(DisplayBackend::X11);
        }

        Err(core_platform::io_not_found(
            operation,
            format!("display event handle {} was not found", handle.0.local_id),
        ))
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = binding;
        let _ = handle;
        core::resolve_default_backend(operation)
    }
}

/// Resolve one owning backend for one opened window-event stream.
fn resolve_window_event_backend_by_handle(
    binding: &BindingCallContext,
    handle: resource::WindowEventHandle,
    operation: &'static str,
) -> RuntimeResult<DisplayBackend> {
    #[cfg(target_os = "linux")]
    {
        // resolve wayland-owned window-event streams first
        if wayland::ensure_window_event_handle_exists(binding, handle, operation).is_ok() {
            return Ok(DisplayBackend::Wayland);
        }

        // resolve x11-owned window-event streams next
        if x11::ensure_window_event_handle_exists(binding, handle, operation).is_ok() {
            return Ok(DisplayBackend::X11);
        }

        Err(core_platform::io_not_found(
            operation,
            format!("window event handle {} was not found", handle.0.local_id),
        ))
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = binding;
        let _ = handle;
        core::resolve_default_backend(operation)
    }
}

/// Resolve one capability mask for one backend-owned window handle.
fn resolve_window_capabilities(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    backend: DisplayBackend,
    _operation: &'static str,
) -> RuntimeResult<DisplayBackendCapabilityFlags> {
    #[cfg(target_os = "linux")]
    {
        // wayland window capabilities depend on live protocol negotiation
        if backend == DisplayBackend::Wayland {
            return wayland::effective_window_capabilities(binding, window, _operation);
        }

        Ok(core::backend_capabilities(binding, backend))
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = window;
        Ok(core::backend_capabilities(binding, backend))
    }
}

/// Close one display endpoint.
pub(crate) unsafe fn destack_display_monitor_close(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_display_backend_by_handle(binding, handle, "destack.display.monitor.close")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.close",
        monitor_close(binding, handle)
    )
}

/// Resolve one requested mode to the closest supported mode.
pub(crate) unsafe fn destack_display_monitor_closest_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
    requested: DisplayMode,
) -> RuntimeResult<()> {
    let backend =
        resolve_display_backend_by_handle(binding, handle, "destack.display.monitor.closestMode")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.closestMode",
        monitor_closest_mode(binding, out, handle, requested)
    )
}

/// Read the current mode for one opened display.
pub(crate) unsafe fn destack_display_monitor_current_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_display_backend_by_handle(binding, handle, "destack.display.monitor.currentMode")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.currentMode",
        monitor_current_mode(binding, out, handle)
    )
}

/// Read descriptor metadata for one opened display.
pub(crate) unsafe fn destack_display_monitor_descriptor(
    binding: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_display_backend_by_handle(binding, handle, "destack.display.monitor.descriptor")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.descriptor",
        monitor_descriptor(binding, out, handle)
    )
}

/// Read the desktop-preferred mode for one opened display.
pub(crate) unsafe fn destack_display_monitor_desktop_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_display_backend_by_handle(binding, handle, "destack.display.monitor.desktopMode")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.desktopMode",
        monitor_desktop_mode(binding, out, handle)
    )
}

/// List available displays.
pub(crate) unsafe fn destack_display_monitor_list(
    binding: &BindingCallContext,
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
        monitor_list(binding, out, request)
    )
}

/// Read available display modes.
pub(crate) unsafe fn destack_display_monitor_modes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_display_backend_by_handle(binding, handle, "destack.display.monitor.modes")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.modes",
        monitor_modes(binding, out, handle)
    )
}

/// Open one display endpoint.
pub(crate) unsafe fn destack_display_monitor_open(
    binding: &BindingCallContext,
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
        monitor_open(binding, out, id, options)
    )
}

/// Read the current primary display handle.
pub(crate) unsafe fn destack_display_monitor_primary(
    binding: &BindingCallContext,
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
        monitor_primary(binding, out, request)
    )
}

/// Apply one display mode.
pub(crate) unsafe fn destack_display_monitor_set_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    let backend =
        resolve_display_backend_by_handle(binding, handle, "destack.display.monitor.setMode")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.setMode",
        monitor_set_mode(binding, handle, mode)
    )
}

/// Read display color state.
pub(crate) unsafe fn destack_display_monitor_color_state(
    binding: &BindingCallContext,
    out: *mut DisplayColorState,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_display_backend_by_handle(binding, handle, "destack.display.monitor.colorState")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.colorState",
        monitor_color_state(binding, out, handle)
    )
}

/// Read display HDR mode.
pub(crate) unsafe fn destack_display_monitor_hdr_mode(
    binding: &BindingCallContext,
    out: *mut DisplayHdrMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_display_backend_by_handle(binding, handle, "destack.display.monitor.hdrMode")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.hdrMode",
        monitor_hdr_mode(binding, out, handle)
    )
}

/// Set display HDR mode.
pub(crate) unsafe fn destack_display_monitor_set_hdr_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayHdrMode,
) -> RuntimeResult<()> {
    let backend =
        resolve_display_backend_by_handle(binding, handle, "destack.display.monitor.setHdrMode")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.setHdrMode",
        monitor_set_hdr_mode(binding, handle, mode)
    )
}

/// Read display gamma ramp.
pub(crate) unsafe fn destack_display_monitor_gamma_ramp(
    binding: &BindingCallContext,
    out: *mut DisplayGammaRamp,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_display_backend_by_handle(binding, handle, "destack.display.monitor.gammaRamp")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.gammaRamp",
        monitor_gamma_ramp(binding, out, handle)
    )
}

/// Set display gamma ramp.
pub(crate) unsafe fn destack_display_monitor_set_gamma_ramp(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRamp,
) -> RuntimeResult<()> {
    let backend =
        resolve_display_backend_by_handle(binding, handle, "destack.display.monitor.setGammaRamp")?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.setGammaRamp",
        monitor_set_gamma_ramp(binding, handle, ramp)
    )
}

/// Close one global monitor-event stream.
pub(crate) unsafe fn destack_display_monitor_event_close(
    binding: &BindingCallContext,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    let backend = resolve_monitor_event_backend_by_handle(
        binding,
        handle,
        "destack.display.monitor.eventClose",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.eventClose",
        monitor_event_close(binding, handle)
    )
}

/// Open one global monitor-event stream.
pub(crate) unsafe fn destack_display_monitor_event_open(
    binding: &BindingCallContext,
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
        monitor_event_open(binding, out, options)
    )
}

/// Wait for one monitor event.
pub(crate) unsafe fn destack_display_monitor_event_read(
    binding: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<()> {
    let backend = resolve_monitor_event_backend_by_handle(
        binding,
        handle,
        "destack.display.monitor.eventRead",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.eventRead",
        monitor_event_read(binding, out, handle, timeout_ns)
    )
}

/// Wait for one batch of monitor events.
pub(crate) unsafe fn destack_display_monitor_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    max_events: u32,
    timeout_ns: u64,
) -> RuntimeResult<()> {
    let backend = resolve_monitor_event_backend_by_handle(
        binding,
        handle,
        "destack.display.monitor.eventReadBatch",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.eventReadBatch",
        monitor_event_read_batch(binding, out, handle, max_events, timeout_ns)
    )
}

/// Poll one monitor event without blocking.
pub(crate) unsafe fn destack_display_monitor_event_try_read(
    binding: &BindingCallContext,
    out: *mut DisplayMonitorEvent,
    handle: resource::DisplayEventHandle,
) -> RuntimeResult<()> {
    let backend = resolve_monitor_event_backend_by_handle(
        binding,
        handle,
        "destack.display.monitor.eventTryRead",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.eventTryRead",
        monitor_event_try_read(binding, out, handle)
    )
}

/// Poll one batch of monitor events without blocking.
pub(crate) unsafe fn destack_display_monitor_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<DisplayMonitorEvent>,
    handle: resource::DisplayEventHandle,
    max_events: u32,
) -> RuntimeResult<()> {
    let backend = resolve_monitor_event_backend_by_handle(
        binding,
        handle,
        "destack.display.monitor.eventTryReadBatch",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.monitor.eventTryReadBatch",
        monitor_event_try_read_batch(binding, out, handle, max_events)
    )
}

/// Close one window.
pub(crate) unsafe fn destack_display_window_close(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.close")?;
    dispatch_backend!(
        backend,
        "destack.display.window.close",
        window_close(binding, window)
    )
}

/// Read descriptor metadata for one window.
pub(crate) unsafe fn destack_display_window_descriptor(
    binding: &BindingCallContext,
    out: *mut WindowDescriptor,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.descriptor")?;
    dispatch_backend!(
        backend,
        "destack.display.window.descriptor",
        window_descriptor(binding, out, window)
    )
}

/// Open one window.
pub(crate) unsafe fn destack_display_window_open(
    binding: &BindingCallContext,
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
        window_open(binding, out, options)
    )
}

/// Request user attention for one window.
pub(crate) unsafe fn destack_display_window_request_attention(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    let backend = resolve_window_backend_by_handle(
        binding,
        window,
        "destack.display.window.requestAttention",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.requestAttention",
        window_request_attention(binding, window, level)
    )
}

/// Request one redraw for one window.
pub(crate) unsafe fn destack_display_window_request_refresh(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.requestRefresh")?;
    dispatch_backend!(
        backend,
        "destack.display.window.requestRefresh",
        window_request_refresh(binding, window)
    )
}

/// Set always-on-top state.
pub(crate) unsafe fn destack_display_window_set_always_on_top(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setAlwaysOnTop")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setAlwaysOnTop",
        window_set_always_on_top(binding, window, alwaysontop)
    )
}

/// Set cursor icon for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_icon(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setCursorIcon")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setCursorIcon",
        window_set_cursor_icon(binding, window, icon)
    )
}

/// Set cursor interaction mode for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_mode(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setCursorMode")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setCursorMode",
        window_set_cursor_mode(binding, window, mode)
    )
}

/// Set cursor position for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_position(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let backend = resolve_window_backend_by_handle(
        binding,
        window,
        "destack.display.window.setCursorPosition",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.setCursorPosition",
        window_set_cursor_position(binding, window, position)
    )
}

/// Set cursor visibility for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_visible(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let backend = resolve_window_backend_by_handle(
        binding,
        window,
        "destack.display.window.setCursorVisible",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.setCursorVisible",
        window_set_cursor_visible(binding, window, visible)
    )
}

/// Set window decoration state.
pub(crate) unsafe fn destack_display_window_set_decorated(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setDecorated")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setDecorated",
        window_set_decorated(binding, window, decorated)
    )
}

/// Set one window mode.
pub(crate) unsafe fn destack_display_window_set_mode(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setMode")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setMode",
        window_set_mode(binding, window, mode)
    )
}

/// Set one window aspect-ratio lock.
pub(crate) unsafe fn destack_display_window_set_aspect_ratio(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    aspectratio: Option<WindowAspectRatio>,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setAspectRatio")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setAspectRatio",
        window_set_aspect_ratio(binding, window, aspectratio)
    )
}

/// Set one window chrome kind.
pub(crate) unsafe fn destack_display_window_set_chrome(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setChrome")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setChrome",
        window_set_chrome(binding, window, chrome)
    )
}

/// Set one window position.
pub(crate) unsafe fn destack_display_window_set_position(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setPosition")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setPosition",
        window_set_position(binding, window, position)
    )
}

/// Set window resizable state.
pub(crate) unsafe fn destack_display_window_set_resizable(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setResizable")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setResizable",
        window_set_resizable(binding, window, resizable)
    )
}

/// Set logical size constraints.
pub(crate) unsafe fn destack_display_window_set_size_constraints(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    let backend = resolve_window_backend_by_handle(
        binding,
        window,
        "destack.display.window.setSizeConstraints",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.setSizeConstraints",
        window_set_size_constraints(binding, window, constraints)
    )
}

/// Set one logical window size.
pub(crate) unsafe fn destack_display_window_set_size_logical(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setSizeLogical")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setSizeLogical",
        window_set_size_logical(binding, window, size)
    )
}

/// Set one physical window size.
pub(crate) unsafe fn destack_display_window_set_size_physical(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    let backend = resolve_window_backend_by_handle(
        binding,
        window,
        "destack.display.window.setSizePhysical",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.setSizePhysical",
        window_set_size_physical(binding, window, size)
    )
}

/// Set one window title string.
pub(crate) unsafe fn destack_display_window_set_title(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setTitle")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setTitle",
        window_set_title(binding, window, title)
    )
}

/// Set one window icon set.
pub(crate) unsafe fn destack_display_window_set_icons(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setIcons")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setIcons",
        window_set_icons(binding, window, icons)
    )
}

/// Set one window modal state.
pub(crate) unsafe fn destack_display_window_set_modal(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setModal")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setModal",
        window_set_modal(binding, window, modal)
    )
}

/// Set one window mouse passthrough state.
pub(crate) unsafe fn destack_display_window_set_mouse_passthrough(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    let backend = resolve_window_backend_by_handle(
        binding,
        window,
        "destack.display.window.setMousePassthrough",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.setMousePassthrough",
        window_set_mouse_passthrough(binding, window, passthrough)
    )
}

/// Set one window opacity.
pub(crate) unsafe fn destack_display_window_set_opacity(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setOpacity")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setOpacity",
        window_set_opacity(binding, window, opacity)
    )
}

/// Read one window opacity.
pub(crate) unsafe fn destack_display_window_opacity(
    binding: &BindingCallContext,
    out: *mut f64,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.opacity")?;
    dispatch_backend!(
        backend,
        "destack.display.window.opacity",
        window_opacity(binding, out, window)
    )
}

/// Focus one window.
pub(crate) unsafe fn destack_display_window_focus(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.focus")?;
    dispatch_backend!(
        backend,
        "destack.display.window.focus",
        window_focus(binding, window)
    )
}

/// Raise one window.
pub(crate) unsafe fn destack_display_window_raise(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.raise")?;
    dispatch_backend!(
        backend,
        "destack.display.window.raise",
        window_raise(binding, window)
    )
}

/// Minimize one window.
pub(crate) unsafe fn destack_display_window_minimize(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.minimize")?;
    dispatch_backend!(
        backend,
        "destack.display.window.minimize",
        window_minimize(binding, window)
    )
}

/// Maximize one window.
pub(crate) unsafe fn destack_display_window_maximize(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.maximize")?;
    dispatch_backend!(
        backend,
        "destack.display.window.maximize",
        window_maximize(binding, window)
    )
}

/// Restore one window.
pub(crate) unsafe fn destack_display_window_restore(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.restore")?;
    dispatch_backend!(
        backend,
        "destack.display.window.restore",
        window_restore(binding, window)
    )
}

/// Set one window parent relationship.
pub(crate) unsafe fn destack_display_window_set_parent(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    parent: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setParent")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setParent",
        window_set_parent(binding, window, parent)
    )
}

/// Set one window transient relationship.
pub(crate) unsafe fn destack_display_window_set_transient_for(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    transientfor: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    let backend = resolve_window_backend_by_handle(
        binding,
        window,
        "destack.display.window.setTransientFor",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.setTransientFor",
        window_set_transient_for(binding, window, transientfor)
    )
}

/// Set one window taskbar visibility state.
pub(crate) unsafe fn destack_display_window_set_taskbar_visible(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    let backend = resolve_window_backend_by_handle(
        binding,
        window,
        "destack.display.window.setTaskbarVisible",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.setTaskbarVisible",
        window_set_taskbar_visible(binding, window, visible)
    )
}

/// Begin one native window move drag.
pub(crate) unsafe fn destack_display_window_begin_move_drag(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.beginMoveDrag")?;
    dispatch_backend!(
        backend,
        "destack.display.window.beginMoveDrag",
        window_begin_move_drag(binding, window)
    )
}

/// Begin one native window resize drag.
pub(crate) unsafe fn destack_display_window_begin_resize_drag(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    edge: WindowResizeEdge,
) -> RuntimeResult<()> {
    let backend = resolve_window_backend_by_handle(
        binding,
        window,
        "destack.display.window.beginResizeDrag",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.beginResizeDrag",
        window_begin_resize_drag(binding, window, edge)
    )
}

/// Set one window visibility state.
pub(crate) unsafe fn destack_display_window_set_visibility(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.setVisibility")?;
    dispatch_backend!(
        backend,
        "destack.display.window.setVisibility",
        window_set_visibility(binding, window, visibility)
    )
}

/// Read one capability mask for one opened window backend.
pub(crate) unsafe fn destack_display_window_capabilities(
    binding: &BindingCallContext,
    out: *mut DisplayBackendCapabilityFlags,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let operation = "destack.display.window.capabilities";
    #[cfg(target_os = "linux")]
    let backend = resolve_window_backend_by_handle(binding, window, operation)?;

    #[cfg(not(target_os = "linux"))]
    let backend = core::resolve_default_backend(operation)?;

    let capabilities = resolve_window_capabilities(binding, window, backend, operation)?;

    unsafe {
        *out = capabilities;
    }

    Ok(())
}

/// Read one window state snapshot.
pub(crate) unsafe fn destack_display_window_state(
    binding: &BindingCallContext,
    out: *mut WindowState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    let backend =
        resolve_window_backend_by_handle(binding, window, "destack.display.window.state")?;
    dispatch_backend!(
        backend,
        "destack.display.window.state",
        window_state(binding, out, window)
    )
}

/// Close one global window-event stream.
pub(crate) unsafe fn destack_display_window_event_close(
    binding: &BindingCallContext,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    let backend = resolve_window_event_backend_by_handle(
        binding,
        handle,
        "destack.display.window.eventClose",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.eventClose",
        window_event_close(binding, handle)
    )
}

/// Open one global window-event stream.
pub(crate) unsafe fn destack_display_window_event_open(
    binding: &BindingCallContext,
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
        window_event_open(binding, out, options)
    )
}

/// Wait for one window event.
pub(crate) unsafe fn destack_display_window_event_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let backend = resolve_window_event_backend_by_handle(
        binding,
        handle,
        "destack.display.window.eventRead",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.eventRead",
        window_event_read(binding, out, handle, timeoutns)
    )
}

/// Wait for one batch of window events.
pub(crate) unsafe fn destack_display_window_event_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    let backend = resolve_window_event_backend_by_handle(
        binding,
        handle,
        "destack.display.window.eventReadBatch",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.eventReadBatch",
        window_event_read_batch(binding, out, handle, maxevents, timeoutns)
    )
}

/// Poll one window event without blocking.
pub(crate) unsafe fn destack_display_window_event_try_read(
    binding: &BindingCallContext,
    out: *mut WindowEvent,
    handle: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    let backend = resolve_window_event_backend_by_handle(
        binding,
        handle,
        "destack.display.window.eventTryRead",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.eventTryRead",
        window_event_try_read(binding, out, handle)
    )
}

/// Poll one batch of window events without blocking.
pub(crate) unsafe fn destack_display_window_event_try_read_batch(
    binding: &BindingCallContext,
    out: *mut NativeArray<WindowEvent>,
    handle: resource::WindowEventHandle,
    maxevents: u32,
) -> RuntimeResult<()> {
    let backend = resolve_window_event_backend_by_handle(
        binding,
        handle,
        "destack.display.window.eventTryReadBatch",
    )?;
    dispatch_backend!(
        backend,
        "destack.display.window.eventTryReadBatch",
        window_event_try_read_batch(binding, out, handle, maxevents)
    )
}
