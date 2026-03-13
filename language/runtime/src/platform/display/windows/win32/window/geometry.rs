use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    GetDC, GetDeviceCaps, LOGPIXELSX, ReleaseDC, UpdateWindow,
};
use windows_sys::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, GetClientRect, GetForegroundWindow, GetWindowRect, HTBOTTOM, HTBOTTOMLEFT,
    HTBOTTOMRIGHT, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, IsIconic, IsWindowVisible,
    IsZoomed, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WMSZ_BOTTOM,
    WMSZ_BOTTOMLEFT, WMSZ_LEFT, WMSZ_RIGHT, WMSZ_TOP, WMSZ_TOPLEFT, WMSZ_TOPRIGHT,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowAspectRatio, WindowLogicalSize, WindowModeOptions, WindowPhysicalSize, WindowPosition,
    WindowResizeEdge, WindowSizeConstraints, WindowVisibility,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{
    current_window_theme, window_ex_style_for_host_state, window_style_for_host_state,
};
use super::mode::{apply_mode_options, same_window_mode};
use crate::platform::display::windows::win32::core::Win32WindowDispatchEntry;
use crate::platform::display::windows::win32::model::Win32WindowHostState;
use crate::platform::display::windows::win32::{core, event, resource as display_resource};

/// Normalize one window logical-size payload.
pub(crate) fn normalize_logical_size(
    size: WindowLogicalSize,
    field: &'static str,
) -> RuntimeResult<WindowLogicalSize> {
    if !size.width.is_finite() || !size.height.is_finite() {
        return Err(core_platform::invalid_argument(
            field,
            "logical size must be finite",
        ));
    }

    if size.width <= 0.0 || size.height <= 0.0 {
        return Err(core_platform::invalid_argument(
            field,
            "logical size dimensions must be greater than zero",
        ));
    }

    Ok(size)
}

/// Normalize one window physical-size payload.
pub(crate) fn normalize_physical_size(
    size: WindowPhysicalSize,
    field: &'static str,
) -> RuntimeResult<WindowPhysicalSize> {
    if size.width == 0 || size.height == 0 {
        return Err(core_platform::invalid_argument(
            field,
            "physical size dimensions must be greater than zero",
        ));
    }

    Ok(size)
}

/// Clamp one logical-size payload against optional size constraints.
pub(crate) fn clamp_logical_size(
    size: WindowLogicalSize,
    constraints: Option<WindowSizeConstraints>,
) -> WindowLogicalSize {
    let Some(constraints) = constraints else {
        return size;
    };

    let mut width = size.width;
    let mut height = size.height;

    if let Some(min) = constraints.min {
        width = width.max(min.width);
        height = height.max(min.height);
    }

    if let Some(max) = constraints.max {
        width = width.min(max.width);
        height = height.min(max.height);
    }

    WindowLogicalSize { width, height }
}

/// Resolve one win32 scale factor for one window handle.
pub(crate) fn window_scale_factor_milli(hwnd: HWND) -> u32 {
    // prefer the per window dpi query on modern windows
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi > 0 {
        return dpi.saturating_mul(1000).saturating_add(48) / 96;
    }

    // fall back to one device context query when the dpi lane is unavailable
    let hdc = unsafe { GetDC(hwnd) };
    if hdc == 0 {
        return 1000;
    }

    let dpi_x = unsafe { GetDeviceCaps(hdc, LOGPIXELSX as i32) };
    unsafe {
        ReleaseDC(hwnd, hdc);
    }

    // normalize scale factor output
    if dpi_x <= 0 {
        return 1000;
    }

    ((dpi_x as u32).saturating_mul(1000) / 96).max(1)
}

/// Convert one milli-scale factor into one Win32 DPI value.
pub(crate) fn dpi_from_scale_factor_milli(scale_factor_milli: u32) -> u32 {
    let scale_factor_milli = scale_factor_milli.max(1);
    scale_factor_milli.saturating_mul(96).saturating_add(500) / 1000
}

