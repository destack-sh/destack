use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsFd, FromRawFd};
use std::sync::{Arc, Mutex};

use wayland_client::Proxy;
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_head_v1, zwlr_output_mode_v1,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayBackend, DisplayColorSpace, DisplayColorState, DisplayDescriptor, DisplayGammaRamp,
    DisplayHdrMode, DisplayMode, DisplayMonitorListRequest, DisplayMonitorOpenOptions,
    DisplaySupportStatus,
};
use crate::platform::{PlatformErrorCode, core as core_platform, resource};
use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef};

use super::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use super::{core, event, resource as display_resource};

/// Return one stable display id for one wl_registry output global.
fn display_id_for_output(global_name: u32) -> String {
    format!("wayland-output-{global_name}")
}

/// Return one built-in panel support value from one output label.
fn builtin_panel_support(label: &str) -> DisplaySupportStatus {
    let upper = label.to_uppercase();
    if upper.contains("EDP") || upper.contains("LVDS") || upper.contains("DSI") {
        return DisplaySupportStatus::Supported;
    }

    DisplaySupportStatus::Unknown
}

/// Convert one output snapshot into one normalized monitor snapshot.
fn monitor_snapshot_from_output(
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
fn variable_refresh_support_for_output(
    dispatch_state: &core::WaylandConnectionDispatchState,
    output: &core::WaylandOutputSnapshot,
) -> DisplaySupportStatus {
    // resolve this output by compositor logical name first
    let Some(logical_name) = output.logical_name.as_deref() else {
        return DisplaySupportStatus::Unknown;
    };
    let head = dispatch_state
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

/// Enumerate monitor snapshots from one wayland connection.
fn enumerate_monitor_snapshots_for_operation(
    context: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<Vec<MonitorSnapshot>> {
    // ensure one initialized connection and pump pending global events
    let _connection = core::connection_state(context, operation)?;
    core::dispatch_pending(context, operation)?;

    // convert runtime-owned output snapshots into monitor snapshots
    let mut snapshots = core::with_connection_dispatch(
        context,
        operation,
        |_connection, _event_queue, dispatch_state| {
            let mut snapshots = Vec::with_capacity(dispatch_state.output_snapshots_by_global.len());
            for output in dispatch_state.output_snapshots_by_global.values() {
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

    // mark one primary display for deterministic behavior
    if let Some(first) = snapshots.first_mut() {
        first.descriptor.primary = true;
    }

    Ok(snapshots)
}

/// Enumerate monitor snapshots from one wayland connection.
pub(crate) fn enumerate_monitor_snapshots(
    context: &BindingCallContext,
) -> RuntimeResult<Vec<MonitorSnapshot>> {
    enumerate_monitor_snapshots_for_operation(context, "destack.display.monitor.list")
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
pub(crate) fn descriptor_from_owned(
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

/// Resolve one wlr-output head id for one runtime display id.
fn output_head_id_for_display_id(
    dispatch_state: &core::WaylandConnectionDispatchState,
    display_id: &str,
) -> Option<wayland_client::backend::ObjectId> {
    // resolve the wl_output global name encoded in this display id
    let output_global_name = core::output_global_name_from_display_id(display_id)?;
    let output_snapshot = dispatch_state
        .output_snapshots_by_global
        .get(&output_global_name)?;
    let output_name = output_snapshot.logical_name.as_deref()?;

    // match the corresponding wlr output-head by logical output name
    dispatch_state
        .wlr_output_heads_by_id
        .iter()
        .find(|(_, value)| value.name.as_deref() == Some(output_name))
        .map(|(key, _)| key.clone())
}

/// Resolve one wlr mode object id for one display mode request.
fn output_mode_id_for_display_mode(
    dispatch_state: &core::WaylandConnectionDispatchState,
    display_id: &str,
    mode: DisplayMode,
) -> Option<wayland_client::backend::ObjectId> {
    // resolve this display to one tracked output head
    let output_head_id = output_head_id_for_display_id(dispatch_state, display_id)?;
    let output_head = dispatch_state.wlr_output_heads_by_id.get(&output_head_id)?;

    // find one exact mode match from head-owned mode object snapshots
    let mut first_match = None;
    for mode_id in &output_head.mode_ids {
        let Some(mode_state) = dispatch_state.wlr_output_modes_by_id.get(mode_id) else {
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

/// Apply one display mode through wlr-output-management.
fn apply_monitor_mode_by_display_id(
    context: &BindingCallContext,
    display_id: &str,
    mode: DisplayMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    core::with_connection_dispatch(
        context,
        operation,
        |connection, event_queue, dispatch_state| {
            // require one negotiated wlr-output-manager global and serial lane
            let manager = dispatch_state
                .wlr_output_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported(operation))?;
            let serial = dispatch_state
                .wlr_output_manager_serial
                .ok_or_else(|| core_platform::not_supported(operation))?;
            let output_head_id = output_head_id_for_display_id(dispatch_state, display_id)
                .ok_or_else(|| core_platform::not_supported(operation))?;

            // resolve the output-head proxy for this mode request
            let output_head =
                zwlr_output_head_v1::ZwlrOutputHeadV1::from_id(connection, output_head_id)
                    .map_err(|error| {
                        core::io_error(operation, format!("invalid output head object id: {error}"))
                    })?;

            // allocate one configuration object and configure this output head
            let configuration_state = Arc::new(core::WaylandOutputConfigurationState::default());
            let queue_handle = event_queue.handle();
            let configuration = manager.create_configuration(
                serial,
                &queue_handle,
                Arc::clone(&configuration_state),
            );
            let configuration_head = configuration.enable_head(&output_head, &queue_handle, ());

            // use exact compositor mode objects when available for robust mode selection
            let mode_id = output_mode_id_for_display_mode(dispatch_state, display_id, mode);
            if let Some(mode_id) = mode_id {
                let mode = zwlr_output_mode_v1::ZwlrOutputModeV1::from_id(connection, mode_id)
                    .map_err(|error| {
                        core::io_error(operation, format!("invalid output mode object id: {error}"))
                    })?;
                configuration_head.set_mode(&mode);
            }
            // otherwise fall back to custom mode request lanes
            else {
                let width = i32::try_from(mode.width).map_err(|_| {
                    core_platform::invalid_argument("mode", "mode width must fit in int32")
                })?;
                let height = i32::try_from(mode.height).map_err(|_| {
                    core_platform::invalid_argument("mode", "mode height must fit in int32")
                })?;
                let refresh = i32::try_from(mode.refresh_milli_hz).map_err(|_| {
                    core_platform::invalid_argument("mode", "mode refresh must fit in int32")
                })?;
                configuration_head.set_custom_mode(width, height, refresh);
            }

            // submit one output configuration apply request
            configuration.apply();
            core::flush_queue(event_queue, operation)?;

            // wait for one terminal configuration outcome from compositor events
            for _ in 0..4 {
                let outcome = configuration_state
                    .outcome
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .as_ref()
                    .cloned();
                if outcome.is_some() {
                    break;
                }

                event_queue.roundtrip(dispatch_state).map_err(|error| {
                    core::io_error(
                        operation,
                        format!("wlr output configuration roundtrip failed: {error}"),
                    )
                })?;
            }

            // return one mapped runtime error for non-success outcomes
            let outcome = configuration_state
                .outcome
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .take();

            match outcome {
                Some(core::WaylandOutputConfigurationOutcome::Succeeded) => Ok(()),
                Some(core::WaylandOutputConfigurationOutcome::Failed) => {
                    Err(core_platform::io_operation_error(
                        operation,
                        None,
                        "compositor rejected output mode configuration",
                    ))
                }
                Some(core::WaylandOutputConfigurationOutcome::Cancelled) => {
                    Err(core_platform::io_operation_error(
                        operation,
                        Some(PlatformErrorCode::IoInterrupted),
                        "compositor cancelled output mode configuration",
                    ))
                }
                None => Err(core_platform::io_would_block(
                    operation,
                    "timed out waiting for output configuration result",
                )),
            }
        },
    )
}

/// Close one display endpoint.
pub(crate) unsafe fn monitor_close(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // resolve display id before handle removal so cache cleanup stays deterministic
    let display_id =
        display_resource::resolve_display_id(context, handle, "destack.display.monitor.close");

    // remove the resource entry from the runtime table
    let removed = context
        .runtime()
        .resources
        .remove(handle.0, Some(context.engine()))
        .is_some();
    if !removed {
        return Err(core::display_not_found(
            "destack.display.monitor.close",
            handle,
        ));
    }

    // drop cached gamma state for this display endpoint when available
    if let Ok(display_id) = display_id {
        let runtime_state = core::runtime_state(context);
        let mut cache = runtime_state
            .gamma_ramps_by_display_id
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        cache.remove(display_id.as_str());
    }

    Ok(())
}

/// Resolve one requested mode to the closest supported mode.
pub(crate) unsafe fn monitor_closest_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
    requested: DisplayMode,
) -> RuntimeResult<()> {
    // validate output pointer and resolve one monitor snapshot
    core_platform::ensure_out(out, "out")?;
    let snapshot =
        snapshot_by_display_handle(context, handle, "destack.display.monitor.closestMode")?;

    // choose one closest mode by area and refresh-rate deltas
    let mut selected = snapshot.current_mode;
    let mut selected_score = u64::MAX;
    for mode in &snapshot.modes {
        let width_delta = mode.width.abs_diff(requested.width) as u64;
        let height_delta = mode.height.abs_diff(requested.height) as u64;
        let refresh_delta = mode.refresh_milli_hz.abs_diff(requested.refresh_milli_hz) as u64;
        let score = width_delta
            .saturating_mul(1_000_000)
            .saturating_add(height_delta.saturating_mul(1_000_000))
            .saturating_add(refresh_delta);
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
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve one monitor snapshot
    core_platform::ensure_out(out, "out")?;
    let snapshot =
        snapshot_by_display_handle(context, handle, "destack.display.monitor.currentMode")?;

    // write current mode payload
    unsafe {
        *out = snapshot.current_mode;
    }

    Ok(())
}

/// Read descriptor metadata for one opened display.
pub(crate) unsafe fn monitor_descriptor(
    context: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve one monitor snapshot
    core_platform::ensure_out(out, "out")?;
    let snapshot =
        snapshot_by_display_handle(context, handle, "destack.display.monitor.descriptor")?;

    // write descriptor payload
    unsafe {
        *out = descriptor_from_owned(context, &snapshot.descriptor);
    }

    Ok(())
}

/// Read the desktop-preferred mode for one opened display.
pub(crate) unsafe fn monitor_desktop_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve one monitor snapshot
    core_platform::ensure_out(out, "out")?;
    let snapshot =
        snapshot_by_display_handle(context, handle, "destack.display.monitor.desktopMode")?;

    // write desktop mode payload
    unsafe {
        *out = snapshot.desktop_mode;
    }

    Ok(())
}

/// List available displays.
pub(crate) unsafe fn monitor_list(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    _request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // enumerate descriptors and store output slice
    let snapshots =
        enumerate_monitor_snapshots_for_operation(context, "destack.display.monitor.list")?;
    let mut values = Vec::with_capacity(snapshots.len());
    for snapshot in snapshots {
        values.push(descriptor_from_owned(context, &snapshot.descriptor));
    }
    unsafe {
        *out = context.store_slice(values);
    }

    Ok(())
}

/// Read available display modes.
pub(crate) unsafe fn monitor_modes(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve one monitor snapshot
    core_platform::ensure_out(out, "out")?;
    let snapshot = snapshot_by_display_handle(context, handle, "destack.display.monitor.modes")?;

    // store mode slice
    unsafe {
        *out = context.store_slice(snapshot.modes.clone());
    }

    Ok(())
}

/// Open one display endpoint.
pub(crate) unsafe fn monitor_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
    _options: DisplayMonitorOpenOptions,
) -> RuntimeResult<()> {
    // validate out pointer and decode one requested display id
    core_platform::ensure_out(out, "out")?;
    let id = unsafe { id.as_str()? };

    // validate that the requested id exists
    let snapshot = snapshot_by_display_id(context, id, "destack.display.monitor.open")?;

    // insert one display handle resource and write output
    let handle = display_resource::open_display_handle(context, snapshot.descriptor.id.clone());
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Read the current primary display handle.
pub(crate) unsafe fn monitor_primary(
    context: &BindingCallContext,
    out: *mut Option<resource::DisplayHandle>,
    _request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // resolve one primary snapshot and create one resource handle
    let snapshots =
        enumerate_monitor_snapshots_for_operation(context, "destack.display.monitor.primary")?;
    let primary = snapshots
        .iter()
        .find(|snapshot| snapshot.descriptor.primary);
    let primary_handle = primary.map(|snapshot| {
        display_resource::open_display_handle(context, snapshot.descriptor.id.clone())
    });

    unsafe {
        *out = primary_handle;
    }

    Ok(())
}

/// Apply one display mode.
pub(crate) unsafe fn monitor_set_mode(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    // resolve this display snapshot and validate the requested mode
    let previous_snapshot =
        snapshot_by_display_handle(context, handle, "destack.display.monitor.setMode")?;
    if !previous_snapshot.modes.contains(&mode) {
        return Err(core_platform::invalid_argument(
            "mode",
            "display mode is not supported by the target display",
        ));
    }

    // apply this mode through compositor output-management protocol lanes
    let display_id = previous_snapshot.descriptor.id.clone();
    apply_monitor_mode_by_display_id(
        context,
        display_id.as_str(),
        mode,
        "destack.display.monitor.setMode",
    )?;
    core::dispatch_pending(context, "destack.display.monitor.setMode")?;

    // publish mode and descriptor deltas from updated monitor snapshots
    let current_snapshot = snapshot_by_display_id(
        context,
        display_id.as_str(),
        "destack.display.monitor.setMode",
    )?;
    let runtime_state = core::runtime_state(context);

    event::publish_monitor_mode_changed(
        &runtime_state,
        display_id.as_str(),
        Some(previous_snapshot.current_mode),
        current_snapshot.current_mode,
    );

    if current_snapshot.descriptor != previous_snapshot.descriptor {
        event::publish_monitor_descriptor_changed(
            &runtime_state,
            Some(previous_snapshot.descriptor),
            current_snapshot.descriptor,
        );
    }

    event::refresh_monitor_topology_cache(context)?;

    // clear cached gamma payloads because compositor mode switches can resize ramps
    let mut gamma_cache = runtime_state
        .gamma_ramps_by_display_id
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    gamma_cache.remove(display_id.as_str());

    Ok(())
}

/// Wayland color-management primaries value for sRGB.
const COLOR_PRIMARIES_SRGB: u32 = 1;
/// Wayland color-management primaries value for BT.2020.
const COLOR_PRIMARIES_BT2020: u32 = 6;
/// Wayland color-management primaries value for Display-P3.
const COLOR_PRIMARIES_DISPLAY_P3: u32 = 9;
/// Wayland color-management transfer-function value for linear.
const COLOR_TRANSFER_EXT_LINEAR: u32 = 5;
/// Wayland color-management transfer-function value for PQ.
const COLOR_TRANSFER_ST2084_PQ: u32 = 11;
/// Wayland color-management transfer-function value for HLG.
const COLOR_TRANSFER_HLG: u32 = 13;
/// Wayland image-description failure cause value for low interface version.
const COLOR_FAILURE_CAUSE_LOW_VERSION: u32 = 0;
/// Wayland image-description failure cause value for unsupported description.
const COLOR_FAILURE_CAUSE_UNSUPPORTED: u32 = 1;
/// Wayland image-description failure cause value for compositor internal failure.
const COLOR_FAILURE_CAUSE_OPERATING_SYSTEM: u32 = 2;
/// Wayland image-description failure cause value for removed output.
const COLOR_FAILURE_CAUSE_NO_OUTPUT: u32 = 3;

/// Return one wl_output global name for one stable display id.
fn output_global_name_for_display_id(display_id: &str) -> RuntimeResult<u32> {
    core::output_global_name_from_display_id(display_id).ok_or_else(|| {
        core_platform::invalid_argument(
            "display",
            format!("display id `{display_id}` is not one wayland output identifier"),
        )
    })
}

/// Return one color-description query snapshot for one display id.
fn color_query_for_display_id(
    context: &BindingCallContext,
    display_id: &str,
    operation: &'static str,
) -> RuntimeResult<core::WaylandColorDescriptionQueryState> {
    core::with_connection_dispatch(
        context,
        operation,
        |_connection, event_queue, dispatch_state| {
            // resolve one color-management output object for this display
            let output_global_name = output_global_name_for_display_id(display_id)?;
            let output = dispatch_state
                .color_outputs_by_global
                .get(&output_global_name)
                .cloned()
                .ok_or_else(|| core_platform::not_supported(operation))?;

            // request one image-description snapshot from the compositor
            let query_state = Arc::new(Mutex::new(
                core::WaylandColorDescriptionQueryState::default(),
            ));
            let image_description =
                output.get_image_description(&event_queue.handle(), Arc::clone(&query_state));
            core::flush_queue(event_queue, operation)?;

            // wait until image-description is ready or failed
            for _ in 0..6 {
                let snapshot = query_state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .clone();
                if snapshot.ready || snapshot.failed_message.is_some() {
                    break;
                }

                event_queue.roundtrip(dispatch_state).map_err(|error| {
                    core::io_error(
                        operation,
                        format!("wayland image-description roundtrip failed: {error}"),
                    )
                })?;
            }

            let snapshot = query_state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
            if let Some(message) = snapshot.failed_message {
                let cause = snapshot.failed_cause.unwrap_or_default();
                if cause == COLOR_FAILURE_CAUSE_LOW_VERSION
                    || cause == COLOR_FAILURE_CAUSE_UNSUPPORTED
                    || cause == COLOR_FAILURE_CAUSE_NO_OUTPUT
                {
                    return Err(core_platform::not_supported(operation));
                }

                if cause == COLOR_FAILURE_CAUSE_OPERATING_SYSTEM {
                    return Err(core_platform::io_operation_error(
                        operation,
                        None,
                        format!("wayland output image description failed: {message}"),
                    ));
                }

                return Err(core::io_error(
                    operation,
                    format!("wayland output image description failed: {message}"),
                ));
            }
            if !snapshot.ready {
                return Err(core_platform::io_would_block(
                    operation,
                    "timed out waiting for wayland output image description",
                ));
            }

            // request one information payload for this ready image description
            image_description.get_information(&event_queue.handle(), Arc::clone(&query_state));
            core::flush_queue(event_queue, operation)?;

            // wait until information delivery is complete
            for _ in 0..6 {
                let snapshot = query_state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .clone();
                if snapshot.info_done || snapshot.failed_message.is_some() {
                    break;
                }

                event_queue.roundtrip(dispatch_state).map_err(|error| {
                    core::io_error(
                        operation,
                        format!("wayland image-description-info roundtrip failed: {error}"),
                    )
                })?;
            }

            // destroy the temporary image-description object before returning
            image_description.destroy();
            core::flush_queue(event_queue, operation)?;

            let snapshot = query_state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
            if let Some(message) = snapshot.failed_message {
                let cause = snapshot.failed_cause.unwrap_or_default();
                if cause == COLOR_FAILURE_CAUSE_LOW_VERSION
                    || cause == COLOR_FAILURE_CAUSE_UNSUPPORTED
                    || cause == COLOR_FAILURE_CAUSE_NO_OUTPUT
                {
                    return Err(core_platform::not_supported(operation));
                }

                if cause == COLOR_FAILURE_CAUSE_OPERATING_SYSTEM {
                    return Err(core_platform::io_operation_error(
                        operation,
                        None,
                        format!("wayland output image description failed: {message}"),
                    ));
                }

                return Err(core::io_error(
                    operation,
                    format!("wayland output image description failed: {message}"),
                ));
            }
            if !snapshot.info_done {
                return Err(core_platform::io_would_block(
                    operation,
                    "timed out waiting for wayland output image description details",
                ));
            }

            Ok(snapshot)
        },
    )
}

/// Return one backend color-space value from one color-description query.
fn color_space_from_query(query: &core::WaylandColorDescriptionQueryState) -> DisplayColorSpace {
    // classify explicit BT.2020 + PQ as HDR10 output signaling
    if query.primaries_named == Some(COLOR_PRIMARIES_BT2020)
        && query.transfer_function_named == Some(COLOR_TRANSFER_ST2084_PQ)
    {
        return DisplayColorSpace::Hdr10;
    }

    // classify Display-P3 primaries directly
    if query.primaries_named == Some(COLOR_PRIMARIES_DISPLAY_P3) {
        return DisplayColorSpace::DisplayP3;
    }

    // classify BT.2020 primaries without PQ signaling
    if query.primaries_named == Some(COLOR_PRIMARIES_BT2020) {
        return DisplayColorSpace::Bt2020;
    }

    // classify extended-linear sRGB as scRGB style transfer
    if query.primaries_named == Some(COLOR_PRIMARIES_SRGB)
        && query.transfer_function_named == Some(COLOR_TRANSFER_EXT_LINEAR)
    {
        return DisplayColorSpace::ScRgb;
    }

    // classify explicit sRGB primaries
    if query.primaries_named == Some(COLOR_PRIMARIES_SRGB) {
        return DisplayColorSpace::Srgb;
    }

    // default to unknown when compositor does not expose named primaries
    DisplayColorSpace::Unknown
}

/// Return one backend HDR-mode value from one color-description query.
fn hdr_mode_from_query(query: &core::WaylandColorDescriptionQueryState) -> DisplayHdrMode {
    let is_hdr_transfer = query.transfer_function_named == Some(COLOR_TRANSFER_ST2084_PQ)
        || query.transfer_function_named == Some(COLOR_TRANSFER_HLG);
    if is_hdr_transfer {
        return DisplayHdrMode::Hdr;
    }

    let max_luminance = query.maximum_luminance.unwrap_or_default();
    if max_luminance > 300 {
        return DisplayHdrMode::Hdr;
    }

    if query.primaries_named.is_some() || query.transfer_function_named.is_some() {
        return DisplayHdrMode::Sdr;
    }

    DisplayHdrMode::Unknown
}

/// Return one gamma-control query snapshot for one display id.
fn gamma_query_for_display_id(
    context: &BindingCallContext,
    display_id: &str,
    operation: &'static str,
) -> RuntimeResult<core::WaylandGammaControlQueryState> {
    core::with_connection_dispatch(
        context,
        operation,
        |_connection, event_queue, dispatch_state| {
            // resolve one wl_output endpoint and one gamma-control manager
            let output_global_name = output_global_name_for_display_id(display_id)?;
            let output = dispatch_state
                .outputs_by_global
                .get(&output_global_name)
                .cloned()
                .ok_or_else(|| core_platform::not_supported(operation))?;
            let manager = dispatch_state
                .gamma_control_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| core_platform::not_supported(operation))?;

            // create one temporary gamma-control object
            let query_state = Arc::new(Mutex::new(core::WaylandGammaControlQueryState::default()));
            let gamma_control =
                manager.get_gamma_control(&output, &event_queue.handle(), Arc::clone(&query_state));
            core::flush_queue(event_queue, operation)?;

            // wait until gamma size or failure arrives
            for _ in 0..6 {
                let snapshot = query_state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .clone();
                if snapshot.gamma_size.is_some() || snapshot.failed {
                    break;
                }

                event_queue.roundtrip(dispatch_state).map_err(|error| {
                    core::io_error(
                        operation,
                        format!("wayland gamma-control roundtrip failed: {error}"),
                    )
                })?;
            }

            // destroy the temporary gamma-control object before returning
            gamma_control.destroy();
            core::flush_queue(event_queue, operation)?;

            let snapshot = query_state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
            if snapshot.failed {
                return Err(core_platform::not_supported(operation));
            }
            if snapshot.gamma_size.is_none() {
                return Err(core_platform::io_would_block(
                    operation,
                    "timed out waiting for wayland gamma size",
                ));
            }

            Ok(snapshot)
        },
    )
}

/// Validate one gamma-ramp payload and decode copied channels.
fn decoded_gamma_ramp(ramp: DisplayGammaRamp) -> RuntimeResult<(Vec<u16>, Vec<u16>, Vec<u16>)> {
    // decode gamma channel arrays from the native payload
    let red = unsafe { ramp.red.as_slice()?.to_vec() };
    let green = unsafe { ramp.green.as_slice()?.to_vec() };
    let blue = unsafe { ramp.blue.as_slice()?.to_vec() };

    // reject empty channel payloads
    if red.is_empty() {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels must not be empty",
        ));
    }

    // reject channel size mismatches
    if red.len() != green.len() || red.len() != blue.len() {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels must have equal lengths",
        ));
    }

    // reject channels that cannot fit one wayland gamma-size value
    if red.len() > u32::MAX as usize {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels are too large for this backend",
        ));
    }

    Ok((red, green, blue))
}

/// Create one anonymous in-memory file descriptor for one gamma table payload.
fn create_gamma_memfd_file(operation: &'static str, length: usize) -> RuntimeResult<File> {
    // build one stable memfd label for diagnostics
    let name = CString::new("destack-wayland-gamma")
        .map_err(|error| core::io_error(operation, format!("invalid memfd name: {error}")))?;

    // reject lengths that cannot fit one off_t value
    if length > i64::MAX as usize {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma table payload is too large",
        ));
    }

    // allocate one anonymous file descriptor
    let file_descriptor = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if file_descriptor < 0 {
        let error = std::io::Error::last_os_error();
        return Err(core::io_error(
            operation,
            format!("memfd_create failed: {error}"),
        ));
    }

    // grow file to requested payload length
    let truncated = unsafe { libc::ftruncate(file_descriptor, length as libc::off_t) };
    if truncated != 0 {
        let error = std::io::Error::last_os_error();
        unsafe {
            libc::close(file_descriptor);
        }
        return Err(core::io_error(
            operation,
            format!("ftruncate failed: {error}"),
        ));
    }

    // transfer ownership to one rust file handle
    Ok(unsafe { File::from_raw_fd(file_descriptor) })
}

