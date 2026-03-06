use std::collections::HashMap;

use x11rb::connection::Connection;
use x11rb::errors::ConnectionError;
use x11rb::protocol::randr::{
    Connection as RandrOutputConnection, ConnectionExt as RandrConnectionExt, Crtc,
    GetOutputInfoReply, Mode, ModeFlag, ModeInfo, Output, Rotation, SetConfig,
};
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as XprotoConnectionExt};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayColorSpace, DisplayColorState, DisplayDescriptor, DisplayGammaRamp, DisplayHdrMode,
    DisplayMode, DisplayMonitorListRequest, DisplayMonitorOpenOptions, DisplayOrientation,
    DisplaySupportStatus,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef};

use super::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use super::{core, event, resource as display_resource};

/// Default x11 color depth assumption when detailed channel depth is unavailable.
const X11_DEFAULT_BITS_PER_CHANNEL: u16 = 8;
/// Fallback refresh-rate used when one mode payload omits timing information.
const X11_DEFAULT_REFRESH_MILLI_HZ: u32 = 60_000;

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
struct RandrOutputState {
    /// Stable output id returned by randr.
    output: Output,
    /// Active crtc associated with the output.
    crtc: Crtc,
    /// Randr config timestamp used for `set_crtc_config`.
    config_timestamp: u32,
    /// Current crtc x coordinate.
    crtc_x: i16,
    /// Current crtc y coordinate.
    crtc_y: i16,
    /// Current output rotation.
    rotation: Rotation,
    /// Output list currently attached to the crtc.
    crtc_outputs: Vec<Output>,
    /// Current active mode id for the crtc.
    current_mode_id: Mode,
    /// Current active display mode payload.
    current_mode: DisplayMode,
    /// Desktop-preferred mode payload.
    desktop_mode: DisplayMode,
    /// Full mode catalog for this output keyed by mode id.
    modes_by_id: Vec<(Mode, DisplayMode)>,
    /// Descriptor payload for this output.
    descriptor: DisplayDescriptorSnapshot,
}

/// Build one stable display id from one randr output id.
fn display_id_for_output(output: Output) -> String {
    format!("{}{output}", display_id_prefix())
}

/// Parse one randr output id from one display id.
fn output_from_display_id(display_id: &str) -> Option<Output> {
    let value = display_id.strip_prefix(display_id_prefix())?;
    value.parse::<u32>().ok()
}

