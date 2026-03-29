use objc2_core_foundation::{CFArray, CFRetained};
use objc2_core_graphics::{
    CGDisplayCopyAllDisplayModes, CGDisplayCopyDisplayMode, CGDisplayMode, CGDisplaySetDisplayMode,
    CGError,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::display::DisplayMode;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::{core as monitor_core, monitor_snapshot_by_display_id};
use crate::platform::display::unix::appkit::{core, event, resource as display_resource};

/// Resolve one requested mode to the closest supported mode.
pub(crate) unsafe fn monitor_closest_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
    requested: DisplayMode,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.closestMode",
    )?;
    let snapshot = monitor_snapshot_by_display_id(&display_id)?
        .ok_or_else(|| core::display_not_found("destack.display.monitor.closestMode", handle))?;

    let mut selected = snapshot.current_mode;
    let mut selected_score = u64::MAX;

    // scan all advertised modes for the closest match
    for mode in &snapshot.modes {
        let width_delta = mode.width.abs_diff(requested.width) as u64;
        let height_delta = mode.height.abs_diff(requested.height) as u64;
        let refresh_delta = mode.refresh_milli_hz.abs_diff(requested.refresh_milli_hz) as u64;
        let score = width_delta
            .saturating_mul(1_000_000)
            .saturating_add(height_delta.saturating_mul(1_000_000))
            .saturating_add(refresh_delta);

        // keep the best-scoring candidate so far
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
    core_platform::ensure_out(out, "out")?;
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.currentMode",
    )?;
    let snapshot = monitor_snapshot_by_display_id(&display_id)?
        .ok_or_else(|| core::display_not_found("destack.display.monitor.currentMode", handle))?;

    unsafe {
        *out = snapshot.current_mode;
    }

    Ok(())
}

/// Read the desktop-preferred mode for one opened display.
pub(crate) unsafe fn monitor_desktop_mode(
    binding: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let display_id = display_resource::resolve_display_id(
        binding,
        handle,
        "destack.display.monitor.desktopMode",
    )?;
    let snapshot = monitor_snapshot_by_display_id(&display_id)?
        .ok_or_else(|| core::display_not_found("destack.display.monitor.desktopMode", handle))?;

    unsafe {
        *out = snapshot.desktop_mode;
    }

    Ok(())
}

/// Read available display modes.
pub(crate) unsafe fn monitor_modes(
    binding: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;
    let display_id =
        display_resource::resolve_display_id(binding, handle, "destack.display.monitor.modes")?;
    let snapshot = monitor_snapshot_by_display_id(&display_id)?
        .ok_or_else(|| core::display_not_found("destack.display.monitor.modes", handle))?;

    unsafe {
        *out = binding.store_slice(snapshot.modes.clone());
    }

    Ok(())
}

/// Apply one display mode.
pub(crate) unsafe fn monitor_set_mode(
    binding: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayMode,
) -> RuntimeResult<()> {
    let display_id =
        display_resource::resolve_display_id(binding, handle, "destack.display.monitor.setMode")?;
    let display = monitor_core::display_from_id(&display_id).ok_or_else(|| {
        core_platform::invalid_argument("handle", "display id is not one AppKit display identifier")
    })?;

    let array = unsafe { CGDisplayCopyAllDisplayModes(display, None) }.ok_or_else(|| {
        core::io_error(
            "destack.display.monitor.setMode",
            format!("CGDisplayCopyAllDisplayModes returned null for display {display}"),
        )
    })?;
    let array = unsafe { CFRetained::cast_unchecked::<CFArray<CGDisplayMode>>(array) };
    let current_mode_native = CGDisplayCopyDisplayMode(display).ok_or_else(|| {
        core::io_error(
            "destack.display.monitor.setMode",
            format!("CGDisplayCopyDisplayMode returned null for display {display}"),
        )
    })?;
    let current_mode = monitor_core::display_mode_from_native(&current_mode_native);

    let mut selected_mode = None;

    // find the native display mode that matches the requested runtime mode
    for candidate in &*array {
        let resolved = monitor_core::display_mode_from_native(&candidate);

        // keep the first exact runtime-mode match
        if resolved == mode {
            selected_mode = Some(candidate);
            break;
        }
    }

    // accept the active mode when CoreGraphics does not round trip it through the mode catalog
    if selected_mode.is_none() && current_mode == mode {
        let runtime_state = core::runtime_state(binding);
        event::publish_monitor_mode_changed(&runtime_state, &display_id, None, mode);
        event::refresh_monitor_topology_cache(&runtime_state)?;

        return Ok(());
    }

    let selected_mode = selected_mode.ok_or_else(|| {
        core_platform::invalid_argument(
            "mode",
            "requested mode is not supported by the selected display",
        )
    })?;

    let status = unsafe { CGDisplaySetDisplayMode(display, Some(&selected_mode), None) };

    // surface CoreGraphics mode-switch failures explicitly
    if status != CGError(0) {
        return Err(core::io_error(
            "destack.display.monitor.setMode",
            format!("CGDisplaySetDisplayMode failed with status {status:?}"),
        ));
    }

    let runtime_state = core::runtime_state(binding);
    event::publish_monitor_mode_changed(&runtime_state, &display_id, None, mode);
    event::publish_monitor_topology_deltas(&runtime_state)?;

    Ok(())
}