/// Encode one gamma-ramp payload into one fd-backed byte stream.
fn encoded_gamma_bytes(red: &[u16], green: &[u16], blue: &[u16]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(red.len().saturating_mul(6));

    for value in red.iter().chain(green.iter()).chain(blue.iter()) {
        encoded.extend_from_slice(&value.to_ne_bytes());
    }

    encoded
}

/// Read display color state.
pub(crate) unsafe fn monitor_color_state(
    context: &BindingCallContext,
    out: *mut DisplayColorState,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate output pointer and resolve stable display id
    core_platform::ensure_out(out, "out")?;
    let display_id = display_resource::resolve_display_id(
        context,
        handle,
        "destack.display.monitor.colorState",
    )?;

    // resolve one protocol-backed output color snapshot
    let query = color_query_for_display_id(
        context,
        display_id.as_str(),
        "destack.display.monitor.colorState",
    )?;
    let state = DisplayColorState {
        hdr_mode: hdr_mode_from_query(&query),
        color_space: color_space_from_query(&query),
        bits_per_channel: None,
    };

    unsafe {
        *out = state;
    }

    Ok(())
}

/// Read display HDR mode.
pub(crate) unsafe fn monitor_hdr_mode(
    context: &BindingCallContext,
    out: *mut DisplayHdrMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate output pointer and resolve stable display id
    core_platform::ensure_out(out, "out")?;
    let display_id =
        display_resource::resolve_display_id(context, handle, "destack.display.monitor.hdrMode")?;

    // resolve one protocol-backed output HDR snapshot
    let query = color_query_for_display_id(
        context,
        display_id.as_str(),
        "destack.display.monitor.hdrMode",
    )?;
    let mode = hdr_mode_from_query(&query);

    unsafe {
        *out = mode;
    }

    Ok(())
}

