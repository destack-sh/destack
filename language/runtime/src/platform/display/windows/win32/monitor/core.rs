use windows_sys::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    CreateDCW, DISPLAY_DEVICE_ACTIVE, DISPLAY_DEVICE_REMOVABLE, DISPLAY_DEVICEW, DeleteDC,
    EnumDisplayDevicesW, EnumDisplayMonitors, GetDeviceCaps, GetMonitorInfoW, HORZSIZE,
    MONITORINFOEXW, VERTSIZE,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{DisplayMode, DisplayPixelFormat, DisplaySupportStatus};

use crate::platform::display::windows::win32::core as win32_core;
/// Number of gamma entries per color channel in Win32.
pub(crate) const GAMMA_RAMP_CHANNEL_ENTRIES: usize = 256;
/// Total number of gamma entries in one Win32 gamma table.
pub(crate) const GAMMA_RAMP_TOTAL_ENTRIES: usize = GAMMA_RAMP_CHANNEL_ENTRIES * 3;
/// One raw monitor-row tuple used by monitor enumeration.
pub(crate) type MonitorRow = (String, RECT, RECT, bool, u32);

/// Enumerate one monitor row and append normalized tuple payload.
unsafe extern "system" fn enumerate_monitor_rows_callback(
    monitor: isize,
    _hdc: isize,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    // decode output vector from callback payload
    let output = unsafe { &mut *(data as *mut Vec<MonitorRow>) };

    // load monitor information for this callback row
    let mut info = unsafe { std::mem::zeroed::<MONITORINFOEXW>() };
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) } == 0 {
        return 1;
    }

    // skip rows without one concrete device id
    let device_id = win32_core::utf16_buffer_to_string(&info.szDevice);
    if device_id.is_empty() {
        return 1;
    }

    // append one normalized monitor row
    let is_primary = (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0;
    let scale_factor_milli = monitor_scale_factor_milli(monitor);
    output.push((
        device_id,
        info.monitorInfo.rcMonitor,
        info.monitorInfo.rcWork,
        is_primary,
        scale_factor_milli,
    ));

    1
}

/// Build one fallback mode payload from one monitor rectangle.
pub(crate) fn display_mode_from_rect(rect: RECT) -> DisplayMode {
    // derive one fallback mode from monitor rectangle bounds
    let width = (rect.right - rect.left).max(1) as u32;
    let height = (rect.bottom - rect.top).max(1) as u32;

    DisplayMode {
        width,
        height,
        refresh_milli_hz: 60_000,
        format: DisplayPixelFormat(0),
        bit_depth: 32,
    }
}

/// Query one monitor display-name string from one monitor device id.
pub(crate) fn display_name_for_device(device_id: &str) -> RuntimeResult<Option<String>> {
    // query one logical monitor display name
    let device_wide = core_platform::wide_from_str("id", device_id)?;

    let mut display_device = unsafe { std::mem::zeroed::<DISPLAY_DEVICEW>() };
    display_device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
    let status = unsafe { EnumDisplayDevicesW(device_wide.as_ptr(), 0, &mut display_device, 0) };
    // return none when monitor row is unavailable
    if status == 0 {
        return Ok(None);
    }

    // normalize empty names to none
    let value = win32_core::utf16_buffer_to_string(&display_device.DeviceString);
    if value.is_empty() {
        return Ok(None);
    }

    Ok(Some(value))
}

/// Resolve one best effort built in panel support value for one display device.
pub(crate) fn display_builtin_panel_support(
    device_id: &str,
) -> RuntimeResult<DisplaySupportStatus> {
    // iterate display devices attached to one monitor row
    let device_wide = core_platform::wide_from_str("id", device_id)?;
    let mut index = 0u32;
    // loop until one branch exits
    loop {
        let mut display_device = unsafe { std::mem::zeroed::<DISPLAY_DEVICEW>() };
        display_device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        let status =
            unsafe { EnumDisplayDevicesW(device_wide.as_ptr(), index, &mut display_device, 0) };
        if status == 0 {
            break;
        }

        index = index.saturating_add(1);
        // ignore inactive and removable connectors
        if (display_device.StateFlags & DISPLAY_DEVICE_ACTIVE) == 0 {
            continue;
        }

        if (display_device.StateFlags & DISPLAY_DEVICE_REMOVABLE) != 0 {
            continue;
        }

        // match common integrated panel identifiers
        let identifier =
            win32_core::utf16_buffer_to_string(&display_device.DeviceID).to_uppercase();
        if identifier.contains("INTERNAL")
            || identifier.contains("EDP")
            || identifier.contains("LVDS")
            || identifier.contains("DSI")
        {
            return Ok(DisplaySupportStatus::Supported);
        }
    }

    Ok(DisplaySupportStatus::Unknown)
}

/// Resolve one effective scale factor for one monitor handle.
fn monitor_scale_factor_milli(monitor: isize) -> u32 {
    // query one per monitor effective dpi lane
    let mut dpi_x = 0u32;
    let mut dpi_y = 0u32;
    let status = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    if status != 0 || dpi_x == 0 || dpi_y == 0 {
        return 1000;
    }

    dpi_x.saturating_mul(1000).saturating_add(48) / 96
}

/// Query physical monitor metrics for one display device.
pub(crate) fn display_metrics_for_device(device_id: &str) -> RuntimeResult<(u32, u32)> {
    // open one display device context for physical metrics
    let device_wide = core_platform::wide_from_str("id", device_id)?;

    let hdc = unsafe {
        CreateDCW(
            device_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    // return conservative defaults when dc open fails
    if hdc == 0 {
        return Ok((0, 0));
    }

    // query and release win32 physical metrics
    let width_mm_raw = unsafe { GetDeviceCaps(hdc, HORZSIZE as i32) };
    let height_mm_raw = unsafe { GetDeviceCaps(hdc, VERTSIZE as i32) };
    unsafe {
        DeleteDC(hdc);
    }

    // normalize raw metrics into host-state payload
    let width_mm = width_mm_raw.max(0) as u32;
    let height_mm = height_mm_raw.max(0) as u32;

    Ok((width_mm, height_mm))
}

/// Enumerate raw monitor rows from the active desktop.
pub(crate) fn enumerate_monitor_rows() -> RuntimeResult<Vec<MonitorRow>> {
    // enumerate active desktop monitors
    let mut rows: Vec<MonitorRow> = Vec::new();
    let status = unsafe {
        EnumDisplayMonitors(
            0,
            std::ptr::null(),
            Some(enumerate_monitor_rows_callback),
            &mut rows as *mut _ as LPARAM,
        )
    };
    // fail when desktop monitor enumeration fails
    if status == 0 {
        return Err(win32_core::io_error(
            "destack.display.monitor.list",
            "EnumDisplayMonitors",
            "failed to enumerate monitors",
        ));
    }

    Ok(rows)
}
