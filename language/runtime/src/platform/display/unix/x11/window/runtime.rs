use std::sync::Arc;

use x11rb::protocol::Event;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowLogicalSize, WindowPhysicalSize, WindowPosition, WindowVisibility,
};
use crate::runtime::BindingCallContext;

use super::super::model::X11WindowBinding;
use super::super::{core, event, resource as display_resource};
use super::constants::*;
use super::drop::{
    clear_xdnd_state, handle_xdnd_drop, handle_xdnd_enter, handle_xdnd_leave, handle_xdnd_position,
    handle_xdnd_selection_notify,
};

/// Drain pending x11 events and publish runtime event deltas.
pub(in super::super) fn pump_window_messages(context: &BindingCallContext) -> RuntimeResult<()> {
    // resolve runtime and connection state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.eventRead")?;

    // drain pending events until queue is empty
    loop {
        let event = connection_state
            .connection
            .poll_for_event()
            .map_err(|error| {
                core::io_error(
                    "destack.display.window.eventRead",
                    format!("poll_for_event failed: {error}"),
                )
            })?;
        let Some(event) = event else {
            break;
        };

        // dispatch one x11 event into runtime state updates
        handle_x11_event(context, &runtime_state, connection_state.as_ref(), event);
    }

    Ok(())
}

