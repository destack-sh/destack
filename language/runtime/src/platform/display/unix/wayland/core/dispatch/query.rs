use std::sync::{Arc, Mutex};

use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::wp::color_management::v1::client::{
    wp_color_management_output_v1, wp_color_manager_v1, wp_image_description_info_v1,
    wp_image_description_v1,
};
use wayland_protocols_wlr::gamma_control::v1::client::zwlr_gamma_control_v1;

use crate::platform::display::unix::wayland::core::{
    WaylandColorDescriptionQueryState, WaylandConnectionDispatchState,
    WaylandGammaControlQueryState,
};

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
