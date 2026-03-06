use crate::diagnostic::RuntimeResult;
use crate::platform::display::{DisplayMode, WindowModeOptions};
use crate::platform::resource;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    CLIENT_MESSAGE_EVENT, ClientMessageData, ClientMessageEvent,
    ConnectionExt as XprotoConnectionExt, EventMask,
};

use super::super::core;
use super::constants::*;

/// Return the selected display handle lane for one window mode.
pub(crate) fn mode_display(mode: WindowModeOptions) -> Option<resource::DisplayHandle> {
    // resolve this variant
    match mode {
        WindowModeOptions::WindowBorderlessModeOptions(value) => value.display,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(value) => Some(value.display),
        _ => None,
    }
}

/// Return the selected display mode lane for one window mode.
pub(crate) fn mode_display_mode(mode: WindowModeOptions) -> Option<DisplayMode> {
    // resolve this variant
    match mode {
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(value) => value.display_mode,
        _ => None,
    }
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