/// Handle one x11 event and publish runtime display events.
fn handle_x11_event(
    context: &BindingCallContext,
    runtime_state: &Arc<core::X11RuntimeState>,
    connection_state: &core::X11ConnectionState,
    event_value: Event,
) {
    // map events that carry one window id into a runtime handle
    let xid = match event_value {
        Event::ClientMessage(value) => Some(value.window),
        Event::DestroyNotify(value) => Some(value.window),
        Event::ConfigureNotify(value) => Some(value.window),
        Event::MapNotify(value) => Some(value.window),
        Event::UnmapNotify(value) => Some(value.window),
        Event::FocusIn(value) => Some(value.event),
        Event::FocusOut(value) => Some(value.event),
        Event::Expose(value) => Some(value.window),
        Event::VisibilityNotify(value) => Some(value.window),
        Event::SelectionNotify(value) => Some(value.requestor),
        _ => None,
    };
    let Some(xid) = xid else {
        return;
    };
    let Some(window_handle) = event::resolve_window_by_xid(runtime_state, xid) else {
        return;
    };

    // resolve one mutable binding and publish state deltas
    let binding = match display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.eventRead",
    ) {
        Ok(binding) => binding,
        Err(_) => return,
    };
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    // process one client-message event
    if let Event::ClientMessage(value) = event_value {
        // process one WM_DELETE_WINDOW request from the window manager
        if value.type_ == connection_state.atoms.wm_protocols
            && value.format == 32
            && value.data.as_data32()[0] == connection_state.atoms.wm_delete_window
            && !binding.close_requested_emitted
        {
            binding.close_requested_emitted = true;
            drop(binding);
            event::publish_window_close_requested(runtime_state, window_handle);
            return;
        }

        // process one xdnd enter message
        if value.type_ == connection_state.atoms.xdnd_enter {
            // evaluate this condition
            if let Err(error) = handle_xdnd_enter(connection_state, &mut binding, &value) {
                context.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("xdnd enter handling failed: {error}"),
                    None,
                );
                clear_xdnd_state(&mut binding);
            }
            return;
        }

        // process one xdnd position message
        if value.type_ == connection_state.atoms.xdnd_position {
            // evaluate this condition
            if let Err(error) = handle_xdnd_position(connection_state, &mut binding, &value) {
                context.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("xdnd position handling failed: {error}"),
                    None,
                );
                clear_xdnd_state(&mut binding);
            }
            return;
        }

        // process one xdnd drop message
        if value.type_ == connection_state.atoms.xdnd_drop {
            // evaluate this condition
            if let Err(error) = handle_xdnd_drop(
                runtime_state,
                window_handle,
                connection_state,
                &mut binding,
                &value,
            ) {
                context.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("xdnd drop handling failed: {error}"),
                    None,
                );
                clear_xdnd_state(&mut binding);
            }
            return;
        }

        // process one xdnd leave message
        if value.type_ == connection_state.atoms.xdnd_leave {
            handle_xdnd_leave(runtime_state, window_handle, &mut binding);
        }
        return;
    }

    // process one xdnd selection notify message
    if let Event::SelectionNotify(value) = event_value {
        // evaluate this condition
        if let Err(error) = handle_xdnd_selection_notify(
            runtime_state,
            window_handle,
            connection_state,
            &mut binding,
            &value,
        ) {
            context.warn(
                "display",
                "destack.display.window.eventRead",
                format!("xdnd selection handling failed: {error}"),
                None,
            );
            clear_xdnd_state(&mut binding);
        }
        return;
    }

    // process one window-destroy event
    if let Event::DestroyNotify(_) = event_value {
        // evaluate this condition
        if !binding.destroyed_emitted {
            binding.destroyed_emitted = true;
            binding.visibility = WindowVisibility::Hidden;
            binding.focused = false;
            event::unregister_xid(runtime_state, xid);
            drop(binding);
            event::publish_window_destroyed(runtime_state, window_handle);
        }
        return;
    }

    // process one expose event
    if let Event::Expose(_) = event_value {
        drop(binding);
        event::publish_window_refresh_requested(runtime_state, window_handle);
        return;
    }

    // process one focus-gained event
    if let Event::FocusIn(_) = event_value {
        // evaluate this condition
        if !binding.focused {
            let previous_focused = binding.focused;
            binding.focused = true;
            drop(binding);
            event::publish_window_focus_changed(
                runtime_state,
                window_handle,
                previous_focused,
                true,
            );
        }
        return;
    }

    // process one focus-lost event
    if let Event::FocusOut(_) = event_value {
        // evaluate this condition
        if binding.focused {
            let previous_focused = binding.focused;
            binding.focused = false;
            drop(binding);
            event::publish_window_focus_changed(
                runtime_state,
                window_handle,
                previous_focused,
                false,
            );
        }
        return;
    }

    // process one map-notify event
    if let Event::MapNotify(_) = event_value {
        // evaluate this condition
        if binding.visibility == WindowVisibility::Hidden
            || binding.visibility == WindowVisibility::Minimized
        {
            let previous_visibility = binding.visibility;
            binding.visibility = WindowVisibility::Visible;
            drop(binding);
            event::publish_window_visibility_changed(
                runtime_state,
                window_handle,
                previous_visibility,
                WindowVisibility::Visible,
            );
        }
        return;
    }

    // process one unmap-notify event
    if let Event::UnmapNotify(_) = event_value {
        // resolve one host wm-state value to preserve hidden vs minimized semantics
        let next_visibility = match query_window_wm_state(
            connection_state,
            xid,
            "destack.display.window.eventRead",
        ) {
            Ok(Some(WINDOW_WM_STATE_ICONIC)) => WindowVisibility::Minimized,
            Ok(Some(WINDOW_WM_STATE_WITHDRAWN) | None) => WindowVisibility::Hidden,
            Ok(Some(_)) => WindowVisibility::Hidden,
            Err(error) => {
                context.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("wm state query failed: {error}"),
                    None,
                );
                WindowVisibility::Hidden
            }
        };

        // evaluate this condition
        if binding.visibility != next_visibility {
            let previous_visibility = binding.visibility;
            binding.visibility = next_visibility;
            drop(binding);
            event::publish_window_visibility_changed(
                runtime_state,
                window_handle,
                previous_visibility,
                next_visibility,
            );
        }
        return;
    }

    // process one configure-notify event
    if let Event::ConfigureNotify(value) = event_value {
        let mut publish_position = None;
        let mut publish_size = None;

        let current_position = WindowPosition {
            x: i32::from(value.x),
            y: i32::from(value.y),
        };

        // evaluate this condition
        if binding.position != current_position {
            let previous_position = binding.position;
            binding.position = current_position;
            publish_position = Some((previous_position, current_position));
        }

        let current_size_physical = WindowPhysicalSize {
            width: value.width as u32,
            height: value.height as u32,
        };
        let current_size_logical = WindowLogicalSize {
            width: value.width as f64,
            height: value.height as f64,
        };

        // evaluate this condition
        if binding.size_physical != current_size_physical {
            let previous_size_logical = binding.size_logical;
            let previous_size_physical = binding.size_physical;
            binding.size_logical = current_size_logical;
            binding.size_physical = current_size_physical;
            publish_size = Some((
                previous_size_logical,
                previous_size_physical,
                current_size_logical,
                current_size_physical,
            ));
        }

        drop(binding);

        // evaluate this condition
        if let Some((previous_position, current_position)) = publish_position {
            event::publish_window_position_changed(
                runtime_state,
                window_handle,
                previous_position,
                current_position,
            );
        }

        // evaluate this condition
        if let Some((
            previous_size_logical,
            previous_size_physical,
            current_size_logical,
            current_size_physical,
        )) = publish_size
        {
            event::publish_window_size_changed(
                runtime_state,
                window_handle,
                previous_size_logical,
                previous_size_physical,
                current_size_logical,
                current_size_physical,
            );
        }
    }
}

/// Query one `WM_STATE` property and return the ICCCM state value when present.
fn query_window_wm_state(
    connection_state: &core::X11ConnectionState,
    window: u32,
    operation: &'static str,
) -> RuntimeResult<Option<u32>> {
    let reply = connection_state
        .connection
        .get_property(
            false,
            window,
            connection_state.atoms.wm_state,
            connection_state.atoms.wm_state,
            0,
            2,
        )
        .map_err(|error| {
            core::io_error(operation, format!("get_property request failed: {error}"))
        })?
        .reply()
        .map_err(|error| {
            core::io_error(operation, format!("get_property reply failed: {error}"))
        })?;

    // decode one optional first field from the wm_state payload
    let value = reply.value32().and_then(|mut values| values.next());
    Ok(value)
}
