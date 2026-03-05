use std::sync::{Arc, Mutex};

use wayland_client::protocol::{wl_data_device, wl_data_device_manager, wl_data_offer, wl_surface};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols::wp::color_management::v1::client::{
    wp_color_management_output_v1, wp_color_manager_v1, wp_image_description_info_v1,
    wp_image_description_v1,
};
use wayland_protocols::wp::fractional_scale::v1::client::wp_fractional_scale_v1;
use wayland_protocols::wp::pointer_constraints::zv1::client::{
    zwp_confined_pointer_v1, zwp_locked_pointer_v1,
};
use wayland_protocols::wp::presentation_time::client::{wp_presentation, wp_presentation_feedback};
use wayland_protocols::wp::relative_pointer::zv1::client::zwp_relative_pointer_v1;
use wayland_protocols::wp::viewporter::client::wp_viewport;
use wayland_protocols::xdg::activation::v1::client::xdg_activation_token_v1;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1;
use wayland_protocols::xdg::shell::client::{xdg_popup, xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_protocols_wlr::gamma_control::v1::client::zwlr_gamma_control_v1;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_v1, zwlr_output_head_v1, zwlr_output_manager_v1, zwlr_output_mode_v1,
};

use crate::platform::display::WindowLogicalSize;

use super::super::{event, window};
use super::{
    WaylandActivationTokenState, WaylandColorDescriptionQueryState, WaylandConnectionDispatchState,
    WaylandGammaControlQueryState, WaylandOutputConfigurationOutcome,
    WaylandOutputConfigurationState, WaylandWindowDispatchToken, WaylandWlrAdaptiveSyncState,
    apply_layer_surface_configure, apply_popup_configure, apply_toplevel_close,
    apply_toplevel_configure, window_handle_from_id,
};

