use windows_sys::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    CDS_FULLSCREEN, ChangeDisplaySettingsExW, CreateDCW, DEVMODEW, DISP_CHANGE_BADFLAGS,
    DISP_CHANGE_BADMODE, DISP_CHANGE_BADPARAM, DISP_CHANGE_SUCCESSFUL, DISPLAY_DEVICE_ACTIVE,
    DISPLAY_DEVICE_REMOVABLE, DISPLAY_DEVICEW, DM_BITSPERPEL, DM_DISPLAYFREQUENCY,
    DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH, DMDO_90, DMDO_180, DMDO_270, DMDO_DEFAULT,
    DeleteDC, ENUM_CURRENT_SETTINGS, ENUM_REGISTRY_SETTINGS, EnumDisplayDevicesW,
    EnumDisplayMonitors, EnumDisplaySettingsExW, GetDeviceCaps, GetMonitorInfoW, HORZSIZE,
    LOGPIXELSX, MONITORINFOEXW, VERTSIZE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayBackend, DisplayDescriptor, DisplayMode, DisplayMonitorListRequest,
    DisplayMonitorOpenOptions, DisplayOrientation, WindowMode, WindowModeOptions,
};
use crate::platform::{NativeSlice, NativeStringRef, resource};
use crate::runtime::BindingCallContext;

use super::model::{DisplayDescriptorOwned, MonitorSnapshot};
use super::{core, event, resource as display_resource};

/// Build one DisplayMode payload from one DEVMODEW value.
fn display_mode_from_devmode(value: &DEVMODEW) -> DisplayMode {
    let bit_depth = if value.dmBitsPerPel > u16::MAX as u32 {
        u16::MAX
    } else {
        value.dmBitsPerPel as u16
    };

    DisplayMode {
        width: value.dmPelsWidth,
        height: value.dmPelsHeight,
        refresh_milli_hz: value.dmDisplayFrequency.saturating_mul(1000),
        format: 0,
        bit_depth,
    }
}

/// Query one raw DEVMODE payload from one monitor device and mode selector.
fn query_raw_devmode(device_id: &str, mode_index: u32) -> RuntimeResult<Option<DEVMODEW>> {
    let device_wide = core::wide_with_nul("id", device_id)?;

    let mut dev_mode = unsafe { std::mem::zeroed::<DEVMODEW>() };
    dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
    let status =
        unsafe { EnumDisplaySettingsExW(device_wide.as_ptr(), mode_index, &mut dev_mode, 0) };
    if status == 0 {
        return Ok(None);
    }

    Ok(Some(dev_mode))
}

/// Resolve one display orientation payload from one raw DEVMODE value.
fn orientation_from_devmode(value: &DEVMODEW) -> DisplayOrientation {
    let has_orientation = (value.dmFields & DM_DISPLAYORIENTATION) != 0;
    if !has_orientation {
        return if value.dmPelsWidth >= value.dmPelsHeight {
            DisplayOrientation::Landscape
        } else {
            DisplayOrientation::Portrait
        };
    }

    let orientation = unsafe { value.Anonymous1.Anonymous2.dmDisplayOrientation };
    if orientation == DMDO_DEFAULT {
        return if value.dmPelsWidth >= value.dmPelsHeight {
            DisplayOrientation::Landscape
        } else {
            DisplayOrientation::Portrait
        };
    }

    if orientation == DMDO_90 {
        return DisplayOrientation::Portrait;
    }

    if orientation == DMDO_180 {
        return DisplayOrientation::LandscapeFlipped;
    }

    if orientation == DMDO_270 {
        return DisplayOrientation::PortraitFlipped;
    }

    DisplayOrientation::Unknown
}

/// Build one fallback mode payload from one monitor rectangle.
fn display_mode_from_rect(rect: RECT) -> DisplayMode {
    let width = (rect.right - rect.left).max(1) as u32;
    let height = (rect.bottom - rect.top).max(1) as u32;

    DisplayMode {
        width,
        height,
        refresh_milli_hz: 60_000,
        format: 0,
        bit_depth: 32,
    }
}

/// Query one display mode from one monitor device and mode selector.
fn query_display_mode(device_id: &str, mode_index: u32) -> RuntimeResult<Option<DisplayMode>> {
    let Some(dev_mode) = query_raw_devmode(device_id, mode_index)? else {
        return Ok(None);
    };
    Ok(Some(display_mode_from_devmode(&dev_mode)))
}

