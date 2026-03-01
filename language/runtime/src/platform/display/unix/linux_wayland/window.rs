use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowAttentionLevel, WindowCursorIcon, WindowCursorMode, WindowDescriptor, WindowLogicalSize,
    WindowModeOptions, WindowOptions, WindowPhysicalSize, WindowPosition, WindowSizeConstraints,
    WindowState, WindowVisibility,
};
use crate::platform::{NativeStringRef, resource};
use crate::runtime::BindingCallContext;

pub(crate) unsafe fn destack_display_window_close(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe { crate::platform::display::unsupported::destack_display_window_close(context, window) }
}

pub(crate) unsafe fn destack_display_window_descriptor(
    context: &BindingCallContext,
    out: *mut WindowDescriptor,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_descriptor(
            context, out, window,
        )
    }
}

pub(crate) unsafe fn destack_display_window_open(
    context: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_open(context, out, options)
    }
}

pub(crate) unsafe fn destack_display_window_request_attention(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_request_attention(
            context, window, level,
        )
    }
}

pub(crate) unsafe fn destack_display_window_request_refresh(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_request_refresh(
            context, window,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_always_on_top(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_always_on_top(
            context,
            window,
            alwaysontop,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_cursor_icon(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_cursor_icon(
            context, window, icon,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_cursor_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_cursor_mode(
            context, window, mode,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_cursor_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_cursor_position(
            context, window, position,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_cursor_visible(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_cursor_visible(
            context, window, visible,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_decorated(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_decorated(
            context, window, decorated,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_mode(
            context, window, mode,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_position(
            context, window, position,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_resizable(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_resizable(
            context, window, resizable,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_size_constraints(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_size_constraints(
            context,
            window,
            constraints,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_size_logical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_size_logical(
            context, window, size,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_size_physical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_size_physical(
            context, window, size,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_title(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_title(
            context, window, title,
        )
    }
}

pub(crate) unsafe fn destack_display_window_set_visibility(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_set_visibility(
            context, window, visibility,
        )
    }
}

pub(crate) unsafe fn destack_display_window_state(
    context: &BindingCallContext,
    out: *mut WindowState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_state(context, out, window)
    }
}

pub(crate) unsafe fn destack_display_window_vsync_wait(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    unsafe {
        crate::platform::display::unsupported::destack_display_window_vsync_wait(
            context, window, timeoutns,
        )
    }
}
