use std::sync::Arc;

use x11rb::protocol::Event;
use x11rb::protocol::xproto::Visibility;

use crate::platform::display::{
    WindowLogicalSize, WindowOcclusionState, WindowPhysicalSize, WindowPosition, WindowVisibility,
};

use super::connection::X11ConnectionState;
use super::runtime::X11RuntimeState;
use crate::platform::display::unix::x11::event;
use crate::platform::display::unix::x11::window::{
    WINDOW_WM_STATE_ICONIC, WINDOW_WM_STATE_WITHDRAWN, clear_xdnd_state, handle_xdnd_drop,
    handle_xdnd_enter, handle_xdnd_leave, handle_xdnd_position, handle_xdnd_selection_notify,
    query_window_wm_state, refresh_net_wm_state,
};

/// Return whether one x11 event changes published monitor topology.
pub(crate) fn event_updates_monitor_topology(event: &Event) -> bool {
    matches!(
        event,
        Event::RandrNotify(_) | Event::RandrScreenChangeNotify(_)
    )
}

/// Handle one x11 event and publish runtime display events.
pub(crate) fn handle_x11_event(
    runtime_state: &Arc<X11RuntimeState>,
    connection_state: &X11ConnectionState,
    event_value: Event,
) {
    // map events that carry one window id into a runtime handle
    let xid = match event_value {
        Event::ClientMessage(value) => Some(value.window),
        Event::DestroyNotify(value) => Some(value.window),
        Event::ConfigureNotify(value) => Some(value.window),
        Event::KeyPress(value) => Some(value.event),
        Event::MapNotify(value) => Some(value.window),
        Event::UnmapNotify(value) => Some(value.window),
        Event::FocusIn(value) => Some(value.event),
        Event::FocusOut(value) => Some(value.event),
        Event::Expose(value) => Some(value.window),
        Event::VisibilityNotify(value) => Some(value.window),
        Event::SelectionNotify(value) => Some(value.requestor),
        Event::PropertyNotify(value) => Some(value.window),
        _ => None,
    };

    let Some(xid) = xid else {
        return;
    };
    let Some(dispatch_entry) = runtime_state.window_dispatch_entry(xid) else {
        return;
    };
    let window_handle = dispatch_entry.window;

    // resolve one mutable host state and publish state deltas
    let Some(host_state) = dispatch_entry.host_state.upgrade() else {
        runtime_state.unregister_xid(xid);
        return;
    };
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // process one text-driving key press before the general window state logic
    if let Event::KeyPress(value) = &event_value {
        match super::super::text::handle_window_text_key_press(
            runtime_state,
            connection_state,
            window_handle,
            value,
        ) {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                runtime_state.diagnostics.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("x11 text key handling failed: {error}"),
                    None,
                );
            }
        }
    }

    // synchronize one native text host focus transition before publishing window focus state
    if let Event::FocusIn(value) = &event_value {
        if let Err(error) = super::super::text::handle_window_text_focus_in(
            runtime_state,
            connection_state,
            window_handle,
            value,
        ) {
            runtime_state.diagnostics.warn(
                "display",
                "destack.display.window.eventRead",
                format!("x11 text focus-in handling failed: {error}"),
                None,
            );
        }
    }

    if let Event::FocusOut(value) = &event_value {
        if let Err(error) = super::super::text::handle_window_text_focus_out(
            runtime_state,
            connection_state,
            window_handle,
            value,
        ) {
            runtime_state.diagnostics.warn(
                "display",
                "destack.display.window.eventRead",
                format!("x11 text focus-out handling failed: {error}"),
                None,
            );
        }
    }

    // process one client-message event
    if let Event::ClientMessage(value) = event_value {
        // process one WM_DELETE_WINDOW request from the window manager
        if value.type_ == connection_state.atoms.wm_protocols
            && value.format == 32
            && value.data.as_data32()[0] == connection_state.atoms.wm_delete_window
            && !host_state.close_requested_emitted
        {
            host_state.close_requested_emitted = true;
            drop(host_state);
            event::publish_window_close_requested(runtime_state, window_handle);
            return;
        }

        // xdnd lifecycle
        if value.type_ == connection_state.atoms.xdnd_enter {
            if let Err(error) = handle_xdnd_enter(connection_state, &mut host_state, &value) {
                runtime_state.diagnostics.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("xdnd enter handling failed: {error}"),
                    None,
                );
                clear_xdnd_state(&mut host_state);
            }
            return;
        }
        if value.type_ == connection_state.atoms.xdnd_position {
            if let Err(error) = handle_xdnd_position(connection_state, &mut host_state, &value) {
                runtime_state.diagnostics.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("xdnd position handling failed: {error}"),
                    None,
                );
                clear_xdnd_state(&mut host_state);
            }
            return;
        }
        if value.type_ == connection_state.atoms.xdnd_drop {
            if let Err(error) = handle_xdnd_drop(
                runtime_state,
                window_handle,
                connection_state,
                &mut host_state,
                &value,
            ) {
                runtime_state.diagnostics.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("xdnd drop handling failed: {error}"),
                    None,
                );
                clear_xdnd_state(&mut host_state);
            }
            return;
        }
        if value.type_ == connection_state.atoms.xdnd_leave {
            handle_xdnd_leave(runtime_state, window_handle, &mut host_state);
        }
        return;
    }

    // process one xdnd selection notify message
    if let Event::SelectionNotify(value) = event_value {
        if let Err(error) = handle_xdnd_selection_notify(
            runtime_state,
            window_handle,
            connection_state,
            &mut host_state,
            &value,
        ) {
            runtime_state.diagnostics.warn(
                "display",
                "destack.display.window.eventRead",
                format!("xdnd selection handling failed: {error}"),
                None,
            );
            clear_xdnd_state(&mut host_state);
        }
        return;
    }

    // process one property-notify update for wm-managed state
    if let Event::PropertyNotify(value) = event_value {
        if value.atom == connection_state.atoms.net_wm_state {
            let previous = host_state.clone();
            if let Err(error) = refresh_net_wm_state(
                connection_state,
                &mut host_state,
                "destack.display.window.eventRead",
            ) {
                runtime_state.diagnostics.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("net wm state query failed: {error}"),
                    None,
                );
                return;
            }
            let current = host_state.clone();
            drop(host_state);
            event::publish_state_deltas(runtime_state, window_handle, &previous, &current);
        }
        return;
    }

    // process one window-destroy event
    if let Event::DestroyNotify(_) = event_value {
        if !host_state.destroyed_emitted {
            let removed_text_context = runtime_state
                .text_input_contexts
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .remove(&window_handle);

            host_state.destroyed_emitted = true;
            host_state.visibility = WindowVisibility::Hidden;
            host_state.focused = false;
            runtime_state.unregister_xid(xid);
            drop(host_state);

            if removed_text_context
                .as_ref()
                .is_some_and(|context| context.is_composing || !context.composition_text.is_empty())
            {
                let _ = crate::platform::input::host::notify_x11_window_end_composition(
                    runtime_state,
                    window_handle,
                );
            }

            runtime_state
                .active_text_sessions
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .remove(&window_handle);

            event::publish_window_destroyed(runtime_state, window_handle);
        }
        return;
    }

    // process one expose event
    if let Event::Expose(_) = event_value {
        drop(host_state);
        event::publish_window_refresh_requested(runtime_state, window_handle);
        return;
    }

    // process one focus-gained event
    if let Event::FocusIn(_) = event_value {
        if !host_state.focused {
            let previous = host_state.clone();
            host_state.focused = true;
            let current = host_state.clone();
            drop(host_state);
            event::publish_state_deltas(runtime_state, window_handle, &previous, &current);
        }
        return;
    }

    // process one focus-lost event
    if let Event::FocusOut(_) = event_value {
        if host_state.focused {
            let previous = host_state.clone();
            host_state.focused = false;
            let current = host_state.clone();
            drop(host_state);
            event::publish_state_deltas(runtime_state, window_handle, &previous, &current);
        }
        return;
    }

    // process one map-notify event
    if let Event::MapNotify(_) = event_value {
        if host_state.visibility == WindowVisibility::Hidden
            || host_state.visibility == WindowVisibility::Minimized
        {
            let previous = host_state.clone();
            host_state.visibility = WindowVisibility::Visible;
            let current = host_state.clone();
            drop(host_state);
            event::publish_state_deltas(runtime_state, window_handle, &previous, &current);
        }
        return;
    }

    // process one unmap-notify event
    if let Event::UnmapNotify(_) = event_value {
        let next_visibility = match query_window_wm_state(
            connection_state,
            xid,
            "destack.display.window.eventRead",
        ) {
            Ok(Some(WINDOW_WM_STATE_ICONIC)) => WindowVisibility::Minimized,
            Ok(Some(WINDOW_WM_STATE_WITHDRAWN) | None) => WindowVisibility::Hidden,
            Ok(Some(_)) => WindowVisibility::Hidden,
            Err(error) => {
                runtime_state.diagnostics.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("wm state query failed: {error}"),
                    None,
                );
                WindowVisibility::Hidden
            }
        };

        if host_state.visibility != next_visibility {
            let previous = host_state.clone();
            host_state.visibility = next_visibility;
            let current = host_state.clone();
            drop(host_state);
            event::publish_state_deltas(runtime_state, window_handle, &previous, &current);
        }
        return;
    }

    // process one visibility-notify event
    if let Event::VisibilityNotify(value) = event_value {
        let next_occlusion = if value.state == Visibility::UNOBSCURED {
            WindowOcclusionState::Unoccluded
        } else {
            WindowOcclusionState::Occluded
        };

        if host_state.occlusion != next_occlusion {
            let previous = host_state.clone();
            host_state.occlusion = next_occlusion;
            let current = host_state.clone();
            drop(host_state);
            event::publish_state_deltas(runtime_state, window_handle, &previous, &current);
        }
        return;
    }

    // process one configure-notify event
    if let Event::ConfigureNotify(value) = event_value {
        let previous = host_state.clone();
        let current_position = WindowPosition {
            x: i32::from(value.x),
            y: i32::from(value.y),
        };
        let current_size_physical = WindowPhysicalSize {
            width: u32::from(value.width),
            height: u32::from(value.height),
        };
        let current_size_logical = WindowLogicalSize {
            width: f64::from(value.width),
            height: f64::from(value.height),
        };

        host_state.position = current_position;
        host_state.size_logical = current_size_logical;
        host_state.size_physical = current_size_physical;
        let current = host_state.clone();

        drop(host_state);
        event::publish_state_deltas(runtime_state, window_handle, &previous, &current);
    }
}
