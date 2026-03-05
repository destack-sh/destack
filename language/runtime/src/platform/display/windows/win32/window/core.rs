use std::sync::Arc;

use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HWND, LPARAM, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{ClientToScreen, GetDeviceCaps, LOGPIXELSX, UpdateWindow};
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows_sys::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, ReleaseCapture};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CURSOR_SHOWING, CURSORINFO, CW_USEDEFAULT, ClipCursor, CreateWindowExW,
    FLASHW_TIMERNOFG, FLASHW_TRAY, FLASHWINFO, FlashWindowEx, GWL_EXSTYLE, GWL_STYLE,
    GWLP_HWNDPARENT, GetClientRect, GetCursorInfo, GetForegroundWindow, GetWindowRect, HTBOTTOM,
    HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT,
    HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST, ICON_BIG, ICON_SMALL, IDC_APPSTARTING, IDC_ARROW,
    IDC_CROSS, IDC_HAND, IDC_HELP, IDC_IBEAM, IDC_NO, IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENS,
    IDC_SIZENWSE, IDC_SIZEWE, IDC_WAIT, IsIconic, IsWindowVisible, IsZoomed, LWA_ALPHA,
    PostMessageW, SIZE_MAXIMIZED, SIZE_MINIMIZED, SPI_GETHIGHCONTRAST, SW_HIDE, SW_MAXIMIZE,
    SW_MINIMIZE, SW_RESTORE, SW_SHOW, SW_SHOWNA, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SendMessageW, SetCursorPos, SetForegroundWindow,
    SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowCursor,
    ShowWindow, SystemParametersInfoW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_NCLBUTTONDOWN, WM_SETICON,
    WMSZ_BOTTOM, WMSZ_BOTTOMLEFT, WMSZ_LEFT, WMSZ_RIGHT, WMSZ_TOP, WMSZ_TOPLEFT, WMSZ_TOPRIGHT,
    WS_CAPTION, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayBackend, WindowAspectRatio, WindowAttentionLevel, WindowChromeKind, WindowCursorIcon,
    WindowCursorMode, WindowDescriptor, WindowIconSet, WindowLogicalSize, WindowModeOptions,
    WindowOcclusionState, WindowOptions, WindowPhysicalSize, WindowPosition, WindowResizeEdge,
    WindowRole, WindowSizeConstraints, WindowState, WindowTheme, WindowVisibility,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::{BindingCallContext, NativeStringRef};

use super::super::model::Win32WindowBinding;
use super::super::{core, monitor, resource as display_resource};
use super::constants::*;
use super::icon::{
    best_icon_index, create_hicon, decode_window_icons, destroy_owned_icons, icon_target_dimensions,
};

/// Resolve whether one cursor mode requests process-global cursor clipping.
fn is_clipped_cursor_mode(mode: WindowCursorMode) -> bool {
    matches!(mode, WindowCursorMode::Confined | WindowCursorMode::Locked)
}

/// Resolve one process-global cursor visibility target from per-window policies.
fn desired_cursor_visibility_from_policies(
    policies: &HashMap<resource::WindowHandle, CursorPolicyState>,
) -> bool {
    !policies
        .values()
        .any(|policy| !policy.cursor_visible || policy.cursor_mode == WindowCursorMode::Hidden)
}

/// Resolve one most-recent cursor-clip policy from per-window policies.
fn desired_cursor_clip_policy(
    policies: &HashMap<resource::WindowHandle, CursorPolicyState>,
) -> Option<CursorPolicyState> {
    policies
        .values()
        .filter(|policy| is_clipped_cursor_mode(policy.cursor_mode))
        .max_by_key(|policy| policy.sequence)
        .copied()
}

/// Synchronize process-global cursor state from per-window policies.
fn refresh_cursor_policy(runtime_state: &Arc<WindowRuntimeState>) -> RuntimeResult<()> {
    // retain valid windows and compute desired global cursor state
    let (desired_visible, desired_clip) = {
        let mut policies = runtime_state
            .cursor_policy_by_window
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        policies.retain(|_, policy| is_live_hwnd(policy.hwnd));

        let desired_visible = desired_cursor_visibility_from_policies(&policies);
        let desired_clip = desired_cursor_clip_policy(&policies);
        (desired_visible, desired_clip)
    };

    // synchronize process-global cursor visibility
    set_cursor_visibility(runtime_state, desired_visible);

    // synchronize process-global cursor clip lane
    if let Some(policy) = desired_clip {
        apply_cursor_mode(
            policy.hwnd,
            policy.cursor_mode,
            "destack.display.window.cursorPolicy.sync",
        )?;
    } else {
        unsafe {
            ClipCursor(std::ptr::null());
        }
    }

    Ok(())
}

