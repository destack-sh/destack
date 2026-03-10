use crate::diagnostic::RuntimeResult;
use crate::platform::display::DisplayDescriptor;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{enumerate_fallback_monitor_snapshots, enumerate_randr_output_states};
use super::mode::unique_modes_from_catalog;
use crate::platform::display::unix::x11::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use crate::platform::display::unix::x11::{core, resource as display_resource};

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
    if !output_states.is_empty() {
        let mut snapshots = Vec::with_capacity(output_states.len());

        // convert randr output states into monitor snapshots
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

    // fall back to one synthetic screen level monitor when randr data is unavailable
    enumerate_fallback_monitor_snapshots(connection_state.as_ref())
}

/// Enumerate one normalized monitor snapshot list for one runtime state.
pub(crate) fn enumerate_monitor_snapshots_for_runtime(
    runtime_state: &std::sync::Arc<core::X11RuntimeState>,
    operation: &'static str,
) -> RuntimeResult<Vec<MonitorSnapshot>> {
    // load one connection snapshot for monitor enumeration
    let connection_state = core::connection_state(runtime_state, operation)?;

    // prefer one real randr output topology when available
    let output_states = enumerate_randr_output_states(connection_state.as_ref(), operation)?;
    if !output_states.is_empty() {
        let mut snapshots = Vec::with_capacity(output_states.len());

        // convert randr output states into monitor snapshots
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

    // fall back to one synthetic screen level monitor when randr data is unavailable
    enumerate_fallback_monitor_snapshots(connection_state.as_ref())
}

/// Convert one owned descriptor into one ABI payload.
pub(crate) fn descriptor_from_value(
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

/// Resolve one monitor snapshot associated with one opened monitor handle.
pub(crate) fn monitor_snapshot_for_handle(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<MonitorSnapshot> {
    let display_id = display_resource::resolve_display_id(binding, handle, operation)?;

    monitor_snapshot_by_display_id(binding, &display_id)?.ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("display id '{display_id}' is no longer available"),
        )
    })
}