/// Resolve one display orientation value from one randr rotation mask.
fn orientation_from_rotation(rotation: Rotation) -> DisplayOrientation {
    let rotation_bits = u16::from(rotation);
    // evaluate this condition
    if (rotation_bits & u16::from(Rotation::ROTATE90)) != 0 {
        return DisplayOrientation::Portrait;
    }
    // evaluate this condition
    if (rotation_bits & u16::from(Rotation::ROTATE180)) != 0 {
        return DisplayOrientation::LandscapeFlipped;
    }
    // evaluate this condition
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
    // evaluate this condition
    if u32::from(mode.mode_flags & ModeFlag::INTERLACE) != 0 {
        numerator = numerator.saturating_mul(2);
    }
    // evaluate this condition
    if u32::from(mode.mode_flags & ModeFlag::DOUBLE_SCAN) != 0 {
        denominator = denominator.saturating_mul(2);
    }
    // evaluate this condition
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
    // evaluate this condition
    if upper.contains("EDP") || upper.contains("LVDS") || upper.contains("DSI") {
        return DisplaySupportStatus::Supported;
    }

    DisplaySupportStatus::Unknown
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
    // evaluate this condition
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

    let x = match i32::try_from(work_area[offset]) {
        Ok(value) => value,
        Err(_) => i32::MAX,
    };
    let y = match i32::try_from(work_area[offset + 1]) {
        Ok(value) => value,
        Err(_) => i32::MAX,
    };
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

    // evaluate this condition
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

/// Resolve one current mode payload from one mode catalog and one active mode id.
fn current_mode_from_catalog(
    modes_by_id: &[(Mode, DisplayMode)],
    active_mode_id: Mode,
) -> Option<DisplayMode> {
    // evaluate this condition
    if let Some((_, mode)) = modes_by_id
        .iter()
        .find(|(mode_id, _)| *mode_id == active_mode_id)
    {
        return Some(*mode);
    }

    modes_by_id.first().map(|(_, mode)| *mode)
}

/// Resolve one desktop-preferred mode payload from one output mode order.
fn desktop_mode_from_catalog(
    output_info: &GetOutputInfoReply,
    modes_by_id: &[(Mode, DisplayMode)],
    current_mode: DisplayMode,
) -> DisplayMode {
    // prefer one mode in the output preferred-mode prefix
    let preferred_count = output_info.num_preferred as usize;
    // iterate this sequence
    for mode_id in output_info.modes.iter().copied().take(preferred_count) {
        // evaluate this condition
        if let Some((_, mode)) = modes_by_id
            .iter()
            .find(|(candidate, _)| *candidate == mode_id)
        {
            return *mode;
        }
    }

    current_mode
}

/// Resolve one deduplicated mode list from one output mode catalog.
fn unique_modes_from_catalog(modes_by_id: &[(Mode, DisplayMode)]) -> Vec<DisplayMode> {
    let mut modes = Vec::with_capacity(modes_by_id.len());
    // iterate this sequence
    for (_, mode) in modes_by_id {
        // evaluate this condition
        if modes.contains(mode) {
            continue;
        }
        modes.push(*mode);
    }

    modes
}

/// Resolve one x11 status-name string for one randr `SetConfig` value.
fn randr_set_config_name(status: SetConfig) -> &'static str {
    // evaluate this condition
    if status == SetConfig::SUCCESS {
        return "success";
    }
    // evaluate this condition
    if status == SetConfig::INVALID_CONFIG_TIME {
        return "invalidConfigTime";
    }
    // evaluate this condition
    if status == SetConfig::INVALID_TIME {
        return "invalidTime";
    }
    // evaluate this condition
    if status == SetConfig::FAILED {
        return "failed";
    }

    "unknown"
}

/// Resolve one compatible mode id for one requested mode payload.
fn select_mode_id_for_request(
    modes_by_id: &[(Mode, DisplayMode)],
    requested: DisplayMode,
) -> Option<(Mode, DisplayMode)> {
    // require at least one mode with matching width and height
    let mut selected: Option<(Mode, DisplayMode)> = None;
    let mut selected_refresh_delta = u32::MAX;
    // iterate this sequence
    for (mode_id, mode) in modes_by_id.iter().copied() {
        // evaluate this condition
        if mode.width != requested.width || mode.height != requested.height {
            continue;
        }

        let refresh_delta = if requested.refresh_milli_hz == 0 {
            0
        } else {
            mode.refresh_milli_hz.abs_diff(requested.refresh_milli_hz)
        };
        // evaluate this condition
        if selected.is_none() || refresh_delta < selected_refresh_delta {
            selected = Some((mode_id, mode));
            selected_refresh_delta = refresh_delta;
        }
    }

    selected
}

/// Resolve one active output state snapshot from one display id.
fn resolve_output_state_by_display_id(
    connection_state: &core::X11ConnectionState,
    display_id: &str,
    operation: &'static str,
) -> RuntimeResult<Option<RandrOutputState>> {
    let Some(output) = output_from_display_id(display_id) else {
        return Ok(None);
    };
    let states = enumerate_randr_output_states(connection_state, operation)?;

    Ok(states.into_iter().find(|state| state.output == output))
}

