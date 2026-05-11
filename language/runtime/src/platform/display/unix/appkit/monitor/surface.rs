use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::display::{
    DisplayColorState, DisplayDescriptor, DisplayHdrMode, DisplayMonitorListRequest,
    DisplayMonitorOpenOptions,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::{descriptor_from_value, enumerate_monitor_snapshots, monitor_snapshot_by_display_id};
use crate::platform::display::unix::appkit::{core, resource as display_resource};

/// Close one display endpoint.
pub(crate) unsafe fn monitor_close(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let removed = binding
        .worker()
        .resources
        .remove(binding.world(), handle.0, Some(binding.engine()))
        .is_some();

    // report unknown handles after resource removal
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
    core_platform::ensure_out(out, "out")?;
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.descriptor",
    )?;
    let snapshot = monitor_snapshot_by_display_id(&display_id)?
        .ok_or_else(|| core::display_not_found("destack.display.monitor.descriptor", handle))?;

    unsafe {
        *out = descriptor_from_value(binding, &snapshot.descriptor);
    }

    Ok(())
}

/// List available displays.
pub(crate) unsafe fn monitor_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    _request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let snapshots = enumerate_monitor_snapshots()?;
    let mut values = Vec::with_capacity(snapshots.len());

    // convert each snapshot descriptor into one ABI value
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
    core_platform::ensure_out(out, "out")?;
    let id = unsafe { id.as_str()? };
    let snapshot = monitor_snapshot_by_display_id(id)?.ok_or_else(|| {
        core_platform::io_not_found(
            "destack.display.monitor.open",
            format!("display id `{id}` was not found"),
        )
    })?;
    let handle = display_resource::open_display_handle(binding, snapshot.descriptor.id);

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
    core_platform::ensure_out(out, "out")?;
    let snapshots = enumerate_monitor_snapshots()?;
    let primary = snapshots
        .into_iter()
        .find(|snapshot| snapshot.descriptor.primary)
        .map(|snapshot| display_resource::open_display_handle(binding, snapshot.descriptor.id));

    unsafe {
        *out = primary;
    }

    Ok(())
}

/// Read display color state.
pub(crate) unsafe fn monitor_color_state(
    binding: &BindingCallContext,
    out: *mut DisplayColorState,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let _display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.colorState",
    )?;

    Err(core_platform::not_supported(
        "destack.display.monitor.colorState",
    ))
}

/// Read display HDR mode.
pub(crate) unsafe fn monitor_hdr_mode(
    binding: &BindingCallContext,
    out: *mut DisplayHdrMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let _display_id =
        display_resource::resolve_display_id(binding, handle, "destack.display.monitor.hdrMode")?;

    Err(core_platform::not_supported(
        "destack.display.monitor.hdrMode",
    ))
}

/// Set display HDR mode.
pub(crate) unsafe fn monitor_set_hdr_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayHdrMode,
) -> RuntimeResult<()> {
    let _display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.setHdrMode",
    )?;
    let _mode = mode;

    Err(core_platform::not_supported(
        "destack.display.monitor.setHdrMode",
    ))
}