/// Enumerate all supported display modes for one monitor device id.
fn enumerate_display_modes(device_id: &str) -> RuntimeResult<Vec<DisplayMode>> {
    let mut modes = Vec::new();
    let mut mode_index = 0u32;

    while let Some(mode) = query_display_mode(device_id, mode_index)? {
        mode_index = mode_index.saturating_add(1);

        if modes.contains(&mode) {
            continue;
        }

        modes.push(mode);
    }

    if modes.is_empty()
        && let Some(current_mode) = query_display_mode(device_id, ENUM_CURRENT_SETTINGS)?
    {
        modes.push(current_mode);
    }

    Ok(modes)
}

/// Query one monitor display-name string from one monitor device id.
fn display_name_for_device(device_id: &str) -> RuntimeResult<Option<String>> {
    let device_wide = core::wide_with_nul("id", device_id)?;

    let mut display_device = unsafe { std::mem::zeroed::<DISPLAY_DEVICEW>() };
    display_device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
    let status = unsafe { EnumDisplayDevicesW(device_wide.as_ptr(), 0, &mut display_device, 0) };
    if status == 0 {
        return Ok(None);
    }

    let value = core::utf16_buffer_to_string(&display_device.DeviceString);
    if value.is_empty() {
        return Ok(None);
    }

    Ok(Some(value))
}

