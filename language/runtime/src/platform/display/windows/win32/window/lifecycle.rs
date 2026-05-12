use windows_sys::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, ICON_BIG, ICON_SMALL, SendMessageW, WM_SETICON,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::drop::unregister_window_drop_target;
use super::mode::restore_exclusive_mode;
use super::{destroy_owned_icons, restore_cursor_after_close, restore_modal_owner_on_close};
use crate::platform::display::windows::win32::core::Win32WindowDispatchEntry;
use crate::platform::display::windows::win32::{core, event, resource as display_resource};

/// Close one window.
pub(crate) unsafe fn window_close(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and snapshot the target window host state
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.close",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // restore exclusive mode if this window owns one
    if let Some(restore) = host_state.exclusive_restore.clone() {
        restore_exclusive_mode(context, &restore, "destack.display.window.close", true)?;
        host_state.exclusive_restore = None;
    }

    // restore process global side effects and detach the drop target
    let runtime_state = core::runtime_state(context);
    restore_cursor_after_close(&runtime_state, window);
    restore_modal_owner_on_close(context, &host_state);
    unregister_window_drop_target(&mut host_state);

    // clear and detach host icon handles
    let hwnd = host_state.hwnd;
    let previous_small_icon = host_state.icon_small;
    let previous_big_icon = host_state.icon_big;
    host_state.icon_small = 0;
    host_state.icon_big = 0;

    // capture lifecycle event emission state before dropping the lock
    let should_emit_close_requested = !host_state.close_requested_emitted;
    if should_emit_close_requested {
        host_state.close_requested_emitted = true;
    }
    drop(host_state);
    let runtime_state = core::runtime_state(context);

    // publish close requested when this is the first close path
    if should_emit_close_requested {
        event::publish_window_close_requested_event(&runtime_state, window);
    }

    // clear host icons and release owned handles
    unsafe {
        let _ = SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, 0);
        let _ = SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, 0);
    }
    destroy_owned_icons(previous_small_icon, previous_big_icon);

    // destroy the host window when it is still live
    if unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) } != 0 {
        let status = unsafe { DestroyWindow(hwnd) };
        if status == 0 {
            return Err(core::io_error(
                "destack.display.window.close",
                "DestroyWindow",
                "failed to destroy window",
            ));
        }
    }

    // emit destroyed once and clear the hwnd runtime registration
    let host_state = display_resource::resolve_window_host_state(
        context,
        window,
        "destack.display.window.close",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    if !host_state.destroyed_emitted {
        host_state.destroyed_emitted = true;
        drop(host_state);
        event::publish_window_destroyed_event(&runtime_state, window);
        Win32WindowDispatchEntry::unregister(hwnd);
    } else {
        drop(host_state);
    }

    // remove the finalized resource entry
    let removed = context.worker().resources.remove_and_finalize(
        context.world(),
        window.0,
        Some(context.engine()),
    );
    if !removed {
        return Err(core_platform::io_not_found(
            "destack.display.window.close",
            format!("window handle {} was not found", window.0.local_id),
        ));
    }

    Ok(())
}
