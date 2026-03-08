use std::sync::Arc;

use wayland_client::protocol::wl_surface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1;
use wayland_protocols::wp::presentation_time::client::{wp_presentation, wp_presentation_feedback};
use wayland_protocols::wp::viewporter::client::wp_viewport;
use wayland_protocols::xdg::activation::v1::client::xdg_activation_token_v1;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
use wayland_protocols::xdg::shell::client::{xdg_popup, xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

use crate::platform::display::WindowLogicalSize;

use crate::platform::display::unix::wayland::core::{
    WaylandActivationTokenState, WaylandConnectionDispatchState, WaylandWindowDispatchToken,
    apply_layer_surface_configure, apply_popup_configure, apply_toplevel_close,
    apply_toplevel_configure, scale_factor_milli_from_fractional_scale,
};
use crate::platform::display::unix::wayland::event;

/// Publish one refresh-requested event when this token still resolves to one live window.
fn publish_window_refresh_if_live(
    state: &WaylandConnectionDispatchState,
    token: &WaylandWindowDispatchToken,
) {
    // reject callbacks for released or destroyed host states
    let Some(host_state) = token.host_state.upgrade() else {
        return;
    };
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    if host_state.destroyed_emitted {
        return;
    }
    drop(host_state);

    // publish one refresh event for the token window
    if let Some(runtime_state) = state.runtime_state.upgrade()
        && let Some(window_handle) = runtime_state.window_handle_from_id(&token.window_id)
    {
        event::publish_window_refresh_requested(&runtime_state, window_handle);
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for WaylandConnectionDispatchState {
    /// Handle xdg_wm_base events.
    fn event(
        _state: &mut Self,
        proxy: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

impl Dispatch<wl_surface::WlSurface, WaylandWindowDispatchToken>
    for WaylandConnectionDispatchState
{
    /// Handle wl_surface events.
    fn event(
        _state: &mut Self,
        _proxy: &wl_surface::WlSurface,
        _event: wl_surface::Event,
        _token: &WaylandWindowDispatchToken,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_fractional_scale_v1::WpFractionalScaleV1, WaylandWindowDispatchToken>
    for WaylandConnectionDispatchState
{
    /// Handle fractional-scale events.
    fn event(
        state: &mut Self,
        _proxy: &wp_fractional_scale_v1::WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        token: &WaylandWindowDispatchToken,
        connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // ignore non-scale events
        let wp_fractional_scale_v1::Event::PreferredScale { scale } = event else {
            return;
        };

        // resolve runtime and host state for this token
        let Some(runtime_state) = state.runtime_state.upgrade() else {
            return;
        };
        let Some(host_state) = token.host_state.upgrade() else {
            return;
        };
        let Some(window_handle) = runtime_state.window_handle_from_id(&token.window_id) else {
            return;
        };

        // apply preferred scale and derive next logical size snapshot
        let (previous, current, viewport_id) = {
            let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
            let previous = host_state.clone();

            // ignore events for already-destroyed windows
            if host_state.destroyed_emitted {
                return;
            }

            let previous_scale_factor_milli = host_state.scale_factor_milli.max(1);
            let current_scale_factor_milli = scale_factor_milli_from_fractional_scale(scale);
            if previous_scale_factor_milli == current_scale_factor_milli {
                return;
            }

            let current_size_physical = host_state.size_physical;

            let scale_factor = current_scale_factor_milli as f64 / 1000.0;
            let current_size_logical = WindowLogicalSize {
                width: (current_size_physical.width as f64 / scale_factor).max(1.0),
                height: (current_size_physical.height as f64 / scale_factor).max(1.0),
            };

            host_state.scale_factor_milli = current_scale_factor_milli;
            host_state.size_logical = current_size_logical;
            let current = host_state.clone();

            (previous, current, host_state.host.viewport.clone())
        };

        // update one viewport destination when this surface has one viewport object
        if let Some(viewport_id) = viewport_id
            && !viewport_id.is_null()
            && let Ok(viewport) = wp_viewport::WpViewport::from_id(connection, viewport_id)
        {
            let destination_width = current
                .size_logical
                .width
                .round()
                .clamp(1.0, i32::MAX as f64) as i32;
            let destination_height = current
                .size_logical
                .height
                .round()
                .clamp(1.0, i32::MAX as f64) as i32;
            viewport.set_destination(destination_width, destination_height);
        }

        // publish all affected state deltas
        event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);
    }
}

impl Dispatch<xdg_surface::XdgSurface, WaylandWindowDispatchToken>
    for WaylandConnectionDispatchState
{
    /// Handle xdg_surface events.
    fn event(
        state: &mut Self,
        proxy: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        token: &WaylandWindowDispatchToken,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            // acknowledge compositor configure serial immediately
            proxy.ack_configure(serial);

            // publish one refresh request for configured windows
            publish_window_refresh_if_live(state, token);
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, WaylandWindowDispatchToken>
    for WaylandConnectionDispatchState
{
    /// Handle xdg_toplevel events.
    fn event(
        state: &mut Self,
        _proxy: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        token: &WaylandWindowDispatchToken,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            // map compositor configure payloads into runtime state updates
            xdg_toplevel::Event::Configure {
                width,
                height,
                states,
            } => {
                apply_toplevel_configure(state, token, width, height, &states);
            }
            // map compositor close requests into closeRequested events
            xdg_toplevel::Event::Close => {
                apply_toplevel_close(state, token);
            }
            _ => {}
        }
    }
}

impl Dispatch<xdg_popup::XdgPopup, WaylandWindowDispatchToken> for WaylandConnectionDispatchState {
    /// Handle xdg_popup events.
    fn event(
        state: &mut Self,
        _proxy: &xdg_popup::XdgPopup,
        event: xdg_popup::Event,
        token: &WaylandWindowDispatchToken,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            // map popup configure payloads into runtime state updates
            xdg_popup::Event::Configure {
                x,
                y,
                width,
                height,
            } => {
                apply_popup_configure(state, token, x, y, width, height);
            }
            // map popup-dismiss requests into closeRequested events
            xdg_popup::Event::PopupDone => {
                apply_toplevel_close(state, token);
            }
            _ => {}
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, WaylandWindowDispatchToken>
    for WaylandConnectionDispatchState
{
    /// Handle layer-surface events.
    fn event(
        state: &mut Self,
        proxy: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        token: &WaylandWindowDispatchToken,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            // acknowledge configure and publish refresh requests
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                proxy.ack_configure(serial);
                apply_layer_surface_configure(state, token, width, height);
                publish_window_refresh_if_live(state, token);
            }
            // map compositor close requests into closeRequested events
            zwlr_layer_surface_v1::Event::Closed => {
                apply_toplevel_close(state, token);
            }
            _ => {}
        }
    }
}

impl Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, WaylandWindowDispatchToken>
    for WaylandConnectionDispatchState
{
    /// Handle xdg-decoration events.
    fn event(
        _state: &mut Self,
        _proxy: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        event: zxdg_toplevel_decoration_v1::Event,
        token: &WaylandWindowDispatchToken,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // resolve the target window host state for this decoration event
        let Some(host_state) = token.host_state.upgrade() else {
            return;
        };
        let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

        // ignore events for already-destroyed windows
        if host_state.destroyed_emitted {
            return;
        }

        // apply compositor decoration mode to runtime state
        if let zxdg_toplevel_decoration_v1::Event::Configure { mode } = event
            && let WEnum::Value(mode) = mode
        {
            host_state.decorated = mode == zxdg_toplevel_decoration_v1::Mode::ServerSide;
        }
    }
}

impl Dispatch<xdg_activation_token_v1::XdgActivationTokenV1, Arc<WaylandActivationTokenState>>
    for WaylandConnectionDispatchState
{
    /// Handle xdg activation token events.
    fn event(
        _state: &mut Self,
        proxy: &xdg_activation_token_v1::XdgActivationTokenV1,
        event: xdg_activation_token_v1::Event,
        token_state: &Arc<WaylandActivationTokenState>,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // capture token payload and dispose this token object
        if let xdg_activation_token_v1::Event::Done { token } = event {
            let mut state = token_state
                .token
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            *state = Some(token);
            proxy.destroy();
        }
    }
}

impl Dispatch<wp_presentation::WpPresentation, ()> for WaylandConnectionDispatchState {
    /// Handle presentation manager events.
    fn event(
        state: &mut Self,
        _proxy: &wp_presentation::WpPresentation,
        event: wp_presentation::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        if let wp_presentation::Event::ClockId { clk_id } = event {
            state.globals.presentation_clock_id = Some(clk_id);
        }
    }
}

impl Dispatch<wp_presentation_feedback::WpPresentationFeedback, WaylandWindowDispatchToken>
    for WaylandConnectionDispatchState
{
    /// Handle presentation feedback events.
    fn event(
        state: &mut Self,
        _proxy: &wp_presentation_feedback::WpPresentationFeedback,
        event: wp_presentation_feedback::Event,
        token: &WaylandWindowDispatchToken,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // ignore sync-output routing metadata
        let is_terminal = matches!(
            event,
            wp_presentation_feedback::Event::Presented { .. }
                | wp_presentation_feedback::Event::Discarded
        );
        if !is_terminal {
            return;
        }

        // publish one refresh request for this presented or discarded frame
        publish_window_refresh_if_live(state, token);
    }
}
