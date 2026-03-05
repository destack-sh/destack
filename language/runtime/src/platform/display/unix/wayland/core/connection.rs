use std::io::ErrorKind;
use std::sync::Arc;

use wayland_client::protocol::{wl_seat, wl_surface};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, WaylandError};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_token_v1;
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::runtime::BindingCallContext;

use super::super::window;
use super::{
    WaylandActivationTokenState, WaylandConnectionDispatchState, WaylandConnectionState,
    WaylandRuntimeState, io_error, resolve_wl_surface, runtime_state,
};

/// Resolve one initialized wayland connection state for one operation.
pub(super) fn connection_state(
    context: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<Arc<WaylandConnectionState>> {
    let runtime_state = runtime_state(context);

    // return one cached connection lane when already initialized
    {
        let cached = runtime_state
            .connection_state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(connection_state) = cached.as_ref() {
            return Ok(Arc::clone(connection_state));
        }
    }

    // create and cache one connection lane on first use
    let initialized = Arc::new(WaylandConnectionState::from_runtime_state(
        &runtime_state,
        operation,
    )?);
    let mut cached = runtime_state
        .connection_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    if let Some(connection_state) = cached.as_ref() {
        return Ok(Arc::clone(connection_state));
    }

    *cached = Some(Arc::clone(&initialized));
    Ok(initialized)
}

/// Execute one callback with mutable queue and dispatch-state access.
pub(super) fn with_connection_dispatch<R>(
    context: &BindingCallContext,
    operation: &'static str,
    callback: impl FnOnce(
        &Connection,
        &mut EventQueue<WaylandConnectionDispatchState>,
        &mut WaylandConnectionDispatchState,
    ) -> RuntimeResult<R>,
) -> RuntimeResult<R> {
    let connection_state = connection_state(context, operation)?;

    // lock queue and dispatch payload with one stable lock order
    let mut event_queue = connection_state
        .event_queue
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut dispatch_state = connection_state
        .dispatch_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    callback(
        &connection_state.connection,
        &mut event_queue,
        &mut dispatch_state,
    )
}

/// Pump pending wayland events for this runtime.
pub(super) fn dispatch_pending(
    context: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<()> {
    with_connection_dispatch(
        context,
        operation,
        |connection, event_queue, dispatch_state| {
            // flush outbound protocol requests before reading
            event_queue
                .flush()
                .map_err(|error| io_error(operation, format!("wayland flush failed: {error}")))?;

            // read one non-blocking batch from the wayland socket
            if let Some(guard) = event_queue.prepare_read() {
                match guard.read() {
                    Ok(_) => {}
                    Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => {}
                    Err(error) => {
                        return Err(io_error(operation, format!("wayland read failed: {error}")));
                    }
                }
            }

            // dispatch all currently queued events
            loop {
                let dispatched = event_queue
                    .dispatch_pending(dispatch_state)
                    .map_err(|error| {
                        io_error(operation, format!("wayland dispatch failed: {error}"))
                    })?;

                if dispatched == 0 {
                    break;
                }
            }

            // finalize pending drop payload transfer after dispatch callbacks
            window::finalize_pending_drop_session(
                connection,
                event_queue,
                dispatch_state,
                operation,
            )?;

            Ok(())
        },
    )
}

/// Execute one callback with active seat and interaction serial lanes.
pub(super) fn with_interaction_serial<R>(
    context: &BindingCallContext,
    operation: &'static str,
    callback: impl FnOnce(
        &Connection,
        &mut EventQueue<WaylandConnectionDispatchState>,
        &mut WaylandConnectionDispatchState,
        wl_seat::WlSeat,
        u32,
    ) -> RuntimeResult<R>,
) -> RuntimeResult<R> {
    with_connection_dispatch(
        context,
        operation,
        |connection, event_queue, dispatch_state| {
            let seat = dispatch_state
                .seat
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported(operation))?;
            let serial = dispatch_state.last_pointer_button_serial.ok_or_else(|| {
                core_platform::io_would_block(
                    operation,
                    "interactive operation requires a recent pointer button serial",
                )
            })?;

            callback(connection, event_queue, dispatch_state, seat, serial)
        },
    )
}

/// Clear one active drop session when it targets one surface being destroyed.
pub(super) fn clear_drop_session_for_surface(
    context: &BindingCallContext,
    surface_id: &wayland_client::backend::ObjectId,
    operation: &'static str,
) -> RuntimeResult<()> {
    with_connection_dispatch(
        context,
        operation,
        |_connection, _event_queue, dispatch_state| {
            let Some(active_surface_id) = dispatch_state.drop_session_state.surface.as_ref() else {
                return Ok(());
            };

            if active_surface_id != surface_id {
                return Ok(());
            }

            window::clear_drop_session(dispatch_state);
            Ok(())
        },
    )
}

/// Flush one event queue and map transport errors to one runtime error payload.
pub(super) fn flush_queue(
    event_queue: &mut EventQueue<WaylandConnectionDispatchState>,
    operation: &'static str,
) -> RuntimeResult<()> {
    event_queue
        .flush()
        .map_err(|error| io_error(operation, format!("wayland flush failed: {error}")))
}

/// Resolve one xdg toplevel object from one raw wayland object id.
pub(super) fn resolve_xdg_toplevel(
    connection: &Connection,
    toplevel_id: wayland_client::backend::ObjectId,
    operation: &'static str,
) -> RuntimeResult<xdg_toplevel::XdgToplevel> {
    xdg_toplevel::XdgToplevel::from_id(connection, toplevel_id)
        .map_err(|error| io_error(operation, format!("invalid xdg_toplevel id: {error}")))
}

/// Resolve one xdg surface object from one raw wayland object id.
pub(super) fn resolve_xdg_surface(
    connection: &Connection,
    surface_id: wayland_client::backend::ObjectId,
    operation: &'static str,
) -> RuntimeResult<xdg_surface::XdgSurface> {
    xdg_surface::XdgSurface::from_id(connection, surface_id)
        .map_err(|error| io_error(operation, format!("invalid xdg_surface id: {error}")))
}

/// Resolve one wl surface object from one raw wayland object id.
pub(super) fn resolve_wl_surface(
    connection: &Connection,
    surface_id: wayland_client::backend::ObjectId,
    operation: &'static str,
) -> RuntimeResult<wl_surface::WlSurface> {
    wl_surface::WlSurface::from_id(connection, surface_id)
        .map_err(|error| io_error(operation, format!("invalid wl_surface id: {error}")))
}

/// Request compositor activation for one wl_surface through xdg-activation.
pub(super) fn request_surface_activation(
    context: &BindingCallContext,
    surface_id: wayland_client::backend::ObjectId,
    operation: &'static str,
) -> RuntimeResult<()> {
    with_connection_dispatch(
        context,
        operation,
        |connection, event_queue, dispatch_state| {
            // require one negotiated activation manager global
            let activation = dispatch_state
                .activation_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported(operation))?;

            // resolve target wl_surface object for activation
            let surface = resolve_wl_surface(connection, surface_id, operation)?;

            // request one activation token and bind request-surface metadata
            let token_state = Arc::new(WaylandActivationTokenState {
                token: std::sync::Mutex::new(None),
            });
            let token_object =
                activation.get_activation_token(&event_queue.handle(), Arc::clone(&token_state));
            token_object.set_surface(&surface);
            token_object.commit();
            flush_queue(event_queue, operation)?;

            // wait one roundtrip for token completion
            event_queue.roundtrip(dispatch_state).map_err(|error| {
                io_error(
                    operation,
                    format!("wayland activation token roundtrip failed: {error}"),
                )
            })?;
            let token = token_state
                .token
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone()
                .ok_or_else(|| io_error(operation, "wayland activation token was not returned"))?;

            // activate the target surface with the resolved token
            activation.activate(token, &surface);
            flush_queue(event_queue, operation)?;

            Ok(())
        },
    )
}

/// Request one presentation-feedback callback for one committed surface.
pub(super) fn request_surface_presentation_feedback(
    dispatch_state: &WaylandConnectionDispatchState,
    event_queue: &EventQueue<WaylandConnectionDispatchState>,
    surface: &wl_surface::WlSurface,
    token: super::WaylandWindowDispatchToken,
) {
    // skip when presentation-time protocol is not available
    let Some(presentation) = dispatch_state.presentation.as_ref().cloned() else {
        return;
    };

    presentation.feedback(surface, &event_queue.handle(), token);
}
