use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    ClientToScreen, LoadCursorW, SetCursor, SetCursorPos,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowCursorIcon, WindowCursorMode, WindowPosition};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::super::{core, resource as display_resource};
use super::core::{
    apply_cursor_mode, cursor_name, ensure_window_thread, refresh_cursor_policy,
    set_cursor_visibility, upsert_cursor_policy, window_runtime_state,
};

/// Set one cursor visibility state for one window.
pub(crate) unsafe fn window_set_cursor_visible(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setCursorVisible",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setCursorVisible")?;

    // apply global cursor visibility and cache per window state
    let runtime_state = window_runtime_state(binding);
    resolved_binding.cursor_visible = visible;
    upsert_cursor_policy(&runtime_state, window, &resolved_binding);
    refresh_cursor_policy(&runtime_state)?;

    Ok(())
}

/// Set one cursor icon selector.
pub(crate) unsafe fn window_set_cursor_icon(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setCursorIcon",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setCursorIcon")?;

    // load the requested cursor icon from system resources
    let cursor = unsafe { LoadCursorW(0, cursor_name(icon)) };
    // evaluate this condition
    if cursor == 0 {
        return Err(core::io_error(
            "destack.display.window.setCursorIcon",
            "LoadCursorW",
            "failed to load cursor icon",
        ));
    }

    // apply host cursor and update resolved_binding cache
    unsafe {
        SetCursor(cursor);
    }
    resolved_binding.cursor_icon = icon;

    Ok(())
}

/// Set one cursor position lane.
pub(crate) unsafe fn window_set_cursor_position(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setCursorPosition",
    )?;
    let resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(
        &resolved_binding,
        "destack.display.window.setCursorPosition",
    )?;

    // map the client-space position into desktop-space coordinates
    let mut point = POINT {
        x: position.x,
        y: position.y,
    };
    // evaluate this condition
    if unsafe { ClientToScreen(resolved_binding.hwnd, &mut point) } == 0 {
        return Err(core::io_error(
            "destack.display.window.setCursorPosition",
            "ClientToScreen",
            "failed to map cursor position",
        ));
    }

    // move the host cursor
    let status = unsafe { SetCursorPos(point.x, point.y) };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setCursorPosition",
            "SetCursorPos",
            "failed to set cursor position",
        ));
    }

    Ok(())
}

/// Set one cursor interaction mode.
pub(crate) unsafe fn window_set_cursor_mode(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setCursorMode",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setCursorMode")?;

    // apply host cursor interaction mode
    apply_cursor_mode(
        resolved_binding.hwnd,
        mode,
        "destack.display.window.setCursorMode",
    )?;

    // keep global visibility in sync for hidden and normal modes
    let runtime_state = window_runtime_state(binding);
    // evaluate this condition
    if mode == WindowCursorMode::Hidden {
        set_cursor_visibility(&runtime_state, false);
        resolved_binding.cursor_visible = false;
    } else if mode == WindowCursorMode::Normal {
        set_cursor_visibility(&runtime_state, true);
        resolved_binding.cursor_visible = true;
    }

    // update the cached mode value
    resolved_binding.cursor_mode = mode;
    upsert_cursor_policy(&runtime_state, window, &resolved_binding);
    refresh_cursor_policy(&runtime_state)?;

    Ok(())
}