/// Synchronize process-global cursor state and suppress best-effort failures.
fn refresh_cursor_policy_best_effort(runtime_state: &Arc<WindowRuntimeState>) {
    // evaluate this condition
    if let Err(error) = refresh_cursor_policy(runtime_state) {
        runtime_state.diagnostics.warn(
            "display",
            "destack.display.window.cursorPolicy.sync",
            format!("cursor cleanup sync failed: {error}"),
            None,
        );
    }
}

/// Insert or update one window cursor policy snapshot.
fn upsert_cursor_policy(
    runtime_state: &Arc<WindowRuntimeState>,
    window: resource::WindowHandle,
    binding: &Win32WindowBinding,
) {
    let sequence = next_cursor_policy_sequence(runtime_state);
    let mut policies = runtime_state
        .cursor_policy_by_window
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    policies.insert(
        window,
        CursorPolicyState {
            hwnd: binding.hwnd,
            cursor_visible: binding.cursor_visible,
            cursor_mode: binding.cursor_mode,
            sequence,
        },
    );
}

/// Remove one window cursor policy snapshot.
fn remove_cursor_policy(runtime_state: &Arc<WindowRuntimeState>, window: resource::WindowHandle) {
    let mut policies = runtime_state
        .cursor_policy_by_window
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    policies.remove(&window);
}

/// Normalize one window logical-size payload.
fn normalize_logical_size(
    size: WindowLogicalSize,
    field: &'static str,
) -> RuntimeResult<WindowLogicalSize> {
    // evaluate this condition
    if !size.width.is_finite() || !size.height.is_finite() {
        return Err(core_platform::invalid_argument(
            field,
            "logical size must be finite",
        ));
    }

    // evaluate this condition
    if size.width <= 0.0 || size.height <= 0.0 {
        return Err(core_platform::invalid_argument(
            field,
            "logical size dimensions must be greater than zero",
        ));
    }

    Ok(size)
}

/// Normalize one window physical-size payload.
fn normalize_physical_size(
    size: WindowPhysicalSize,
    field: &'static str,
) -> RuntimeResult<WindowPhysicalSize> {
    // evaluate this condition
    if size.width == 0 || size.height == 0 {
        return Err(core_platform::invalid_argument(
            field,
            "physical size dimensions must be greater than zero",
        ));
    }

    Ok(size)
}

