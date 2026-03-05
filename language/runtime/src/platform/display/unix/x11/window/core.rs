use std::sync::Arc;

use x11rb::connection::Connection;
use x11rb::errors::ConnectionError;
use x11rb::properties::{AspectRatio as X11AspectRatio, WmSizeHints};
use x11rb::protocol::Event;
use x11rb::protocol::shape::{ConnectionExt as ShapeConnectionExt, SK, SO};
use x11rb::protocol::xproto::{
    Atom, AtomEnum, CLIENT_MESSAGE_EVENT, ClientMessageData, ClientMessageEvent, ClipOrdering,
    ConnectionExt as XprotoConnectionExt, EventMask, PropMode, SelectionNotifyEvent,
};
use x11rb::wrapper::ConnectionExt as X11WrapperConnectionExt;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayMode, WindowAspectRatio, WindowChromeKind, WindowLogicalSize, WindowModeOptions,
    WindowPhysicalSize, WindowPosition, WindowSizeConstraints, WindowVisibility,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

#[path = "action.rs"]
mod action;
#[path = "appearance.rs"]
mod appearance;
#[path = "cursor.rs"]
mod cursor;
#[path = "drop.rs"]
mod drop;
#[path = "geometry.rs"]
mod geometry;
#[path = "icon.rs"]
mod icon;
#[path = "lifecycle.rs"]
mod lifecycle;
#[path = "relation.rs"]
mod relation;
#[path = "state.rs"]
mod state;

pub(in crate::platform::display::host::unix) use action::*;
pub(in crate::platform::display::host::unix) use appearance::*;
pub(in crate::platform::display::host::unix) use cursor::*;
pub(in crate::platform::display::host::unix) use geometry::*;
pub(in crate::platform::display::host::unix) use lifecycle::*;
pub(in crate::platform::display::host::unix) use relation::*;
pub(in crate::platform::display::host::unix) use state::*;

use self::drop::*;
use super::super::model::{X11WindowBinding, XdndPayload};
use super::super::{core, event, resource as display_resource};

/// `_MOTIF_WM_HINTS` flag bit for the `decorations` field.
const MOTIF_HINTS_DECORATIONS_FLAG: u32 = 1 << 1;
/// XDND acceptance bit in status and finished client messages.
const XDND_ACCEPTED: u32 = 1;

/// Drain pending x11 events and publish runtime event deltas.
pub(in super::super) fn pump_window_messages(binding: &BindingCallContext) -> RuntimeResult<()> {
    // resolve runtime and connection state
    let runtime_state = core::runtime_state(binding);
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
        handle_x11_event(binding, &runtime_state, connection_state.as_ref(), event);
    }

    Ok(())
}

