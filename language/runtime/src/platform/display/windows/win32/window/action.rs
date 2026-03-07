use windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FLASHW_TIMERNOFG, FLASHW_TRAY, FLASHWINFO, FlashWindowEx, GetForegroundWindow, HTCAPTION,
    HWND_TOP, PostMessageW, SW_SHOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetForegroundWindow,
    SetWindowPos, ShowWindow, WM_NCLBUTTONDOWN,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowAttentionLevel, WindowResizeEdge};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::geometry::{refresh_window_snapshot, resize_hit_test};
use crate::platform::display::windows::win32::{core, event, resource as display_resource};

/// Request one user-attention pulse for one window.
pub(crate) unsafe fn window_request_attention(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.requestAttention",
    )?;
    let resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // map attention level into flash count and prepare host payload
    let flash_count = if level == WindowAttentionLevel::Critical {
        7
    } else {
        3
    };
    let info = FLASHWINFO {
        cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
        hwnd: resolved_host_state.hwnd,
        dwFlags: FLASHW_TRAY | FLASHW_TIMERNOFG,
        uCount: flash_count,
        dwTimeout: 0,
    };

    // request host-level taskbar flash
    let status = unsafe { FlashWindowEx(&info) };
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
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.requestRefresh",
    )?;
    let resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // publish a refresh request event into runtime streams
    drop(resolved_host_state);
    let runtime_state = core::runtime_state(binding);
    event::publish_window_refresh_event(&runtime_state, window);

    Ok(())
}

/// Focus one window and bring it to the foreground.
pub(crate) unsafe fn window_focus(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.focus",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // capture previous state for delta publication
    let previous = resolved_host_state.clone();

    // show and focus the host window
    unsafe {
        ShowWindow(resolved_host_state.hwnd, SW_SHOW);
    }
    let status = unsafe { SetForegroundWindow(resolved_host_state.hwnd) };
    if status == 0 && unsafe { GetForegroundWindow() } != resolved_host_state.hwnd {
        return Err(core::io_error(
            "destack.display.window.focus",
            "SetForegroundWindow",
            "failed to grant foreground focus",
        ));
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut resolved_host_state);
    let next = resolved_host_state.clone();
    drop(resolved_host_state);

    let runtime_state = core::runtime_state(binding);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Raise one window in the z-order.
pub(crate) unsafe fn window_raise(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.raise",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // capture previous state for delta publication
    let previous = resolved_host_state.clone();

    // raise the window in z-order without changing focus
    let status = unsafe {
        SetWindowPos(
            resolved_host_state.hwnd,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.raise",
            "SetWindowPos",
            "failed to raise window",
        ));
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut resolved_host_state);
    let next = resolved_host_state.clone();
    drop(resolved_host_state);

    let runtime_state = core::runtime_state(binding);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Begin one native move-drag interaction.
pub(crate) unsafe fn window_begin_move_drag(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.beginMoveDrag",
    )?;
    let resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // release current capture before posting non-client drag message
    unsafe {
        let _ = ReleaseCapture();
    }

    // post host drag begin message
    let status = unsafe {
        PostMessageW(
            resolved_host_state.hwnd,
            WM_NCLBUTTONDOWN,
            HTCAPTION as usize,
            0,
        )
    };
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
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    edge: WindowResizeEdge,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.beginResizeDrag",
    )?;
    let resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // release current capture before posting non-client drag message
    unsafe {
        let _ = ReleaseCapture();
    }

    // post host resize begin message
    let status = unsafe {
        PostMessageW(
            resolved_host_state.hwnd,
            WM_NCLBUTTONDOWN,
            resize_hit_test(edge),
            0,
        )
    };
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
    use crate::platform::display::WindowTheme;

    use super::theme_from_preferences;

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
