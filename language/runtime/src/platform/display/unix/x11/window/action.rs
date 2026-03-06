use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    CLIENT_MESSAGE_EVENT, ClientMessageData, ClientMessageEvent, ConfigureWindowAux,
    ConnectionExt as XprotoConnectionExt, EventMask, InputFocus, StackMode,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowAttentionLevel, WindowResizeEdge};
use crate::platform::resource::WindowHandle;
use crate::runtime::BindingCallContext;

use super::super::super::{core, event, resource as display_resource};
use super::{ensure_window_thread, set_net_wm_state};

/// `_NET_WM_MOVERESIZE` direction value for move.
const MOVERESIZE_DIRECTION_MOVE: u32 = 8;
/// `_NET_WM_MOVERESIZE` direction value for north resize.
const MOVERESIZE_DIRECTION_NORTH: u32 = 0;
/// `_NET_WM_MOVERESIZE` direction value for south resize.
const MOVERESIZE_DIRECTION_SOUTH: u32 = 1;
/// `_NET_WM_MOVERESIZE` direction value for east resize.
const MOVERESIZE_DIRECTION_EAST: u32 = 2;
/// `_NET_WM_MOVERESIZE` direction value for west resize.
const MOVERESIZE_DIRECTION_WEST: u32 = 7;
/// `_NET_WM_MOVERESIZE` direction value for north-east resize.
const MOVERESIZE_DIRECTION_NORTH_EAST: u32 = 3;
/// `_NET_WM_MOVERESIZE` direction value for north-west resize.
const MOVERESIZE_DIRECTION_NORTH_WEST: u32 = 4;
/// `_NET_WM_MOVERESIZE` direction value for south-east resize.
const MOVERESIZE_DIRECTION_SOUTH_EAST: u32 = 5;
/// `_NET_WM_MOVERESIZE` direction value for south-west resize.
const MOVERESIZE_DIRECTION_SOUTH_WEST: u32 = 6;
/// `_NET_WM_MOVERESIZE` left-button marker.
const MOVERESIZE_BUTTON_LEFT: u32 = 1;
/// `_NET_WM_MOVERESIZE` source indication for application requests.
const MOVERESIZE_SOURCE_APPLICATION: u32 = 1;

/// Resolve one root-pointer position for move and resize drag requests.
fn root_pointer_position(
    connection_state: &core::X11ConnectionState,
    operation: &'static str,
) -> RuntimeResult<(u32, u32)> {
    // query root pointer location from the x11 server
    let reply = connection_state
        .connection
        .query_pointer(connection_state.root)
        .map_err(|error| {
            core::io_error(operation, format!("query_pointer request failed: {error}"))
        })?
        .reply()
        .map_err(|error| {
            core::io_error(operation, format!("query_pointer reply failed: {error}"))
        })?;

    Ok((reply.root_x as u32, reply.root_y as u32))
}

/// Send one `_NET_WM_MOVERESIZE` client message to the root window.
fn send_moveresize_request(
    connection_state: &core::X11ConnectionState,
    window: u32,
    root_x: u32,
    root_y: u32,
    direction: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    // send one root-window message that asks the wm to start interactive drag
    let message = ClientMessageEvent {
        response_type: CLIENT_MESSAGE_EVENT,
        format: 32,
        sequence: 0,
        window,
        type_: connection_state.atoms.net_wm_moveresize,
        data: ClientMessageData::from([
            root_x,
            root_y,
            direction,
            MOVERESIZE_BUTTON_LEFT,
            MOVERESIZE_SOURCE_APPLICATION,
        ]),
    };
    connection_state
        .connection
        .send_event(
            false,
            connection_state.root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            message,
        )
        .map_err(|error| core::io_error(operation, format!("send_event failed: {error}")))?;
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Request one user-attention pulse for one window.
pub(crate) unsafe fn window_request_attention(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    // resolve runtime and window resolved_binding lanes
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.requestAttention")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.requestAttention",
    )?;
    let resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.requestAttention")?;

    // resolve one attention-state toggle from the requested attention level
    let should_raise = matches!(level, WindowAttentionLevel::Critical);

    // request one attention pulse through EWMH state toggles
    set_net_wm_state(
        connection_state.as_ref(),
        resolved_binding.window,
        connection_state.atoms.net_wm_state_demands_attention,
        true,
    )?;

    // raise critical attention requests to surface the window quickly
    if should_raise {
        connection_state
            .connection
            .configure_window(
                resolved_binding.window,
                &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
            )
            .map_err(|error| {
                core::io_error(
                    "destack.display.window.requestAttention",
                    format!("configure_window failed: {error}"),
                )
            })?;
        connection_state.connection.flush().map_err(|error| {
            core::io_error(
                "destack.display.window.requestAttention",
                format!("flush failed: {error}"),
            )
        })?;
    }

    Ok(())
}

