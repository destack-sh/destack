use crate::diagnostic::RuntimeResult;
use crate::platform::display::unix::x11::model::X11WindowHostState;
use crate::platform::display::{DisplayMode, WindowModeOptions};
use crate::platform::resource;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, CLIENT_MESSAGE_EVENT, ClientMessageData, ClientMessageEvent,
    ConnectionExt as XprotoConnectionExt, EventMask,
};

use super::constants::*;
use crate::platform::display::unix::x11::core;

/// Return the selected display handle lane for one window mode.
pub(crate) fn mode_display(mode: WindowModeOptions) -> Option<resource::DisplayHandle> {
    match mode {
        WindowModeOptions::WindowBorderlessModeOptions(value) => value.display,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(value) => Some(value.display),
        _ => None,
    }
}

/// Return the selected display mode lane for one window mode.
pub(crate) fn mode_display_mode(mode: WindowModeOptions) -> Option<DisplayMode> {
    match mode {
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(value) => value.display_mode,
        _ => None,
    }
}

/// Return whether two window-mode payloads describe the same effective mode.
pub(crate) fn same_window_mode(left: WindowModeOptions, right: WindowModeOptions) -> bool {
    mode_display(left) == mode_display(right) && mode_display_mode(left) == mode_display_mode(right)
}

/// Apply one fullscreen state lane through EWMH state messages.
pub(crate) fn apply_fullscreen_state(
    connection_state: &core::X11ConnectionState,
    window: u32,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    // resolve whether this mode requests fullscreen semantics
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
pub(crate) fn apply_maximized_state(
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

/// Set one `_NET_WM_STATE` atom lane through one client message.
pub(crate) fn set_net_wm_state(
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

/// Request one ICCCM `IconicState` transition through `WM_CHANGE_STATE`.
pub(crate) fn request_window_minimize(
    connection_state: &core::X11ConnectionState,
    window: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    let event = ClientMessageEvent {
        response_type: CLIENT_MESSAGE_EVENT,
        format: 32,
        sequence: 0,
        window,
        type_: connection_state.atoms.wm_change_state,
        data: ClientMessageData::from([WINDOW_WM_STATE_ICONIC, 0, 0, 0, 0]),
    };

    // ask the window manager to iconify this toplevel
    connection_state
        .connection
        .send_event(
            false,
            connection_state.root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            event,
        )
        .map_err(|error| core::io_error(operation, format!("send_event failed: {error}")))?;

    Ok(())
}

/// Query one ICCCM `WM_STATE` value for the target x11 window.
pub(crate) fn query_window_wm_state(
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

    Ok(reply.value32().and_then(|mut value| value.next()))
}

/// Query the current `_NET_WM_STATE` atom set for one window.
pub(crate) fn query_net_wm_state_atoms(
    connection_state: &core::X11ConnectionState,
    window: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<u32>> {
    let reply = connection_state
        .connection
        .get_property(
            false,
            window,
            connection_state.atoms.net_wm_state,
            AtomEnum::ATOM,
            0,
            64,
        )
        .map_err(|error| {
            core::io_error(operation, format!("get_property request failed: {error}"))
        })?
        .reply()
        .map_err(|error| {
            core::io_error(operation, format!("get_property reply failed: {error}"))
        })?;

    Ok(reply
        .value32()
        .map(|values| values.collect::<Vec<_>>())
        .unwrap_or_default())
}

/// Refresh one cached X11 window state from `_NET_WM_STATE`.
pub(crate) fn refresh_net_wm_state(
    connection_state: &core::X11ConnectionState,
    host_state: &mut X11WindowHostState,
    operation: &'static str,
) -> RuntimeResult<()> {
    let state_atoms = query_net_wm_state_atoms(connection_state, host_state.window, operation)?;
    let is_fullscreen = state_atoms
        .iter()
        .any(|atom| *atom == connection_state.atoms.net_wm_state_fullscreen);

    host_state.always_on_top = state_atoms
        .iter()
        .any(|atom| *atom == connection_state.atoms.net_wm_state_above);
    host_state.taskbar_visible = !state_atoms
        .iter()
        .any(|atom| *atom == connection_state.atoms.net_wm_state_skip_taskbar);
    host_state.modal = state_atoms
        .iter()
        .any(|atom| *atom == connection_state.atoms.net_wm_state_modal);

    // confirm or clear one pending mode request from wm state
    if let Some(pending_mode) = host_state.pending_mode {
        if is_fullscreen
            && matches!(
                pending_mode,
                WindowModeOptions::WindowBorderlessModeOptions(_)
                    | WindowModeOptions::WindowExclusiveFullscreenModeOptions(_)
            )
        {
            host_state.mode = pending_mode;
            host_state.display = mode_display(pending_mode);
            host_state.pending_mode = None;
        } else if !is_fullscreen
            && matches!(
                pending_mode,
                WindowModeOptions::WindowWindowedModeOptions(_)
            )
        {
            host_state.mode = pending_mode;
            host_state.display = None;
            host_state.pending_mode = None;
        }
    }

    Ok(())
}