/// Resolve one active randr CRTC for gamma and color operations.
fn resolve_randr_crtc(
    connection_state: &core::X11ConnectionState,
    operation: &'static str,
) -> RuntimeResult<Option<Crtc>> {
    // read current randr screen resources for this root window
    let resources_cookie = match connection_state
        .connection
        .randr_get_screen_resources_current(connection_state.root)
    {
        Ok(cookie) => cookie,
        Err(ConnectionError::UnsupportedExtension) => return Ok(None),
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
    // evaluate this condition
    if resources.crtcs.is_empty() {
        return Ok(None);
    }

    // prefer the primary output CRTC when available
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
        })?;
    // evaluate this condition
    if primary_output.output != 0 {
        let output_info = connection_state
            .connection
            .randr_get_output_info(primary_output.output, resources.config_timestamp)
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
        // evaluate this condition
        if output_info.crtc != 0 {
            return Ok(Some(output_info.crtc));
        }
    }

    // otherwise use the first connected output that has an active CRTC
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
        // evaluate this condition
        if output_info.crtc != 0 {
            return Ok(Some(output_info.crtc));
        }
    }

    // fall back to the first advertised CRTC lane
    Ok(resources.crtcs.iter().copied().find(|crtc| *crtc != 0))
}

/// Resolve one active randr CRTC for one display id.
fn resolve_randr_crtc_for_display(
    connection_state: &core::X11ConnectionState,
    display_id: &str,
    operation: &'static str,
) -> RuntimeResult<Option<Crtc>> {
    // prefer the monitor-specific output when one randr-backed id is available
    if let Some(state) =
        resolve_output_state_by_display_id(connection_state, display_id, operation)?
    {
        return Ok(Some(state.crtc));
    }

    resolve_randr_crtc(connection_state, operation)
}

/// Read one randr gamma size for one CRTC.
fn read_randr_gamma_size(
    connection_state: &core::X11ConnectionState,
    crtc: Crtc,
    operation: &'static str,
) -> RuntimeResult<usize> {
    let reply = connection_state
        .connection
        .randr_get_crtc_gamma_size(crtc)
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_crtc_gamma_size request failed: {error}"),
            )
        })?
        .reply()
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_crtc_gamma_size reply failed: {error}"),
            )
        })?;

    Ok(reply.size as usize)
}

/// Read one randr gamma ramp payload for one CRTC.
fn read_randr_gamma_ramp(
    connection_state: &core::X11ConnectionState,
    crtc: Crtc,
    operation: &'static str,
) -> RuntimeResult<(Vec<u16>, Vec<u16>, Vec<u16>)> {
    let reply = connection_state
        .connection
        .randr_get_crtc_gamma(crtc)
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_crtc_gamma request failed: {error}"),
            )
        })?
        .reply()
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_get_crtc_gamma reply failed: {error}"),
            )
        })?;
    // evaluate this condition
    if reply.red.is_empty()
        || reply.red.len() != reply.green.len()
        || reply.red.len() != reply.blue.len()
    {
        return Err(core::io_error(
            operation,
            "randr_get_crtc_gamma returned invalid channel lengths",
        ));
    }

    Ok((reply.red, reply.green, reply.blue))
}

/// Write one randr gamma ramp payload for one CRTC.
fn write_randr_gamma_ramp(
    connection_state: &core::X11ConnectionState,
    crtc: Crtc,
    red: &[u16],
    green: &[u16],
    blue: &[u16],
    operation: &'static str,
) -> RuntimeResult<()> {
    let cookie = connection_state
        .connection
        .randr_set_crtc_gamma(crtc, red, green, blue)
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_set_crtc_gamma request failed: {error}"),
            )
        })?;
    cookie.check().map_err(|error| {
        core::io_error(
            operation,
            format!("randr_set_crtc_gamma check failed: {error}"),
        )
    })?;
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    Ok(())
}

/// Return whether monitor mode-set is supported for the active x11 connection.
pub(crate) fn monitor_mode_set_supported(
    connection_state: &core::X11ConnectionState,
) -> RuntimeResult<bool> {
    // reject mode-set when randr is unavailable
    if !connection_state.extensions.randr {
        return Ok(false);
    }
    let states = enumerate_randr_output_states(connection_state, "destack.display.backend.list")?;

    Ok(states
        .iter()
        .any(|state| state.crtc != 0 && !state.modes_by_id.is_empty()))
}

