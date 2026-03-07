use x11rb::connection::Connection;
use x11rb::protocol::randr::{
    ConnectionExt as RandrConnectionExt, GetOutputInfoReply, Mode, SetConfig,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::DisplayMode;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{RandrOutputState, enumerate_randr_output_states, output_from_display_id};
use super::snapshot::monitor_snapshot_by_display_id;
use crate::platform::display::unix::x11::{core, event, resource as display_resource};

/// Resolve one current mode payload from one mode catalog and one active mode id.
pub(crate) fn current_mode_from_catalog(
    modes_by_id: &[(Mode, DisplayMode)],
    active_mode_id: Mode,
) -> Option<DisplayMode> {
    if let Some((_, mode)) = modes_by_id
        .iter()
        .find(|(mode_id, _)| *mode_id == active_mode_id)
    {
        return Some(*mode);
    }

    modes_by_id.first().map(|(_, mode)| *mode)
}

/// Resolve one desktop-preferred mode payload from one output mode order.
pub(crate) fn desktop_mode_from_catalog(
    output_info: &GetOutputInfoReply,
    modes_by_id: &[(Mode, DisplayMode)],
    current_mode: DisplayMode,
) -> DisplayMode {
    // prefer one mode in the output preferred-mode prefix
    let preferred_count = output_info.num_preferred as usize;
    for mode_id in output_info.modes.iter().copied().take(preferred_count) {
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
pub(crate) fn unique_modes_from_catalog(modes_by_id: &[(Mode, DisplayMode)]) -> Vec<DisplayMode> {
    let mut modes = Vec::with_capacity(modes_by_id.len());
    for (_, mode) in modes_by_id {
        if modes.contains(mode) {
            continue;
        }
        modes.push(*mode);
    }

    modes
}

/// Resolve one x11 status-name string for one randr `SetConfig` value.
pub(crate) fn randr_set_config_name(status: SetConfig) -> &'static str {
    if status == SetConfig::SUCCESS {
        return "success";
    }
    if status == SetConfig::INVALID_CONFIG_TIME {
        return "invalidConfigTime";
    }
    if status == SetConfig::INVALID_TIME {
        return "invalidTime";
    }
    if status == SetConfig::FAILED {
        return "failed";
    }

    "unknown"
}

/// Resolve one compatible mode id for one requested mode payload.
pub(crate) fn select_mode_id_for_request(
    modes_by_id: &[(Mode, DisplayMode)],
    requested: DisplayMode,
) -> Option<(Mode, DisplayMode)> {
    // require at least one mode with matching width and height
    let mut selected: Option<(Mode, DisplayMode)> = None;
    let mut selected_refresh_delta = u32::MAX;
    for (mode_id, mode) in modes_by_id.iter().copied() {
        if mode.width != requested.width || mode.height != requested.height {
            continue;
        }

        let refresh_delta = if requested.refresh_milli_hz == 0 {
            0
        } else {
            mode.refresh_milli_hz.abs_diff(requested.refresh_milli_hz)
        };
        if selected.is_none() || refresh_delta < selected_refresh_delta {
            selected = Some((mode_id, mode));
            selected_refresh_delta = refresh_delta;
        }
    }

    selected
}

/// Resolve one active output state snapshot from one display id.
pub(crate) fn resolve_output_state_by_display_id(
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
    else {
        return Err(core_platform::not_supported(operation));
    };
    let Some((selected_mode_id, selected_mode)) =
        select_mode_id_for_request(&output_state.modes_by_id, mode)
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
    if set_mode_reply.status == SetConfig::INVALID_CONFIG_TIME
        || set_mode_reply.status == SetConfig::INVALID_TIME
    {
        let Some(retry_output_state) =
            resolve_output_state_by_display_id(connection_state.as_ref(), display_id, operation)?
        else {
            return Err(core_platform::not_supported(operation));
        };
        let Some((retry_mode_id, _)) =
            select_mode_id_for_request(&retry_output_state.modes_by_id, mode)
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
    if set_mode_reply.status != SetConfig::SUCCESS {
        return Err(core::io_error(
            operation,
            format!(
                "randr_set_crtc_config failed with status {}",
                randr_set_config_name(set_mode_reply.status)
            ),
        ));
    }

    // flush the connection before reading back the applied mode
    connection_state
        .connection
        .flush()
        .map_err(|error| core::io_error(operation, format!("flush failed: {error}")))?;

    // read one post-apply mode snapshot from host state
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

    // publish mode and descriptor deltas
    let runtime_state = core::runtime_state(binding);
    event::publish_monitor_mode_changed(
        &runtime_state,
        &display_id,
        Some(previous_mode),
        current_mode,
    );
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
