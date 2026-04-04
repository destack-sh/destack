use objc2_app_kit::NSScreen;
use objc2_core_foundation::{CFArray, CFRetained};
use objc2_core_graphics::{
    CGDirectDisplayID, CGDisplayBounds, CGDisplayCopyAllDisplayModes, CGDisplayCopyDisplayMode,
    CGDisplayIsBuiltin, CGDisplayMode, CGDisplayPixelsHigh, CGDisplayPixelsWide,
    CGDisplayScreenSize, CGError, CGGetActiveDisplayList, CGMainDisplayID,
};

use super::core::{display_id as monitor_display_id, display_orientation};
use crate::diagnostic::RuntimeResult;
use crate::host::os::apple::call::with_process_main_context_marker_if_needed;
use crate::platform;
use crate::platform::display::unix::appkit::core;
use crate::platform::display::unix::appkit::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use crate::platform::display::unix::appkit::window::display_id_from_screen;
use crate::platform::display::{DisplayDescriptor, DisplayMode, DisplaySupportStatus};
use crate::runtime::BindingCallContext;

/// Enumerate all active CoreGraphics display identifiers.
fn active_displays(operation: &'static str) -> RuntimeResult<Vec<CGDirectDisplayID>> {
    let mut expected_count = 0u32;
    let status = unsafe { CGGetActiveDisplayList(0, std::ptr::null_mut(), &mut expected_count) };

    // surface CoreGraphics display-list failures explicitly
    if status != CGError(0) {
        return Err(core::io_error(
            operation,
            format!("CGGetActiveDisplayList probe failed with status {status:?}"),
        ));
    }

    let expected_count = platform::core::u32_to_usize(expected_count);
    let mut displays = vec![0u32; expected_count];
    let mut actual_count = 0u32;
    let status = unsafe {
        CGGetActiveDisplayList(
            expected_count as u32,
            displays.as_mut_ptr(),
            &mut actual_count,
        )
    };

    // surface CoreGraphics primary-display failures explicitly
    if status != CGError(0) {
        return Err(core::io_error(
            operation,
            format!("CGGetActiveDisplayList read failed with status {status:?}"),
        ));
    }

    let actual_count = platform::core::u32_to_usize(actual_count);
    displays.truncate(actual_count);
    Ok(displays)
}

/// Enumerate all supported display modes for one display.
fn display_modes_for_display(
    display: CGDirectDisplayID,
    operation: &'static str,
) -> RuntimeResult<Vec<DisplayMode>> {
    let array = unsafe { CGDisplayCopyAllDisplayModes(display, None) }.ok_or_else(|| {
        core::io_error(
            operation,
            format!("CGDisplayCopyAllDisplayModes returned null for display {display}"),
        )
    })?;
    let array = unsafe { CFRetained::cast_unchecked::<CFArray<CGDisplayMode>>(array) };

    let mut modes = Vec::new();

    // collect unique runtime modes from the native mode catalog
    for mode in &*array {
        let resolved = super::core::display_mode_from_native(&mode);

        // skip duplicate runtime modes after normalization
        if !modes.contains(&resolved) {
            modes.push(resolved);
        }
    }

    Ok(modes)
}

/// Build one monitor snapshot from one CoreGraphics display identifier.
fn monitor_snapshot(
    display: CGDirectDisplayID,
    operation: &'static str,
) -> RuntimeResult<MonitorSnapshot> {
    let bounds = CGDisplayBounds(display);
    let size_mm = CGDisplayScreenSize(display);
    let name = format!("Display {display}");
    let current_mode_native = CGDisplayCopyDisplayMode(display).ok_or_else(|| {
        core::io_error(
            operation,
            format!("CGDisplayCopyDisplayMode returned null for display {display}"),
        )
    })?;
    let current_mode = super::core::display_mode_from_native(&current_mode_native);
    let mut modes = display_modes_for_display(display, operation)?;
    let desktop_mode = current_mode;

    // keep the active mode representable even when CoreGraphics catalog entries normalize it differently
    if !modes.contains(&current_mode) {
        modes.push(current_mode);
    }

    // keep the desktop mode representable even when it aliases the active mode today
    if !modes.contains(&desktop_mode) {
        modes.push(desktop_mode);
    }

    let width_px = CGDisplayPixelsWide(display) as u32;
    let height_px = CGDisplayPixelsHigh(display) as u32;
    let scale_factor_milli = display_scale_factor_milli(display);
    let descriptor = DisplayDescriptorSnapshot {
        backend: core::selected_backend(),
        id: super::core::display_id(display),
        name,
        primary: display == CGMainDisplayID(),
        x: bounds.origin.x.round() as i32,
        y: bounds.origin.y.round() as i32,
        width_px,
        height_px,
        work_area_x: bounds.origin.x.round() as i32,
        work_area_y: bounds.origin.y.round() as i32,
        work_area_width_px: width_px,
        work_area_height_px: height_px,
        width_mm: size_mm.width.max(0.0).round() as u32,
        height_mm: size_mm.height.max(0.0).round() as u32,
        scale_factor_milli,
        orientation: display_orientation(display),
        builtin_panel: if CGDisplayIsBuiltin(display) {
            DisplaySupportStatus::Supported
        } else {
            DisplaySupportStatus::Unsupported
        },
        variable_refresh_support: DisplaySupportStatus::Unknown,
        hdr_support: DisplaySupportStatus::Unknown,
    };

    Ok(MonitorSnapshot {
        descriptor,
        current_mode,
        desktop_mode,
        modes,
    })
}

/// Resolve one AppKit display scale factor in milli-scale units.
fn display_scale_factor_milli(display: CGDirectDisplayID) -> u32 {
    with_process_main_context_marker_if_needed(|mtm| {
        let screens = NSScreen::screens(mtm);

        // match the current CoreGraphics display against live AppKit screens
        for screen in screens.iter() {
            let Some(display_id) = display_id_from_screen(&screen) else {
                continue;
            };

            if display_id == monitor_display_id(display) {
                let scale_factor_milli = (screen.backingScaleFactor() * 1000.0).round() as u32;
                return Ok(scale_factor_milli.max(1));
            }
        }

        Ok(1000)
    })
    .unwrap_or(1000)
}

/// Enumerate current monitor snapshots.
pub(crate) fn enumerate_monitor_snapshots() -> RuntimeResult<Vec<MonitorSnapshot>> {
    let displays = active_displays("destack.display.monitor.list")?;
    let mut snapshots = Vec::with_capacity(displays.len());

    // snapshot each current display
    for display in displays {
        snapshots.push(monitor_snapshot(display, "destack.display.monitor.list")?);
    }

    Ok(snapshots)
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
        color_state: None,
    }
}

/// Resolve one monitor snapshot by stable display id.
pub(crate) fn monitor_snapshot_by_display_id(
    display_id: &str,
) -> RuntimeResult<Option<MonitorSnapshot>> {
    let snapshots = enumerate_monitor_snapshots()?;

    Ok(snapshots
        .into_iter()
        .find(|snapshot| snapshot.descriptor.id == display_id))
}
