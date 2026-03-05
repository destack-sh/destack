use wayland_client::protocol::wl_output;
use wayland_client::{Connection, Dispatch, QueueHandle};

use crate::platform::display::DisplayMode;

use super::{
    WAYLAND_DEFAULT_BIT_DEPTH, WAYLAND_DEFAULT_REFRESH_MILLI_HZ, WaylandConnectionDispatchState,
    orientation_from_transform, parse_mode_flags,
};

impl Dispatch<wl_output::WlOutput, u32> for WaylandConnectionDispatchState {
    /// Handle wl_output events.
    fn event(
        state: &mut Self,
        _proxy: &wl_output::WlOutput,
        event: wl_output::Event,
        output_name: &u32,
        _connection: &Connection,
        _queue_handle: &QueueHandle<Self>,
    ) {
        let Some(output) = state.output_snapshots_by_global.get_mut(output_name) else {
            return;
        };

        match event {
            // capture geometry metadata and orientation
            wl_output::Event::Geometry {
                x,
                y,
                physical_width,
                physical_height,
                subpixel: _,
                make,
                model,
                transform,
            } => {
                output.x = x;
                output.y = y;
                output.width_mm = physical_width.max(0) as u32;
                output.height_mm = physical_height.max(0) as u32;
                output.make = if make.is_empty() { None } else { Some(make) };
                output.model = if model.is_empty() { None } else { Some(model) };
                output.orientation = orientation_from_transform(transform);
            }
            // capture mode metadata and current or preferred mode markers
            wl_output::Event::Mode {
                flags,
                width,
                height,
                refresh,
            } => {
                if width <= 0 || height <= 0 {
                    return;
                }

                let refresh_milli_hz = if refresh <= 0 {
                    WAYLAND_DEFAULT_REFRESH_MILLI_HZ
                } else {
                    refresh as u32
                };
                let mode = DisplayMode {
                    width: width as u32,
                    height: height as u32,
                    refresh_milli_hz,
                    format: 0,
                    bit_depth: WAYLAND_DEFAULT_BIT_DEPTH,
                };
                if !output.modes.contains(&mode) {
                    output.modes.push(mode);
                }

                let (is_current, is_preferred) = parse_mode_flags(flags);
                if is_current {
                    output.current_mode = Some(mode);
                }
                if is_preferred {
                    output.desktop_mode = Some(mode);
                }
            }
            // capture scale updates
            wl_output::Event::Scale { factor } => {
                output.scale_factor = factor.max(1) as u32;
            }
            // capture logical names
            wl_output::Event::Name { name } => {
                if !name.is_empty() {
                    output.logical_name = Some(name);
                }
            }
            // capture human-readable descriptions
            wl_output::Event::Description { description } => {
                if !description.is_empty() {
                    output.description = Some(description);
                }
            }
            _ => {}
        }
    }
}
