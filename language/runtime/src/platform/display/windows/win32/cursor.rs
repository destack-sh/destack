use super::*;

/// Set one cursor visibility state for one window.
pub(crate) unsafe fn window_set_cursor_visible(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setCursorVisible",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setCursorVisible")?;

    // apply global cursor visibility and cache per window state
    let runtime_state = window_runtime_state(context);
    binding.cursor_visible = visible;
    upsert_cursor_policy(&runtime_state, window, &binding);
    refresh_cursor_policy(&runtime_state)?;

    Ok(())
}

/// Set one cursor icon selector.
pub(crate) unsafe fn window_set_cursor_icon(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setCursorIcon",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setCursorIcon")?;

    // load the requested cursor icon from system resources
    let cursor = unsafe { LoadCursorW(0, cursor_name(icon)) };
    if cursor == 0 {
        return Err(core::io_error(
            "destack.display.window.setCursorIcon",
            "LoadCursorW",
            "failed to load cursor icon",
        ));
    }

    // apply host cursor and update binding cache
    unsafe {
        SetCursor(cursor);
    }
    binding.cursor_icon = icon;

    Ok(())
}

/// Set one cursor position lane.
pub(crate) unsafe fn window_set_cursor_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setCursorPosition",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setCursorPosition")?;

    // map the client-space position into desktop-space coordinates
    let mut point = POINT {
        x: position.x,
        y: position.y,
    };
    if unsafe { ClientToScreen(binding.hwnd, &mut point) } == 0 {
        return Err(core::io_error(
            "destack.display.window.setCursorPosition",
            "ClientToScreen",
            "failed to map cursor position",
        ));
    }

    // move the host cursor
    let status = unsafe { SetCursorPos(point.x, point.y) };
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
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setCursorMode",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setCursorMode")?;

    // apply host cursor interaction mode
    apply_cursor_mode(binding.hwnd, mode, "destack.display.window.setCursorMode")?;

    // keep global visibility in sync for hidden and normal modes
    let runtime_state = window_runtime_state(context);
    if mode == WindowCursorMode::Hidden {
        set_cursor_visibility(&runtime_state, false);
        binding.cursor_visible = false;
    } else if mode == WindowCursorMode::Normal {
        set_cursor_visibility(&runtime_state, true);
        binding.cursor_visible = true;
    }

    // update the cached mode value
    binding.cursor_mode = mode;
    upsert_cursor_policy(&runtime_state, window, &binding);
    refresh_cursor_policy(&runtime_state)?;

    Ok(())
}
