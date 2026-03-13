use std::collections::HashMap;

use x11rb::connection::Connection;
use x11rb::errors::ConnectionError;
use x11rb::protocol::randr::{
    Connection as RandrOutputConnection, ConnectionExt as RandrConnectionExt, Crtc, Mode, ModeFlag,
    ModeInfo, Output, Rotation,
};
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as XprotoConnectionExt};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{DisplayMode, DisplayOrientation, DisplaySupportStatus};

use super::mode::{current_mode_from_catalog, desktop_mode_from_catalog};
use crate::platform::display::unix::x11::core;
use crate::platform::display::unix::x11::model::{DisplayDescriptorSnapshot, MonitorSnapshot};

/// Default x11 color depth assumption when detailed channel depth is unavailable.
pub(crate) const X11_DEFAULT_BITS_PER_CHANNEL: u16 = 8;
/// Fallback refresh-rate used when one mode payload omits timing information.
const X11_DEFAULT_REFRESH_MILLI_HZ: u32 = 60_000;
/// Conventional desktop DPI used for one scale factor of `1.0`.
const X11_DEFAULT_DESKTOP_DPI: f64 = 96.0;

/// Return one stable display-id prefix for the active x11-compatible backend.
fn display_id_prefix() -> &'static str {
    match core::selected_backend_name() {
        "wayland" => "wayland-output-",
        "appkit" => "appkit-output-",
        _ => "x11-output-",
    }
}

/// Runtime snapshot for one active x11 randr output lane.
#[derive(Debug, Clone)]
pub(crate) struct RandrOutputState {
    /// Stable output id returned by randr.
    pub(crate) output: Output,
    /// Active crtc associated with the output.
    pub(crate) crtc: Crtc,
    /// Randr config timestamp used for `set_crtc_config`.
    pub(crate) config_timestamp: u32,
    /// Current crtc x coordinate.
    pub(crate) crtc_x: i16,
    /// Current crtc y coordinate.
    pub(crate) crtc_y: i16,
    /// Current output rotation.
    pub(crate) rotation: Rotation,
    /// Output list currently attached to the crtc.
    pub(crate) crtc_outputs: Vec<Output>,
    /// Current active mode id for the crtc.
    pub(crate) current_mode_id: Mode,
    /// Current active display mode payload.
    pub(crate) current_mode: DisplayMode,
    /// Desktop-preferred mode payload.
    pub(crate) desktop_mode: DisplayMode,
    /// Full mode catalog for this output keyed by mode id.
    pub(crate) modes_by_id: Vec<(Mode, DisplayMode)>,
    /// Descriptor payload for this output.
    pub(crate) descriptor: DisplayDescriptorSnapshot,
}

/// Build one stable display id from one randr output id.
fn display_id_for_output(output: Output) -> String {
    format!("{}{output}", display_id_prefix())
}

/// Parse one randr output id from one display id.
pub(crate) fn output_from_display_id(display_id: &str) -> Option<Output> {
    let value = display_id.strip_prefix(display_id_prefix())?;
    value.parse::<u32>().ok()
}

/// Resolve one display orientation value from one randr rotation mask.
fn orientation_from_rotation(rotation: Rotation) -> DisplayOrientation {
    let rotation_bits = u16::from(rotation);
    // resolve portrait orientation for one ninety-degree rotation
    if (rotation_bits & u16::from(Rotation::ROTATE90)) != 0 {
        return DisplayOrientation::Portrait;
    }
    // resolve flipped landscape orientation for one one-eighty rotation
    if (rotation_bits & u16::from(Rotation::ROTATE180)) != 0 {
        return DisplayOrientation::LandscapeFlipped;
    }
    // resolve flipped portrait orientation for one two-seventy rotation
    if (rotation_bits & u16::from(Rotation::ROTATE270)) != 0 {
        return DisplayOrientation::PortraitFlipped;
    }

    DisplayOrientation::Landscape
}

/// Build one best-effort refresh-rate payload from one randr mode record.
fn refresh_milli_hz_from_mode(mode: &ModeInfo) -> u32 {
    // return a conservative fallback when timing metadata is missing
    if mode.dot_clock == 0 || mode.htotal == 0 || mode.vtotal == 0 {
        return X11_DEFAULT_REFRESH_MILLI_HZ;
    }

    // derive refresh from pixel clock and total horizontal and vertical timing
    let mut numerator = u64::from(mode.dot_clock).saturating_mul(1000);
    let mut denominator = u64::from(mode.htotal).saturating_mul(u64::from(mode.vtotal));
    // double refresh for interlaced timings
    if u32::from(mode.mode_flags & ModeFlag::INTERLACE) != 0 {
        numerator = numerator.saturating_mul(2);
    }
    // halve refresh for double-scan timings
    if u32::from(mode.mode_flags & ModeFlag::DOUBLE_SCAN) != 0 {
        denominator = denominator.saturating_mul(2);
    }
    // fall back when the timing payload is still invalid
    if denominator == 0 {
        return X11_DEFAULT_REFRESH_MILLI_HZ;
    }

    (numerator / denominator).min(u64::from(u32::MAX)) as u32
}