/// Set display HDR mode.
pub(crate) unsafe fn monitor_set_hdr_mode(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayHdrMode,
) -> RuntimeResult<()> {
    // validate display handle and mode payload
    let display_id = display_resource::resolve_display_id(
        context,
        handle,
        "destack.display.monitor.setHdrMode",
    )?;
    if mode == DisplayHdrMode::Unknown {
        return Err(core_platform::invalid_argument(
            "mode",
            "hdr mode must be one concrete mode",
        ));
    }

    // allow host-default policy as one no-op request
    if mode == DisplayHdrMode::System {
        return Ok(());
    }

    // accept idempotent requests when the compositor already matches this mode
    let query = color_query_for_display_id(
        context,
        display_id.as_str(),
        "destack.display.monitor.setHdrMode",
    )?;
    let current_mode = hdr_mode_from_query(&query);
    if current_mode == mode {
        return Ok(());
    }

    // wayland has no generic output policy write lane for hdr mode switching
    Err(core_platform::not_supported(
        "destack.display.monitor.setHdrMode",
    ))
}

/// Read display gamma ramp.
pub(crate) unsafe fn monitor_gamma_ramp(
    context: &BindingCallContext,
    out: *mut DisplayGammaRamp,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate output pointer and resolve stable display id
    core_platform::ensure_out(out, "out")?;
    let display_id =
        display_resource::resolve_display_id(context, handle, "destack.display.monitor.gammaRamp")?;

    // read one cached ramp snapshot when this runtime already set gamma values
    let runtime_state = core::runtime_state(context);
    let cached = runtime_state
        .gamma_ramps_by_display_id
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(display_id.as_str())
        .cloned();
    if cached.is_none() {
        // validate this display exposes gamma-control and report truthful readback support
        gamma_query_for_display_id(
            context,
            display_id.as_str(),
            "destack.display.monitor.gammaRamp",
        )?;
        return Err(core_platform::not_supported(
            "destack.display.monitor.gammaRamp",
        ));
    }

    // resolve one cached snapshot after explicit presence check
    let Some(snapshot) = cached else {
        return Err(core_platform::invalid_state(
            "missing cached gamma ramp snapshot",
        ));
    };

    unsafe {
        *out = DisplayGammaRamp {
            red: context.store_slice(snapshot.red),
            green: context.store_slice(snapshot.green),
            blue: context.store_slice(snapshot.blue),
        };
    }

    Ok(())
}

