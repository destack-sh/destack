use windows_sys::Win32::Foundation::{ERROR_SUCCESS, GetLastError, HWND, RECT, SetLastError};
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GWL_STYLE, HWND_NOTOPMOST, HWND_TOPMOST, LWA_ALPHA, SPI_GETHIGHCONTRAST, SW_HIDE,
    SW_MAXIMIZE, SW_MINIMIZE, SW_SHOW, SW_SHOWNA, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, SystemParametersInfoW,
    WINDOW_EX_STYLE, WINDOW_STYLE, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU,
    WS_THICKFRAME,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{
    WindowChromeKind, WindowOcclusionState, WindowTheme, WindowVisibility,
};

use super::mode::is_windowed_mode;
use crate::platform::display::windows::win32::core;
use crate::platform::display::windows::win32::model::Win32WindowHostState;

/// Normalize one whole-window opacity payload.
pub(crate) fn normalize_opacity(opacity: f64, field: &'static str) -> RuntimeResult<f64> {
    // validate finite value
    if !opacity.is_finite() {
        return Err(core_platform::invalid_argument(
            field,
            "opacity must be finite",
        ));
    }

    // validate normalized range
    if !(0.0..=1.0).contains(&opacity) {
        return Err(core_platform::invalid_argument(
            field,
            "opacity must be between 0.0 and 1.0 inclusive",
        ));
    }

    Ok(opacity)
}

/// Convert one utf8 string into one nul-terminated utf16 buffer.
fn wide_with_nul(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Resolve one apps-use-light-theme preference from the current user profile.
fn apps_use_light_theme() -> Option<bool> {
    // read windows theme personalization value
    let key = wide_with_nul("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value_name = wide_with_nul("AppsUseLightTheme");

    let mut data = 0u32;
    let mut data_size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut data as *mut u32).cast(),
            &mut data_size,
        )
    };
    // return none when the registry lane is unavailable
    if status != ERROR_SUCCESS {
        return None;
    }

    Some(data != 0)
}

/// Resolve whether the host high-contrast accessibility lane is enabled.
fn high_contrast_enabled() -> Option<bool> {
    // query high contrast accessibility state
    let mut high_contrast = HIGHCONTRASTW {
        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
        dwFlags: 0,
        lpszDefaultScheme: std::ptr::null_mut(),
    };
    let status = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            high_contrast.cbSize,
            (&mut high_contrast as *mut HIGHCONTRASTW).cast(),
            0,
        )
    };
    // return none when host query fails
    if status == 0 {
        return None;
    }

    Some((high_contrast.dwFlags & HCF_HIGHCONTRASTON) != 0)
}

/// Resolve one theme value from host preference state.
pub(crate) fn theme_from_preferences(
    high_contrast: bool,
    prefers_light: Option<bool>,
) -> WindowTheme {
    // prioritize high contrast mapping
    if high_contrast {
        if prefers_light == Some(true) {
            return WindowTheme::HighContrastLight;
        }

        return WindowTheme::HighContrastDark;
    }

    // otherwise map light and dark preference lanes
    match prefers_light {
        Some(true) => WindowTheme::Light,
        Some(false) => WindowTheme::Dark,
        None => WindowTheme::Unknown,
    }
}

/// Resolve one current window theme from host preference state.
pub(crate) fn current_window_theme() -> WindowTheme {
    let high_contrast = high_contrast_enabled().unwrap_or(false);
    let prefers_light = apps_use_light_theme();
    theme_from_preferences(high_contrast, prefers_light)
}

/// Return one ShowWindow command for one visibility state.
pub(crate) fn show_command(visibility: WindowVisibility) -> i32 {
    match visibility {
        WindowVisibility::Visible => SW_SHOW,
        WindowVisibility::Hidden => SW_HIDE,
        WindowVisibility::Minimized => SW_MINIMIZE,
        WindowVisibility::Maximized => SW_MAXIMIZE,
    }
}

/// Return one open-time ShowWindow command that respects focus behavior.
pub(crate) fn show_command_on_open(visibility: WindowVisibility, focus_on_show: bool) -> i32 {
    match visibility {
        WindowVisibility::Visible if !focus_on_show => SW_SHOWNA,
        _ => show_command(visibility),
    }
}

/// Resolve one occlusion value from one window visibility state.
pub(crate) fn occlusion_from_visibility(visibility: WindowVisibility) -> WindowOcclusionState {
    // hidden and minimized windows are not visible to presentation
    if visibility == WindowVisibility::Hidden || visibility == WindowVisibility::Minimized {
        return WindowOcclusionState::Occluded;
    }

    // win32 backend does not yet expose one compositor-accurate occlusion signal for visible windows
    WindowOcclusionState::Unknown
}

