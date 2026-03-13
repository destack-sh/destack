use crate::diagnostic::RuntimeResult;
use crate::platform::display::DisplayDescriptor;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{monitor_snapshot_from_output, variable_refresh_support_for_output};
use crate::platform::display::unix::wayland::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use crate::platform::display::unix::wayland::{core, resource as display_resource};

/// Enumerate monitor snapshots for one operation.
pub(crate) fn enumerate_monitor_snapshots_for_operation(
    context: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<Vec<MonitorSnapshot>> {
    // ensure one initialized connection and pump pending global events
    let _connection = core::connection_state(context, operation)?;
    core::dispatch_pending(context, operation)?;

    // convert runtime owned output snapshots into monitor snapshots
    let mut snapshots = core::with_connection_dispatch(
        context,
        operation,
        |_connection, _event_queue, dispatch_state| {
            let mut snapshots =
                Vec::with_capacity(dispatch_state.output.output_snapshots_by_global.len());
            for output in dispatch_state.output.output_snapshots_by_global.values() {
                let variable_refresh_support =
                    variable_refresh_support_for_output(dispatch_state, output);
                let Some(snapshot) = monitor_snapshot_from_output(output, variable_refresh_support)
                else {
                    continue;
                };
                snapshots.push(snapshot);
            }

            Ok(snapshots)
        },
    )?;

    // keep the returned list deterministic even though wayland does not expose a primary output
    snapshots.sort_by(|left, right| {
        left.descriptor
            .y
            .cmp(&right.descriptor.y)
            .then(left.descriptor.x.cmp(&right.descriptor.x))
            .then(left.descriptor.id.cmp(&right.descriptor.id))
    });

    Ok(snapshots)
}

/// Enumerate monitor snapshots from one wayland connection.
pub(crate) fn enumerate_monitor_snapshots(
    context: &BindingCallContext,
) -> RuntimeResult<Vec<MonitorSnapshot>> {
    enumerate_monitor_snapshots_for_operation(context, "destack.display.monitor.list")
}

/// Enumerate monitor snapshots for one runtime-owned Wayland state.
pub(crate) fn enumerate_monitor_snapshots_for_runtime(
    runtime_state: &std::sync::Arc<core::WaylandRuntimeState>,
    operation: &'static str,
) -> RuntimeResult<Vec<MonitorSnapshot>> {
    // ensure one initialized connection and pump pending global events
    let _connection = core::connection_state_for_runtime(runtime_state, operation)?;
    core::dispatch_pending_for_runtime(runtime_state, operation)?;

    // convert runtime owned output snapshots into monitor snapshots
    let mut snapshots = core::with_connection_dispatch_for_runtime(
        runtime_state,
        operation,
        |_connection, _event_queue, dispatch_state| {
            let mut snapshots =
                Vec::with_capacity(dispatch_state.output.output_snapshots_by_global.len());
            for output in dispatch_state.output.output_snapshots_by_global.values() {
                let variable_refresh_support =
                    variable_refresh_support_for_output(dispatch_state, output);
                let Some(snapshot) = monitor_snapshot_from_output(output, variable_refresh_support)
                else {
                    continue;
                };
                snapshots.push(snapshot);
            }

            Ok(snapshots)
        },
    )?;

    // keep the returned list deterministic even though wayland does not expose a primary output
    snapshots.sort_by(|left, right| {
        left.descriptor
            .y
            .cmp(&right.descriptor.y)
            .then(left.descriptor.x.cmp(&right.descriptor.x))
            .then(left.descriptor.id.cmp(&right.descriptor.id))
    });

    Ok(snapshots)
}

/// Resolve one monitor snapshot by display id.
pub(crate) fn snapshot_by_display_id(
    context: &BindingCallContext,
    display_id: &str,
    operation: &'static str,
) -> RuntimeResult<MonitorSnapshot> {
    // find one matching snapshot in the current topology
    let snapshots = enumerate_monitor_snapshots_for_operation(context, operation)?;
    let snapshot = snapshots
        .into_iter()
        .find(|snapshot| snapshot.descriptor.id == display_id)
        .ok_or_else(|| {
            core_platform::io_not_found(
                operation,
                format!("display id `{display_id}` was not found"),
            )
        })?;

    Ok(snapshot)
}

/// Resolve one monitor snapshot by opened display handle.
pub(crate) fn snapshot_by_display_handle(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<MonitorSnapshot> {
    // resolve one stable display id from the resource handle
    let display_id = display_resource::resolve_display_id(context, handle, operation)?;

    // resolve one monitor snapshot by display id
    snapshot_by_display_id(context, display_id.as_str(), operation)
}

/// Convert one owned descriptor into one ABI payload.
pub(crate) fn descriptor_from_value(
    context: &BindingCallContext,
    value: &DisplayDescriptorSnapshot,
) -> DisplayDescriptor {
    DisplayDescriptor {
        backend: value.backend,
        id: context.store_string(&value.id),
        name: context.store_string(&value.name),
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