/// Set display gamma ramp.
pub(crate) unsafe fn monitor_set_gamma_ramp(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRamp,
) -> RuntimeResult<()> {
    // resolve stable display id and decode one copied ramp payload
    let display_id = display_resource::resolve_display_id(
        context,
        handle,
        "destack.display.monitor.setGammaRamp",
    )?;
    let (red, green, blue) = decoded_gamma_ramp(ramp)?;

    // resolve expected channel length from compositor gamma-size response
    let query = gamma_query_for_display_id(
        context,
        display_id.as_str(),
        "destack.display.monitor.setGammaRamp",
    )?;
    let expected = usize::try_from(query.gamma_size.unwrap_or_default())
        .map_err(|_| core_platform::invalid_state("wayland gamma size overflow"))?;
    if expected == 0 {
        return Err(core_platform::not_supported(
            "destack.display.monitor.setGammaRamp",
        ));
    }
    if red.len() != expected {
        return Err(core_platform::invalid_argument(
            "ramp",
            format!("gamma ramp channels must each contain exactly {expected} entries"),
        ));
    }

    // encode this ramp payload once for fd-backed upload
    let encoded = encoded_gamma_bytes(&red, &green, &blue);

    // apply gamma table through one short-lived gamma-control object
    core::with_connection_dispatch(
        context,
        "destack.display.monitor.setGammaRamp",
        |_connection, event_queue, dispatch_state| {
            let output_global_name = output_global_name_for_display_id(display_id.as_str())?;
            let output = dispatch_state
                .outputs_by_global
                .get(&output_global_name)
                .cloned()
                .ok_or_else(|| {
                    core_platform::not_supported("destack.display.monitor.setGammaRamp")
                })?;
            let manager = dispatch_state
                .gamma_control_manager
                .as_ref()
                .cloned()
                .ok_or_else(|| {
                    core_platform::not_supported("destack.display.monitor.setGammaRamp")
                })?;

            let query_state = Arc::new(Mutex::new(core::WaylandGammaControlQueryState::default()));
            let gamma_control =
                manager.get_gamma_control(&output, &event_queue.handle(), Arc::clone(&query_state));
            core::flush_queue(event_queue, "destack.display.monitor.setGammaRamp")?;

            // wait until one gamma-size event confirms control readiness
            for _ in 0..6 {
                let snapshot = query_state
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .clone();
                if snapshot.gamma_size.is_some() || snapshot.failed {
                    break;
                }

                event_queue.roundtrip(dispatch_state).map_err(|error| {
                    core::io_error(
                        "destack.display.monitor.setGammaRamp",
                        format!("wayland gamma-control roundtrip failed: {error}"),
                    )
                })?;
            }

            let snapshot = query_state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone();
            if snapshot.failed {
                gamma_control.destroy();
                core::flush_queue(event_queue, "destack.display.monitor.setGammaRamp")?;
                return Err(core_platform::not_supported(
                    "destack.display.monitor.setGammaRamp",
                ));
            }
            if snapshot.gamma_size != Some(expected as u32) {
                gamma_control.destroy();
                core::flush_queue(event_queue, "destack.display.monitor.setGammaRamp")?;
                return Err(core_platform::io_operation_error(
                    "destack.display.monitor.setGammaRamp",
                    None,
                    "wayland gamma size changed while applying gamma ramp",
                ));
            }

            // upload one packed gamma table through one memfd payload
            let mut file =
                create_gamma_memfd_file("destack.display.monitor.setGammaRamp", encoded.len())?;
            file.write_all(encoded.as_slice()).map_err(|error| {
                core::io_error(
                    "destack.display.monitor.setGammaRamp",
                    format!("gamma table write failed: {error}"),
                )
            })?;
            file.flush().map_err(|error| {
                core::io_error(
                    "destack.display.monitor.setGammaRamp",
                    format!("gamma table flush failed: {error}"),
                )
            })?;
            gamma_control.set_gamma(file.as_fd());
            core::flush_queue(event_queue, "destack.display.monitor.setGammaRamp")?;

            // roundtrip once so failed events are visible before returning
            event_queue.roundtrip(dispatch_state).map_err(|error| {
                core::io_error(
                    "destack.display.monitor.setGammaRamp",
                    format!("wayland gamma-control apply roundtrip failed: {error}"),
                )
            })?;
            let failed = query_state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .failed;

            gamma_control.destroy();
            core::flush_queue(event_queue, "destack.display.monitor.setGammaRamp")?;

            if failed {
                return Err(core_platform::io_operation_error(
                    "destack.display.monitor.setGammaRamp",
                    None,
                    "compositor rejected wayland gamma table update",
                ));
            }

            Ok(())
        },
    )?;

    // cache the last-applied ramp for deterministic readback behavior
    let runtime_state = core::runtime_state(context);
    let mut cache = runtime_state
        .gamma_ramps_by_display_id
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    cache.insert(
        display_id,
        core::WaylandGammaRampSnapshot { red, green, blue },
    );

    Ok(())
}