/// Enumerate one active randr-output state list.
fn enumerate_randr_output_states(
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
    // iterate this sequence
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
    // iterate this sequence
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

        // evaluate this condition
        if output_info.connection != RandrOutputConnection::CONNECTED {
            continue;
        }
        // evaluate this condition
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
        // evaluate this condition
        if crtc_info.mode == 0 {
            continue;
        }

        let mut modes_by_id = Vec::with_capacity(output_info.modes.len());
        // iterate this sequence
        for mode_id in output_info.modes.iter().copied() {
            let Some(mode_info) = mode_info_by_id.get(&mode_id).copied() else {
                continue;
            };
            let mode = display_mode_from_randr_mode(&mode_info, bit_depth);
            modes_by_id.push((mode_id, mode));
        }
        // evaluate this condition
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
            scale_factor_milli: 1000,
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

    // assign one fallback primary display when no output reports primary
    if !states.is_empty() && !states.iter().any(|state| state.descriptor.primary) {
        // evaluate this condition
        if let Some(first_state) = states.first_mut() {
            first_state.descriptor.primary = true;
        }
    }

    Ok(states)
}

/// Enumerate one fallback monitor snapshot when randr output discovery is unavailable.
fn enumerate_fallback_monitor_snapshots(
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
        scale_factor_milli: 1000,
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

/// Enumerate one normalized monitor snapshot list for x11.
pub(crate) fn enumerate_monitor_snapshots(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<MonitorSnapshot>> {
    // load one connection snapshot for monitor enumeration
    let runtime_state = core::runtime_state(binding);
    let connection_state = core::connection_state(&runtime_state, "destack.display.monitor.list")?;

    // prefer one real randr output topology when available
    let output_states =
        enumerate_randr_output_states(connection_state.as_ref(), "destack.display.monitor.list")?;
    // evaluate this condition
    if !output_states.is_empty() {
        let mut snapshots = Vec::with_capacity(output_states.len());
        // iterate this sequence
        for state in output_states {
            let snapshot = MonitorSnapshot {
                descriptor: state.descriptor,
                current_mode: state.current_mode,
                desktop_mode: state.desktop_mode,
                modes: unique_modes_from_catalog(&state.modes_by_id),
            };
            snapshots.push(snapshot);
        }
        return Ok(snapshots);
    }

    // fall back to one synthetic screen-level monitor when randr data is unavailable
    enumerate_fallback_monitor_snapshots(connection_state.as_ref())
}

/// Convert one owned descriptor into one ABI payload.
pub(crate) fn descriptor_from_owned(
    binding: &BindingCallContext,
    value: &DisplayDescriptorSnapshot,
) -> DisplayDescriptor {
    DisplayDescriptor {
        backend: value.backend,
        id: binding.store_string(&value.id),
        name: binding.store_string(&value.name),
        primary: value.primary,
        x: value.x,
        y: value.y,
        width_px: value.width_px,
        height_px: value.height_px,
        work_area_x: value.work_area_x,
        work_area_y: value.work_area_y,
        work_area_width_px: value.work_area_width_px,
        work_area_height_px: value.work_area_height_px,
        width_mm: value.width_mm,
        height_mm: value.height_mm,
        scale_factor_milli: value.scale_factor_milli,
        orientation: value.orientation,
        builtin_panel: value.builtin_panel,
        variable_refresh_support: value.variable_refresh_support,
        hdr_support: value.hdr_support,
    }
}

/// Close one display endpoint.
pub(crate) unsafe fn monitor_close(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // remove the resource entry from the runtime table
    let removed = binding
        .agent()
        .resources
        .remove(binding.world(), handle.0, Some(binding.engine()))
        .is_some();
    // evaluate this condition
    if !removed {
        return Err(core::display_not_found(
            "destack.display.monitor.close",
            handle,
        ));
    }

    Ok(())
}

/// Resolve one requested mode to the closest supported mode.
pub(crate) unsafe fn monitor_closest_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
    requested: DisplayMode,
) -> RuntimeResult<()> {
    // validate out pointer and resolve the monitor id from the handle
    core_platform::ensure_out(out, "out")?;
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.closestMode",
    )?;

    // resolve the current monitor snapshot for this handle
    let snapshots = enumerate_monitor_snapshots(binding)?;
    let snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.descriptor.id == display_id)
        .ok_or_else(|| core::display_not_found("destack.display.monitor.closestMode", handle))?;

    // choose one closest mode using area delta and refresh-rate delta
    let mut selected = snapshot.current_mode;
    let mut selected_score = u64::MAX;
    // iterate this sequence
    for mode in &snapshot.modes {
        let width_delta = mode.width.abs_diff(requested.width) as u64;
        let height_delta = mode.height.abs_diff(requested.height) as u64;
        let refresh_delta = mode.refresh_milli_hz.abs_diff(requested.refresh_milli_hz) as u64;
        let score = width_delta
            .saturating_mul(1_000_000)
            .saturating_add(height_delta.saturating_mul(1_000_000))
            .saturating_add(refresh_delta);
        // evaluate this condition
        if score < selected_score {
            selected_score = score;
            selected = *mode;
        }
    }

    unsafe {
        *out = selected;
    }

    Ok(())
}