/// Convert one randr mode payload into one display-mode value.
fn display_mode_from_randr_mode(mode: &ModeInfo, bit_depth: u16) -> DisplayMode {
    DisplayMode {
        width: mode.width as u32,
        height: mode.height as u32,
        refresh_milli_hz: refresh_milli_hz_from_mode(mode),
        format: 0,
        bit_depth,
    }
}

/// Resolve one builtin-panel support value from one monitor name.
fn builtin_panel_support(name: &str) -> DisplaySupportStatus {
    let upper = name.to_uppercase();
    // detect common builtin panel connector names
    if upper.contains("EDP") || upper.contains("LVDS") || upper.contains("DSI") {
        return DisplaySupportStatus::Supported;
    }

    DisplaySupportStatus::Unknown
}

/// Parse one `Xft.dpi` entry from one X11 resource-manager payload.
fn xft_dpi_from_resource_manager(value: &str) -> Option<f64> {
    for line in value.lines() {
        let line = line.split('!').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }

        let (key, value) = line.split_once(':')?;
        if !key.trim().eq_ignore_ascii_case("Xft.dpi") {
            continue;
        }

        let dpi = value.trim().parse::<f64>().ok()?;
        if dpi.is_finite() && dpi > 0.0 {
            return Some(dpi);
        }
    }

    None
}

/// Convert one DPI value into milli-scale units.
fn scale_factor_milli_from_dpi(dpi: f64) -> Option<u32> {
    if !dpi.is_finite() || dpi <= 0.0 {
        return None;
    }

    let scale_factor = (dpi / X11_DEFAULT_DESKTOP_DPI) * 1000.0;
    let scale_factor = scale_factor.round().clamp(1.0, u32::MAX as f64);
    Some(scale_factor as u32)
}

/// Read one global X11 desktop scale factor from the resource manager when available.
pub(crate) fn global_scale_factor_milli(
    connection_state: &core::X11ConnectionState,
    operation: &'static str,
) -> RuntimeResult<Option<u32>> {
    let reply = connection_state
        .connection
        .get_property(
            false,
            connection_state.root,
            connection_state.atoms.resource_manager,
            AtomEnum::STRING,
            0,
            u32::MAX,
        )
        .map_err(|error| {
            core::io_error(operation, format!("get_property request failed: {error}"))
        })?
        .reply()
        .map_err(|error| {
            core::io_error(operation, format!("get_property reply failed: {error}"))
        })?;
    let value = std::str::from_utf8(reply.value.as_slice())
        .ok()
        .and_then(xft_dpi_from_resource_manager)
        .and_then(scale_factor_milli_from_dpi);

    Ok(value)
}

/// Build one fallback scale factor from physical monitor size when no desktop scale exists.
fn physical_scale_factor_milli(
    width_px: u32,
    height_px: u32,
    width_mm: u32,
    height_mm: u32,
) -> Option<u32> {
    if width_px == 0 || height_px == 0 || width_mm == 0 || height_mm == 0 {
        return None;
    }

    let width_dpi = (width_px as f64 * 25.4) / width_mm as f64;
    let height_dpi = (height_px as f64 * 25.4) / height_mm as f64;
    let average_dpi = (width_dpi + height_dpi) / 2.0;

    scale_factor_milli_from_dpi(average_dpi)
}

/// Resolve one best-effort scale factor for one x11 output.
fn output_scale_factor_milli(
    connection_state: &core::X11ConnectionState,
    width_px: u32,
    height_px: u32,
    width_mm: u32,
    height_mm: u32,
    operation: &'static str,
) -> RuntimeResult<u32> {
    if let Some(scale_factor_milli) = global_scale_factor_milli(connection_state, operation)? {
        return Ok(scale_factor_milli);
    }

    if let Some(scale_factor_milli) =
        physical_scale_factor_milli(width_px, height_px, width_mm, height_mm)
    {
        return Ok(scale_factor_milli);
    }

    Ok(1000)
}