/// Request one redraw for one window.
pub(crate) unsafe fn window_request_refresh(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve one window resolved_binding and publish refresh event
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.requestRefresh",
    )?;
    let resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.requestRefresh")?;
    drop(resolved_binding);

    let runtime_state = core::runtime_state(binding);
    event::publish_window_refresh_requested(&runtime_state, window_handle);

    Ok(())
}

/// Focus one window.
pub(crate) unsafe fn window_focus(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve runtime and resolved_binding lanes
    let runtime_state = core::runtime_state(binding);
    let connection_state = core::connection_state(&runtime_state, "destack.display.window.focus")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.focus",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.focus")?;

    // request focus through x11 input focus
    connection_state
        .connection
        .set_input_focus(
            InputFocus::PARENT,
            resolved_binding.window,
            x11rb::CURRENT_TIME,
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.focus",
                format!("set_input_focus failed: {error}"),
            )
        })?;
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.focus",
            format!("flush failed: {error}"),
        )
    })?;

    // publish focus event when state changes
    if !resolved_binding.focused {
        let previous_focused = resolved_binding.focused;
        resolved_binding.focused = true;
        drop(resolved_binding);
        event::publish_window_focus_changed(&runtime_state, window_handle, previous_focused, true);
    }

    Ok(())
}

/// Raise one window.
pub(crate) unsafe fn window_raise(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve runtime and resolved_binding lanes
    let runtime_state = core::runtime_state(binding);
    let connection_state = core::connection_state(&runtime_state, "destack.display.window.raise")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.raise",
    )?;
    let resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.raise")?;

    // raise window to the top of the stacking order
    connection_state
        .connection
        .configure_window(
            resolved_binding.window,
            &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.raise",
                format!("configure_window failed: {error}"),
            )
        })?;
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.raise",
            format!("flush failed: {error}"),
        )
    })?;

    Ok(())
}

/// Begin one native window move drag.
pub(crate) unsafe fn window_begin_move_drag(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve runtime and window resolved_binding lanes
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.beginMoveDrag")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.beginMoveDrag",
    )?;
    let resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.beginMoveDrag")?;

    // read pointer position and request interactive move on this window
    let (root_x, root_y) = root_pointer_position(
        connection_state.as_ref(),
        "destack.display.window.beginMoveDrag",
    )?;
    send_moveresize_request(
        connection_state.as_ref(),
        resolved_binding.window,
        root_x,
        root_y,
        MOVERESIZE_DIRECTION_MOVE,
        "destack.display.window.beginMoveDrag",
    )
}

/// Begin one native window resize drag.
pub(crate) unsafe fn window_begin_resize_drag(
    binding: &BindingCallContext,
    window_handle: WindowHandle,
    edge: WindowResizeEdge,
) -> RuntimeResult<()> {
    // map the requested edge to one ewmh moveresize direction
    let direction = match edge {
        WindowResizeEdge::North => MOVERESIZE_DIRECTION_NORTH,
        WindowResizeEdge::South => MOVERESIZE_DIRECTION_SOUTH,
        WindowResizeEdge::East => MOVERESIZE_DIRECTION_EAST,
        WindowResizeEdge::West => MOVERESIZE_DIRECTION_WEST,
        WindowResizeEdge::NorthEast => MOVERESIZE_DIRECTION_NORTH_EAST,
        WindowResizeEdge::NorthWest => MOVERESIZE_DIRECTION_NORTH_WEST,
        WindowResizeEdge::SouthEast => MOVERESIZE_DIRECTION_SOUTH_EAST,
        WindowResizeEdge::SouthWest => MOVERESIZE_DIRECTION_SOUTH_WEST,
    };

    // resolve runtime and window resolved_binding lanes
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.beginResizeDrag")?;
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.beginResizeDrag",
    )?;
    let resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.beginResizeDrag")?;

    // read pointer position and request interactive resize on this window
    let (root_x, root_y) = root_pointer_position(
        connection_state.as_ref(),
        "destack.display.window.beginResizeDrag",
    )?;
    send_moveresize_request(
        connection_state.as_ref(),
        resolved_binding.window,
        root_x,
        root_y,
        direction,
        "destack.display.window.beginResizeDrag",
    )
}
