#[cfg(target_os = "android")]
use super::android as backend_window;
#[cfg(target_os = "ios")]
use super::ios as backend_window;
#[cfg(target_os = "linux")]
use super::linux_x11 as backend_window;
#[cfg(target_os = "macos")]
use super::macos as backend_window;
#[cfg(all(
    unix,
    not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos"
    ))
))]
use super::other as backend_window;
use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowAttentionLevel, WindowCursorIcon, WindowCursorMode, WindowDescriptor, WindowLogicalSize,
    WindowModeOptions, WindowOptions, WindowPhysicalSize, WindowPosition, WindowSizeConstraints,
    WindowState, WindowVisibility,
};
use crate::platform::{NativeStringRef, resource};
use crate::runtime::BindingCallContext;

/// Close one window.
pub(crate) unsafe fn destack_display_window_close(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_close(context, window) }
}

/// Read descriptor metadata for one window.
pub(crate) unsafe fn destack_display_window_descriptor(
    context: &BindingCallContext,
    out: *mut WindowDescriptor,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_descriptor(context, out, window) }
}

/// Open one window.
pub(crate) unsafe fn destack_display_window_open(
    context: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_open(context, out, options) }
}

/// Request user attention for one window.
pub(crate) unsafe fn destack_display_window_request_attention(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_request_attention(context, window, level) }
}

/// Request one redraw for one window.
pub(crate) unsafe fn destack_display_window_request_refresh(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_request_refresh(context, window) }
}

/// Set always-on-top state.
pub(crate) unsafe fn destack_display_window_set_always_on_top(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    unsafe {
        backend_window::destack_display_window_set_always_on_top(context, window, alwaysontop)
    }
}

/// Set cursor icon for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_icon(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_set_cursor_icon(context, window, icon) }
}

/// Set cursor interaction mode for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_set_cursor_mode(context, window, mode) }
}

/// Set cursor position for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_set_cursor_position(context, window, position) }
}

/// Set cursor visibility for one window.
pub(crate) unsafe fn destack_display_window_set_cursor_visible(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_set_cursor_visible(context, window, visible) }
}

/// Set window decoration state.
pub(crate) unsafe fn destack_display_window_set_decorated(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_set_decorated(context, window, decorated) }
}

/// Set one window mode.
pub(crate) unsafe fn destack_display_window_set_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_set_mode(context, window, mode) }
}

/// Set one window position.
pub(crate) unsafe fn destack_display_window_set_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_set_position(context, window, position) }
}

/// Set window resizable state.
pub(crate) unsafe fn destack_display_window_set_resizable(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_set_resizable(context, window, resizable) }
}

/// Set logical size constraints.
pub(crate) unsafe fn destack_display_window_set_size_constraints(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    unsafe {
        backend_window::destack_display_window_set_size_constraints(context, window, constraints)
    }
}

/// Set one logical window size.
pub(crate) unsafe fn destack_display_window_set_size_logical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_set_size_logical(context, window, size) }
}

/// Set one physical window size.
pub(crate) unsafe fn destack_display_window_set_size_physical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_set_size_physical(context, window, size) }
}

/// Set one window title string.
pub(crate) unsafe fn destack_display_window_set_title(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_set_title(context, window, title) }
}

/// Set one window visibility state.
pub(crate) unsafe fn destack_display_window_set_visibility(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_set_visibility(context, window, visibility) }
}

/// Read one window state snapshot.
pub(crate) unsafe fn destack_display_window_state(
    context: &BindingCallContext,
    out: *mut WindowState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_state(context, out, window) }
}

/// Present one frame interval marker.
pub(crate) unsafe fn destack_display_window_vsync_wait(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe { backend_window::destack_display_window_vsync_wait(context, window, timeoutns) }
}
