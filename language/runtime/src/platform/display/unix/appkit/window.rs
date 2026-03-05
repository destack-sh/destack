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
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_close(binding, window) }
}

/// Read descriptor metadata for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_descriptor(
    binding: &BindingCallContext,
    out: *mut WindowDescriptor,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_descriptor(binding, out, window) }
}

/// Open one window.
pub(in crate::platform::display::host::unix) unsafe fn window_open(
    binding: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_open(binding, out, options) }
}

/// Request user attention for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_request_attention(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_request_attention(binding, window, level) }
}

/// Request one redraw for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_request_refresh(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_request_refresh(binding, window) }
}

/// Set always-on-top state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_always_on_top(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_always_on_top(binding, window, alwaysontop) }
}

/// Set cursor icon for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_set_cursor_icon(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_cursor_icon(binding, window, icon) }
}

/// Set cursor interaction mode for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_set_cursor_mode(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_cursor_mode(binding, window, mode) }
}

/// Set cursor position for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_set_cursor_position(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_cursor_position(binding, window, position) }
}

/// Set cursor visibility for one window.
pub(in crate::platform::display::host::unix) unsafe fn window_set_cursor_visible(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_cursor_visible(binding, window, visible) }
}

/// Set window decoration state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_decorated(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_decorated(binding, window, decorated) }
}

/// Set one window mode.
pub(in crate::platform::display::host::unix) unsafe fn window_set_mode(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_mode(binding, window, mode) }
}

/// Set one window position.
pub(in crate::platform::display::host::unix) unsafe fn window_set_position(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_position(binding, window, position) }
}

/// Set window resizable state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_resizable(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_resizable(binding, window, resizable) }
}

/// Set logical size constraints.
pub(in crate::platform::display::host::unix) unsafe fn window_set_size_constraints(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    unsafe {
        unsupported::destack_display_window_set_size_constraints(binding, window, constraints)
    }
}

/// Set one logical window size.
pub(in crate::platform::display::host::unix) unsafe fn window_set_size_logical(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_size_logical(binding, window, size) }
}

/// Set one physical window size.
pub(in crate::platform::display::host::unix) unsafe fn window_set_size_physical(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_size_physical(binding, window, size) }
}

/// Set one window title string.
pub(in crate::platform::display::host::unix) unsafe fn window_set_title(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_title(binding, window, title) }
}

/// Set one window visibility state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_visibility(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_visibility(binding, window, visibility) }
}

/// Read one window state snapshot.
pub(in crate::platform::display::host::unix) unsafe fn window_state(
    binding: &BindingCallContext,
    out: *mut WindowState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_state(binding, out, window) }
}

/// Set one window aspect-ratio lock.
pub(in crate::platform::display::host::unix) unsafe fn window_set_aspect_ratio(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    aspectratio: Option<WindowAspectRatio>,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_aspect_ratio(binding, window, aspectratio) }
}

/// Set one window chrome kind.
pub(in crate::platform::display::host::unix) unsafe fn window_set_chrome(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_chrome(binding, window, chrome) }
}

/// Set one window icon set.
pub(in crate::platform::display::host::unix) unsafe fn window_set_icons(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_icons(binding, window, icons) }
}

/// Set one window modal state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_modal(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    modal: bool,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_modal(binding, window, modal) }
}

/// Set one window mouse passthrough state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_mouse_passthrough(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    unsafe {
        unsupported::destack_display_window_set_mouse_passthrough(binding, window, passthrough)
    }
}

/// Set one window opacity.
pub(in crate::platform::display::host::unix) unsafe fn window_set_opacity(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_opacity(binding, window, opacity) }
}

/// Read one window opacity.
pub(in crate::platform::display::host::unix) unsafe fn window_opacity(
    binding: &BindingCallContext,
    out: *mut f64,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_opacity(binding, out, window) }
}

/// Focus one window.
pub(in crate::platform::display::host::unix) unsafe fn window_focus(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_focus(binding, window) }
}

/// Raise one window.
pub(in crate::platform::display::host::unix) unsafe fn window_raise(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_raise(binding, window) }
}

/// Minimize one window.
pub(in crate::platform::display::host::unix) unsafe fn window_minimize(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_minimize(binding, window) }
}

/// Maximize one window.
pub(in crate::platform::display::host::unix) unsafe fn window_maximize(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_maximize(binding, window) }
}

/// Restore one window.
pub(in crate::platform::display::host::unix) unsafe fn window_restore(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_restore(binding, window) }
}

/// Set one window parent relationship.
pub(in crate::platform::display::host::unix) unsafe fn window_set_parent(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    parent: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_parent(binding, window, parent) }
}

/// Set one window transient relationship.
pub(in crate::platform::display::host::unix) unsafe fn window_set_transient_for(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    transientfor: Option<resource::WindowHandle>,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_transient_for(binding, window, transientfor) }
}

/// Set one window taskbar visibility state.
pub(in crate::platform::display::host::unix) unsafe fn window_set_taskbar_visible(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_set_taskbar_visible(binding, window, visible) }
}

/// Begin one native window move drag.
pub(in crate::platform::display::host::unix) unsafe fn window_begin_move_drag(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_begin_move_drag(binding, window) }
}

/// Begin one native window resize drag.
pub(in crate::platform::display::host::unix) unsafe fn window_begin_resize_drag(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    edge: WindowResizeEdge,
) -> RuntimeResult<()> {
    unsafe { unsupported::destack_display_window_begin_resize_drag(binding, window, edge) }
}
