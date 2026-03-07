use wayland_client::{Connection, Proxy};
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1;
use wayland_protocols::wp::pointer_constraints::zv1::client::{
    zwp_confined_pointer_v1, zwp_locked_pointer_v1, zwp_pointer_constraints_v1,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{WindowCursorIcon, WindowCursorMode};
use crate::runtime::BindingCallContext;

use super::{
    WaylandConnectionDispatchState, flush_queue, resolve_wl_surface, with_connection_dispatch,
};
use crate::platform::display::unix::wayland::model::WaylandWindowHostState;

/// Clear pointer-focus state when one focused surface is being destroyed.
pub(crate) fn clear_pointer_focus_for_surface(
    dispatch_state: &mut WaylandConnectionDispatchState,
    surface_id: &wayland_client::backend::ObjectId,
) {
    let is_focused_surface = dispatch_state
        .input
        .pointer_focus_surface
        .as_ref()
        .is_some_and(|value| value == surface_id);
    if !is_focused_surface {
        return;
    }

    dispatch_state.input.pointer_focus_surface = None;
    dispatch_state.input.last_pointer_enter_serial = None;
    dispatch_state.input.last_pointer_button_serial = None;
}

/// Return one cursor-shape protocol code for one runtime cursor icon.
fn cursor_shape_code(icon: WindowCursorIcon) -> wp_cursor_shape_device_v1::Shape {
    match icon {
        WindowCursorIcon::Default => wp_cursor_shape_device_v1::Shape::Default,
        WindowCursorIcon::Text => wp_cursor_shape_device_v1::Shape::Text,
        WindowCursorIcon::Crosshair => wp_cursor_shape_device_v1::Shape::Crosshair,
        WindowCursorIcon::Pointer => wp_cursor_shape_device_v1::Shape::Pointer,
        WindowCursorIcon::Help => wp_cursor_shape_device_v1::Shape::Help,
        WindowCursorIcon::Wait => wp_cursor_shape_device_v1::Shape::Wait,
        WindowCursorIcon::Progress => wp_cursor_shape_device_v1::Shape::Progress,
        WindowCursorIcon::Move => wp_cursor_shape_device_v1::Shape::AllResize,
        WindowCursorIcon::NotAllowed => wp_cursor_shape_device_v1::Shape::NotAllowed,
        WindowCursorIcon::Grab => wp_cursor_shape_device_v1::Shape::Grab,
        WindowCursorIcon::Grabbing => wp_cursor_shape_device_v1::Shape::Grabbing,
        WindowCursorIcon::EResize => wp_cursor_shape_device_v1::Shape::EResize,
        WindowCursorIcon::NResize => wp_cursor_shape_device_v1::Shape::NResize,
        WindowCursorIcon::NeResize => wp_cursor_shape_device_v1::Shape::NeResize,
        WindowCursorIcon::NwResize => wp_cursor_shape_device_v1::Shape::NwResize,
        WindowCursorIcon::SResize => wp_cursor_shape_device_v1::Shape::SResize,
        WindowCursorIcon::SeResize => wp_cursor_shape_device_v1::Shape::SeResize,
        WindowCursorIcon::SwResize => wp_cursor_shape_device_v1::Shape::SwResize,
        WindowCursorIcon::WResize => wp_cursor_shape_device_v1::Shape::WResize,
        WindowCursorIcon::EwResize => wp_cursor_shape_device_v1::Shape::EwResize,
        WindowCursorIcon::NsResize => wp_cursor_shape_device_v1::Shape::NsResize,
        WindowCursorIcon::NeswResize => wp_cursor_shape_device_v1::Shape::NeswResize,
        WindowCursorIcon::NwseResize => wp_cursor_shape_device_v1::Shape::NwseResize,
        WindowCursorIcon::ColResize => wp_cursor_shape_device_v1::Shape::ColResize,
        WindowCursorIcon::RowResize => wp_cursor_shape_device_v1::Shape::RowResize,
        WindowCursorIcon::AllScroll => wp_cursor_shape_device_v1::Shape::AllScroll,
        WindowCursorIcon::ZoomIn => wp_cursor_shape_device_v1::Shape::ZoomIn,
        WindowCursorIcon::ZoomOut => wp_cursor_shape_device_v1::Shape::ZoomOut,
    }
}

/// Return whether one cursor policy should hide the pointer.
fn should_hide_cursor(cursor_visible: bool, cursor_mode: WindowCursorMode) -> bool {
    !cursor_visible
        || matches!(
            cursor_mode,
            WindowCursorMode::Hidden | WindowCursorMode::Locked
        )
}

/// Apply one pointer cursor policy on the currently focused wayland surface.
pub(crate) fn apply_pointer_cursor_state(
    dispatch_state: &WaylandConnectionDispatchState,
    cursor_visible: bool,
    cursor_mode: WindowCursorMode,
    cursor_icon: WindowCursorIcon,
) {
    let Some(pointer) = dispatch_state.input.pointer.as_ref() else {
        return;
    };
    let Some(serial) = dispatch_state.input.last_pointer_enter_serial else {
        return;
    };

    // hide the pointer using wl_pointer.set_cursor
    if should_hide_cursor(cursor_visible, cursor_mode) {
        pointer.set_cursor(serial, None, 0, 0);
        return;
    }

    // otherwise apply one compositor-managed cursor shape
    let Some(shape_device) = dispatch_state.input.cursor_shape_device.as_ref() else {
        return;
    };
    shape_device.set_shape(serial, cursor_shape_code(cursor_icon));
}

/// Destroy one locked-pointer object id when it is valid.
fn destroy_locked_pointer(
    connection: &Connection,
    locked_pointer_id: Option<wayland_client::backend::ObjectId>,
) {
    let Some(locked_pointer_id) = locked_pointer_id else {
        return;
    };
    if locked_pointer_id.is_null() {
        return;
    }

    if let Ok(locked_pointer) =
        zwp_locked_pointer_v1::ZwpLockedPointerV1::from_id(connection, locked_pointer_id)
    {
        locked_pointer.destroy();
    }
}

/// Destroy one confined-pointer object id when it is valid.
fn destroy_confined_pointer(
    connection: &Connection,
    confined_pointer_id: Option<wayland_client::backend::ObjectId>,
) {
    let Some(confined_pointer_id) = confined_pointer_id else {
        return;
    };
    if confined_pointer_id.is_null() {
        return;
    }

    if let Ok(confined_pointer) =
        zwp_confined_pointer_v1::ZwpConfinedPointerV1::from_id(connection, confined_pointer_id)
    {
        confined_pointer.destroy();
    }
}

/// Apply cursor visibility, icon, and mode policy for one window host state.
pub(crate) fn apply_window_cursor_policy(
    context: &BindingCallContext,
    host_state: &mut WaylandWindowHostState,
    operation: &'static str,
) -> RuntimeResult<()> {
    let surface_id = host_state.host.surface.clone();
    let cursor_visible = host_state.cursor_visible;
    let cursor_mode = host_state.cursor_mode;
    let cursor_icon = host_state.cursor_icon;
    let locked_pointer_id = host_state.host.locked_pointer.clone();
    let confined_pointer_id = host_state.host.confined_pointer.clone();

    let (next_locked_pointer_id, next_confined_pointer_id) = with_connection_dispatch(
        context,
        operation,
        |connection, event_queue, dispatch_state| {
            if !dispatch_state.supports_cursor_shape() {
                return Err(core_platform::not_supported(operation));
            }

            let surface = resolve_wl_surface(connection, surface_id.clone(), operation)?;

            // release active confine lane when mode no longer requires it
            if cursor_mode != WindowCursorMode::Confined {
                destroy_confined_pointer(connection, confined_pointer_id.clone());
            }

            // release active lock lane when mode no longer requires it
            if cursor_mode != WindowCursorMode::Locked {
                destroy_locked_pointer(connection, locked_pointer_id.clone());
            }

            let mut next_locked_pointer_id = locked_pointer_id.clone();
            let mut next_confined_pointer_id = confined_pointer_id.clone();

            // apply cursor lock through pointer-constraints
            if cursor_mode == WindowCursorMode::Locked {
                let pointer_constraints_manager = dispatch_state
                    .input
                    .pointer_constraints_manager
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| core_platform::not_supported(operation))?;
                let pointer = dispatch_state
                    .input
                    .pointer
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| core_platform::not_supported(operation))?;

                if next_locked_pointer_id.is_none() {
                    let queue_handle = event_queue.handle();
                    let locked_pointer = pointer_constraints_manager.lock_pointer(
                        &surface,
                        &pointer,
                        None,
                        zwp_pointer_constraints_v1::Lifetime::Persistent,
                        &queue_handle,
                        (),
                    );
                    next_locked_pointer_id = Some(locked_pointer.id());
                }
                next_confined_pointer_id = None;
            }

            // apply cursor confine through pointer-constraints
            if cursor_mode == WindowCursorMode::Confined {
                let pointer_constraints_manager = dispatch_state
                    .input
                    .pointer_constraints_manager
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| core_platform::not_supported(operation))?;
                let pointer = dispatch_state
                    .input
                    .pointer
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| core_platform::not_supported(operation))?;

                if next_confined_pointer_id.is_none() {
                    let queue_handle = event_queue.handle();
                    let confined_pointer = pointer_constraints_manager.confine_pointer(
                        &surface,
                        &pointer,
                        None,
                        zwp_pointer_constraints_v1::Lifetime::Persistent,
                        &queue_handle,
                        (),
                    );
                    next_confined_pointer_id = Some(confined_pointer.id());
                }
                next_locked_pointer_id = None;
            }

            let is_focused_surface = dispatch_state
                .input
                .pointer_focus_surface
                .as_ref()
                .is_some_and(|value| value == &surface_id);

            // apply pointer icon and visibility for focused surfaces only
            if is_focused_surface {
                apply_pointer_cursor_state(
                    dispatch_state,
                    cursor_visible,
                    cursor_mode,
                    cursor_icon,
                );
            }

            flush_queue(event_queue, operation)?;

            Ok((next_locked_pointer_id, next_confined_pointer_id))
        },
    )?;

    host_state.host.locked_pointer = next_locked_pointer_id;
    host_state.host.confined_pointer = next_confined_pointer_id;

    Ok(())
}