/// Handle one x11 event and publish runtime display events.
fn handle_x11_event(
    binding: &BindingCallContext,
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

    // resolve one mutable resolved_binding and publish state deltas
    let resolved_binding = match display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.eventRead",
    ) {
        Ok(resolved_binding) => resolved_binding,
        Err(_) => return,
    };
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // process one client-message event
    if let Event::ClientMessage(value) = event_value {
        // process one WM_DELETE_WINDOW request from the window manager
        if value.type_ == connection_state.atoms.wm_protocols
            && value.format == 32
            && value.data.as_data32()[0] == connection_state.atoms.wm_delete_window
            && !resolved_binding.close_requested_emitted
        {
            resolved_binding.close_requested_emitted = true;
            drop(resolved_binding);
            event::publish_window_close_requested(runtime_state, window_handle);
            return;
        }

        // process one xdnd enter message
        if value.type_ == connection_state.atoms.xdnd_enter {
            // evaluate this condition
            if let Err(error) = handle_xdnd_enter(connection_state, &mut resolved_binding, &value) {
                binding.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("xdnd enter handling failed: {error}"),
                    None,
                );
                clear_xdnd_state(&mut resolved_binding);
            }
            return;
        }

        // process one xdnd position message
        if value.type_ == connection_state.atoms.xdnd_position {
            // evaluate this condition
            if let Err(error) =
                handle_xdnd_position(connection_state, &mut resolved_binding, &value)
            {
                binding.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("xdnd position handling failed: {error}"),
                    None,
                );
                clear_xdnd_state(&mut resolved_binding);
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
                &mut resolved_binding,
                &value,
            ) {
                binding.warn(
                    "display",
                    "destack.display.window.eventRead",
                    format!("xdnd drop handling failed: {error}"),
                    None,
                );
                clear_xdnd_state(&mut resolved_binding);
            }
            return;
        }

        // process one xdnd leave message
        if value.type_ == connection_state.atoms.xdnd_leave {
            handle_xdnd_leave(runtime_state, window_handle, &mut resolved_binding);
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
            &mut resolved_binding,
            &value,
        ) {
            binding.warn(
                "display",
                "destack.display.window.eventRead",
                format!("xdnd selection handling failed: {error}"),
                None,
            );
            clear_xdnd_state(&mut resolved_binding);
        }
        return;
    }

    // process one window-destroy event
    if let Event::DestroyNotify(_) = event_value {
        // evaluate this condition
        if !resolved_binding.destroyed_emitted {
            resolved_binding.destroyed_emitted = true;
            resolved_binding.visibility = WindowVisibility::Hidden;
            resolved_binding.focused = false;
            event::unregister_xid(runtime_state, xid);
            drop(resolved_binding);
            event::publish_window_destroyed(runtime_state, window_handle);
        }
        return;
    }

    // process one expose event
    if let Event::Expose(_) = event_value {
        drop(resolved_binding);
        event::publish_window_refresh_requested(runtime_state, window_handle);
        return;
    }

    // process one focus-gained event
    if let Event::FocusIn(_) = event_value {
        // evaluate this condition
        if !resolved_binding.focused {
            let previous_focused = resolved_binding.focused;
            resolved_binding.focused = true;
            drop(resolved_binding);
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
        if resolved_binding.focused {
            let previous_focused = resolved_binding.focused;
            resolved_binding.focused = false;
            drop(resolved_binding);
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
        if resolved_binding.visibility == WindowVisibility::Hidden {
            let previous_visibility = resolved_binding.visibility;
            resolved_binding.visibility = WindowVisibility::Visible;
            drop(resolved_binding);
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
        // evaluate this condition
        if resolved_binding.visibility != WindowVisibility::Hidden
            && resolved_binding.visibility != WindowVisibility::Minimized
        {
            let previous_visibility = resolved_binding.visibility;
            resolved_binding.visibility = WindowVisibility::Hidden;
            drop(resolved_binding);
            event::publish_window_visibility_changed(
                runtime_state,
                window_handle,
                previous_visibility,
                WindowVisibility::Hidden,
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
        if resolved_binding.position != current_position {
            let previous_position = resolved_binding.position;
            resolved_binding.position = current_position;
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
        if resolved_binding.size_physical != current_size_physical {
            let previous_size_logical = resolved_binding.size_logical;
            let previous_size_physical = resolved_binding.size_physical;
            resolved_binding.size_logical = current_size_logical;
            resolved_binding.size_physical = current_size_physical;
            publish_size = Some((
                previous_size_logical,
                previous_size_physical,
                current_size_logical,
                current_size_physical,
            ));
        }

        drop(resolved_binding);
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

/// Return the selected display handle lane for one window mode.
pub(super) fn mode_display(mode: WindowModeOptions) -> Option<resource::DisplayHandle> {
    // resolve this variant
    match mode {
        WindowModeOptions::WindowBorderlessModeOptions(value) => value.display,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(value) => Some(value.display),
        _ => None,
    }
}

/// Return the selected display mode lane for one window mode.
pub(super) fn mode_display_mode(mode: WindowModeOptions) -> Option<DisplayMode> {
    // resolve this variant
    match mode {
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(value) => value.display_mode,
        _ => None,
    }
}

/// Apply one fullscreen state lane through EWMH state messages.
pub(super) fn apply_fullscreen_state(
    connection_state: &core::X11ConnectionState,
    window: u32,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    let is_fullscreen = matches!(
        mode,
        WindowModeOptions::WindowBorderlessModeOptions(_)
            | WindowModeOptions::WindowExclusiveFullscreenModeOptions(_)
    );
    set_net_wm_state(
        connection_state,
        window,
        connection_state.atoms.net_wm_state_fullscreen,
        is_fullscreen,
    )?;

    Ok(())
}

/// Apply one maximized state lane through EWMH state messages.
pub(super) fn apply_maximized_state(
    connection_state: &core::X11ConnectionState,
    window: u32,
    enabled: bool,
) -> RuntimeResult<()> {
    set_net_wm_state(
        connection_state,
        window,
        connection_state.atoms.net_wm_state_maximized_horz,
        enabled,
    )?;
    set_net_wm_state(
        connection_state,
        window,
        connection_state.atoms.net_wm_state_maximized_vert,
        enabled,
    )?;

    Ok(())
}

/// Apply one decoration policy through `_MOTIF_WM_HINTS`.
pub(super) fn apply_window_decorated(
    connection_state: &core::X11ConnectionState,
    window: u32,
    decorated: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // encode one motif-hints payload with the decorations lane enabled
    let motif_hints = [
        MOTIF_HINTS_DECORATIONS_FLAG,
        0,
        // evaluate this condition
        if decorated { 1 } else { 0 },
        0,
        0,
    ];

    // write `_MOTIF_WM_HINTS` and flush the request stream
    connection_state
        .connection
        .change_property32(
            PropMode::REPLACE,
            window,
            connection_state.atoms.motif_wm_hints,
            connection_state.atoms.motif_wm_hints,
            &motif_hints,
        )
        .map_err(|error| core::io_error(operation, format!("change_property32 failed: {error}")))?;
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Apply one window chrome kind through `_NET_WM_WINDOW_TYPE`.
pub(super) fn apply_window_chrome(
    connection_state: &core::X11ConnectionState,
    window: u32,
    chrome: WindowChromeKind,
    operation: &'static str,
) -> RuntimeResult<()> {
    // map one chrome kind to one EWMH window-type atom
    let window_type = match chrome {
        WindowChromeKind::Standard => connection_state.atoms.net_wm_window_type_normal,
        WindowChromeKind::Tool => connection_state.atoms.net_wm_window_type_utility,
        WindowChromeKind::Popup => connection_state.atoms.net_wm_window_type_popup_menu,
    };

    // write one `_NET_WM_WINDOW_TYPE` value and flush the request stream
    connection_state
        .connection
        .change_property32(
            PropMode::REPLACE,
            window,
            connection_state.atoms.net_wm_window_type,
            AtomEnum::ATOM,
            &[window_type],
        )
        .map_err(|error| core::io_error(operation, format!("change_property32 failed: {error}")))?;
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Apply one normal-hints payload for resizable, constraint, and aspect-ratio lanes.
pub(super) fn apply_window_size_hints(
    connection_state: &core::X11ConnectionState,
    window: u32,
    resizable: bool,
    constraints: Option<WindowSizeConstraints>,
    aspect_ratio: Option<WindowAspectRatio>,
    current_size: WindowPhysicalSize,
    operation: &'static str,
) -> RuntimeResult<()> {
    // begin with one empty size-hints payload
    let mut hints = WmSizeHints::new();

    // lock min and max to the current size when the window is non-resizable
    if !resizable {
        let current_width = (current_size.width.max(1)).min(i32::MAX as u32) as i32;
        let current_height = (current_size.height.max(1)).min(i32::MAX as u32) as i32;
        hints.min_size = Some((current_width, current_height));
        hints.max_size = Some((current_width, current_height));
    }
    // otherwise apply explicit min and max constraints when present
    else if let Some(constraints) = constraints {
        // evaluate this condition
        if let Some(minimum) = constraints.min {
            let min_width = normalized_constraint_component(minimum.width);
            let min_height = normalized_constraint_component(minimum.height);
            hints.min_size = Some((min_width, min_height));
        }

        // evaluate this condition
        if let Some(maximum) = constraints.max {
            let max_width = normalized_constraint_component(maximum.width);
            let max_height = normalized_constraint_component(maximum.height);
            hints.max_size = Some((max_width, max_height));
        }
    }

    // apply one fixed aspect-ratio lane when requested
    if let Some(aspect_ratio) = aspect_ratio {
        let numerator = (aspect_ratio.numerator.max(1)).min(i32::MAX as u32) as i32;
        let denominator = (aspect_ratio.denominator.max(1)).min(i32::MAX as u32) as i32;
        let ratio = X11AspectRatio::new(numerator, denominator);
        hints.aspect = Some((ratio, ratio));
    }

    // write WM normal hints and flush request bytes
    let cookie = hints
        .set_normal_hints(connection_state.connection.as_ref(), window)
        .map_err(|error| core::io_error(operation, format!("set_normal_hints failed: {error}")))?;
    cookie.check().map_err(|error| {
        core::io_error(operation, format!("set_normal_hints check failed: {error}"))
    })?;
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Apply one mouse-passthrough input-shape policy through the shape extension.
pub(super) fn apply_window_mouse_passthrough(
    connection_state: &core::X11ConnectionState,
    window: u32,
    passthrough: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // apply one empty input region for passthrough windows
    if passthrough {
        let request = connection_state.connection.shape_rectangles(
            SO::SET,
            SK::INPUT,
            ClipOrdering::UNSORTED,
            window,
            0,
            0,
            &[],
        );
        let cookie = match request {
            Ok(cookie) => cookie,
            Err(ConnectionError::UnsupportedExtension) => {
                return Err(core_platform::not_supported(operation));
            }
            Err(error) => {
                return Err(core::io_error(
                    operation,
                    format!("shape_rectangles failed: {error}"),
                ));
            }
        };
        cookie.check().map_err(|error| {
            core::io_error(operation, format!("shape_rectangles check failed: {error}"))
        })?;
    }
    // otherwise restore default window input shape
    else {
        let request =
            connection_state
                .connection
                .shape_mask(SO::SET, SK::INPUT, window, 0, 0, 0u32);
        let cookie = match request {
            Ok(cookie) => cookie,
            Err(ConnectionError::UnsupportedExtension) => return Ok(()),
            Err(error) => {
                return Err(core::io_error(
                    operation,
                    format!("shape_mask failed: {error}"),
                ));
            }
        };
        cookie.check().map_err(|error| {
            core::io_error(operation, format!("shape_mask check failed: {error}"))
        })?;
    }

    // flush one shape mutation request
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Apply one transient-owner relationship through `WM_TRANSIENT_FOR`.
pub(super) fn apply_window_transient_owner(
    connection_state: &core::X11ConnectionState,
    window: u32,
    owner_window: Option<u32>,
    operation: &'static str,
) -> RuntimeResult<()> {
    // write or clear the transient-owner property
    if let Some(owner_window) = owner_window {
        connection_state
            .connection
            .change_property32(
                PropMode::REPLACE,
                window,
                AtomEnum::WM_TRANSIENT_FOR,
                AtomEnum::WINDOW,
                &[owner_window],
            )
            .map_err(|error| {
                core::io_error(operation, format!("change_property32 failed: {error}"))
            })?;
    } else {
        connection_state
            .connection
            .delete_property(window, AtomEnum::WM_TRANSIENT_FOR.into())
            .map_err(|error| {
                core::io_error(operation, format!("delete_property failed: {error}"))
            })?;
    }

    // flush one transient-owner update
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Set one `_NET_WM_STATE` atom lane through one client message.
pub(super) fn set_net_wm_state(
    connection_state: &core::X11ConnectionState,
    window: u32,
    atom: u32,
    enabled: bool,
) -> RuntimeResult<()> {
    let action = if enabled { 1 } else { 0 };
    let data = ClientMessageData::from([action, atom, 0, 0, 0]);
    let event = ClientMessageEvent {
        response_type: CLIENT_MESSAGE_EVENT,
        format: 32,
        sequence: 0,
        window,
        type_: connection_state.atoms.net_wm_state,
        data,
    };

    connection_state
        .connection
        .send_event(
            false,
            connection_state.root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.setMode",
                format!("send_event failed: {error}"),
            )
        })?;
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.setMode",
            format!("flush failed: {error}"),
        )
    })?;

    Ok(())
}

/// Return one normalized i32 constraint component.
fn normalized_constraint_component(value: f64) -> i32 {
    value.round().clamp(1.0, i32::MAX as f64).max(1.0) as i32
}

/// Set one window title across ICCCM and EWMH properties.
pub(super) fn set_window_title(
    connection_state: &core::X11ConnectionState,
    window: u32,
    title: &str,
) -> RuntimeResult<()> {
    connection_state
        .connection
        .change_property8(
            PropMode::REPLACE,
            window,
            connection_state.atoms.wm_name,
            AtomEnum::STRING,
            title.as_bytes(),
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.setTitle",
                format!("change_property8 WM_NAME failed: {error}"),
            )
        })?;
    connection_state
        .connection
        .change_property8(
            PropMode::REPLACE,
            window,
            connection_state.atoms.net_wm_name,
            connection_state.atoms.utf8_string,
            title.as_bytes(),
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.setTitle",
                format!("change_property8 _NET_WM_NAME failed: {error}"),
            )
        })?;

    Ok(())
}

/// Enforce owner-thread affinity for one window binding.
pub(super) fn ensure_window_thread(
    binding: &X11WindowBinding,
    operation: &'static str,
) -> RuntimeResult<()> {
    let current_thread_id = std::thread::current().id();
    // evaluate this condition
    if binding.owner_thread_id == current_thread_id {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        "window",
        format!("{operation} must run on the owner thread for this window"),
    ))
}