/// Read the current mode for one opened display.
pub(crate) unsafe fn monitor_current_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve monitor id
    core_platform::ensure_out(out, "out")?;
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.currentMode",
    )?;

    // read the monitor snapshot and write current mode
    let snapshots = enumerate_monitor_snapshots(binding)?;
    let snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.descriptor.id == display_id)
        .ok_or_else(|| core::display_not_found("destack.display.monitor.currentMode", handle))?;
    unsafe {
        *out = snapshot.current_mode;
    }

    Ok(())
}

/// Read descriptor metadata for one opened display.
pub(crate) unsafe fn monitor_descriptor(
    binding: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve monitor id
    core_platform::ensure_out(out, "out")?;
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.descriptor",
    )?;

    // read the monitor snapshot and write descriptor payload
    let snapshots = enumerate_monitor_snapshots(binding)?;
    let snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.descriptor.id == display_id)
        .ok_or_else(|| core::display_not_found("destack.display.monitor.descriptor", handle))?;
    unsafe {
        *out = descriptor_from_owned(binding, &snapshot.descriptor);
    }

    Ok(())
}

/// Read the desktop-preferred mode for one opened display.
pub(crate) unsafe fn monitor_desktop_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve monitor id
    core_platform::ensure_out(out, "out")?;
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.desktopMode",
    )?;

    // read the monitor snapshot and write desktop mode payload
    let snapshots = enumerate_monitor_snapshots(binding)?;
    let snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.descriptor.id == display_id)
        .ok_or_else(|| core::display_not_found("destack.display.monitor.desktopMode", handle))?;
    unsafe {
        *out = snapshot.desktop_mode;
    }

    Ok(())
}

/// List available displays.
pub(crate) unsafe fn monitor_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    _request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // enumerate monitor descriptors and store output slice
    let snapshots = enumerate_monitor_snapshots(binding)?;
    let mut values = Vec::with_capacity(snapshots.len());
    // iterate this sequence
    for snapshot in snapshots {
        values.push(descriptor_from_owned(binding, &snapshot.descriptor));
    }
    unsafe {
        *out = binding.store_slice(values);
    }

    Ok(())
}

/// Read available display modes.
pub(crate) unsafe fn monitor_modes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve monitor id
    core_platform::ensure_out(out, "out")?;
    let display_id =
        display_resource::resolve_display_id(binding, handle, "destack.display.monitor.modes")?;

    // read monitor modes and store output slice
    let snapshots = enumerate_monitor_snapshots(binding)?;
    let snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.descriptor.id == display_id)
        .ok_or_else(|| core::display_not_found("destack.display.monitor.modes", handle))?;
    unsafe {
        *out = binding.store_slice(snapshot.modes.clone());
    }

    Ok(())
}

