use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::display::{
    DisplayDescriptor, DisplayMode, DisplayMonitorListRequest, DisplayMonitorOpenOptions,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::snapshot::{
    descriptor_from_value, enumerate_monitor_snapshots_for_operation, snapshot_by_display_handle,
    snapshot_by_display_id,
};
use crate::platform::display::unix::wayland::{core, resource as display_resource};

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
        .worker()
        .resources
        .remove(&context.world(), handle.0, Some(context.engine()))
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

    // choose one closest mode by area and refresh rate deltas
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
        *out = descriptor_from_value(context, &snapshot.descriptor);
    }

    Ok(())
}

/// Read the desktop preferred mode for one opened display.
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
        values.push(descriptor_from_value(context, &snapshot.descriptor));
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