/// Resolve variable-refresh support from one randr output property when available.
fn variable_refresh_support(
    connection_state: &core::X11ConnectionState,
    output: Output,
) -> DisplaySupportStatus {
    // read one optional vrr-capable property value
    let property_cookie = match connection_state.connection.randr_get_output_property(
        output,
        connection_state.atoms.vrr_capable,
        AtomEnum::INTEGER,
        0,
        1,
        false,
        false,
    ) {
        Ok(value) => value,
        Err(_) => return DisplaySupportStatus::Unknown,
    };
    let property_reply = match property_cookie.reply() {
        Ok(value) => value,
        Err(_) => return DisplaySupportStatus::Unknown,
    };

    // decode one first cardinal payload value when present
    if property_reply.format != 32 {
        return DisplaySupportStatus::Unknown;
    }

    if property_reply.data.len() < std::mem::size_of::<u32>() {
        return DisplaySupportStatus::Unknown;
    }

    let value_bytes = [
        property_reply.data[0],
        property_reply.data[1],
        property_reply.data[2],
        property_reply.data[3],
    ];
    let value = u32::from_ne_bytes(value_bytes);

    // map zero and non-zero property payloads to support status
    if value == 0 {
        return DisplaySupportStatus::Unsupported;
    }

    DisplaySupportStatus::Supported
}

