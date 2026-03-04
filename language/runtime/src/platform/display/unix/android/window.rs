use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowAspectRatio, WindowAttentionLevel, WindowChromeKind, WindowCursorIcon, WindowCursorMode,
    WindowDescriptor, WindowIconSet, WindowLogicalSize, WindowModeOptions, WindowOptions,
    WindowPhysicalSize, WindowPosition, WindowResizeEdge, WindowSizeConstraints, WindowState,
    WindowVisibility, unsupported,
};
use crate::platform::resource;
use crate::runtime::{BindingCallContext, NativeStringRef};

/// Close one window.
pub(in crate::platform::display::host::unix) unsafe fn window_close(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_close(context, window) }
}

/// Read descriptor metadata for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_descriptor(
    context: &BindingCallContext,
    out: *mut WindowDescriptor,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_descriptor(context, out, window) }
}

/// Open one window.
pub(in crate::platform::display::host::unix) unsafe fn window_open(
    context: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_open(context, out, options) }
}

/// Request user attention for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_request_attention(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_request_attention(context, window, level) }
}

/// Request one redraw for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_request_refresh(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_request_refresh(context, window) }
}

/// Set always-on-top state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_always_on_top(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_always_on_top(context, window, alwaysontop) }
}

/// Set cursor icon for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_set_cursor_icon(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_cursor_icon(context, window, icon) }
}

/// Set cursor interaction mode for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_set_cursor_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_cursor_mode(context, window, mode) }
}

/// Set cursor position for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_set_cursor_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_cursor_position(context, window, position) }
}

/// Set cursor visibility for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_set_cursor_visible(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_cursor_visible(context, window, visible) }
}

/// Set window decoration state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_decorated(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_decorated(context, window, decorated) }
}

/// Set one window mode.
pub(in crate::platform::display::host::unix) unsafe fn window_set_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_mode(context, window, mode) }
}

/// Set one window position.
pub(in crate::platform::display::host::unix) unsafe fn window_set_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_position(context, window, position) }
}

/// Set window resizable state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_resizable(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_resizable(context, window, resizable) }
}

/// Set logical size constraints.
pub(in crate::platform::display::host::unix) unsafe fn window_set_size_constraints(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    unsafe {
        unsupported::destack_display_window_set_size_constraints(context, window, constraints)
    }
}

/// Set one logical window size.
pub(in crate::platform::display::host::unix) unsafe fn window_set_size_logical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_size_logical(context, window, size) }
}

/// Set one physical window size.
pub(in crate::platform::display::host::unix) unsafe fn window_set_size_physical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_size_physical(context, window, size) }
}

/// Set one window title string.
pub(in crate::platform::display::host::unix) unsafe fn window_set_title(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_title(context, window, title) }
}

/// Set one window visibility state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_visibility(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_visibility(context, window, visibility) }
}

/// Read one window state snapshot.
pub(in crate::platform::display::host::unix) unsafe fn window_state(
    context: &BindingCallContext,
    out: *mut WindowState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_state(context, out, window) }
}

/// Set one window aspect-ratio lock.
pub(in crate::platform::display::host::unix) unsafe fn window_set_aspect_ratio(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    aspectratio: Option<WindowAspectRatio>,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_aspect_ratio(context, window, aspectratio) }
}

/// Set one window chrome kind.
pub(in crate::platform::display::host::unix) unsafe fn window_set_chrome(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_chrome(context, window, chrome) }
}

/// Set one window icon set.
pub(in crate::platform::display::host::unix) unsafe fn window_set_icons(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_icons(context, window, icons) }
}

/// Set one window modal state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_modal(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_modal(context, window, modal) }
}

/// Set one window mouse passthrough state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_mouse_passthrough(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    unsafe {
        unsupported::destack_display_window_set_mouse_passthrough(context, window, passthrough)
    }
}

/// Set one window opacity.
pub(in crate::platform::display::host::unix) unsafe fn window_set_opacity(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_opacity(context, window, opacity) }
}

/// Read one window opacity.
pub(in crate::platform::display::host::unix) unsafe fn window_opacity(
    context: &BindingCallContext,
    out: *mut f64,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_opacity(context, out, window) }
}

/// Focus one window.
pub(in crate::platform::display::host::unix) unsafe fn window_focus(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_focus(context, window) }
}

/// Raise one window.
pub(in crate::platform::display::host::unix) unsafe fn window_raise(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_raise(context, window) }
}

/// Minimize one window.
pub(in crate::platform::display::host::unix) unsafe fn window_minimize(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_minimize(context, window) }
}

/// Maximize one window.
pub(in crate::platform::display::host::unix) unsafe fn window_maximize(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_maximize(context, window) }
}

/// Restore one window.
pub(in crate::platform::display::host::unix) unsafe fn window_restore(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_restore(context, window) }
}

/// Set one window parent relationship.
pub(in crate::platform::display::host::unix) unsafe fn window_set_parent(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    parent: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_parent(context, window, parent) }
}

/// Set one window transient relationship.
pub(in crate::platform::display::host::unix) unsafe fn window_set_transient_for(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    transientfor: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_transient_for(context, window, transientfor) }
}

/// Set one window taskbar visibility state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_taskbar_visible(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_taskbar_visible(context, window, visible) }
}

/// Begin one native window move drag.
pub(in crate::platform::display::host::unix) unsafe fn window_begin_move_drag(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_begin_move_drag(context, window) }
}

/// Begin one native window resize drag.
pub(in crate::platform::display::host::unix) unsafe fn window_begin_resize_drag(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    edge: WindowResizeEdge,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_begin_resize_drag(context, window, edge) }
}
