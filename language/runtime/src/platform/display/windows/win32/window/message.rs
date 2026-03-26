use windows_sys::Win32::Foundation::{
    ERROR_CLASS_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, DestroyWindow, LoadCursorW, RegisterClassW, SetCursor, WM_ACTIVATE, WM_CLOSE,
    WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_KILLFOCUS, WM_MOVE, WM_SETCURSOR, WM_SETFOCUS,
    WM_SETTINGCHANGE, WM_SHOWWINDOW, WM_SIZE, WM_SIZING, WM_THEMECHANGED, WM_WINDOWPOSCHANGED,
    WNDCLASSW,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::WindowVisibility;

use super::core::current_window_theme;
use super::cursor::{cursor_name, refresh_cursor_policy_best_effort, remove_cursor_policy};
use super::drop::unregister_window_drop_target;
use super::geometry::{apply_aspect_ratio_on_sizing, refresh_window_snapshot, window_class_name};
use crate::platform::display::windows::win32::core::Win32WindowDispatchEntry;
use crate::platform::display::windows::win32::{core, event, handle_window_text_message};

/// Apply one incoming host message to cached runtime state and publish state deltas.
fn apply_window_message_snapshot(
    entry: &Win32WindowDispatchEntry,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) {
    let Some(host_state) = entry.host_state.upgrade() else {
        return;
    };
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();
    let mut monitor_topology_changed = false;

    // resolve geometry after move and size related messages
    if matches!(
        message,
        WM_MOVE | WM_SIZE | WM_WINDOWPOSCHANGED | WM_DPICHANGED | WM_SHOWWINDOW
    ) {
        refresh_window_snapshot(&mut host_state);
    }

    // mirror focus state from host message lanes
    if message == WM_SETFOCUS {
        host_state.focused = true;
    }
    if message == WM_KILLFOCUS {
        host_state.focused = false;
    }
    if message == WM_ACTIVATE {
        host_state.focused =
            wparam as u32 != windows_sys::Win32::UI::WindowsAndMessaging::WA_INACTIVE;
    }

    // handle dpi suggested rectangle updates before snapshot publish
    if message == WM_DPICHANGED {
        let recommended = unsafe { *(lparam as *const windows_sys::Win32::Foundation::RECT) };
        let status = unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                host_state.hwnd,
                0,
                recommended.left,
                recommended.top,
                (recommended.right - recommended.left).max(1),
                (recommended.bottom - recommended.top).max(1),
                windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE
                    | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
            )
        };

        if status == 0 {
            let error_code = core_platform::last_error_code() as u32;
            entry.runtime_state.diagnostics.warn(
                "display",
                "destack.display.window.wndproc",
                "WM_DPICHANGED SetWindowPos failed while applying recommended rectangle",
                Some(error_code),
            );
        }
    }

    if message == WM_THEMECHANGED || message == WM_SETTINGCHANGE {
        host_state.theme = current_window_theme();
    }

    if message == WM_DISPLAYCHANGE {
        monitor_topology_changed = true;
    }

    if message == WM_DESTROY {
        host_state.destroyed_emitted = true;
        host_state.visibility = WindowVisibility::Hidden;
        host_state.focused = false;
    }

    refresh_window_snapshot(&mut host_state);
    let next = host_state.clone();
    drop(host_state);

    if monitor_topology_changed
        && let Err(error) = event::publish_monitor_topology_deltas(&entry.runtime_state)
    {
        entry.runtime_state.diagnostics.warn(
            "display",
            "destack.display.window.messageDispatch.monitorTopology",
            format!("failed to publish monitor topology deltas: {error}"),
            None,
        );
    }

    event::publish_state_deltas(&entry.runtime_state, entry.window, &previous, &next);
}

