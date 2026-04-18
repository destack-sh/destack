use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::display::{
    DisplayDescriptor, DisplayMode, DisplayMonitorListRequest, DisplayMonitorOpenOptions,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::snapshot::{
    descriptor_from_value, enumerate_monitor_snapshots, monitor_snapshot_for_handle,
};
use crate::platform::display::unix::x11::{core, resource as display_resource};

/// List available displays.
pub(crate) unsafe fn monitor_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    _request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // enumerate monitor descriptors and store them in one slice
    let snapshots = enumerate_monitor_snapshots(binding)?;
    let mut values = Vec::with_capacity(snapshots.len());
    for snapshot in snapshots {
        values.push(descriptor_from_value(binding, &snapshot.descriptor));
    }

    unsafe {
        *out = binding.store_slice(values);
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
    // validate out pointer and parse the requested id
    core_platform::ensure_out(out, "out")?;
    let id = unsafe { id.as_str()? };

    // validate that the requested display exists
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

    // insert one display handle resource and return it
    let handle = display_resource::open_display_handle(binding, snapshot.descriptor.id.clone());
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Close one display endpoint.
pub(crate) unsafe fn monitor_close(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // remove the resource entry from the runtime table
    let removed = binding
        .worker()
        .resources
        .remove(&binding.world(), handle.0, Some(binding.engine()))
        .is_some();
    if !removed {
        return Err(core::display_not_found(
            "destack.display.monitor.close",
            handle,
        ));
    }

    Ok(())
}

/// Read descriptor metadata for one opened display.
pub(crate) unsafe fn monitor_descriptor(
    binding: &BindingCallContext,
    out: *mut DisplayDescriptor,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve one monitor snapshot
    core_platform::ensure_out(out, "out")?;
    let snapshot =
        monitor_snapshot_for_handle(binding, handle, "destack.display.monitor.descriptor")?;

    unsafe {
        *out = descriptor_from_value(binding, &snapshot.descriptor);
    }

    Ok(())
}

/// Read available display modes.
pub(crate) unsafe fn monitor_modes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve one monitor snapshot
    core_platform::ensure_out(out, "out")?;
    let snapshot = monitor_snapshot_for_handle(binding, handle, "destack.display.monitor.modes")?;

    unsafe {
        *out = binding.store_slice(snapshot.modes);
    }

    Ok(())
}

/// Read the current mode for one opened display.
pub(crate) unsafe fn monitor_current_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve one monitor snapshot
    core_platform::ensure_out(out, "out")?;
    let snapshot =
        monitor_snapshot_for_handle(binding, handle, "destack.display.monitor.currentMode")?;

    unsafe {
        *out = snapshot.current_mode;
    }

    Ok(())
}

/// Read the desktop preferred mode for one opened display.
pub(crate) unsafe fn monitor_desktop_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    // validate out pointer and resolve one monitor snapshot
    core_platform::ensure_out(out, "out")?;
    let snapshot =
        monitor_snapshot_for_handle(binding, handle, "destack.display.monitor.desktopMode")?;

    unsafe {
        *out = snapshot.desktop_mode;
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
    // validate out pointer and resolve one monitor snapshot
    core_platform::ensure_out(out, "out")?;
    let snapshot =
        monitor_snapshot_for_handle(binding, handle, "destack.display.monitor.closestMode")?;

    // choose the closest mode by width, height, and refresh delta
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

/// Read the current primary display handle.
pub(crate) unsafe fn monitor_primary(
    binding: &BindingCallContext,
    out: *mut Option<resource::DisplayHandle>,
    _request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // resolve the primary monitor and open one handle when present
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