/// Publish one refresh-requested event when this token still resolves to one live window.
fn publish_window_refresh_if_live(
    state: &WaylandConnectionDispatchState,
    token: &WaylandWindowDispatchToken,
) {
    // reject callbacks for released or destroyed bindings
    let Some(binding) = token.binding.upgrade() else {
        return;
    };
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    if binding.destroyed_emitted {
        return;
    }
    drop(binding);

    // publish one refresh event for the token window
    if let Some(runtime_state) = state.runtime_state.upgrade()
        && let Some(window_handle) = window_handle_from_id(&runtime_state, &token.window_id)
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

        // resolve runtime and binding state for this token
        let Some(runtime_state) = state.runtime_state.upgrade() else {
            return;
        };
        let Some(binding) = token.binding.upgrade() else {
            return;
        };
        let Some(window_handle) = window_handle_from_id(&runtime_state, &token.window_id) else {
            return;
        };

        // apply preferred scale and derive next logical size snapshot
        let (
            previous_scale_factor_milli,
            current_scale_factor_milli,
            previous_size_logical,
            previous_size_physical,
            current_size_logical,
            current_size_physical,
            viewport_id,
        ) = {
            let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

            // ignore events for already-destroyed windows
            if binding.destroyed_emitted {
                return;
            }

            let previous_scale_factor_milli = binding.scale_factor_milli.max(1);
            let current_scale_factor_milli = super::scale_factor_milli_from_fractional_scale(scale);
            if previous_scale_factor_milli == current_scale_factor_milli {
                return;
            }

            let previous_size_logical = binding.size_logical;
            let previous_size_physical = binding.size_physical;
            let current_size_physical = binding.size_physical;

            let scale_factor = current_scale_factor_milli as f64 / 1000.0;
            let current_size_logical = WindowLogicalSize {
                width: (current_size_physical.width as f64 / scale_factor).max(1.0),
                height: (current_size_physical.height as f64 / scale_factor).max(1.0),
            };

            binding.scale_factor_milli = current_scale_factor_milli;
            binding.size_logical = current_size_logical;

            (
                previous_scale_factor_milli,
                current_scale_factor_milli,
                previous_size_logical,
                previous_size_physical,
                current_size_logical,
                current_size_physical,
                binding.host.viewport.clone(),
            )
        };

        // update one viewport destination when this surface has one viewport object
        if let Some(viewport_id) = viewport_id
            && !viewport_id.is_null()
            && let Ok(viewport) = wp_viewport::WpViewport::from_id(connection, viewport_id)
        {
            let destination_width = current_size_logical
                .width
                .round()
                .clamp(1.0, i32::MAX as f64) as i32;
            let destination_height = current_size_logical
                .height
                .round()
                .clamp(1.0, i32::MAX as f64) as i32;
            viewport.set_destination(destination_width, destination_height);
        }

        // publish scale and size deltas
        event::publish_window_scale_factor_changed(
            &runtime_state,
            window_handle,
            previous_scale_factor_milli,
            current_scale_factor_milli,
        );

        if previous_size_logical != current_size_logical {
            event::publish_window_size_changed(
                &runtime_state,
                window_handle,
                previous_size_logical,
                previous_size_physical,
                current_size_logical,
                current_size_physical,
            );
        }
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
        // resolve the target window binding for this decoration event
        let Some(binding) = token.binding.upgrade() else {
            return;
        };
        let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

        // ignore events for already-destroyed windows
        if binding.destroyed_emitted {
            return;
        }

        // apply compositor decoration mode to runtime state
        if let zxdg_toplevel_decoration_v1::Event::Configure { mode } = event {
            if let WEnum::Value(mode) = mode {
                binding.decorated = mode == zxdg_toplevel_decoration_v1::Mode::ServerSide;
            }
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
            state.presentation_clock_id = Some(clk_id);
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

impl Dispatch<zwlr_output_manager_v1::ZwlrOutputManagerV1, ()> for WaylandConnectionDispatchState {
    /// Handle wlr-output-manager events.
    fn event(
        state: &mut Self,
        _proxy: &zwlr_output_manager_v1::ZwlrOutputManagerV1,
        event: zwlr_output_manager_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        match event {
            // track newly advertised output-head objects
            zwlr_output_manager_v1::Event::Head { head } => {
                state.wlr_output_heads_by_id.entry(head.id()).or_default();
            }
            // capture the latest configuration serial for mode-set operations
            zwlr_output_manager_v1::Event::Done { serial } => {
                state.wlr_output_manager_serial = Some(serial);
            }
            // clear manager-derived state when the manager becomes invalid
            zwlr_output_manager_v1::Event::Finished => {
                state.wlr_output_manager = None;
                state.wlr_output_manager_serial = None;
                state.wlr_output_heads_by_id.clear();
                state.wlr_output_modes_by_id.clear();
            }
            _ => {}
        }
    }
}

impl Dispatch<zwlr_output_head_v1::ZwlrOutputHeadV1, ()> for WaylandConnectionDispatchState {
    /// Handle wlr-output-head events.
    fn event(
        state: &mut Self,
        proxy: &zwlr_output_head_v1::ZwlrOutputHeadV1,
        event: zwlr_output_head_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // route this output-head event into the head snapshot map
        match event {
            zwlr_output_head_v1::Event::Mode { mode } => {
                let mode_id = mode.id();

                // record this mode under the owning output head
                let head = state.wlr_output_heads_by_id.entry(proxy.id()).or_default();
                if !head.mode_ids.iter().any(|value| value == &mode_id) {
                    head.mode_ids.push(mode_id.clone());
                }

                // allocate one mode snapshot entry
                state.wlr_output_modes_by_id.entry(mode_id).or_default();
            }
            zwlr_output_head_v1::Event::Name { name } => {
                if let Some(head) = state.wlr_output_heads_by_id.get_mut(&proxy.id()) {
                    head.name = Some(name);
                }
            }
            zwlr_output_head_v1::Event::AdaptiveSync {
                state: adaptive_sync,
            } => {
                let adaptive_sync = match adaptive_sync {
                    WEnum::Value(zwlr_output_head_v1::AdaptiveSyncState::Disabled) => {
                        Some(WaylandWlrAdaptiveSyncState::Disabled)
                    }
                    WEnum::Value(zwlr_output_head_v1::AdaptiveSyncState::Enabled) => {
                        Some(WaylandWlrAdaptiveSyncState::Enabled)
                    }
                    WEnum::Unknown(_) => None,
                    _ => None,
                };

                let head = state.wlr_output_heads_by_id.entry(proxy.id()).or_default();
                head.adaptive_sync = adaptive_sync;
            }
            zwlr_output_head_v1::Event::Finished => {
                let removed_head = state.wlr_output_heads_by_id.remove(&proxy.id());
                if let Some(removed_head) = removed_head {
                    for mode_id in removed_head.mode_ids {
                        state.wlr_output_modes_by_id.remove(&mode_id);
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<zwlr_output_mode_v1::ZwlrOutputModeV1, ()> for WaylandConnectionDispatchState {
    /// Handle wlr-output-mode events.
    fn event(
        state: &mut Self,
        proxy: &zwlr_output_mode_v1::ZwlrOutputModeV1,
        event: zwlr_output_mode_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // resolve one mutable mode snapshot entry for this proxy id
        let mode_id = proxy.id();
        let mode = state
            .wlr_output_modes_by_id
            .entry(mode_id.clone())
            .or_default();

        // apply one event variant to the mode snapshot
        match event {
            zwlr_output_mode_v1::Event::Size { width, height } => {
                if width > 0 {
                    mode.width = Some(width as u32);
                }

                if height > 0 {
                    mode.height = Some(height as u32);
                }
            }
            zwlr_output_mode_v1::Event::Refresh { refresh } => {
                if refresh > 0 {
                    mode.refresh_milli_hz = Some(refresh as u32);
                }
            }
            zwlr_output_mode_v1::Event::Preferred => {
                mode.is_preferred = true;
            }
            zwlr_output_mode_v1::Event::Finished => {
                state.wlr_output_modes_by_id.remove(&mode_id);

                // remove stale mode ids from every tracked output head
                for head in state.wlr_output_heads_by_id.values_mut() {
                    head.mode_ids.retain(|value| value != &mode_id);
                }
            }
            _ => {}
        }
    }
}

impl
    Dispatch<
        zwlr_output_configuration_v1::ZwlrOutputConfigurationV1,
        Arc<WaylandOutputConfigurationState>,
    > for WaylandConnectionDispatchState
{
    /// Handle wlr-output-configuration events.
    fn event(
        _state: &mut Self,
        proxy: &zwlr_output_configuration_v1::ZwlrOutputConfigurationV1,
        event: zwlr_output_configuration_v1::Event,
        data: &Arc<WaylandOutputConfigurationState>,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        // store terminal configuration outcomes for the waiting operation
        let outcome = match event {
            zwlr_output_configuration_v1::Event::Succeeded => {
                Some(WaylandOutputConfigurationOutcome::Succeeded)
            }
            zwlr_output_configuration_v1::Event::Failed => {
                Some(WaylandOutputConfigurationOutcome::Failed)
            }
            zwlr_output_configuration_v1::Event::Cancelled => {
                Some(WaylandOutputConfigurationOutcome::Cancelled)
            }
            _ => None,
        };

        if let Some(outcome) = outcome {
            let mut state = data
                .outcome
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            *state = Some(outcome);
            proxy.destroy();
        }
    }
}

impl Dispatch<wl_data_device_manager::WlDataDeviceManager, ()> for WaylandConnectionDispatchState {
    /// Handle wl_data_device_manager events.
    fn event(
        _state: &mut Self,
        _proxy: &wl_data_device_manager::WlDataDeviceManager,
        _event: wl_data_device_manager::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_color_manager_v1::WpColorManagerV1, ()> for WaylandConnectionDispatchState {
    /// Handle wp_color_manager events.
    fn event(
        _state: &mut Self,
        _proxy: &wp_color_manager_v1::WpColorManagerV1,
        _event: wp_color_manager_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_color_management_output_v1::WpColorManagementOutputV1, u32>
    for WaylandConnectionDispatchState
{
    /// Handle wp_color_management_output events.
    fn event(
        _state: &mut Self,
        _proxy: &wp_color_management_output_v1::WpColorManagementOutputV1,
        _event: wp_color_management_output_v1::Event,
        _output_global_name: &u32,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}

impl
    Dispatch<
        wp_image_description_v1::WpImageDescriptionV1,
        Arc<Mutex<WaylandColorDescriptionQueryState>>,
    > for WaylandConnectionDispatchState
{
    /// Handle wp_image_description events.
    fn event(
        _state: &mut Self,
        _proxy: &wp_image_description_v1::WpImageDescriptionV1,
        event: wp_image_description_v1::Event,
        query: &Arc<Mutex<WaylandColorDescriptionQueryState>>,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        let mut query = query.lock().unwrap_or_else(|error| error.into_inner());

        match event {
            wp_image_description_v1::Event::Ready { identity: _ } => {
                query.ready = true;
            }
            wp_image_description_v1::Event::Ready2 {
                identity_hi: _,
                identity_lo: _,
            } => {
                query.ready = true;
            }
            wp_image_description_v1::Event::Failed { cause, msg } => {
                query.failed_cause = match cause {
                    WEnum::Value(value) => Some(value as u32),
                    WEnum::Unknown(value) => Some(value),
                };
                query.failed_message = Some(msg);
            }
            _ => {}
        }
    }
}

impl
    Dispatch<
        wp_image_description_info_v1::WpImageDescriptionInfoV1,
        Arc<Mutex<WaylandColorDescriptionQueryState>>,
    > for WaylandConnectionDispatchState
{
    /// Handle wp_image_description_info events.
    fn event(
        _state: &mut Self,
        _proxy: &wp_image_description_info_v1::WpImageDescriptionInfoV1,
        event: wp_image_description_info_v1::Event,
        query: &Arc<Mutex<WaylandColorDescriptionQueryState>>,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        let mut query = query.lock().unwrap_or_else(|error| error.into_inner());

        match event {
            wp_image_description_info_v1::Event::PrimariesNamed { primaries } => {
                query.primaries_named = match primaries {
                    WEnum::Value(value) => Some(value as u32),
                    WEnum::Unknown(value) => Some(value),
                };
            }
            wp_image_description_info_v1::Event::TfNamed { tf } => {
                query.transfer_function_named = match tf {
                    WEnum::Value(value) => Some(value as u32),
                    WEnum::Unknown(value) => Some(value),
                };
            }
            wp_image_description_info_v1::Event::Luminances {
                min_lum,
                max_lum,
                reference_lum,
            } => {
                query.minimum_luminance = Some(min_lum);
                query.maximum_luminance = Some(max_lum);
                query.reference_luminance = Some(reference_lum);
            }
            wp_image_description_info_v1::Event::TargetLuminance { min_lum, max_lum } => {
                query.minimum_luminance = Some(min_lum);
                query.maximum_luminance = Some(max_lum);
            }
            wp_image_description_info_v1::Event::TargetMaxCll { max_cll } => {
                query.target_max_cll = Some(max_cll);
            }
            wp_image_description_info_v1::Event::TargetMaxFall { max_fall } => {
                query.target_max_fall = Some(max_fall);
            }
            wp_image_description_info_v1::Event::Done => {
                query.info_done = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_data_offer::WlDataOffer, ()> for WaylandConnectionDispatchState {
    /// Handle wl_data_offer events.
    fn event(
        state: &mut Self,
        proxy: &wl_data_offer::WlDataOffer,
        event: wl_data_offer::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        window::handle_data_offer_event(state, proxy, event);
    }
}

impl Dispatch<wl_data_device::WlDataDevice, ()> for WaylandConnectionDispatchState {
    /// Handle wl_data_device events.
    fn event(
        state: &mut Self,
        _proxy: &wl_data_device::WlDataDevice,
        event: wl_data_device::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        window::handle_data_device_event(state, event);
    }
}

impl Dispatch<zwlr_gamma_control_v1::ZwlrGammaControlV1, Arc<Mutex<WaylandGammaControlQueryState>>>
    for WaylandConnectionDispatchState
{
    /// Handle zwlr_gamma_control events.
    fn event(
        _state: &mut Self,
        _proxy: &zwlr_gamma_control_v1::ZwlrGammaControlV1,
        event: zwlr_gamma_control_v1::Event,
        query: &Arc<Mutex<WaylandGammaControlQueryState>>,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        let mut query = query.lock().unwrap_or_else(|error| error.into_inner());

        match event {
            zwlr_gamma_control_v1::Event::GammaSize { size } => {
                query.gamma_size = Some(size);
            }
            zwlr_gamma_control_v1::Event::Failed => {
                query.failed = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<zwp_locked_pointer_v1::ZwpLockedPointerV1, ()> for WaylandConnectionDispatchState {
    /// Handle zwp_locked_pointer events.
    fn event(
        _state: &mut Self,
        _proxy: &zwp_locked_pointer_v1::ZwpLockedPointerV1,
        _event: zwp_locked_pointer_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_confined_pointer_v1::ZwpConfinedPointerV1, ()>
    for WaylandConnectionDispatchState
{
    /// Handle zwp_confined_pointer events.
    fn event(
        _state: &mut Self,
        _proxy: &zwp_confined_pointer_v1::ZwpConfinedPointerV1,
        _event: zwp_confined_pointer_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_relative_pointer_v1::ZwpRelativePointerV1, ()>
    for WaylandConnectionDispatchState
{
    /// Handle zwp_relative_pointer events.
    fn event(
        _state: &mut Self,
        _proxy: &zwp_relative_pointer_v1::ZwpRelativePointerV1,
        _event: zwp_relative_pointer_v1::Event,
        _data: &(),
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
    }
}