/// Convert one logical-size payload into one physical-size payload.
pub(crate) fn logical_to_physical(
    size: WindowLogicalSize,
    scale_factor_milli: u32,
) -> WindowPhysicalSize {
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
pub(crate) fn physical_to_logical(
    size: WindowPhysicalSize,
    scale_factor_milli: u32,
) -> WindowLogicalSize {
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

/// Convert one physical dimension into one i32 window-api dimension.
pub(crate) fn dimension_to_i32(value: u32, field: &'static str) -> RuntimeResult<i32> {
    if value > i32::MAX as u32 {
        return Err(core_platform::invalid_argument(
            field,
            "dimension exceeds Win32 i32 range",
        ));
    }

    Ok(value as i32)
}

/// Resolve one non-client outer size from one requested client size.
pub(crate) fn outer_size_from_client_size(
    hwnd: HWND,
    client_size: WindowPhysicalSize,
    style: WINDOW_STYLE,
    ex_style: WINDOW_EX_STYLE,
    operation: &'static str,
) -> RuntimeResult<(i32, i32)> {
    let dpi = unsafe { GetDpiForWindow(hwnd) };

    outer_size_from_client_size_for_dpi(client_size, style, ex_style, dpi, operation)
}

/// Resolve one non-client outer size from one requested client size and one explicit dpi value.
pub(crate) fn outer_size_from_client_size_for_dpi(
    client_size: WindowPhysicalSize,
    style: WINDOW_STYLE,
    ex_style: WINDOW_EX_STYLE,
    dpi: u32,
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

    let status = if dpi > 0 {
        unsafe { AdjustWindowRectExForDpi(&mut rectangle, style, 0, ex_style, dpi) }
    } else {
        unsafe { AdjustWindowRectEx(&mut rectangle, style, 0, ex_style) }
    };
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

/// Refresh one cached window state snapshot from one live hwnd.
pub(crate) fn refresh_window_snapshot(host_state: &mut Win32WindowHostState) {
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
    if unsafe { GetWindowRect(host_state.hwnd, &mut window_rect) } != 0 {
        host_state.position = WindowPosition {
            x: window_rect.left,
            y: window_rect.top,
        };
    }

    // refresh cached physical size from client rect
    if unsafe { GetClientRect(host_state.hwnd, &mut client_rect) } != 0 {
        let width = (client_rect.right - client_rect.left).max(1) as u32;
        let height = (client_rect.bottom - client_rect.top).max(1) as u32;
        host_state.size_physical = WindowPhysicalSize { width, height };
    }

    // refresh derived state lanes
    host_state.scale_factor_milli = window_scale_factor_milli(host_state.hwnd);
    host_state.size_logical =
        physical_to_logical(host_state.size_physical, host_state.scale_factor_milli);
    host_state.visibility = visibility_from_hwnd(host_state.hwnd);
    host_state.focused = unsafe { GetForegroundWindow() } == host_state.hwnd;
    host_state.theme = current_window_theme();
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
    if sizing_edge_has_left_anchor(edge) {
        rect.left = rect.right.saturating_sub(width);
    } else {
        rect.right = rect.left.saturating_add(width);
    }
}

/// Set one outer-rect height according to one interactive sizing edge.
fn set_outer_height_for_edge(rect: &mut RECT, edge: u32, height: i32) {
    if sizing_edge_has_top_anchor(edge) {
        rect.top = rect.bottom.saturating_sub(height);
    } else {
        rect.bottom = rect.top.saturating_add(height);
    }
}

/// Apply one aspect-ratio lock to one live WM_SIZING rectangle update.
pub(crate) fn apply_aspect_ratio_on_sizing(
    entry: &Win32WindowDispatchEntry,
    edge: WPARAM,
    rect_ptr: LPARAM,
) {
    // abort when the sizing rectangle pointer is absent
    if rect_ptr == 0 {
        return;
    }

    // upgrade host state and lock mutable state
    let Some(host_state) = entry.host_state.upgrade() else {
        return;
    };
    let host_state = match host_state.try_lock() {
        Ok(host_state) => host_state,
        Err(std::sync::TryLockError::WouldBlock) => return,
        Err(std::sync::TryLockError::Poisoned(error)) => error.into_inner(),
    };
    let Some(aspect_ratio) = host_state.aspect_ratio else {
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
    if unsafe { GetWindowRect(host_state.hwnd, &mut outer_rect) } == 0 {
        return;
    }
    if unsafe { GetClientRect(host_state.hwnd, &mut client_rect) } == 0 {
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
    let target_logical = physical_to_logical(target_physical, host_state.scale_factor_milli);
    let clamped_logical = clamp_logical_size(target_logical, host_state.constraints);
    let clamped_physical = logical_to_physical(clamped_logical, host_state.scale_factor_milli);

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
pub(crate) fn window_class_name() -> Vec<u16> {
    core_platform::wide_with_nul("destack_display_win32")
}

/// Map one resize edge selector to one non-client hit-test value.
pub(crate) fn resize_hit_test(edge: WindowResizeEdge) -> usize {
    // map the edge enum to the Win32 non-client hit-test value
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

/// Apply one optional aspect-ratio lock to one logical-size payload.
fn apply_aspect_ratio_lock(
    size: WindowLogicalSize,
    aspect_ratio: Option<WindowAspectRatio>,
) -> WindowLogicalSize {
    // return original size when no lock is configured
    let Some(aspect_ratio) = aspect_ratio else {
        return size;
    };

    // resolve ratio and clamp adjusted height
    let numerator = aspect_ratio.numerator as f64;
    let denominator = aspect_ratio.denominator as f64;
    let adjusted_height = ((size.width * denominator) / numerator).max(1.0);
    WindowLogicalSize {
        width: size.width,
        height: adjusted_height,
    }
}

/// Set one window position.
pub(crate) unsafe fn window_set_position(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setPosition",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // capture previous state for delta publication
    let previous = host_state.clone();

    // apply host position update
    let status = unsafe {
        SetWindowPos(
            host_state.hwnd,
            0,
            position.x,
            position.y,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setPosition",
            "SetWindowPos",
            "failed to set window position",
        ));
    }

    // refresh cached state and publish deltas
    host_state.position = position;
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one logical window size.
pub(crate) unsafe fn window_set_size_logical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    // validate logical size payload
    let size = normalize_logical_size(size, "size")?;

    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setSizeLogical",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // capture previous state for delta publication
    let previous = host_state.clone();

    // resolve constrained target size and host outer rectangle
    let locked_size = apply_aspect_ratio_lock(size, host_state.aspect_ratio);
    let clamped_size = clamp_logical_size(locked_size, host_state.constraints);
    let size_physical = logical_to_physical(clamped_size, host_state.scale_factor_milli);
    let style = window_style_for_host_state(&host_state);
    let ex_style = window_ex_style_for_host_state(&host_state);
    let (outer_width, outer_height) = outer_size_from_client_size(
        host_state.hwnd,
        size_physical,
        style,
        ex_style,
        "destack.display.window.setSizeLogical",
    )?;

    // apply host size update
    let status = unsafe {
        SetWindowPos(
            host_state.hwnd,
            0,
            0,
            0,
            outer_width,
            outer_height,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setSizeLogical",
            "SetWindowPos",
            "failed to set logical window size",
        ));
    }

    // refresh cached state and publish deltas
    host_state.size_logical = clamped_size;
    host_state.size_physical = size_physical;
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one physical window size.
pub(crate) unsafe fn window_set_size_physical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    // validate physical size payload
    let size = normalize_physical_size(size, "size")?;

    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setSizePhysical",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // capture previous state for delta publication
    let previous = host_state.clone();

    // resolve constrained target size and host outer rectangle
    let size_logical = physical_to_logical(size, host_state.scale_factor_milli);
    let size_logical = apply_aspect_ratio_lock(size_logical, host_state.aspect_ratio);
    let clamped_logical = clamp_logical_size(size_logical, host_state.constraints);
    let clamped_physical = logical_to_physical(clamped_logical, host_state.scale_factor_milli);
    let style = window_style_for_host_state(&host_state);
    let ex_style = window_ex_style_for_host_state(&host_state);
    let (outer_width, outer_height) = outer_size_from_client_size(
        host_state.hwnd,
        clamped_physical,
        style,
        ex_style,
        "destack.display.window.setSizePhysical",
    )?;

    // apply host size update
    let status = unsafe {
        SetWindowPos(
            host_state.hwnd,
            0,
            0,
            0,
            outer_width,
            outer_height,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setSizePhysical",
            "SetWindowPos",
            "failed to set physical window size",
        ));
    }

    // refresh cached state and publish deltas
    host_state.size_logical = clamped_logical;
    host_state.size_physical = clamped_physical;
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Set logical size constraints.
pub(crate) unsafe fn window_set_size_constraints(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setSizeConstraints",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // update cached constraints
    host_state.constraints = constraints;

    Ok(())
}

/// Set one window mode payload.
pub(crate) unsafe fn window_set_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setMode",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // short circuit no-op mode transitions
    if same_window_mode(host_state.mode, mode) {
        return Ok(());
    }

    // apply transition and rollback in-memory state on host failure
    let previous = host_state.clone();
    if let Err(error) = apply_mode_options(
        context,
        &mut host_state,
        mode,
        "destack.display.window.setMode",
        true,
    ) {
        let rollback = apply_mode_options(
            context,
            &mut host_state,
            previous.mode,
            "destack.display.window.setMode.rollback",
            false,
        );

        // preserve the primary transition error, log rollback failure for diagnostics
        if let Err(rollback_error) = rollback {
            context.warn(
                "display",
                "destack.display.window.setMode.rollback",
                rollback_error.message(),
                Some(rollback_error.sub_code()),
            );
        }

        host_state.mode = previous.mode;
        host_state.display = previous.display;
        host_state.exclusive_restore = previous.exclusive_restore.clone();
        refresh_window_snapshot(&mut host_state);

        return Err(error);
    }

    // refresh cached state and publish semantic deltas
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    if !same_window_mode(previous.mode, next.mode) {
        event::publish_window_mode_event(&runtime_state, window, previous.mode, next.mode);
    }
    if previous.display != next.display {
        event::publish_window_display_event(&runtime_state, window, previous.display, next.display);
    }
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one window aspect-ratio lock.
pub(crate) unsafe fn window_set_aspect_ratio(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    aspectratio: Option<WindowAspectRatio>,
) -> RuntimeResult<()> {
    // validate non-zero ratio payload
    if let Some(aspectratio) = aspectratio
        && (aspectratio.numerator == 0 || aspectratio.denominator == 0)
    {
        return Err(core_platform::invalid_argument(
            "aspectRatio",
            "aspect ratio numerator and denominator must be greater than zero",
        ));
    }

    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.setAspectRatio",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // capture previous state and apply aspect ratio update
    let previous = host_state.clone();
    host_state.aspect_ratio = aspectratio;

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Minimize one window.
pub(crate) unsafe fn window_minimize(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.minimize",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // capture previous state for delta publication
    let previous = host_state.clone();

    // apply host minimize transition
    unsafe {
        ShowWindow(host_state.hwnd, SW_MINIMIZE);
        UpdateWindow(host_state.hwnd);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Maximize one window.
pub(crate) unsafe fn window_maximize(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.maximize",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // capture previous state for delta publication
    let previous = host_state.clone();

    // apply host maximize transition
    unsafe {
        ShowWindow(host_state.hwnd, SW_MAXIMIZE);
        UpdateWindow(host_state.hwnd);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}

/// Restore one window from minimized or maximized state.
pub(crate) unsafe fn window_restore(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.restore",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // capture previous state for delta publication
    let previous = host_state.clone();

    // apply host restore transition
    unsafe {
        ShowWindow(host_state.hwnd, SW_RESTORE);
        UpdateWindow(host_state.hwnd);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    let runtime_state = core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window, &previous, &next);

    Ok(())
}