/// Open one display endpoint.
pub(crate) unsafe fn monitor_open(
    binding: &BindingCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
    _options: DisplayMonitorOpenOptions,
) -> RuntimeResult<()> {
    // validate out pointer and parse requested id
    core_platform::ensure_out(out, "out")?;
    let id = unsafe { id.as_str()? };

    // validate that the requested id exists
    let snapshots = enumerate_monitor_snapshots(binding)?;
    let snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.descriptor.id == id)
        .ok_or_else(|| {
            core_platform::io_not_found(
                "destack.display.monitor.open",
                format!("display id `{id}` was not found"),
            )
        })?;

    // insert one display handle resource and write output
    let handle = display_resource::open_display_handle(binding, snapshot.descriptor.id.clone());
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Read the current primary display handle.
pub(crate) unsafe fn monitor_primary(
    binding: &BindingCallContext,
    out: *mut Option<resource::DisplayHandle>,
    _request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // resolve one primary snapshot and create one resource handle
    let snapshots = enumerate_monitor_snapshots(binding)?;
    let primary = snapshots
        .iter()
        .find(|snapshot| snapshot.descriptor.primary);
    let primary_handle = primary.map(|snapshot| {
        display_resource::open_display_handle(binding, snapshot.descriptor.id.clone())
    });

    unsafe {
        *out = primary_handle;
    }

    Ok(())
}

/// Resolve one monitor snapshot by stable display id.
pub(crate) fn monitor_snapshot_by_display_id(
    binding: &BindingCallContext,
    display_id: &str,
) -> RuntimeResult<Option<MonitorSnapshot>> {
    let snapshots = enumerate_monitor_snapshots(binding)?;

    Ok(snapshots
        .into_iter()
        .find(|snapshot| snapshot.descriptor.id == display_id))
}

/// Apply one display mode by stable display id and return the effective current mode.
pub(crate) fn apply_monitor_mode_by_display_id(
    binding: &BindingCallContext,
    display_id: &str,
    mode: DisplayMode,
    operation: &'static str,
) -> RuntimeResult<DisplayMode> {
    // validate mode dimensions
    if mode.width == 0 || mode.height == 0 {
        return Err(core_platform::invalid_argument(
            "mode",
            "mode width and height must be greater than zero",
        ));
    }

    // validate that the target display still exists
    let Some(previous_snapshot) = monitor_snapshot_by_display_id(binding, display_id)? else {
        return Err(core_platform::io_not_found(
            operation,
            format!("display id '{display_id}' is no longer available"),
        ));
    };

    // resolve one output state and selected compatible mode id
    let runtime_state = core::runtime_state(binding);
    let connection_state = core::connection_state(&runtime_state, operation)?;
    let Some(output_state) =
        resolve_output_state_by_display_id(connection_state.as_ref(), display_id, operation)?
    // otherwise fall back
    else {
        return Err(core_platform::not_supported(operation));
    };
    let Some((selected_mode_id, selected_mode)) =
        select_mode_id_for_request(&output_state.modes_by_id, mode)
    // otherwise fall back
    else {
        return Err(core_platform::invalid_argument(
            "mode",
            "requested mode is not supported by the selected display",
        ));
    };

    // return early when the mode is already active
    if output_state.current_mode_id == selected_mode_id {
        return Ok(selected_mode);
    }

    // apply one randr mode transition and retry once for stale timestamps
    let mut set_mode_reply = connection_state
        .connection
        .randr_set_crtc_config(
            output_state.crtc,
            0,
            output_state.config_timestamp,
            output_state.crtc_x,
            output_state.crtc_y,
            selected_mode_id,
            output_state.rotation,
            &output_state.crtc_outputs,
        )
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_set_crtc_config request failed: {error}"),
            )
        })?
        .reply()
        .map_err(|error| {
            core::io_error(
                operation,
                format!("randr_set_crtc_config reply failed: {error}"),
            )
        })?;
    // evaluate this condition
    if set_mode_reply.status == SetConfig::INVALID_CONFIG_TIME
        || set_mode_reply.status == SetConfig::INVALID_TIME
    {
        let Some(retry_output_state) =
            resolve_output_state_by_display_id(connection_state.as_ref(), display_id, operation)?
        // otherwise fall back
        else {
            return Err(core_platform::not_supported(operation));
        };
        let Some((retry_mode_id, _)) =
            select_mode_id_for_request(&retry_output_state.modes_by_id, mode)
        // otherwise fall back
        else {
            return Err(core_platform::invalid_argument(
                "mode",
                "requested mode is not supported by the selected display",
            ));
        };

        set_mode_reply = connection_state
            .connection
            .randr_set_crtc_config(
                retry_output_state.crtc,
                set_mode_reply.timestamp,
                retry_output_state.config_timestamp,
                retry_output_state.crtc_x,
                retry_output_state.crtc_y,
                retry_mode_id,
                retry_output_state.rotation,
                &retry_output_state.crtc_outputs,
            )
            .map_err(|error| {
                core::io_error(
                    operation,
                    format!("randr_set_crtc_config request failed: {error}"),
                )
            })?
            .reply()
            .map_err(|error| {
                core::io_error(
                    operation,
                    format!("randr_set_crtc_config reply failed: {error}"),
                )
            })?;
    }
    // evaluate this condition
    if set_mode_reply.status != SetConfig::SUCCESS {
        return Err(core::io_error(
            operation,
            format!(
                "randr_set_crtc_config failed with status {}",
                randr_set_config_name(set_mode_reply.status)
            ),
        ));
    }
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    // read back one post-apply mode snapshot from host state
    let current_mode = monitor_snapshot_by_display_id(binding, display_id)?
        .map(|snapshot| snapshot.current_mode)
        .unwrap_or(previous_snapshot.current_mode);

    Ok(current_mode)
}