/// Read one root cardinal property payload from the selected x11 root window.
fn root_cardinal_property(
    connection_state: &core::X11ConnectionState,
    property: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<u32>> {
    let reply = connection_state
        .connection
        .get_property(
            false,
            connection_state.root,
            property,
            AtomEnum::CARDINAL,
            0,
            u32::MAX,
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
        .map_or_else(Vec::new, |value| value.collect()))
}

/// Read one root work-area payload from EWMH properties when available.
fn root_work_area(
    connection_state: &core::X11ConnectionState,
    operation: &'static str,
) -> RuntimeResult<Option<(i32, i32, u32, u32)>> {
    // read one desktop work-area list and reject empty payloads
    let work_area = root_cardinal_property(
        connection_state,
        connection_state.atoms.net_work_area,
        operation,
    )?;
    // reject missing work-area tuples
    if work_area.len() < 4 {
        return Ok(None);
    }

    // resolve current desktop lane and clamp to available work-area tuples
    let current_desktop = root_cardinal_property(
        connection_state,
        connection_state.atoms.net_current_desktop,
        operation,
    )?
    .first()
    .copied()
    .unwrap_or(0) as usize;
    let desktop_count = work_area.len() / 4;
    let selected_desktop = current_desktop.min(desktop_count.saturating_sub(1));
    let offset = selected_desktop.saturating_mul(4);

    let x = i32::try_from(work_area[offset]).unwrap_or(i32::MAX);
    let y = i32::try_from(work_area[offset + 1]).unwrap_or(i32::MAX);
    let width = work_area[offset + 2].max(1);
    let height = work_area[offset + 3].max(1);

    Ok(Some((x, y, width, height)))
}

/// Resolve one monitor-local work-area rectangle from one monitor and root work-area tuple.
fn monitor_work_area(
    monitor_x: i32,
    monitor_y: i32,
    monitor_width: u32,
    monitor_height: u32,
    root_work_area: Option<(i32, i32, u32, u32)>,
) -> (i32, i32, u32, u32) {
    let Some((root_x, root_y, root_width, root_height)) = root_work_area else {
        return (monitor_x, monitor_y, monitor_width, monitor_height);
    };

    let monitor_right = monitor_x.saturating_add_unsigned(monitor_width);
    let monitor_bottom = monitor_y.saturating_add_unsigned(monitor_height);
    let root_right = root_x.saturating_add_unsigned(root_width);
    let root_bottom = root_y.saturating_add_unsigned(root_height);

    let intersection_left = monitor_x.max(root_x);
    let intersection_top = monitor_y.max(root_y);
    let intersection_right = monitor_right.min(root_right);
    let intersection_bottom = monitor_bottom.min(root_bottom);

    if intersection_right <= intersection_left || intersection_bottom <= intersection_top {
        return (monitor_x, monitor_y, monitor_width, monitor_height);
    }

    let width = match u32::try_from(intersection_right - intersection_left) {
        Ok(value) => value,
        Err(_) => monitor_width,
    };
    let height = match u32::try_from(intersection_bottom - intersection_top) {
        Ok(value) => value,
        Err(_) => monitor_height,
    };

    (intersection_left, intersection_top, width, height)
}

/// Enumerate one active randr-output state list.
pub(crate) fn enumerate_randr_output_states(
    connection_state: &core::X11ConnectionState,
    operation: &'static str,
) -> RuntimeResult<Vec<RandrOutputState>> {
    // reject when randr is unavailable on this connection
    if !connection_state.extensions.randr {
        return Ok(Vec::new());
    }

    // read current randr resources and build one mode lookup map
    let resources_cookie = match connection_state
        .connection
        .randr_get_screen_resources_current(connection_state.root)
    {
        Ok(cookie) => cookie,
        Err(ConnectionError::UnsupportedExtension) => return Ok(Vec::new()),
        Err(error) => {
            return Err(core::io_error(
                operation,
                format!("randr_get_screen_resources_current request failed: {error}"),
            ));
        }
    };
    let resources = resources_cookie.reply().map_err(|error| {
        core::io_error(
            operation,
            format!("randr_get_screen_resources_current reply failed: {error}"),
        )
    })?;
    let mut mode_info_by_id = HashMap::with_capacity(resources.modes.len());
    for mode in &resources.modes {
        mode_info_by_id.insert(mode.id, *mode);
    }

    // resolve primary output when available
    let primary_output = connection_state
        .connection
        .randr_get_output_primary(connection_state.root)
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_output_primary request failed: {error}"),
            )
        })?
        .reply()
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_output_primary reply failed: {error}"),
            )
        })?
        .output;

    // resolve the selected x11 root depth for mode payload conversion
    let screen = connection_state
        .connection
        .setup()
        .roots
        .get(connection_state.screen_index)
        .ok_or_else(|| core_platform::invalid_state("x11 setup missing selected screen root"))?;
    let bit_depth = screen.root_depth as u16;
    let root_work_area = root_work_area(connection_state, operation)?;

    // build one monitor-state row for each connected and active output
    let mut states = Vec::new();
    for output in resources.outputs.iter().copied() {
        let output_info = connection_state
            .connection
            .randr_get_output_info(output, resources.config_timestamp)
            .map_err(|error| {
                core::io_error(
                    operation,
                    format!("randr_get_output_info request failed: {error}"),
                )
            })?
            .reply()
            .map_err(|error| {
                core::io_error(
                    operation,
                    format!("randr_get_output_info reply failed: {error}"),
                )
            })?;

        if output_info.connection != RandrOutputConnection::CONNECTED {
            continue;
        }
        if output_info.crtc == 0 {
            continue;
        }

        let crtc_info = connection_state
            .connection
            .randr_get_crtc_info(output_info.crtc, resources.config_timestamp)
            .map_err(|error| {
                core::io_error(
                    operation,
                    format!("randr_get_crtc_info request failed: {error}"),
                )
            })?
            .reply()
            .map_err(|error| {
                core::io_error(
                    operation,
                    format!("randr_get_crtc_info reply failed: {error}"),
                )
            })?;
        if crtc_info.mode == 0 {
            continue;
        }

        let mut modes_by_id = Vec::with_capacity(output_info.modes.len());
        for mode_id in output_info.modes.iter().copied() {
            let Some(mode_info) = mode_info_by_id.get(&mode_id).copied() else {
                continue;
            };
            let mode = display_mode_from_randr_mode(&mode_info, bit_depth);
            modes_by_id.push((mode_id, mode));
        }
        if modes_by_id.is_empty() {
            continue;
        }

        let Some(current_mode) = current_mode_from_catalog(&modes_by_id, crtc_info.mode) else {
            continue;
        };
        let desktop_mode = desktop_mode_from_catalog(&output_info, &modes_by_id, current_mode);
        let output_name = String::from_utf8_lossy(&output_info.name).to_string();
        let output_name = if output_name.trim().is_empty() {
            format!("{}-output-{output}", core::selected_backend_name())
        } else {
            output_name
        };

        let width_px = if crtc_info.width == 0 {
            current_mode.width
        } else {
            crtc_info.width as u32
        };
        let height_px = if crtc_info.height == 0 {
            current_mode.height
        } else {
            crtc_info.height as u32
        };
        let primary = output == primary_output && primary_output != 0;
        let builtin_panel = builtin_panel_support(&output_name);
        let (work_area_x, work_area_y, work_area_width_px, work_area_height_px) = monitor_work_area(
            i32::from(crtc_info.x),
            i32::from(crtc_info.y),
            width_px,
            height_px,
            root_work_area,
        );

        let descriptor = DisplayDescriptorSnapshot {
            backend: core::selected_backend(),
            id: display_id_for_output(output),
            name: output_name,
            primary,
            x: i32::from(crtc_info.x),
            y: i32::from(crtc_info.y),
            width_px,
            height_px,
            work_area_x,
            work_area_y,
            work_area_width_px,
            work_area_height_px,
            width_mm: output_info.mm_width,
            height_mm: output_info.mm_height,
            scale_factor_milli: output_scale_factor_milli(
                connection_state,
                width_px,
                height_px,
                output_info.mm_width,
                output_info.mm_height,
                operation,
            )?,
            orientation: orientation_from_rotation(crtc_info.rotation),
            builtin_panel,
            variable_refresh_support: variable_refresh_support(connection_state, output),
            hdr_support: DisplaySupportStatus::Unsupported,
        };

        let state = RandrOutputState {
            output,
            crtc: output_info.crtc,
            config_timestamp: resources.config_timestamp,
            crtc_x: crtc_info.x,
            crtc_y: crtc_info.y,
            rotation: crtc_info.rotation,
            crtc_outputs: if crtc_info.outputs.is_empty() {
                vec![output]
            } else {
                crtc_info.outputs
            },
            current_mode_id: crtc_info.mode,
            current_mode,
            desktop_mode,
            modes_by_id,
            descriptor,
        };
        states.push(state);
    }

    Ok(states)
}

