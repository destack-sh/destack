use std::collections::HashMap;
use std::sync::Arc;

use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CURSOR_SHOWING, CURSORINFO, ClipCursor, GetClientRect, GetCursorInfo, IDC_APPSTARTING,
    IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_HELP, IDC_IBEAM, IDC_NO, IDC_SIZEALL, IDC_SIZENESW,
    IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_WAIT, LoadCursorW, SetCursor, SetCursorPos,
    ShowCursor,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowCursorIcon, WindowCursorMode, WindowPosition};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use crate::platform::display::windows::win32::model::Win32WindowHostState;
use crate::platform::display::windows::win32::{core, resource as display_resource};

/// Resolve whether one cursor mode requests process-global cursor clipping.
fn is_clipped_cursor_mode(mode: WindowCursorMode) -> bool {
    matches!(mode, WindowCursorMode::Confined | WindowCursorMode::Locked)
}

/// Resolve one process-global cursor visibility target from per-window policies.
fn desired_cursor_visibility_from_policies(
    policies: &HashMap<resource::WindowHandle, core::CursorPolicyState>,
) -> bool {
    !policies
        .values()
        .any(|policy| !policy.cursor_visible || policy.cursor_mode == WindowCursorMode::Hidden)
}

/// Resolve one most-recent cursor-clip policy from per-window policies.
fn desired_cursor_clip_policy(
    policies: &HashMap<resource::WindowHandle, core::CursorPolicyState>,
) -> Option<core::CursorPolicyState> {
    policies
        .values()
        .filter(|policy| is_clipped_cursor_mode(policy.cursor_mode))
        .max_by_key(|policy| policy.sequence)
        .copied()
}

/// Synchronize process-global cursor state from per-window policies.
pub(crate) fn refresh_cursor_policy(
    runtime_state: &Arc<core::Win32RuntimeState>,
) -> RuntimeResult<()> {
    // retain valid windows and compute desired global cursor state
    let (desired_visible, desired_clip) = {
        let mut policies = runtime_state
            .cursor_policy_by_window
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        policies.retain(|_, policy| core::is_live_hwnd(policy.hwnd));

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
pub(crate) fn refresh_cursor_policy_best_effort(runtime_state: &Arc<core::Win32RuntimeState>) {
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
pub(crate) fn upsert_cursor_policy(
    runtime_state: &Arc<core::Win32RuntimeState>,
    window: resource::WindowHandle,
    host_state: &Win32WindowHostState,
) {
    let sequence = runtime_state.next_cursor_policy_sequence();
    let mut policies = runtime_state
        .cursor_policy_by_window
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    policies.insert(
        window,
        core::CursorPolicyState {
            hwnd: host_state.hwnd,
            cursor_visible: host_state.cursor_visible,
            cursor_mode: host_state.cursor_mode,
            sequence,
        },
    );
}

/// Remove one window cursor policy snapshot.
pub(crate) fn remove_cursor_policy(
    runtime_state: &Arc<core::Win32RuntimeState>,
    window: resource::WindowHandle,
) {
    let mut policies = runtime_state
        .cursor_policy_by_window
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    policies.remove(&window);
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
    if status == 0 {
        return None;
    }

    Some((cursor.flags & CURSOR_SHOWING) != 0)
}

/// Set one cursor visibility lane.
pub(crate) fn set_cursor_visibility(runtime_state: &Arc<core::Win32RuntimeState>, visible: bool) {
    // skip host calls when cursor visibility already matches
    let mut state = runtime_state
        .cursor_visible_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if *state == Some(visible) && host_cursor_visible() == Some(visible) {
        return;
    }

    // drive win32 global cursor display counter toward target visibility
    unsafe {
        if visible {
            let mut display_count = ShowCursor(1);

            // raise the win32 cursor display count until the cursor becomes visible
            while display_count < 0 {
                display_count = ShowCursor(1);
            }
        } else {
            let mut display_count = ShowCursor(0);

            // lower the win32 cursor display count until the cursor becomes hidden
            while display_count >= 0 {
                display_count = ShowCursor(0);
            }
        }
    }

    *state = Some(host_cursor_visible().unwrap_or(visible));
}

/// Restore global cursor state when one window is closed.
pub(crate) fn restore_cursor_after_close(
    runtime_state: &Arc<core::Win32RuntimeState>,
    window: resource::WindowHandle,
) {
    remove_cursor_policy(runtime_state, window);
    refresh_cursor_policy_best_effort(runtime_state);
}

/// Apply one cursor interaction mode for one window.
pub(crate) fn apply_cursor_mode(
    hwnd: HWND,
    mode: WindowCursorMode,
    operation: &'static str,
) -> RuntimeResult<()> {
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
            if unsafe { ClientToScreen(hwnd, &mut center_screen) } == 0 {
                return Err(core::io_error(
                    operation,
                    "ClientToScreen",
                    "failed to map cursor lock point",
                ));
            }

            let status = unsafe { SetCursorPos(center_screen.x, center_screen.y) };
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
pub(crate) fn cursor_name(icon: WindowCursorIcon) -> *const u16 {
    // map the cursor icon enum to the Win32 cursor selector
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

/// Set one cursor visibility state for one window.
pub(crate) unsafe fn window_set_cursor_visible(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.setCursorVisible",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // apply global cursor visibility and cache per window state
    let runtime_state = core::runtime_state(binding);
    resolved_host_state.cursor_visible = visible;
    upsert_cursor_policy(&runtime_state, window, &resolved_host_state);
    refresh_cursor_policy(&runtime_state)?;

    Ok(())
}

/// Set one cursor icon selector.
pub(crate) unsafe fn window_set_cursor_icon(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.setCursorIcon",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // load the requested cursor icon from system resources
    let cursor = unsafe { LoadCursorW(0, cursor_name(icon)) };
    if cursor == 0 {
        return Err(core::io_error(
            "destack.display.window.setCursorIcon",
            "LoadCursorW",
            "failed to load cursor icon",
        ));
    }

    // apply the host cursor and update cached host state
    unsafe {
        SetCursor(cursor);
    }
    resolved_host_state.cursor_icon = icon;

    Ok(())
}

/// Set one cursor position lane.
pub(crate) unsafe fn window_set_cursor_position(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.setCursorPosition",
    )?;
    let resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // map the client-space position into desktop-space coordinates
    let mut point = POINT {
        x: position.x,
        y: position.y,
    };
    if unsafe { ClientToScreen(resolved_host_state.hwnd, &mut point) } == 0 {
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
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    // resolve and validate the target window host state
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window,
        "destack.display.window.setCursorMode",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // apply host cursor interaction mode
    apply_cursor_mode(
        resolved_host_state.hwnd,
        mode,
        "destack.display.window.setCursorMode",
    )?;

    // keep global visibility in sync for hidden and normal modes
    let runtime_state = core::runtime_state(binding);
    if mode == WindowCursorMode::Hidden {
        set_cursor_visibility(&runtime_state, false);
        resolved_host_state.cursor_visible = false;
    } else if mode == WindowCursorMode::Normal {
        set_cursor_visibility(&runtime_state, true);
        resolved_host_state.cursor_visible = true;
    }

    // update the cached mode value
    resolved_host_state.cursor_mode = mode;
    upsert_cursor_policy(&runtime_state, window, &resolved_host_state);
    refresh_cursor_policy(&runtime_state)?;

    Ok(())
}
