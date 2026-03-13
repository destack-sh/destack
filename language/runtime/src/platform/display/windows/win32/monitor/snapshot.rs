use windows_sys::Win32::Graphics::Gdi::{ENUM_CURRENT_SETTINGS, ENUM_REGISTRY_SETTINGS};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayBackend, DisplayDescriptor, DisplayOrientation, DisplaySupportStatus,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{
    display_builtin_panel_support, display_metrics_for_device, display_mode_from_rect,
    display_name_for_device, enumerate_monitor_rows,
};
use super::gamma::display_hdr_support_map;
use super::mode::{
    enumerate_display_modes, orientation_from_devmode, query_display_mode, query_raw_devmode,
};
use crate::platform::display::windows::win32::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use crate::platform::display::windows::win32::resource as display_resource;

/// Enumerate monitor snapshots for the current desktop.
pub(crate) fn enumerate_monitor_snapshots() -> RuntimeResult<Vec<MonitorSnapshot>> {
    // query raw monitor rows and HDR support map
    let rows = enumerate_monitor_rows()?;
    let supports_hdr_by_device_id = display_hdr_support_map();
    let mut snapshots = Vec::with_capacity(rows.len());

    // build one snapshot per monitor row
    for (device_id, bounds, work_area, primary, scale_factor_milli) in rows {
        let name = display_name_for_device(&device_id)?.unwrap_or_else(|| device_id.clone());
        let current_mode = query_display_mode(&device_id, ENUM_CURRENT_SETTINGS)?
            .unwrap_or_else(|| display_mode_from_rect(bounds));
        let orientation = query_raw_devmode(&device_id, ENUM_CURRENT_SETTINGS)?
            .map(|mode| orientation_from_devmode(&mode))
            .unwrap_or_else(|| {
                if current_mode.width >= current_mode.height {
                    DisplayOrientation::Landscape
                } else {
                    DisplayOrientation::Portrait
                }
            });
        let desktop_mode =
            query_display_mode(&device_id, ENUM_REGISTRY_SETTINGS)?.unwrap_or(current_mode);
        let mut modes = enumerate_display_modes(&device_id)?;

        // keep current and desktop modes representable in the enumerated table
        if !modes.contains(&current_mode) {
            modes.push(current_mode);
        }
        if !modes.contains(&desktop_mode) {
            modes.push(desktop_mode);
        }

        // resolve descriptor support lanes and geometry
        let (width_mm, height_mm) = display_metrics_for_device(&device_id)?;
        let builtin_panel = display_builtin_panel_support(&device_id)?;
        let width_px = (bounds.right - bounds.left).max(1) as u32;
        let height_px = (bounds.bottom - bounds.top).max(1) as u32;
        let work_area_width_px = (work_area.right - work_area.left).max(1) as u32;
        let work_area_height_px = (work_area.bottom - work_area.top).max(1) as u32;

        let hdr_key = device_id.to_uppercase();
        let hdr_support = supports_hdr_by_device_id
            .get(&hdr_key)
            .copied()
            .map(|supports_hdr| {
                if supports_hdr {
                    DisplaySupportStatus::Supported
                } else {
                    DisplaySupportStatus::Unsupported
                }
            })
            .unwrap_or(DisplaySupportStatus::Unknown);
        let descriptor = DisplayDescriptorSnapshot {
            backend: DisplayBackend::Win32,
            id: device_id,
            name,
            primary,
            x: bounds.left,
            y: bounds.top,
            width_px,
            height_px,
            work_area_x: work_area.left,
            work_area_y: work_area.top,
            work_area_width_px,
            work_area_height_px,
            width_mm,
            height_mm,
            scale_factor_milli,
            orientation,
            builtin_panel,
            variable_refresh_support: DisplaySupportStatus::Unknown,
            hdr_support,
        };

        // append snapshot payload
        snapshots.push(MonitorSnapshot {
            descriptor,
            current_mode,
            desktop_mode,
            modes,
        });
    }

    // return stable primary first ordering
    snapshots
        .sort_by_key(|snapshot| (!snapshot.descriptor.primary, snapshot.descriptor.id.clone()));

    Ok(snapshots)
}

/// Resolve one monitor snapshot by identifier.
pub(crate) fn monitor_snapshot_by_id(id: &str) -> RuntimeResult<Option<MonitorSnapshot>> {
    let snapshots = enumerate_monitor_snapshots()?;

    Ok(snapshots
        .into_iter()
        .find(|snapshot| snapshot.descriptor.id == id))
}

/// Convert one owned descriptor payload into one ABI descriptor payload.
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

/// Resolve one monitor snapshot associated with one opened monitor handle.
pub(crate) fn monitor_snapshot_for_handle(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<MonitorSnapshot> {
    let id = display_resource::resolve_display_id(context, handle, operation)?;

    monitor_snapshot_by_id(&id)?.ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("display id '{id}' is no longer available"),
        )
    })
}