/// Resolve whether one display likely maps to one built-in panel.
fn display_is_builtin(device_id: &str) -> RuntimeResult<bool> {
    let device_wide = core::wide_with_nul("id", device_id)?;
    let mut index = 0u32;
    loop {
        let mut display_device = unsafe { std::mem::zeroed::<DISPLAY_DEVICEW>() };
        display_device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        let status =
            unsafe { EnumDisplayDevicesW(device_wide.as_ptr(), index, &mut display_device, 0) };
        if status == 0 {
            break;
        }

        index = index.saturating_add(1);
        if (display_device.StateFlags & DISPLAY_DEVICE_ACTIVE) == 0 {
            continue;
        }

        if (display_device.StateFlags & DISPLAY_DEVICE_REMOVABLE) != 0 {
            continue;
        }

        let identifier = core::utf16_buffer_to_string(&display_device.DeviceID).to_uppercase();
        if identifier.contains("INTERNAL")
            || identifier.contains("EDP")
            || identifier.contains("LVDS")
            || identifier.contains("DSI")
        {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Query physical monitor metrics and dpi-derived scale for one display device.
fn display_metrics_for_device(device_id: &str) -> RuntimeResult<(u32, u32, u32)> {
    let device_wide = core::wide_with_nul("id", device_id)?;

    let hdc = unsafe {
        CreateDCW(
            device_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if hdc == 0 {
        return Ok((0, 0, 1000));
    }

    let width_mm_raw = unsafe { GetDeviceCaps(hdc, HORZSIZE as i32) };
    let height_mm_raw = unsafe { GetDeviceCaps(hdc, VERTSIZE as i32) };
    let dpi_x = unsafe { GetDeviceCaps(hdc, LOGPIXELSX as i32) };

    unsafe {
        DeleteDC(hdc);
    }

    let width_mm = width_mm_raw.max(0) as u32;
    let height_mm = height_mm_raw.max(0) as u32;
    let scale_factor_milli = if dpi_x <= 0 {
        1000
    } else {
        (dpi_x as u32).saturating_mul(1000) / 96
    }
    .max(1);

    Ok((width_mm, height_mm, scale_factor_milli))
}

/// Enumerate raw monitor rows from the active desktop.
fn enumerate_monitor_rows() -> RuntimeResult<Vec<(String, RECT, RECT, bool)>> {
    unsafe extern "system" fn callback(
        monitor: isize,
        _hdc: isize,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let output = unsafe { &mut *(data as *mut Vec<(String, RECT, RECT, bool)>) };

        let mut info = unsafe { std::mem::zeroed::<MONITORINFOEXW>() };
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

        if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) } == 0 {
            return 1;
        }

        let device_id = core::utf16_buffer_to_string(&info.szDevice);
        if device_id.is_empty() {
            return 1;
        }

        let is_primary = (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0;
        output.push((
            device_id,
            info.monitorInfo.rcMonitor,
            info.monitorInfo.rcWork,
            is_primary,
        ));

        1
    }

    let mut rows: Vec<(String, RECT, RECT, bool)> = Vec::new();
    let status = unsafe {
        EnumDisplayMonitors(
            0,
            std::ptr::null(),
            Some(callback),
            &mut rows as *mut _ as LPARAM,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.monitor.list",
            "EnumDisplayMonitors",
            "failed to enumerate monitors",
        ));
    }

    Ok(rows)
}

/// Enumerate monitor snapshots for the current desktop.
pub(super) fn enumerate_monitor_snapshots() -> RuntimeResult<Vec<MonitorSnapshot>> {
    let rows = enumerate_monitor_rows()?;
    let mut snapshots = Vec::with_capacity(rows.len());

    for (device_id, bounds, work_area, primary) in rows {
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
        if !modes.contains(&current_mode) {
            modes.push(current_mode);
        }
        if !modes.contains(&desktop_mode) {
            modes.push(desktop_mode);
        }

        let (width_mm, height_mm, scale_factor_milli) = display_metrics_for_device(&device_id)?;
        let is_builtin = display_is_builtin(&device_id)?;
        let width_px = (bounds.right - bounds.left).max(1) as u32;
        let height_px = (bounds.bottom - bounds.top).max(1) as u32;
        let work_area_width_px = (work_area.right - work_area.left).max(1) as u32;
        let work_area_height_px = (work_area.bottom - work_area.top).max(1) as u32;

        let descriptor = DisplayDescriptorOwned {
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
            is_builtin,
            supports_variable_refresh: false,
            supports_hdr: false,
        };

        snapshots.push(MonitorSnapshot {
            descriptor,
            current_mode,
            desktop_mode,
            modes,
        });
    }

    snapshots
        .sort_by_key(|snapshot| (!snapshot.descriptor.primary, snapshot.descriptor.id.clone()));
    Ok(snapshots)
}

/// Resolve one monitor snapshot by identifier.
pub(super) fn monitor_snapshot_by_id(id: &str) -> RuntimeResult<Option<MonitorSnapshot>> {
    let snapshots = enumerate_monitor_snapshots()?;
    Ok(snapshots
        .into_iter()
        .find(|snapshot| snapshot.descriptor.id == id))
}

/// Convert one owned descriptor payload into one ABI descriptor payload.
pub(super) fn descriptor_from_owned(
    context: &BindingCallContext,
    value: &DisplayDescriptorOwned,
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
        is_builtin: value.is_builtin,
        supports_variable_refresh: value.supports_variable_refresh,
        supports_hdr: value.supports_hdr,
    }
}

/// Resolve one monitor snapshot associated with one opened monitor handle.
fn monitor_snapshot_for_handle(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    operation: &'static str,
) -> RuntimeResult<MonitorSnapshot> {
    let id = display_resource::resolve_display_id(context, handle, operation)?;

    monitor_snapshot_by_id(&id)?.ok_or_else(|| {
        core::not_found(
            operation,
            format!("display id '{id}' is no longer available"),
        )
    })
}

/// Apply one monitor mode through ChangeDisplaySettingsExW.
pub(super) fn apply_monitor_mode_by_id(
    id: &str,
    mode: DisplayMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    if mode.width == 0 || mode.height == 0 {
        return Err(core::invalid_argument(
            "mode",
            "mode dimensions must be greater than zero",
        ));
    }

    let device_wide = core::wide_with_nul("id", id)?;

    let mut dev_mode = unsafe { std::mem::zeroed::<DEVMODEW>() };
    dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
    dev_mode.dmPelsWidth = mode.width;
    dev_mode.dmPelsHeight = mode.height;
    dev_mode.dmBitsPerPel = u32::from(mode.bit_depth);
    dev_mode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_BITSPERPEL;

    if mode.refresh_milli_hz > 0 {
        dev_mode.dmDisplayFrequency = (mode.refresh_milli_hz / 1000).max(1);
        dev_mode.dmFields |= DM_DISPLAYFREQUENCY;
    }

    let result = unsafe {
        ChangeDisplaySettingsExW(
            device_wide.as_ptr(),
            &dev_mode,
            0,
            CDS_FULLSCREEN,
            std::ptr::null(),
        )
    };

    if result == DISP_CHANGE_SUCCESSFUL {
        return Ok(());
    }

    if result == DISP_CHANGE_BADMODE {
        return Err(core::invalid_argument(
            "mode",
            "requested display mode is not supported",
        ));
    }

    if result == DISP_CHANGE_BADPARAM || result == DISP_CHANGE_BADFLAGS {
        return Err(core::invalid_argument(
            "mode",
            "requested display mode parameters are invalid",
        ));
    }

    Err(core::io_error_with_code(
        operation,
        "ChangeDisplaySettingsExW",
        result as u32,
        "failed to apply display mode",
    ))
}

/// Resolve one monitor target rectangle for one mode transition.
pub(super) fn mode_target_rect(
    context: &BindingCallContext,
    mode: WindowModeOptions,
    display: Option<resource::DisplayHandle>,
) -> RuntimeResult<Option<RECT>> {
    match mode.mode {
        WindowMode::Windowed => Ok(None),
        WindowMode::Borderless | WindowMode::ExclusiveFullscreen => {
            let display = mode.display.or(display).ok_or_else(|| {
                core::invalid_argument(
                    "mode.display",
                    "fullscreen mode requires one display target",
                )
            })?;
            let id = display_resource::resolve_display_id(
                context,
                display,
                "destack.display.window.setMode",
            )?;
            let snapshot = monitor_snapshot_by_id(&id)?.ok_or_else(|| {
                core::not_found(
                    "destack.display.window.setMode",
                    format!("display id '{id}' is no longer available"),
                )
            })?;

            Ok(Some(RECT {
                left: snapshot.descriptor.x,
                top: snapshot.descriptor.y,
                right: snapshot.descriptor.x + snapshot.descriptor.width_px as i32,
                bottom: snapshot.descriptor.y + snapshot.descriptor.height_px as i32,
            }))
        }
    }
}

/// List available displays.
pub(crate) unsafe fn monitor_list(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;

    let descriptors = enumerate_monitor_snapshots()?
        .into_iter()
        .map(|snapshot| descriptor_from_owned(context, &snapshot.descriptor))
        .collect::<Vec<_>>();

    unsafe {
        *out = context.store_slice(descriptors);
    }

    let _ = request;
    Ok(())
}

/// Open one display endpoint.
pub(crate) unsafe fn monitor_open(
    context: &BindingCallContext,
    out: *mut resource::DisplayHandle,
    id: NativeStringRef,
    options: DisplayMonitorOpenOptions,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;

    let id = unsafe { id.as_str()? };
    let snapshot = monitor_snapshot_by_id(id)?.ok_or_else(|| {
        core::not_found(
            "destack.display.monitor.open",
            format!("display id '{id}' was not found"),
        )
    })?;

    let handle = display_resource::open_display_handle(context, snapshot.descriptor.id);
    unsafe {
        *out = handle;
    }

    let _ = options;
    Ok(())
}

/// Close one display endpoint.
pub(crate) unsafe fn monitor_close(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    let _ = display_resource::resolve_display_id(context, handle, "destack.display.monitor.close")?;

    let removed = context
        .runtime()
        .resources
        .remove(handle.0, Some(context.engine()))
        .is_some();
    if !removed {
        return Err(core::not_found(
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
    core::ensure_out(out, "out")?;

    let snapshot =
        monitor_snapshot_for_handle(context, handle, "destack.display.monitor.descriptor")?;
    unsafe {
        *out = descriptor_from_owned(context, &snapshot.descriptor);
    }

    Ok(())
}

/// Read available display modes.
pub(crate) unsafe fn monitor_modes(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayMode>,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;

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
    core::ensure_out(out, "out")?;

    let snapshot =
        monitor_snapshot_for_handle(context, handle, "destack.display.monitor.currentMode")?;
    unsafe {
        *out = snapshot.current_mode;
    }

    Ok(())
}

/// Read the desktop-preferred mode for one opened display.
pub(crate) unsafe fn monitor_desktop_mode(
    context: &BindingCallContext,
    out: *mut DisplayMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;

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
    core::ensure_out(out, "out")?;

    let snapshot =
        monitor_snapshot_for_handle(context, handle, "destack.display.monitor.closestMode")?;
    if snapshot.modes.is_empty() {
        return Err(core::not_found(
            "destack.display.monitor.closestMode",
            "display mode table is empty",
        ));
    }

    let mut best = snapshot.modes[0];
    let mut best_score = u64::MAX;
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
    request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    core::ensure_out(out, "out")?;

    let snapshots = enumerate_monitor_snapshots()?;
    let primary = snapshots
        .into_iter()
        .find(|snapshot| snapshot.descriptor.primary)
        .map(|snapshot| display_resource::open_display_handle(context, snapshot.descriptor.id));

    unsafe {
        *out = primary;
    }

    let _ = request;
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

    Ok(())
}
