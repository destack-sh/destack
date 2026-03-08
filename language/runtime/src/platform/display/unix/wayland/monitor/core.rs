use crate::platform::display::{DisplayBackend, DisplayMode, DisplaySupportStatus};

use crate::platform::display::unix::wayland::core;
use crate::platform::display::unix::wayland::model::{DisplayDescriptorSnapshot, MonitorSnapshot};

/// Return one stable display id for one wl_registry output global.
pub(crate) fn display_id_for_output(global_name: u32) -> String {
    format!("wayland-output-{global_name}")
}

/// Return one built-in panel support value from one output label.
pub(crate) fn builtin_panel_support(label: &str) -> DisplaySupportStatus {
    let upper = label.to_uppercase();
    if upper.contains("EDP") || upper.contains("LVDS") || upper.contains("DSI") {
        return DisplaySupportStatus::Supported;
    }

    DisplaySupportStatus::Unknown
}

/// Convert one output snapshot into one normalized monitor snapshot.
pub(crate) fn monitor_snapshot_from_output(
    output: &core::WaylandOutputSnapshot,
    variable_refresh_support: DisplaySupportStatus,
) -> Option<MonitorSnapshot> {
    if output.modes.is_empty() {
        return None;
    }

    let fallback_mode = output.modes[0];
    let current_mode = output.current_mode.unwrap_or(fallback_mode);
    let desktop_mode = output.desktop_mode.unwrap_or(current_mode);
    let descriptor_name = output
        .logical_name
        .as_deref()
        .or(output.description.as_deref())
        .or(output.make.as_deref())
        .unwrap_or("wayland-output")
        .to_string();
    let builtin_panel = builtin_panel_support(&descriptor_name);
    let descriptor = DisplayDescriptorSnapshot {
        backend: DisplayBackend::Wayland,
        id: display_id_for_output(output.global_name),
        name: descriptor_name,
        primary: false,
        x: output.x,
        y: output.y,
        width_px: current_mode.width,
        height_px: current_mode.height,
        work_area_x: output.x,
        work_area_y: output.y,
        work_area_width_px: current_mode.width,
        work_area_height_px: current_mode.height,
        width_mm: output.width_mm,
        height_mm: output.height_mm,
        scale_factor_milli: output.scale_factor.saturating_mul(1000),
        orientation: output.orientation,
        builtin_panel,
        variable_refresh_support,
        hdr_support: DisplaySupportStatus::Unknown,
    };

    Some(MonitorSnapshot {
        descriptor,
        current_mode,
        desktop_mode,
        modes: output.modes.clone(),
    })
}

/// Resolve variable-refresh support for one wl_output snapshot.
pub(crate) fn variable_refresh_support_for_output(
    dispatch_state: &core::WaylandConnectionDispatchState,
    output: &core::WaylandOutputSnapshot,
) -> DisplaySupportStatus {
    // resolve the output by compositor logical name first
    let Some(logical_name) = output.logical_name.as_deref() else {
        return DisplaySupportStatus::Unknown;
    };
    let head = dispatch_state
        .output
        .wlr_output_heads_by_id
        .values()
        .find(|value| value.name.as_deref() == Some(logical_name));
    let Some(head) = head else {
        return DisplaySupportStatus::Unknown;
    };

    // any advertised adaptive-sync state implies VRR support
    if head.adaptive_sync.is_some() {
        return DisplaySupportStatus::Supported;
    }

    DisplaySupportStatus::Unknown
}

/// Resolve one wlr-output head id for one runtime display id.
pub(crate) fn output_head_id_for_display_id(
    dispatch_state: &core::WaylandConnectionDispatchState,
    display_id: &str,
) -> Option<wayland_client::backend::ObjectId> {
    // resolve the wl_output global name encoded in this display id
    let output_global_name = core::output_global_name_from_display_id(display_id)?;
    let output_snapshot = dispatch_state
        .output
        .output_snapshots_by_global
        .get(&output_global_name)?;
    let output_name = output_snapshot.logical_name.as_deref()?;

    // match the corresponding wlr output-head by logical output name
    dispatch_state
        .output
        .wlr_output_heads_by_id
        .iter()
        .find(|(_, value)| value.name.as_deref() == Some(output_name))
        .map(|(key, _)| key.clone())
}

/// Resolve one wlr mode object id for one display mode request.
pub(crate) fn output_mode_id_for_display_mode(
    dispatch_state: &core::WaylandConnectionDispatchState,
    display_id: &str,
    mode: DisplayMode,
) -> Option<wayland_client::backend::ObjectId> {
    // resolve the target display to one tracked output head
    let output_head_id = output_head_id_for_display_id(dispatch_state, display_id)?;
    let output_head = dispatch_state
        .output
        .wlr_output_heads_by_id
        .get(&output_head_id)?;

    // find one exact mode match from head-owned mode object snapshots
    let mut first_match = None;
    for mode_id in &output_head.mode_ids {
        let Some(mode_state) = dispatch_state.output.wlr_output_modes_by_id.get(mode_id) else {
            continue;
        };
        if mode_state.width != Some(mode.width) || mode_state.height != Some(mode.height) {
            continue;
        }

        if let Some(refresh_milli_hz) = mode_state.refresh_milli_hz
            && refresh_milli_hz != mode.refresh_milli_hz
        {
            continue;
        }

        if mode_state.is_preferred {
            return Some(mode_id.clone());
        }

        if first_match.is_none() {
            first_match = Some(mode_id.clone());
        }
    }

    first_match
}
