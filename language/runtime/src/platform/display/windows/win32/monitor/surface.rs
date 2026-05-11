use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::display::{
    DisplayDescriptor, DisplayMode, DisplayMonitorListRequest, DisplayMonitorOpenOptions,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::mode::apply_monitor_mode_by_id;
use super::snapshot::{
    descriptor_from_value, enumerate_monitor_snapshots, monitor_snapshot_by_id,
    monitor_snapshot_for_handle,
};
use crate::platform::display::windows::win32::{core, event, resource as display_resource};

/// List available displays.
pub(crate) unsafe fn monitor_list(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    _request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let descriptors = enumerate_monitor_snapshots()?
        .into_iter()
        .map(|snapshot| descriptor_from_value(context, &snapshot.descriptor))
        .collect::<Vec<_>>();
    unsafe {
        *out = context.store_slice(descriptors);
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
    core_platform::ensure_out(out, "out")?;

    let id = unsafe { id.as_str()? };
    let snapshot = monitor_snapshot_by_id(id)?.ok_or_else(|| {
        core_platform::io_not_found(
            "destack.display.monitor.open",
            format!("display id '{id}' was not found"),
        )
    })?;

    let handle = display_resource::open_display_handle(context, snapshot.descriptor.id);
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Close one display endpoint.
pub(crate) unsafe fn monitor_close(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    display_resource::resolve_display_id(context, handle, "destack.display.monitor.close")?;

    let removed = context
        .worker()
        .resources
        .remove(context.world(), handle.0, Some(context.engine()))
        .is_some();
    if !removed {
        return Err(core_platform::io_not_found(
            "destack.display.monitor.close",
            format!("display handle {} was not found", handle.0.0),
        ));
    }

    Ok(())
}

/// Read descriptor metadata for one opened display.
pub(crate) unsafe fn monitor_descriptor(
    context: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let snapshot =
        monitor_snapshot_for_handle(context, handle, "destack.display.monitor.descriptor")?;
    unsafe {
        *out = descriptor_from_value(context, &snapshot.descriptor);
    }

    Ok(())
}

/// Read available display modes.
pub(crate) unsafe fn monitor_modes(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let snapshot = monitor_snapshot_for_handle(context, handle, "destack.display.monitor.modes")?;
    unsafe {
        *out = context.store_slice(snapshot.modes);
    }

    Ok(())
}

/// Read the current mode for one opened display.
pub(crate) unsafe fn monitor_current_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let snapshot =
        monitor_snapshot_for_handle(context, handle, "destack.display.monitor.currentMode")?;
    unsafe {
        *out = snapshot.current_mode;
    }

    Ok(())
}

/// Read the desktop preferred mode for one opened display.
pub(crate) unsafe fn monitor_desktop_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let snapshot =
        monitor_snapshot_for_handle(context, handle, "destack.display.monitor.desktopMode")?;
    unsafe {
        *out = snapshot.desktop_mode;
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
    core_platform::ensure_out(out, "out")?;

    let snapshot =
        monitor_snapshot_for_handle(context, handle, "destack.display.monitor.closestMode")?;
    if snapshot.modes.is_empty() {
        return Err(core_platform::io_not_found(
            "destack.display.monitor.closestMode",
            "display mode table is empty",
        ));
    }

    let mut best = snapshot.modes[0];
    let mut best_score = u64::MAX;

    // choose one mode by width, height, refresh, and bit depth deltas
    for mode in snapshot.modes {
        let width_delta = mode.width.abs_diff(requested.width) as u64;
        let height_delta = mode.height.abs_diff(requested.height) as u64;
        let refresh_delta = mode.refresh_milli_hz.abs_diff(requested.refresh_milli_hz) as u64;
        let bit_depth_delta = mode.bit_depth.abs_diff(requested.bit_depth) as u64;
        let score = width_delta
            .saturating_mul(1_000_000)
            .saturating_add(height_delta.saturating_mul(1_000_000))
            .saturating_add(refresh_delta.saturating_mul(1000))
            .saturating_add(bit_depth_delta);
        if score < best_score {
            best = mode;
            best_score = score;
        }
    }

    unsafe {
        *out = best;
    }

    Ok(())
}

/// Read the current primary display handle.
pub(crate) unsafe fn monitor_primary(
    context: &BindingCallContext,
    out: *mut Option<resource::DisplayHandle>,
    _request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let snapshots = enumerate_monitor_snapshots()?;
    let primary = snapshots
        .into_iter()
        .find(|snapshot| snapshot.descriptor.primary)
        .map(|snapshot| display_resource::open_display_handle(context, snapshot.descriptor.id));
    unsafe {
        *out = primary;
    }

    Ok(())
}

/// Apply one display mode.
pub(crate) unsafe fn monitor_set_mode(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    let id =
        display_resource::resolve_display_id(context, handle, "destack.display.monitor.setMode")?;

    apply_monitor_mode_by_id(&id, mode, "destack.display.monitor.setMode")?;
    if let Some(snapshot) = monitor_snapshot_by_id(&id)? {
        event::publish_mode_changed_event(context, &id, snapshot.current_mode);
        event::publish_descriptor_changed_event(
            context,
            &snapshot.descriptor,
            core::DISPLAY_CHANGED_MASK_BOUNDS
                | core::DISPLAY_CHANGED_MASK_WORKAREA
                | core::DISPLAY_CHANGED_MASK_SCALE
                | core::DISPLAY_CHANGED_MASK_ORIENTATION,
        );
    } else {
        event::publish_mode_changed_event(context, &id, mode);
    }

    event::refresh_monitor_topology_cache(context)?;
    Ok(())
}