/// Wndproc for display windows.
pub(crate) unsafe extern "system" fn display_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // apply live aspect ratio constraints during interactive sizing
    if message == WM_SIZING
        && let Some(entry) = Win32WindowDispatchEntry::from_hwnd(hwnd)
    {
        apply_aspect_ratio_on_sizing(&entry, wparam, lparam);
        return 1;
    }

    // intercept close request and publish close requested once
    if message == WM_CLOSE
        && let Some(entry) = Win32WindowDispatchEntry::from_hwnd(hwnd)
    {
        let is_forced_close = wparam == core::WINDOW_CLOSE_FORCE_WPARAM;

        if let Some(host_state) = entry.host_state.upgrade() {
            let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

            if !host_state.close_requested_emitted {
                host_state.close_requested_emitted = true;
                drop(host_state);
                event::publish_window_close_requested_event(&entry.runtime_state, entry.window);
            }
        }

        if is_forced_close {
            let status = unsafe { DestroyWindow(hwnd) };

            if status != 0 {
                return 0;
            }

            let error_code = core_platform::last_error_code() as u32;
            entry.runtime_state.diagnostics.warn(
                "display",
                "destack.display.window.wndproc",
                "forced close DestroyWindow failed, falling back to DefWindowProcW",
                Some(error_code),
            );
            return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
        }

        return 0;
    }

    // cleanup runtime entry and publish destroyed on teardown
    if message == WM_DESTROY
        && let Some(entry) = Win32WindowDispatchEntry::from_hwnd(hwnd)
    {
        let mut should_emit_destroyed = true;

        if let Some(host_state) = entry.host_state.upgrade() {
            let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
            unregister_window_drop_target(&mut host_state);
            should_emit_destroyed = !host_state.destroyed_emitted;
            host_state.destroyed_emitted = true;
        }

        apply_window_message_snapshot(&entry, message, wparam, lparam);

        if should_emit_destroyed {
            event::publish_window_destroyed_event(&entry.runtime_state, entry.window);
        }

        remove_cursor_policy(&entry.runtime_state, entry.window);
        refresh_cursor_policy_best_effort(&entry.runtime_state);
        Win32WindowDispatchEntry::unregister(hwnd);
        return 0;
    }

    // apply stored cursor icon for this window during host cursor updates
    if message == WM_SETCURSOR
        && let Some(entry) = Win32WindowDispatchEntry::from_hwnd(hwnd)
        && let Some(host_state) = entry.host_state.upgrade()
    {
        let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        let cursor = unsafe { LoadCursorW(0, cursor_name(host_state.cursor_icon)) };

        if cursor != 0 {
            unsafe {
                SetCursor(cursor);
            }
            return 1;
        }
    }

    // route native window text and ime messages into active text sessions
    if let Some(entry) = Win32WindowDispatchEntry::from_hwnd(hwnd)
        && let Some(result) = handle_win32_window_text_message(
            &entry.runtime_state,
            entry.window,
            hwnd,
            message,
            wparam,
            lparam,
        )?
    {
        return result;
    }

    // mirror host message deltas into cached runtime state
    if matches!(
        message,
        WM_MOVE
            | WM_SIZE
            | WM_SETFOCUS
            | WM_KILLFOCUS
            | WM_THEMECHANGED
            | WM_SETTINGCHANGE
            | WM_DPICHANGED
            | WM_DISPLAYCHANGE
            | WM_ACTIVATE
            | WM_SHOWWINDOW
            | WM_WINDOWPOSCHANGED
    ) && let Some(entry) = Win32WindowDispatchEntry::from_hwnd(hwnd)
    {
        apply_window_message_snapshot(&entry, message, wparam, lparam);
    }

    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

/// Ensure the display window class is registered.
pub(crate) fn ensure_window_class_registered() -> RuntimeResult<()> {
    // resolve process module instance for class registration
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) } as HINSTANCE;

    if instance == 0 {
        return Err(core::io_error(
            "destack.display.window.open",
            "GetModuleHandleW",
            "failed to resolve module handle",
        ));
    }

    // build and register one window class
    let class_name = window_class_name();
    let class = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(display_window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: 0,
        hCursor: 0,
        hbrBackground: 0,
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };

    let atom = unsafe { RegisterClassW(&class) };

    // tolerate duplicate class registration by name
    if atom == 0 {
        let code = core_platform::last_error_code() as u32;

        if code != ERROR_CLASS_ALREADY_EXISTS {
            return Err(core::io_error_with_code(
                "destack.display.window.open",
                "RegisterClassW",
                code,
                "failed to register display window class",
            ));
        }
    }

    Ok(())
}
