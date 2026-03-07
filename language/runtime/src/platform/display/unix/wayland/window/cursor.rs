use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowCursorIcon, WindowCursorMode, WindowPosition};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::{resolve_window_host_state, with_window_host_state_mut};
use crate::platform::display::unix::wayland::core as wayland_core;

/// Set cursor icon for one window.
pub(crate) unsafe fn window_set_cursor_icon(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    // resolve target window host state and update icon state
    with_window_host_state_mut(
        context,
        window_handle,
        "destack.display.window.setCursorIcon",
        |host_state| {
            let previous_icon = host_state.cursor_icon;
            host_state.cursor_icon = icon;
            let result = wayland_core::apply_window_cursor_policy(
                context,
                host_state,
                "destack.display.window.setCursorIcon",
            );
            if result.is_err() {
                host_state.cursor_icon = previous_icon;
            }

            result
        },
    )
}

/// Set cursor interaction mode for one window.
pub(crate) unsafe fn window_set_cursor_mode(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    mode: WindowCursorMode,
) -> RuntimeResult<()> {
    // resolve target window host state and update mode state
    with_window_host_state_mut(
        context,
        window_handle,
        "destack.display.window.setCursorMode",
        |host_state| {
            let previous_mode = host_state.cursor_mode;
            host_state.cursor_mode = mode;
            let result = wayland_core::apply_window_cursor_policy(
                context,
                host_state,
                "destack.display.window.setCursorMode",
            );
            if result.is_err() {
                host_state.cursor_mode = previous_mode;
            }

            result
        },
    )
}

/// Set cursor position for one window.
pub(crate) unsafe fn window_set_cursor_position(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    // resolve target window host state
    let host_state = resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setCursorPosition",
    )?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let surface_id = host_state.host.surface.clone();

    // request one compositor cursor warp through pointer-warp extension
    wayland_core::with_connection_dispatch(
        context,
        "destack.display.window.setCursorPosition",
        |connection, event_queue, dispatch_state| {
            // require one negotiated pointer-warp manager
            let pointer_warp_manager = dispatch_state
                .input
                .pointer_warp_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| {
                    core_platform::not_supported("destack.display.window.setCursorPosition")
                })?;

            // require one active pointer lane and one enter serial
            let pointer = dispatch_state
                .input
                .pointer
                .as_ref()
                .cloned()
                .ok_or_else(|| {
                    core_platform::not_supported("destack.display.window.setCursorPosition")
                })?;
            let serial = dispatch_state
                .input
                .last_pointer_enter_serial
                .ok_or_else(|| {
                    core_platform::io_would_block(
                        "destack.display.window.setCursorPosition",
                        "cursor warp requires one recent pointer enter serial",
                    )
                })?;

            // resolve the target surface and issue the warp request
            let surface = wayland_core::resolve_wl_surface(
                connection,
                surface_id,
                "destack.display.window.setCursorPosition",
            )?;
            pointer_warp_manager.warp_pointer(
                &surface,
                &pointer,
                position.x as f64,
                position.y as f64,
                serial,
            );
            wayland_core::flush_queue(event_queue, "destack.display.window.setCursorPosition")?;

            Ok(())
        },
    )
}

/// Set cursor visibility for one window.
pub(crate) unsafe fn window_set_cursor_visible(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve target window host state and update visibility state
    with_window_host_state_mut(
        context,
        window_handle,
        "destack.display.window.setCursorVisible",
        |host_state| {
            let previous_visible = host_state.cursor_visible;
            host_state.cursor_visible = visible;
            let result = wayland_core::apply_window_cursor_policy(
                context,
                host_state,
                "destack.display.window.setCursorVisible",
            );
            if result.is_err() {
                host_state.cursor_visible = previous_visible;
            }

            result
        },
    )
}
