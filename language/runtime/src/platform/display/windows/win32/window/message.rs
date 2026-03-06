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
use crate::runtime::BindingCallContext;

use super::super::{core, event};
use super::drop::unregister_window_drop_target;
use super::{
    WindowRuntimeEntry, apply_aspect_ratio_on_sizing, current_window_theme, cursor_name,
    refresh_cursor_policy_best_effort, refresh_window_snapshot, remove_cursor_policy,
    runtime_window_entry, unregister_runtime_window, window_class_name,
};

/// Drain pending window-thread messages and dispatch them through the registered wndproc.
pub(in super::super) fn pump_window_messages(context: &BindingCallContext) -> RuntimeResult<()> {
    // drain pending thread messages while preserving host-managed quit lifecycle
    context.host().pump_pending_thread_messages(true)?;

    Ok(())
}

/// Apply one incoming host message to cached runtime state and publish state deltas.
fn apply_window_message_snapshot(
    entry: &WindowRuntimeEntry,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) {
    let Some(binding) = entry.binding.upgrade() else {
        return;
    };
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    let previous = binding.clone();
    let mut monitor_topology_changed = false;

    // resolve geometry after move and size related messages
    if matches!(
        message,
        WM_MOVE | WM_SIZE | WM_WINDOWPOSCHANGED | WM_DPICHANGED | WM_SHOWWINDOW
    ) {
        refresh_window_snapshot(&mut binding);
    }

    // mirror focus state from host message lanes
    if message == WM_SETFOCUS {
        binding.focused = true;
    }
    if message == WM_KILLFOCUS {
        binding.focused = false;
    }
    if message == WM_ACTIVATE {
        binding.focused = wparam as u32 != windows_sys::Win32::UI::WindowsAndMessaging::WA_INACTIVE;
    }

    // handle dpi suggested rectangle updates before snapshot publish
    if message == WM_DPICHANGED {
        let recommended = unsafe { *(lparam as *const windows_sys::Win32::Foundation::RECT) };
        let status = unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                binding.hwnd,
                0,
                recommended.left,
                recommended.top,
                (recommended.right - recommended.left).max(1),
                (recommended.bottom - recommended.top).max(1),
                windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOACTIVATE
                    | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOZORDER,
            )
        };

        // evaluate this condition
        if status == 0 {
            let error_code = core_platform::last_error_code() as u32;
            entry.window_runtime_state.diagnostics.warn(
                "display",
                "destack.display.window.wndproc",
                "WM_DPICHANGED SetWindowPos failed while applying recommended rectangle",
                Some(error_code),
            );
        }
    }

    // evaluate this condition
    if message == WM_THEMECHANGED || message == WM_SETTINGCHANGE {
        binding.theme = current_window_theme();
    }

    // evaluate this condition
    if message == WM_DISPLAYCHANGE {
        monitor_topology_changed = true;
    }

    // evaluate this condition
    if message == WM_DESTROY {
        binding.destroyed_emitted = true;
        binding.visibility = WindowVisibility::Hidden;
        binding.focused = false;
    }

    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    // evaluate this condition
    if monitor_topology_changed
        && let Err(error) = event::publish_monitor_topology_deltas(&entry.event_runtime_state)
    {
        entry.window_runtime_state.diagnostics.warn(
            "display",
            "destack.display.window.messageDispatch.monitorTopology",
            format!("failed to publish monitor topology deltas: {error}"),
            None,
        );
    }

    event::publish_state_deltas(&entry.event_runtime_state, entry.window, &previous, &next);
}

/// Wndproc for display windows.
pub(super) unsafe extern "system" fn display_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // apply live aspect ratio constraints during interactive sizing
    if message == WM_SIZING
        && let Some(entry) = runtime_window_entry(hwnd)
    {
        apply_aspect_ratio_on_sizing(&entry, wparam, lparam);
        return 1;
    }

    // intercept close request and publish close requested once
    if message == WM_CLOSE
        && let Some(entry) = runtime_window_entry(hwnd)
    {
        let is_forced_close = wparam == core::WINDOW_CLOSE_FORCE_WPARAM;

        // evaluate this condition
        if let Some(binding) = entry.binding.upgrade() {
            let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

            // evaluate this condition
            if !binding.close_requested_emitted {
                binding.close_requested_emitted = true;
                drop(binding);
                event::publish_window_close_requested_event(
                    &entry.event_runtime_state,
                    entry.window,
                );
            }
        }

        // evaluate this condition
        if is_forced_close {
            let status = unsafe { DestroyWindow(hwnd) };

            // evaluate this condition
            if status != 0 {
                return 0;
            }

            let error_code = core_platform::last_error_code() as u32;
            entry.window_runtime_state.diagnostics.warn(
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
        && let Some(entry) = runtime_window_entry(hwnd)
    {
        let mut should_emit_destroyed = true;

        // evaluate this condition
        if let Some(binding) = entry.binding.upgrade() {
            let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
            unregister_window_drop_target(&mut binding);
            should_emit_destroyed = !binding.destroyed_emitted;
            binding.destroyed_emitted = true;
        }

        apply_window_message_snapshot(&entry, message, wparam, lparam);

        // evaluate this condition
        if should_emit_destroyed {
            event::publish_window_destroyed_event(&entry.event_runtime_state, entry.window);
        }

        remove_cursor_policy(&entry.window_runtime_state, entry.window);
        refresh_cursor_policy_best_effort(&entry.window_runtime_state);
        unregister_runtime_window(hwnd);
        return 0;
    }

    // apply stored cursor icon for this window during host cursor updates
    if message == WM_SETCURSOR
        && let Some(entry) = runtime_window_entry(hwnd)
        && let Some(binding) = entry.binding.upgrade()
    {
        let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        let cursor = unsafe { LoadCursorW(0, cursor_name(binding.cursor_icon)) };

        // evaluate this condition
        if cursor != 0 {
            unsafe {
                SetCursor(cursor);
            }
            return 1;
        }
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
    ) && let Some(entry) = runtime_window_entry(hwnd)
    {
        apply_window_message_snapshot(&entry, message, wparam, lparam);
    }

    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

/// Ensure the display window class is registered.
pub(super) fn ensure_window_class_registered() -> RuntimeResult<()> {
    // resolve process module instance for class registration
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) } as HINSTANCE;

    // evaluate this condition
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

        // evaluate this condition
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