/// Enumerate one fallback monitor snapshot when randr output discovery is unavailable.
pub(crate) fn enumerate_fallback_monitor_snapshots(
    connection_state: &core::X11ConnectionState,
) -> RuntimeResult<Vec<MonitorSnapshot>> {
    // resolve one x11 setup screen for this connection
    let screen = connection_state
        .connection
        .setup()
        .roots
        .get(connection_state.screen_index)
        .ok_or_else(|| core_platform::invalid_state("x11 setup missing selected screen root"))?;

    // build one synthetic descriptor and one current mode from setup geometry
    let width_px = screen.width_in_pixels as u32;
    let height_px = screen.height_in_pixels as u32;
    let width_mm = screen.width_in_millimeters as u32;
    let height_mm = screen.height_in_millimeters as u32;
    let current_mode = DisplayMode {
        width: width_px,
        height: height_px,
        refresh_milli_hz: X11_DEFAULT_REFRESH_MILLI_HZ,
        format: 0,
        bit_depth: screen.root_depth as u16,
    };
    let descriptor = DisplayDescriptorSnapshot {
        backend: core::selected_backend(),
        id: format!(
            "{}screen-{}",
            core::selected_backend_name(),
            connection_state.screen_index
        ),
        name: std::env::var("DISPLAY")
            .unwrap_or_else(|_| format!("{}-display", core::selected_backend_name())),
        primary: true,
        x: 0,
        y: 0,
        width_px,
        height_px,
        work_area_x: 0,
        work_area_y: 0,
        work_area_width_px: width_px,
        work_area_height_px: height_px,
        width_mm,
        height_mm,
        scale_factor_milli: output_scale_factor_milli(
            connection_state,
            width_px,
            height_px,
            width_mm,
            height_mm,
            "destack.display.monitor.list",
        )?,
        orientation: DisplayOrientation::Landscape,
        builtin_panel: DisplaySupportStatus::Unknown,
        variable_refresh_support: DisplaySupportStatus::Unknown,
        hdr_support: DisplaySupportStatus::Unsupported,
    };

    Ok(vec![MonitorSnapshot {
        descriptor,
        current_mode,
        desktop_mode: current_mode,
        modes: vec![current_mode],
    }])
}

#[cfg(test)]
mod tests {
    use super::{
        physical_scale_factor_milli, scale_factor_milli_from_dpi, xft_dpi_from_resource_manager,
    };

    #[test]
    fn test_x11_resource_manager_parses_xft_dpi() {
        let value = "\nXcursor.size:\t24\nXft.dpi:\t192\n";

        assert_eq!(xft_dpi_from_resource_manager(value), Some(192.0));
    }

    #[test]
    fn test_x11_scale_factor_milli_from_dpi_uses_desktop_baseline() {
        assert_eq!(scale_factor_milli_from_dpi(96.0), Some(1000));
        assert_eq!(scale_factor_milli_from_dpi(192.0), Some(2000));
    }

    #[test]
    fn test_x11_physical_scale_factor_milli_uses_monitor_dpi() {
        let scale_factor_milli = physical_scale_factor_milli(3840, 2160, 600, 340);

        assert_eq!(scale_factor_milli, Some(1687));
    }
}
