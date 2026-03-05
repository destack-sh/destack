use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowCursorIcon, WindowCursorMode, WindowPosition};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::core as backend_core;

/// Set cursor icon for one window.
pub(crate) unsafe fn window_set_cursor_icon(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    icon: WindowCursorIcon,
) -> RuntimeResult<()> {
    // resolve one owner-thread window binding and update icon state
    super::with_window_binding_mut(
        context,
        window_handle,
        "destack.display.window.setCursorIcon",
        |binding| {
            let previous_icon = binding.cursor_icon;
            binding.cursor_icon = icon;
            let result = backend_core::apply_window_cursor_policy(
                context,
                binding,
                "destack.display.window.setCursorIcon",
            );
            if result.is_err() {
                binding.cursor_icon = previous_icon;
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
    // resolve one owner-thread window binding and update mode state
    super::with_window_binding_mut(
        context,
        window_handle,
        "destack.display.window.setCursorMode",
        |binding| {
            let previous_mode = binding.cursor_mode;
            binding.cursor_mode = mode;
            let result = backend_core::apply_window_cursor_policy(
                context,
                binding,
                "destack.display.window.setCursorMode",
            );
            if result.is_err() {
                binding.cursor_mode = previous_mode;
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
    // resolve one owner-thread window binding
    let binding = super::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setCursorPosition",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    let surface_id = binding.host.surface.clone();

    // request one compositor cursor warp through pointer-warp extension
    backend_core::with_connection_dispatch(
        context,
        "destack.display.window.setCursorPosition",
        |connection, event_queue, dispatch_state| {
            // require one negotiated pointer-warp manager
            let pointer_warp_manager = dispatch_state
                .pointer_warp_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| {
                    core_platform::not_supported("destack.display.window.setCursorPosition")
                })?;

            // require one active pointer lane and one enter serial
            let pointer = dispatch_state.pointer.as_ref().cloned().ok_or_else(|| {
                core_platform::not_supported("destack.display.window.setCursorPosition")
            })?;
            let serial = dispatch_state.last_pointer_enter_serial.ok_or_else(|| {
                core_platform::io_would_block(
                    "destack.display.window.setCursorPosition",
                    "cursor warp requires one recent pointer enter serial",
                )
            })?;

            // resolve this window surface and issue one warp request
            let surface = backend_core::resolve_wl_surface(
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
            backend_core::flush_queue(event_queue, "destack.display.window.setCursorPosition")?;

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
    // resolve one owner-thread window binding and update visibility state
    super::with_window_binding_mut(
        context,
        window_handle,
        "destack.display.window.setCursorVisible",
        |binding| {
            let previous_visible = binding.cursor_visible;
            binding.cursor_visible = visible;
            let result = backend_core::apply_window_cursor_policy(
                context,
                binding,
                "destack.display.window.setCursorVisible",
            );
            if result.is_err() {
                binding.cursor_visible = previous_visible;
            }

            result
        },
    )
}
