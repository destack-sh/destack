use super::*;

/// Request one user-attention pulse for one window.
pub(crate) unsafe fn window_request_attention(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.requestAttention",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.requestAttention")?;

    // map attention level into flash count and prepare host payload
    let flash_count = if level == WindowAttentionLevel::Critical {
        7
    } else {
        3
    };
    let info = FLASHWINFO {
        cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
        hwnd: binding.hwnd,
        dwFlags: FLASHW_TRAY | FLASHW_TIMERNOFG,
        uCount: flash_count,
        dwTimeout: 0,
    };

    // request host-level taskbar flash
    let status = unsafe { FlashWindowEx(&info) };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.requestAttention",
            "FlashWindowEx",
            "failed to request user attention",
        ));
    }

    Ok(())
}

/// Request one redraw for one window.
pub(crate) unsafe fn window_request_refresh(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.requestRefresh",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.requestRefresh")?;

    // publish a refresh request event into runtime streams
    drop(binding);
    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_window_refresh_event(&event_runtime_state, window);

    Ok(())
}

/// Focus one window and bring it to the foreground.
pub(crate) unsafe fn window_focus(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding =
        display_resource::resolve_window_binding(context, window, "destack.display.window.focus")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.focus")?;

    // capture previous state for delta publication
    let previous = binding.clone();

    // show and focus the host window
    unsafe {
        ShowWindow(binding.hwnd, SW_SHOW);
    }
    let status = unsafe { SetForegroundWindow(binding.hwnd) };
    // evaluate this condition
    if status == 0 && unsafe { GetForegroundWindow() } != binding.hwnd {
        return Err(core::io_error(
            "destack.display.window.focus",
            "SetForegroundWindow",
            "failed to grant foreground focus",
        ));
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Raise one window in the z-order.
pub(crate) unsafe fn window_raise(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding =
        display_resource::resolve_window_binding(context, window, "destack.display.window.raise")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.raise")?;

    // capture previous state for delta publication
    let previous = binding.clone();

    // raise the window in z-order without changing focus
    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.raise",
            "SetWindowPos",
            "failed to raise window",
        ));
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Begin one native move-drag interaction.
pub(crate) unsafe fn window_begin_move_drag(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.beginMoveDrag",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.beginMoveDrag")?;

    // release current capture before posting non-client drag message
    unsafe {
        let _ = ReleaseCapture();
    }

    // post host drag begin message
    let status = unsafe { PostMessageW(binding.hwnd, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0) };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.beginMoveDrag",
            "PostMessageW",
            "failed to begin move drag",
        ));
    }

    Ok(())
}

/// Begin one native resize-drag interaction.
pub(crate) unsafe fn window_begin_resize_drag(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    edge: WindowResizeEdge,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.beginResizeDrag",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.beginResizeDrag")?;

    // release current capture before posting non-client drag message
    unsafe {
        let _ = ReleaseCapture();
    }

    // post host resize begin message
    let status = unsafe { PostMessageW(binding.hwnd, WM_NCLBUTTONDOWN, resize_hit_test(edge), 0) };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.beginResizeDrag",
            "PostMessageW",
            "failed to begin resize drag",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Resolve high-contrast light theme when both lanes request it.
    #[test]
    fn test_theme_from_preferences_returns_high_contrast_light() {
        assert_eq!(
            theme_from_preferences(true, Some(true)),
            WindowTheme::HighContrastLight
        );
    }

    /// Resolve high-contrast dark theme when high-contrast is enabled without light preference.
    #[test]
    fn test_theme_from_preferences_returns_high_contrast_dark() {
        assert_eq!(
            theme_from_preferences(true, Some(false)),
            WindowTheme::HighContrastDark
        );
        assert_eq!(
            theme_from_preferences(true, None),
            WindowTheme::HighContrastDark
        );
    }

    /// Resolve standard theme values when high-contrast is disabled.
    #[test]
    fn test_theme_from_preferences_returns_standard_theme_values() {
        assert_eq!(
            theme_from_preferences(false, Some(true)),
            WindowTheme::Light
        );
        assert_eq!(
            theme_from_preferences(false, Some(false)),
            WindowTheme::Dark
        );
        assert_eq!(theme_from_preferences(false, None), WindowTheme::Unknown);
    }
}