/// Build one Win32 style payload from one window host state snapshot.
pub(crate) fn window_style_for_host_state(host_state: &Win32WindowHostState) -> WINDOW_STYLE {
    // seed style from mode and chrome lanes
    let mut style: WINDOW_STYLE = if !is_windowed_mode(host_state.mode) {
        WS_POPUP
    } else {
        match host_state.chrome {
            WindowChromeKind::Standard => WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            WindowChromeKind::Tool => WS_CAPTION | WS_SYSMENU,
            WindowChromeKind::Popup => WS_POPUP,
        }
    };

    // apply resize affordances for decorated windowed windows
    if host_state.resizable
        && is_windowed_mode(host_state.mode)
        && host_state.chrome != WindowChromeKind::Popup
    {
        style |= WS_THICKFRAME | WS_MAXIMIZEBOX;
    }

    // strip decorations when decorated lane is disabled
    if !host_state.decorated {
        style &= !(WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU);
        style |= WS_POPUP;
    }

    style
}

/// Build one Win32 ex-style payload from one window host state snapshot.
pub(crate) fn window_ex_style_for_host_state(host_state: &Win32WindowHostState) -> WINDOW_EX_STYLE {
    // seed empty ex style payload
    let mut ex_style: WINDOW_EX_STYLE = 0;

    // enable layered style for transparency lanes
    if host_state.transparent || host_state.opacity < 1.0 || host_state.mouse_passthrough {
        ex_style |= WS_EX_LAYERED;
    }

    // enable input passthrough flag when requested
    if host_state.mouse_passthrough {
        ex_style |= WS_EX_TRANSPARENT;
    }

    // apply always on top style lane
    if host_state.always_on_top {
        ex_style |= WS_EX_TOPMOST;
    }

    // route taskbar and tool window behavior
    if !host_state.taskbar_visible || host_state.chrome == WindowChromeKind::Tool {
        ex_style |= WS_EX_TOOLWINDOW;
    } else {
        ex_style |= WS_EX_APPWINDOW;
    }

    ex_style
}

/// Apply one layered alpha payload for one window.
fn apply_window_alpha(
    host_state: &Win32WindowHostState,
    operation: &'static str,
) -> RuntimeResult<()> {
    // skip alpha call when layered style is not active
    let ex_style = window_ex_style_for_host_state(host_state);
    if (ex_style & WS_EX_LAYERED) == 0 {
        return Ok(());
    }

    // apply normalized opacity as win32 alpha channel
    let alpha = (host_state.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    let status = unsafe { SetLayeredWindowAttributes(host_state.hwnd, 0, alpha, LWA_ALPHA) };
    if status == 0 {
        return Err(core::io_error(
            operation,
            "SetLayeredWindowAttributes",
            "failed to apply window opacity",
        ));
    }

    Ok(())
}

/// Set one mutable window long pointer lane and report Win32 failure correctly.
pub(crate) fn set_window_long_ptr_checked(
    hwnd: HWND,
    index: i32,
    value: isize,
    operation: &'static str,
    syscall: &'static str,
) -> RuntimeResult<()> {
    unsafe {
        SetLastError(0);
        let previous = SetWindowLongPtrW(hwnd, index, value);
        if previous == 0 {
            let error_code = GetLastError();
            if error_code != 0 {
                return Err(core::io_error_with_code(
                    operation,
                    syscall,
                    error_code,
                    "failed to update window style lane",
                ));
            }
        }
    }

    Ok(())
}

/// Apply one desktop rectangle to one window.
pub(crate) fn apply_window_rect(
    host_state: &Win32WindowHostState,
    rectangle: RECT,
    operation: &'static str,
) -> RuntimeResult<()> {
    let status = unsafe {
        SetWindowPos(
            host_state.hwnd,
            if host_state.always_on_top {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            },
            rectangle.left,
            rectangle.top,
            (rectangle.right - rectangle.left).max(1),
            (rectangle.bottom - rectangle.top).max(1),
            SWP_NOACTIVATE,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            operation,
            "SetWindowPos",
            "failed to apply target window rectangle",
        ));
    }

    Ok(())
}

/// Apply one style and ex-style update to one live window.
pub(crate) fn apply_window_style(
    host_state: &Win32WindowHostState,
    operation: &'static str,
) -> RuntimeResult<()> {
    // compute target style lanes
    let style = window_style_for_host_state(host_state);
    let ex_style = window_ex_style_for_host_state(host_state);

    // update style and ex style slots
    set_window_long_ptr_checked(
        host_state.hwnd,
        GWL_STYLE,
        style as isize,
        operation,
        "SetWindowLongPtrW",
    )?;
    set_window_long_ptr_checked(
        host_state.hwnd,
        GWL_EXSTYLE,
        ex_style as isize,
        operation,
        "SetWindowLongPtrW",
    )?;

    // commit frame changes through set window pos
    let status = unsafe {
        SetWindowPos(
            host_state.hwnd,
            if host_state.always_on_top {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            },
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            operation,
            "SetWindowPos",
            "failed to apply window style",
        ));
    }

    // apply layered opacity lane when needed
    apply_window_alpha(host_state, operation)?;

    Ok(())
}