/// Apply one display mode.
pub(crate) unsafe fn monitor_set_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    // resolve one concrete display id and previous mode snapshot
    let display_id =
        display_resource::resolve_display_id(binding, handle, "destack.display.monitor.setMode")?;
    let previous_snapshot = monitor_snapshot_by_display_id(binding, &display_id)?
        .ok_or_else(|| core::display_not_found("destack.display.monitor.setMode", handle))?;
    let previous_mode = previous_snapshot.current_mode;

    // apply mode and resolve one current snapshot after mutation
    let applied_mode = apply_monitor_mode_by_display_id(
        binding,
        &display_id,
        mode,
        "destack.display.monitor.setMode",
    )?;
    let current_snapshot = monitor_snapshot_by_display_id(binding, &display_id)?;
    let current_mode = current_snapshot
        .as_ref()
        .map(|snapshot| snapshot.current_mode)
        .unwrap_or(applied_mode);

    // publish monitor mode and descriptor deltas
    let runtime_state = core::runtime_state(binding);
    event::publish_monitor_mode_changed(
        &runtime_state,
        &display_id,
        Some(previous_mode),
        current_mode,
    );
    // evaluate this condition
    if let Some(current_snapshot) = current_snapshot
        && current_snapshot.descriptor != previous_snapshot.descriptor
    {
        event::publish_monitor_descriptor_changed(
            &runtime_state,
            Some(previous_snapshot.descriptor),
            current_snapshot.descriptor,
        );
    }
    event::refresh_monitor_topology_cache(binding)?;

    Ok(())
}

/// Read display color state.
pub(crate) unsafe fn monitor_color_state(
    binding: &BindingCallContext,
    out: *mut DisplayColorState,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and display handle
    core_platform::ensure_out(out, "out")?;
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.colorState",
    )?;
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.monitor.colorState")?;
    let crtc = resolve_randr_crtc_for_display(
        connection_state.as_ref(),
        &display_id,
        "destack.display.monitor.colorState",
    )?
    .ok_or_else(|| core_platform::not_supported("destack.display.monitor.colorState"))?;

    // require one randr gamma lane for color-state support
    let gamma_size = read_randr_gamma_size(
        connection_state.as_ref(),
        crtc,
        "destack.display.monitor.colorState",
    )?;
    // evaluate this condition
    if gamma_size == 0 {
        return Err(core_platform::not_supported(
            "destack.display.monitor.colorState",
        ));
    }

    let state = DisplayColorState {
        hdr_mode: DisplayHdrMode::Unknown,
        color_space: DisplayColorSpace::Srgb,
        bits_per_channel: Some(X11_DEFAULT_BITS_PER_CHANNEL),
    };

    unsafe {
        *out = state;
    }

    Ok(())
}