/// Normalize one whole-window opacity payload.
fn normalize_opacity(opacity: f64, field: &'static str) -> RuntimeResult<f64> {
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

/// Clamp one logical-size payload against optional size constraints.
fn clamp_logical_size(
    size: WindowLogicalSize,
    constraints: Option<WindowSizeConstraints>,
) -> WindowLogicalSize {
    let Some(constraints) = constraints else {
        return size;
    };

    let mut width = size.width;
    let mut height = size.height;

    // evaluate this condition
    if let Some(min) = constraints.min {
        width = width.max(min.width);
        height = height.max(min.height);
    }

    // evaluate this condition
    if let Some(max) = constraints.max {
        width = width.min(max.width);
        height = height.min(max.height);
    }

    WindowLogicalSize { width, height }
}

/// Resolve one win32 scale factor for one window handle.
fn window_scale_factor_milli(hwnd: HWND) -> u32 {
    // open one device context for dpi query
    let hdc = unsafe { windows_sys::Win32::Graphics::Gdi::GetDC(hwnd) };
    // evaluate this condition
    if hdc == 0 {
        return 1000;
    }

    // query and release dpi lane
    let dpi_x = unsafe { GetDeviceCaps(hdc, LOGPIXELSX as i32) };
    unsafe {
        windows_sys::Win32::Graphics::Gdi::ReleaseDC(hwnd, hdc);
    }

    // normalize scale factor output
    if dpi_x <= 0 {
        return 1000;
    }

    ((dpi_x as u32).saturating_mul(1000) / 96).max(1)
}

/// Convert one logical-size payload into one physical-size payload.
fn logical_to_physical(size: WindowLogicalSize, scale_factor_milli: u32) -> WindowPhysicalSize {
    // resolve numeric scale from milli factor
    let scale = if scale_factor_milli == 0 {
        1.0
    } else {
        scale_factor_milli as f64 / 1000.0
    };

    // convert logical dimensions to clamped physical pixels
    let width = (size.width * scale).round().max(1.0) as u32;
    let height = (size.height * scale).round().max(1.0) as u32;
    WindowPhysicalSize { width, height }
}

/// Convert one physical-size payload into one logical-size payload.
fn physical_to_logical(size: WindowPhysicalSize, scale_factor_milli: u32) -> WindowLogicalSize {
    // resolve numeric scale from milli factor
    let scale = if scale_factor_milli == 0 {
        1.0
    } else {
        scale_factor_milli as f64 / 1000.0
    };

    // convert physical pixels to clamped logical dimensions
    WindowLogicalSize {
        width: (size.width as f64 / scale).max(1.0),
        height: (size.height as f64 / scale).max(1.0),
    }
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
fn theme_from_preferences(high_contrast: bool, prefers_light: Option<bool>) -> WindowTheme {
    // prioritize high contrast mapping
    if high_contrast {
        // evaluate this condition
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
fn current_window_theme() -> WindowTheme {
    let high_contrast = high_contrast_enabled().unwrap_or(false);
    let prefers_light = apps_use_light_theme();
    theme_from_preferences(high_contrast, prefers_light)
}

/// Convert one physical dimension into one i32 window-api dimension.
fn dimension_to_i32(value: u32, field: &'static str) -> RuntimeResult<i32> {
    // evaluate this condition
    if value > i32::MAX as u32 {
        return Err(core_platform::invalid_argument(
            field,
            "dimension exceeds Win32 i32 range",
        ));
    }

    Ok(value as i32)
}

/// Resolve one non-client outer size from one requested client size.
fn outer_size_from_client_size(
    hwnd: HWND,
    client_size: WindowPhysicalSize,
    style: WINDOW_STYLE,
    ex_style: WINDOW_EX_STYLE,
    operation: &'static str,
) -> RuntimeResult<(i32, i32)> {
    let width = dimension_to_i32(client_size.width, "size.width")?;
    let height = dimension_to_i32(client_size.height, "size.height")?;
    let mut rectangle = RECT {
        left: 0,
        top: 0,
        right: width,
        bottom: height,
    };

    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let status = if dpi > 0 {
        unsafe { AdjustWindowRectExForDpi(&mut rectangle, style, 0, ex_style, dpi) }
    } else {
        unsafe { AdjustWindowRectEx(&mut rectangle, style, 0, ex_style) }
    };
    // evaluate this condition
    if status == 0 {
        let syscall = if dpi > 0 {
            "AdjustWindowRectExForDpi"
        } else {
            "AdjustWindowRectEx"
        };
        return Err(core::io_error(
            operation,
            syscall,
            "failed to compute non-client window rectangle",
        ));
    }

    let outer_width = (rectangle.right - rectangle.left).max(1);
    let outer_height = (rectangle.bottom - rectangle.top).max(1);
    Ok((outer_width, outer_height))
}

/// Return one ShowWindow command for one visibility state.
fn show_command(visibility: WindowVisibility) -> i32 {
    // resolve this variant
    match visibility {
        WindowVisibility::Visible => SW_SHOW,
        WindowVisibility::Hidden => SW_HIDE,
        WindowVisibility::Minimized => SW_MINIMIZE,
        WindowVisibility::Maximized => SW_MAXIMIZE,
    }
}

/// Return one open-time ShowWindow command that respects focus behavior.
fn show_command_on_open(visibility: WindowVisibility, focus_on_show: bool) -> i32 {
    // resolve this variant
    match visibility {
        WindowVisibility::Visible if !focus_on_show => SW_SHOWNA,
        _ => show_command(visibility),
    }
}

/// Resolve visibility from one live hwnd state.
fn visibility_from_hwnd(hwnd: HWND) -> WindowVisibility {
    // hidden windows are always hidden
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return WindowVisibility::Hidden;
    }

    // iconic windows map to minimized visibility
    if unsafe { IsIconic(hwnd) } != 0 {
        return WindowVisibility::Minimized;
    }

    // zoomed windows map to maximized visibility
    if unsafe { IsZoomed(hwnd) } != 0 {
        return WindowVisibility::Maximized;
    }

    WindowVisibility::Visible
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

/// Build one Win32 style payload from one window binding snapshot.
fn window_style_for_binding(binding: &Win32WindowBinding) -> WINDOW_STYLE {
    // seed style from mode and chrome lanes
    let mut style: WINDOW_STYLE = if !is_windowed_mode(binding.mode) {
        WS_POPUP
    } else {
        // resolve this variant
        match binding.chrome {
            WindowChromeKind::Standard => WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            WindowChromeKind::Tool => WS_CAPTION | WS_SYSMENU,
            WindowChromeKind::Popup => WS_POPUP,
        }
    };

    // apply resize affordances for decorated windowed windows
    if binding.resizable
        && is_windowed_mode(binding.mode)
        && binding.chrome != WindowChromeKind::Popup
    {
        style |= WS_THICKFRAME | WS_MAXIMIZEBOX;
    }

    // strip decorations when decorated lane is disabled
    if !binding.decorated {
        style &= !(WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU);
        style |= WS_POPUP;
    }

    style
}

/// Build one Win32 ex-style payload from one window binding snapshot.
fn window_ex_style_for_binding(binding: &Win32WindowBinding) -> WINDOW_EX_STYLE {
    // seed empty ex style payload
    let mut ex_style: WINDOW_EX_STYLE = 0;

    // enable layered style for transparency lanes
    if binding.transparent || binding.opacity < 1.0 || binding.mouse_passthrough {
        ex_style |= WS_EX_LAYERED;
    }

    // enable input passthrough flag when requested
    if binding.mouse_passthrough {
        ex_style |= WS_EX_TRANSPARENT;
    }

    // apply always on top style lane
    if binding.always_on_top {
        ex_style |= WS_EX_TOPMOST;
    }

    // route taskbar and tool window behavior
    if !binding.taskbar_visible || binding.chrome == WindowChromeKind::Tool {
        ex_style |= WS_EX_TOOLWINDOW;
    } else {
        ex_style |= WS_EX_APPWINDOW;
    }

    ex_style
}

/// Apply one layered alpha payload for one window.
fn apply_window_alpha(binding: &Win32WindowBinding, operation: &'static str) -> RuntimeResult<()> {
    // skip alpha call when layered style is not active
    let ex_style = window_ex_style_for_binding(binding);
    // evaluate this condition
    if (ex_style & WS_EX_LAYERED) == 0 {
        return Ok(());
    }

    // apply normalized opacity as win32 alpha channel
    let alpha = (binding.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    let status = unsafe { SetLayeredWindowAttributes(binding.hwnd, 0, alpha, LWA_ALPHA) };
    // evaluate this condition
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
fn set_window_long_ptr_checked(
    hwnd: HWND,
    index: i32,
    value: isize,
    operation: &'static str,
    syscall: &'static str,
) -> RuntimeResult<()> {
    unsafe {
        SetLastError(0);
        let previous = SetWindowLongPtrW(hwnd, index, value);
        // evaluate this condition
        if previous == 0 {
            let error_code = GetLastError();
            // evaluate this condition
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
fn apply_window_rect(
    binding: &Win32WindowBinding,
    rectangle: RECT,
    operation: &'static str,
) -> RuntimeResult<()> {
    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            // evaluate this condition
            if binding.always_on_top {
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
    // evaluate this condition
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
fn apply_window_style(binding: &Win32WindowBinding, operation: &'static str) -> RuntimeResult<()> {
    // compute target style lanes
    let style = window_style_for_binding(binding);
    let ex_style = window_ex_style_for_binding(binding);

    // update style and ex style slots
    set_window_long_ptr_checked(
        binding.hwnd,
        GWL_STYLE,
        style as isize,
        operation,
        "SetWindowLongPtrW",
    )?;
    set_window_long_ptr_checked(
        binding.hwnd,
        GWL_EXSTYLE,
        ex_style as isize,
        operation,
        "SetWindowLongPtrW",
    )?;

    // commit frame changes through set window pos
    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            // evaluate this condition
            if binding.always_on_top {
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
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            operation,
            "SetWindowPos",
            "failed to apply window style",
        ));
    }

    // apply layered opacity lane when needed
    apply_window_alpha(binding, operation)?;

    Ok(())
}

/// Refresh one cached window state snapshot from one live hwnd.
fn refresh_window_snapshot(binding: &mut Win32WindowBinding) {
    // query current outer and client rectangles
    let mut window_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut client_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };

    // refresh cached position from outer rect
    if unsafe { GetWindowRect(binding.hwnd, &mut window_rect) } != 0 {
        binding.position = WindowPosition {
            x: window_rect.left,
            y: window_rect.top,
        };
    }

    // refresh cached physical size from client rect
    if unsafe { GetClientRect(binding.hwnd, &mut client_rect) } != 0 {
        let width = (client_rect.right - client_rect.left).max(1) as u32;
        let height = (client_rect.bottom - client_rect.top).max(1) as u32;
        binding.size_physical = WindowPhysicalSize { width, height };
    }

    // refresh derived state lanes
    binding.scale_factor_milli = window_scale_factor_milli(binding.hwnd);
    binding.size_logical = physical_to_logical(binding.size_physical, binding.scale_factor_milli);
    binding.visibility = visibility_from_hwnd(binding.hwnd);
    binding.focused = unsafe { GetForegroundWindow() } == binding.hwnd;
    binding.theme = current_window_theme();
}

/// Return whether one sizing-edge code anchors width from the left side.
fn sizing_edge_has_left_anchor(edge: u32) -> bool {
    matches!(edge, WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT)
}

/// Return whether one sizing-edge code anchors height from the top side.
fn sizing_edge_has_top_anchor(edge: u32) -> bool {
    matches!(edge, WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT)
}

/// Return whether one sizing-edge code is a pure horizontal resize edge.
fn sizing_edge_is_horizontal(edge: u32) -> bool {
    matches!(edge, WMSZ_LEFT | WMSZ_RIGHT)
}

/// Return whether one sizing-edge code is a pure vertical resize edge.
fn sizing_edge_is_vertical(edge: u32) -> bool {
    matches!(edge, WMSZ_TOP | WMSZ_BOTTOM)
}

/// Set one outer-rect width according to one interactive sizing edge.
fn set_outer_width_for_edge(rect: &mut RECT, edge: u32, width: i32) {
    // evaluate this condition
    if sizing_edge_has_left_anchor(edge) {
        rect.left = rect.right.saturating_sub(width);
    } else {
        rect.right = rect.left.saturating_add(width);
    }
}

/// Set one outer-rect height according to one interactive sizing edge.
fn set_outer_height_for_edge(rect: &mut RECT, edge: u32, height: i32) {
    // evaluate this condition
    if sizing_edge_has_top_anchor(edge) {
        rect.top = rect.bottom.saturating_sub(height);
    } else {
        rect.bottom = rect.top.saturating_add(height);
    }
}

/// Apply one aspect-ratio lock to one live WM_SIZING rectangle update.
fn apply_aspect_ratio_on_sizing(entry: &WindowRuntimeEntry, edge: WPARAM, rect_ptr: LPARAM) {
    // abort when the sizing rectangle pointer is absent
    if rect_ptr == 0 {
        return;
    }

    // upgrade binding and lock mutable state
    let Some(binding) = entry.binding.upgrade() else {
        return;
    };
    let binding = match binding.try_lock() {
        Ok(binding) => binding,
        Err(std::sync::TryLockError::WouldBlock) => return,
        Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
    };
    let Some(aspect_ratio) = binding.aspect_ratio else {
        return;
    };

    // load current frame and client metrics
    let edge = edge as u32;
    let rect = unsafe { &mut *(rect_ptr as *mut RECT) };

    let mut outer_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut client_rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // evaluate this condition
    if unsafe { GetWindowRect(binding.hwnd, &mut outer_rect) } == 0 {
        return;
    }
    // evaluate this condition
    if unsafe { GetClientRect(binding.hwnd, &mut client_rect) } == 0 {
        return;
    }

    // compute proposed client size and ratio adjusted target
    let frame_width =
        ((outer_rect.right - outer_rect.left) - (client_rect.right - client_rect.left)).max(0);
    let frame_height =
        ((outer_rect.bottom - outer_rect.top) - (client_rect.bottom - client_rect.top)).max(0);

    let proposed_outer_width = (rect.right - rect.left).max(1);
    let proposed_outer_height = (rect.bottom - rect.top).max(1);
    let proposed_client_width = (proposed_outer_width - frame_width).max(1);
    let proposed_client_height = (proposed_outer_height - frame_height).max(1);

    let numerator = aspect_ratio.numerator as f64;
    let denominator = aspect_ratio.denominator as f64;
    let from_width = ((proposed_client_width as f64 * denominator) / numerator).round() as i32;
    let from_height = ((proposed_client_height as f64 * numerator) / denominator).round() as i32;
    let from_width = from_width.max(1);
    let from_height = from_height.max(1);

    let (target_width, target_height) = if sizing_edge_is_horizontal(edge) {
        (proposed_client_width, from_width)
    } else if sizing_edge_is_vertical(edge) {
        (from_height, proposed_client_height)
    } else {
        let height_error = from_width.abs_diff(proposed_client_height);
        let width_error = from_height.abs_diff(proposed_client_width);
        // evaluate this condition
        if height_error <= width_error {
            (proposed_client_width, from_width)
        } else {
            (from_height, proposed_client_height)
        }
    };

    // clamp through logical constraints and map back to outer rect
    let target_physical = WindowPhysicalSize {
        width: target_width as u32,
        height: target_height as u32,
    };
    let target_logical = physical_to_logical(target_physical, binding.scale_factor_milli);
    let clamped_logical = clamp_logical_size(target_logical, binding.constraints);
    let clamped_physical = logical_to_physical(clamped_logical, binding.scale_factor_milli);

    let target_outer_width = (clamped_physical.width as i32)
        .saturating_add(frame_width)
        .max(1);
    let target_outer_height = (clamped_physical.height as i32)
        .saturating_add(frame_height)
        .max(1);
    set_outer_width_for_edge(rect, edge, target_outer_width);
    set_outer_height_for_edge(rect, edge, target_outer_height);
}

/// Resolve one class-name payload for Win32 window registration.
fn window_class_name() -> Vec<u16> {
    core_platform::wide_with_nul("destack_display_win32")
}

/// Set one cursor visibility lane.
fn host_cursor_visible() -> Option<bool> {
    let mut cursor = CURSORINFO {
        cbSize: std::mem::size_of::<CURSORINFO>() as u32,
        flags: 0,
        hCursor: 0,
        ptScreenPos: POINT { x: 0, y: 0 },
    };
    let status = unsafe { GetCursorInfo(&mut cursor) };
    // evaluate this condition
    if status == 0 {
        return None;
    }

    Some((cursor.flags & CURSOR_SHOWING) != 0)
}

/// Set one cursor visibility lane.
fn set_cursor_visibility(runtime_state: &Arc<WindowRuntimeState>, visible: bool) {
    // skip host calls when cursor visibility already matches
    let mut state = runtime_state
        .cursor_visible_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    // evaluate this condition
    if *state == Some(visible) && host_cursor_visible() == Some(visible) {
        return;
    }

    // drive win32 global cursor display counter toward target visibility
    unsafe {
        // evaluate this condition
        if visible {
            let mut display_count = ShowCursor(1);
            // iterate while this condition holds
            while display_count < 0 {
                display_count = ShowCursor(1);
            }
        } else {
            let mut display_count = ShowCursor(0);
            // iterate while this condition holds
            while display_count >= 0 {
                display_count = ShowCursor(0);
            }
        }
    }

    *state = Some(host_cursor_visible().unwrap_or(visible));
}

/// Restore global cursor state when one window is closed.
fn restore_cursor_after_close(
    runtime_state: &Arc<WindowRuntimeState>,
    window: resource::WindowHandle,
) {
    remove_cursor_policy(runtime_state, window);
    refresh_cursor_policy_best_effort(runtime_state);
}

/// Resolve one referenced relationship window handle to one hwnd.
fn resolve_relationship_hwnd(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    operation: &'static str,
) -> RuntimeResult<HWND> {
    let binding = display_resource::resolve_window_binding(context, window, operation)?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    Ok(binding.hwnd)
}

/// Resolve the owner relationship for one window binding.
fn owner_relationship(binding: &Win32WindowBinding) -> Option<resource::WindowHandle> {
    binding.transient_for.or(binding.parent)
}

/// Apply owner relationship style for one window.
fn apply_owner_relationship(
    context: &BindingCallContext,
    binding: &Win32WindowBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    let owner_hwnd = if let Some(owner) = owner_relationship(binding) {
        let owner_hwnd = resolve_relationship_hwnd(context, owner, operation)?;
        // evaluate this condition
        if owner_hwnd == binding.hwnd {
            return Err(core_platform::invalid_argument(
                "window",
                "window relationship cannot target itself",
            ));
        }

        owner_hwnd
    } else {
        0
    };

    set_window_long_ptr_checked(
        binding.hwnd,
        GWLP_HWNDPARENT,
        owner_hwnd,
        operation,
        "SetWindowLongPtrW",
    )?;
    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            0,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
        )
    };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            operation,
            "SetWindowPos",
            "failed to apply window owner relationship",
        ));
    }

    Ok(())
}

/// Set one owner-window enabled state.
fn set_owner_enabled(
    context: &BindingCallContext,
    owner: resource::WindowHandle,
    enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    let owner_hwnd = resolve_relationship_hwnd(context, owner, operation)?;
    let enabled = if enabled { 1 } else { 0 };

    unsafe {
        SetLastError(0);
        let previous = EnableWindow(owner_hwnd, enabled);
        // evaluate this condition
        if previous == 0 {
            let error_code = GetLastError();
            // evaluate this condition
            if error_code != 0 {
                return Err(core::io_error_with_code(
                    operation,
                    "EnableWindow",
                    error_code as u32,
                    "failed to update owner enabled state",
                ));
            }
        }
    }

    Ok(())
}

/// Apply one modal-owner state transition for one window.
fn apply_modal_owner_transition(
    context: &BindingCallContext,
    previous_owner: Option<resource::WindowHandle>,
    previous_modal: bool,
    next_owner: Option<resource::WindowHandle>,
    next_modal: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // evaluate this condition
    if next_modal && next_owner.is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // evaluate this condition
    if previous_modal
        && let Some(owner) = previous_owner
        && (!next_modal || Some(owner) != next_owner)
    {
        set_owner_enabled(context, owner, true, operation)?;
    }

    // evaluate this condition
    if next_modal && let Some(owner) = next_owner {
        set_owner_enabled(context, owner, false, operation)?;
    }

    Ok(())
}

/// Re-enable one modal owner as part of window close cleanup.
fn restore_modal_owner_on_close(context: &BindingCallContext, binding: &Win32WindowBinding) {
    // evaluate this condition
    if !binding.modal {
        return;
    }

    // evaluate this condition
    if let Some(owner) = owner_relationship(binding)
        && let Err(error) = set_owner_enabled(context, owner, true, "destack.display.window.close")
    {
        window_runtime_state(context).diagnostics.warn(
            "display",
            "destack.display.window.close",
            format!("failed to restore modal owner after close: {error}"),
            None,
        );
    }
}

/// Map one resize edge selector to one non-client hit-test value.
fn resize_hit_test(edge: WindowResizeEdge) -> usize {
    // map binding edge enum to win32 non client hit test value
    match edge {
        WindowResizeEdge::North => HTTOP as usize,
        WindowResizeEdge::South => HTBOTTOM as usize,
        WindowResizeEdge::East => HTRIGHT as usize,
        WindowResizeEdge::West => HTLEFT as usize,
        WindowResizeEdge::NorthEast => HTTOPRIGHT as usize,
        WindowResizeEdge::NorthWest => HTTOPLEFT as usize,
        WindowResizeEdge::SouthEast => HTBOTTOMRIGHT as usize,
        WindowResizeEdge::SouthWest => HTBOTTOMLEFT as usize,
    }
}

/// Apply one cursor interaction mode for one window.
fn apply_cursor_mode(
    hwnd: HWND,
    mode: WindowCursorMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve this variant
    match mode {
        WindowCursorMode::Normal | WindowCursorMode::Hidden => unsafe {
            ClipCursor(std::ptr::null());
            Ok(())
        },
        WindowCursorMode::Confined => {
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            // evaluate this condition
            if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
                return Err(core::io_error(
                    operation,
                    "GetClientRect",
                    "failed to query client rectangle",
                ));
            }

            let mut top_left = POINT {
                x: rect.left,
                y: rect.top,
            };
            let mut bottom_right = POINT {
                x: rect.right,
                y: rect.bottom,
            };

            // evaluate this condition
            if unsafe { ClientToScreen(hwnd, &mut top_left) } == 0
                || unsafe { ClientToScreen(hwnd, &mut bottom_right) } == 0
            {
                return Err(core::io_error(
                    operation,
                    "ClientToScreen",
                    "failed to map client rectangle",
                ));
            }

            rect.left = top_left.x;
            rect.top = top_left.y;
            rect.right = bottom_right.x;
            rect.bottom = bottom_right.y;

            // evaluate this condition
            if unsafe { ClipCursor(&rect) } == 0 {
                return Err(core::io_error(
                    operation,
                    "ClipCursor",
                    "failed to confine cursor",
                ));
            }

            Ok(())
        }
        WindowCursorMode::Locked => {
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            // evaluate this condition
            if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
                return Err(core::io_error(
                    operation,
                    "GetClientRect",
                    "failed to query client rectangle",
                ));
            }

            let center = POINT {
                x: rect.left + (rect.right - rect.left) / 2,
                y: rect.top + (rect.bottom - rect.top) / 2,
            };
            let mut center_screen = center;
            // evaluate this condition
            if unsafe { ClientToScreen(hwnd, &mut center_screen) } == 0 {
                return Err(core::io_error(
                    operation,
                    "ClientToScreen",
                    "failed to map cursor lock point",
                ));
            }

            let status = unsafe { SetCursorPos(center_screen.x, center_screen.y) };
            // evaluate this condition
            if status == 0 {
                return Err(core::io_error(
                    operation,
                    "SetCursorPos",
                    "failed to lock cursor position",
                ));
            }

            let lock_rect = RECT {
                left: center_screen.x,
                top: center_screen.y,
                right: center_screen.x.saturating_add(1),
                bottom: center_screen.y.saturating_add(1),
            };
            // evaluate this condition
            if unsafe { ClipCursor(&lock_rect) } == 0 {
                return Err(core::io_error(
                    operation,
                    "ClipCursor",
                    "failed to lock cursor region",
                ));
            }

            Ok(())
        }
    }
}

/// Resolve one Win32 cursor name selector from one cursor icon enum.
fn cursor_name(icon: WindowCursorIcon) -> *const u16 {
    // map binding cursor icon enum to win32 cursor selector
    match icon {
        WindowCursorIcon::Default => IDC_ARROW,
        WindowCursorIcon::Pointer => IDC_HAND,
        WindowCursorIcon::Text => IDC_IBEAM,
        WindowCursorIcon::Crosshair => IDC_CROSS,
        WindowCursorIcon::Grab | WindowCursorIcon::Grabbing | WindowCursorIcon::AllScroll => {
            IDC_SIZEALL
        }
        WindowCursorIcon::Move => IDC_SIZEALL,
        WindowCursorIcon::EResize
        | WindowCursorIcon::WResize
        | WindowCursorIcon::EwResize
        | WindowCursorIcon::ColResize => IDC_SIZEWE,
        WindowCursorIcon::NResize
        | WindowCursorIcon::SResize
        | WindowCursorIcon::NsResize
        | WindowCursorIcon::RowResize => IDC_SIZENS,
        WindowCursorIcon::NeResize | WindowCursorIcon::SwResize | WindowCursorIcon::NeswResize => {
            IDC_SIZENESW
        }
        WindowCursorIcon::NwResize | WindowCursorIcon::SeResize | WindowCursorIcon::NwseResize => {
            IDC_SIZENWSE
        }
        WindowCursorIcon::Wait => IDC_WAIT,
        WindowCursorIcon::Progress => IDC_APPSTARTING,
        WindowCursorIcon::NotAllowed => IDC_NO,
        WindowCursorIcon::ZoomIn | WindowCursorIcon::ZoomOut => IDC_ARROW,
        WindowCursorIcon::Help => IDC_HELP,
    }
}