/// Read display HDR mode.
pub(crate) unsafe fn monitor_hdr_mode(
    binding: &BindingCallContext,
    out: *mut DisplayHdrMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and display handle
    core_platform::ensure_out(out, "out")?;
    display_resource::resolve_display_id(binding, handle, "destack.display.monitor.hdrMode")?;

    unsafe {
        *out = DisplayHdrMode::Unknown;
    }

    Ok(())
}

/// Set display HDR mode.
pub(crate) unsafe fn monitor_set_hdr_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayHdrMode,
) -> RuntimeResult<()> {
    // validate display handle and mode payload
    display_resource::resolve_display_id(binding, handle, "destack.display.monitor.setHdrMode")?;
    // evaluate this condition
    if mode == DisplayHdrMode::Unknown {
        return Err(core_platform::invalid_argument(
            "mode",
            "hdr mode must be one concrete mode",
        ));
    }

    // x11 has no portable hdr-control primitive: system is no-op and concrete modes are unsupported
    if mode == DisplayHdrMode::System {
        return Ok(());
    }
    // evaluate this condition
    if mode == DisplayHdrMode::Hdr || mode == DisplayHdrMode::Sdr {
        return Err(core_platform::not_supported(
            "destack.display.monitor.setHdrMode",
        ));
    }

    Ok(())
}

/// Read display gamma ramp.
pub(crate) unsafe fn monitor_gamma_ramp(
    binding: &BindingCallContext,
    out: *mut DisplayGammaRamp,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and display handle
    core_platform::ensure_out(out, "out")?;
    let display_id =
        display_resource::resolve_display_id(binding, handle, "destack.display.monitor.gammaRamp")?;
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.monitor.gammaRamp")?;
    let crtc = resolve_randr_crtc_for_display(
        connection_state.as_ref(),
        &display_id,
        "destack.display.monitor.gammaRamp",
    )?
    .ok_or_else(|| core_platform::not_supported("destack.display.monitor.gammaRamp"))?;

    // read and encode the current gamma channels
    let (red, green, blue) = read_randr_gamma_ramp(
        connection_state.as_ref(),
        crtc,
        "destack.display.monitor.gammaRamp",
    )?;
    unsafe {
        *out = DisplayGammaRamp {
            red: binding.store_slice(red),
            green: binding.store_slice(green),
            blue: binding.store_slice(blue),
        };
    }

    Ok(())
}

/// Set display gamma ramp.
pub(crate) unsafe fn monitor_set_gamma_ramp(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRamp,
) -> RuntimeResult<()> {
    // validate display handle and decode channel payloads
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.setGammaRamp",
    )?;
    let red = unsafe { ramp.red.as_slice()? };
    let green = unsafe { ramp.green.as_slice()? };
    let blue = unsafe { ramp.blue.as_slice()? };
    // evaluate this condition
    if red.is_empty() {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels must not be empty",
        ));
    }
    // evaluate this condition
    if red.len() != green.len() || red.len() != blue.len() {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels must have equal lengths",
        ));
    }

    // resolve one active CRTC and validate gamma table size
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.monitor.setGammaRamp")?;
    let crtc = resolve_randr_crtc_for_display(
        connection_state.as_ref(),
        &display_id,
        "destack.display.monitor.setGammaRamp",
    )?
    .ok_or_else(|| core_platform::not_supported("destack.display.monitor.setGammaRamp"))?;
    let expected = read_randr_gamma_size(
        connection_state.as_ref(),
        crtc,
        "destack.display.monitor.setGammaRamp",
    )?;
    // evaluate this condition
    if red.len() != expected {
        return Err(core_platform::invalid_argument(
            "ramp",
            format!("gamma ramp channels must each contain exactly {expected} entries",),
        ));
    }

    // write the new gamma ramp through randr
    write_randr_gamma_ramp(
        connection_state.as_ref(),
        crtc,
        red,
        green,
        blue,
        "destack.display.monitor.setGammaRamp",
    )
}
