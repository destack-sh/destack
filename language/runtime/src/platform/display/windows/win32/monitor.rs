use std::collections::HashMap;

use windows_sys::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO, DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
    DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO,
    DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, DisplayConfigGetDeviceInfo, DisplayConfigSetDeviceInfo,
    GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};
use windows_sys::Win32::Foundation::{
    BOOL, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, HWND, LPARAM, LUID, RECT,
};
use windows_sys::Win32::Graphics::Gdi::{
    CDS_FULLSCREEN, ChangeDisplaySettingsExW, CreateDCW, DEVMODEW, DISP_CHANGE_BADFLAGS,
    DISP_CHANGE_BADMODE, DISP_CHANGE_BADPARAM, DISP_CHANGE_SUCCESSFUL, DISPLAY_DEVICE_ACTIVE,
    DISPLAY_DEVICE_REMOVABLE, DISPLAY_DEVICEW, DISPLAYCONFIG_COLOR_ENCODING_RGB, DM_BITSPERPEL,
    DM_DISPLAYFREQUENCY, DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH, DMDO_90, DMDO_180,
    DMDO_270, DMDO_DEFAULT, DeleteDC, ENUM_CURRENT_SETTINGS, ENUM_REGISTRY_SETTINGS,
    EnumDisplayDevicesW, EnumDisplayMonitors, EnumDisplaySettingsExW, GetDeviceCaps,
    GetMonitorInfoW, HORZSIZE, LOGPIXELSX, MONITOR_DEFAULTTONEAREST, MONITORINFOEXW,
    MonitorFromWindow, VERTSIZE,
};
use windows_sys::Win32::UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp};
use windows_sys::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayBackend, DisplayColorSpace, DisplayColorState, DisplayDescriptor, DisplayGammaRamp,
    DisplayHdrMode, DisplayMode, DisplayMonitorListRequest, DisplayMonitorOpenOptions,
    DisplayOrientation, DisplaySupportStatus, WindowModeOptions,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef};

use super::model::{DisplayDescriptorSnapshot, MonitorSnapshot};
use super::{core, event, resource as display_resource};

/// One advanced-color flag bit indicating support.
const ADVANCED_COLOR_SUPPORTED_BIT: u32 = 0x1;
/// One advanced-color flag bit indicating active HDR output.
const ADVANCED_COLOR_ENABLED_BIT: u32 = 0x2;
/// One bit in set-advanced-color payload that enables HDR output.
const SET_ADVANCED_COLOR_ENABLE_BIT: u32 = 0x1;
/// Number of gamma entries per color channel in Win32.
const GAMMA_RAMP_CHANNEL_ENTRIES: usize = 256;
/// Total number of gamma entries in one Win32 gamma table.
const GAMMA_RAMP_TOTAL_ENTRIES: usize = GAMMA_RAMP_CHANNEL_ENTRIES * 3;
/// One raw monitor-row tuple used by monitor enumeration.
type MonitorRow = (String, RECT, RECT, bool);

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
    // evaluate this condition
    if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) } == 0 {
        return 1;
    }

    // skip rows without one concrete device id
    let device_id = core::utf16_buffer_to_string(&info.szDevice);
    // evaluate this condition
    if device_id.is_empty() {
        return 1;
    }

    // append one normalized monitor row
    let is_primary = (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0;
    output.push((
        device_id,
        info.monitorInfo.rcMonitor,
        info.monitorInfo.rcWork,
        is_primary,
    ));

    1
}

/// Build one DisplayMode payload from one DEVMODEW value.
fn display_mode_from_devmode(value: &DEVMODEW) -> DisplayMode {
    // clamp bit depth into binding representation
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
    // normalize device id into win32 wide string
    let device_wide = core_platform::wide_from_str("id", device_id)?;

    // query one indexed devmode payload
    let mut dev_mode = unsafe { std::mem::zeroed::<DEVMODEW>() };
    dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
    let status =
        unsafe { EnumDisplaySettingsExW(device_wide.as_ptr(), mode_index, &mut dev_mode, 0) };
    // evaluate this condition
    if status == 0 {
        return Ok(None);
    }

    Ok(Some(dev_mode))
}

/// Resolve one display orientation payload from one raw DEVMODE value.
fn orientation_from_devmode(value: &DEVMODEW) -> DisplayOrientation {
    // infer orientation from dimensions when the flag is absent
    let has_orientation = (value.dmFields & DM_DISPLAYORIENTATION) != 0;
    // evaluate this condition
    if !has_orientation {
        return if value.dmPelsWidth >= value.dmPelsHeight {
            DisplayOrientation::Landscape
        } else {
            DisplayOrientation::Portrait
        };
    }

    // map explicit win32 orientation values
    let orientation = unsafe { value.Anonymous1.Anonymous2.dmDisplayOrientation };
    // evaluate this condition
    if orientation == DMDO_DEFAULT {
        return if value.dmPelsWidth >= value.dmPelsHeight {
            DisplayOrientation::Landscape
        } else {
            DisplayOrientation::Portrait
        };
    }

    // evaluate this condition
    if orientation == DMDO_90 {
        return DisplayOrientation::Portrait;
    }

    // evaluate this condition
    if orientation == DMDO_180 {
        return DisplayOrientation::LandscapeFlipped;
    }

    // evaluate this condition
    if orientation == DMDO_270 {
        return DisplayOrientation::PortraitFlipped;
    }

    DisplayOrientation::Unknown
}

/// Build one fallback mode payload from one monitor rectangle.
fn display_mode_from_rect(rect: RECT) -> DisplayMode {
    // derive one fallback mode from monitor rectangle bounds
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
    // enumerate unique indexed display modes
    let mut modes = Vec::new();
    let mut mode_index = 0u32;

    // iterate while this condition holds
    while let Some(mode) = query_display_mode(device_id, mode_index)? {
        mode_index = mode_index.saturating_add(1);

        // evaluate this condition
        if modes.contains(&mode) {
            continue;
        }

        modes.push(mode);
    }

    // fall back to current mode when indexed enumeration is empty
    if modes.is_empty()
        && let Some(current_mode) = query_display_mode(device_id, ENUM_CURRENT_SETTINGS)?
    {
        modes.push(current_mode);
    }

    Ok(modes)
}

/// Query one monitor display-name string from one monitor device id.
fn display_name_for_device(device_id: &str) -> RuntimeResult<Option<String>> {
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
    let value = core::utf16_buffer_to_string(&display_device.DeviceString);
    // evaluate this condition
    if value.is_empty() {
        return Ok(None);
    }

    Ok(Some(value))
}

/// Resolve whether one display likely maps to one built-in panel.
fn display_is_builtin(device_id: &str) -> RuntimeResult<bool> {
    // iterate display devices attached to one monitor row
    let device_wide = core_platform::wide_from_str("id", device_id)?;
    let mut index = 0u32;
    // loop until one branch exits
    loop {
        let mut display_device = unsafe { std::mem::zeroed::<DISPLAY_DEVICEW>() };
        display_device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        let status =
            unsafe { EnumDisplayDevicesW(device_wide.as_ptr(), index, &mut display_device, 0) };
        // evaluate this condition
        if status == 0 {
            break;
        }

        index = index.saturating_add(1);
        // ignore inactive and removable connectors
        if (display_device.StateFlags & DISPLAY_DEVICE_ACTIVE) == 0 {
            continue;
        }

        // evaluate this condition
        if (display_device.StateFlags & DISPLAY_DEVICE_REMOVABLE) != 0 {
            continue;
        }

        // match common integrated panel identifiers
        let identifier = core::utf16_buffer_to_string(&display_device.DeviceID).to_uppercase();
        // evaluate this condition
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
        return Ok((0, 0, 1000));
    }

    // query and release win32 physical metrics
    let width_mm_raw = unsafe { GetDeviceCaps(hdc, HORZSIZE as i32) };
    let height_mm_raw = unsafe { GetDeviceCaps(hdc, VERTSIZE as i32) };
    let dpi_x = unsafe { GetDeviceCaps(hdc, LOGPIXELSX as i32) };

    unsafe {
        DeleteDC(hdc);
    }

    // normalize raw metrics into binding payload
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

/// Resolve one source-device name from one active display-config path.
fn source_name_for_path(path: &DISPLAYCONFIG_PATH_INFO) -> Option<String> {
    // query source display name for one active path
    let mut source_name = unsafe { std::mem::zeroed::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() };
    source_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
    source_name.header.size = std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
    source_name.header.adapterId = path.sourceInfo.adapterId;
    source_name.header.id = path.sourceInfo.id;

    let status = unsafe { DisplayConfigGetDeviceInfo(&mut source_name.header) };
    // reject paths that fail source query
    if status != ERROR_SUCCESS as i32 {
        return None;
    }

    // normalize empty source names to none
    let value = core::utf16_buffer_to_string(&source_name.viewGdiDeviceName);
    // evaluate this condition
    if value.is_empty() {
        return None;
    }

    Some(value)
}

/// Resolve whether one display-config target path reports hdr capability.
fn target_supports_hdr(path: &DISPLAYCONFIG_PATH_INFO) -> Option<bool> {
    // query advanced color info for one target path
    let mut hdr_info = unsafe { std::mem::zeroed::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() };
    hdr_info.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO;
    hdr_info.header.size = std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32;
    hdr_info.header.adapterId = path.targetInfo.adapterId;
    hdr_info.header.id = path.targetInfo.id;

    let status = unsafe { DisplayConfigGetDeviceInfo(&mut hdr_info.header) };
    // reject paths that do not expose advanced color info
    if status != ERROR_SUCCESS as i32 {
        return None;
    }

    let flags = unsafe { hdr_info.Anonymous.value };
    Some((flags & 0x1) != 0)
}

/// Build one hdr-support map keyed by uppercased display identifier.
fn display_hdr_support_map() -> HashMap<String, bool> {
    // query active display config path table with one resize retry
    let mut supports_hdr_by_device_id = HashMap::new();
    let mut attempt_count = 0u8;
    // loop until one branch exits
    loop {
        let mut path_count = 0u32;
        let mut mode_count = 0u32;

        let count_status = unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        };
        // stop when host cannot provide display config sizes
        if count_status != ERROR_SUCCESS {
            return supports_hdr_by_device_id;
        }

        let mut paths =
            vec![unsafe { std::mem::zeroed::<DISPLAYCONFIG_PATH_INFO>() }; path_count as usize];
        let mut modes = vec![
            unsafe {
                std::mem::zeroed::<windows_sys::Win32::Devices::Display::DISPLAYCONFIG_MODE_INFO>()
            };
            mode_count as usize
        ];
        let query_status = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };
        // retry once on buffer growth races
        if query_status == ERROR_INSUFFICIENT_BUFFER && attempt_count == 0 {
            attempt_count = 1;
            continue;
        }

        // stop on query failures after retry
        if query_status != ERROR_SUCCESS {
            return supports_hdr_by_device_id;
        }

        // merge per path hdr support by source display id
        for path in paths.iter().take(path_count as usize) {
            let Some(source_name) = source_name_for_path(path) else {
                continue;
            };
            let Some(supports_hdr) = target_supports_hdr(path) else {
                continue;
            };
            let key = source_name.to_uppercase();
            supports_hdr_by_device_id
                .entry(key)
                .and_modify(|value| *value |= supports_hdr)
                .or_insert(supports_hdr);
        }

        return supports_hdr_by_device_id;
    }
}

/// Resolve active display-config paths for one snapshot query.
fn active_display_paths() -> Option<Vec<DISPLAYCONFIG_PATH_INFO>> {
    // query active path table with one resize retry
    let mut attempt_count = 0u8;
    // loop until one branch exits
    loop {
        let mut path_count = 0u32;
        let mut mode_count = 0u32;
        let count_status = unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        };
        // stop when host cannot provide display config sizes
        if count_status != ERROR_SUCCESS {
            return None;
        }

        let mut paths =
            vec![unsafe { std::mem::zeroed::<DISPLAYCONFIG_PATH_INFO>() }; path_count as usize];
        let mut modes = vec![
            unsafe {
                std::mem::zeroed::<windows_sys::Win32::Devices::Display::DISPLAYCONFIG_MODE_INFO>()
            };
            mode_count as usize
        ];
        let query_status = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };
        // retry once on buffer growth races
        if query_status == ERROR_INSUFFICIENT_BUFFER && attempt_count == 0 {
            attempt_count = 1;
            continue;
        }

        // return none after failed query
        if query_status != ERROR_SUCCESS {
            return None;
        }

        // truncate to reported path count
        paths.truncate(path_count as usize);
        return Some(paths);
    }
}

/// Resolve one active target tuple and advanced-color info for one display id.
fn advanced_color_target_for_device(
    device_id: &str,
) -> Option<(LUID, u32, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO)> {
    let paths = active_display_paths()?;
    let target_id = device_id.to_uppercase();

    // iterate this sequence
    for path in &paths {
        let Some(source_name) = source_name_for_path(path) else {
            continue;
        };
        // evaluate this condition
        if source_name.to_uppercase() != target_id {
            continue;
        }

        let mut hdr_info = unsafe { std::mem::zeroed::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() };
        hdr_info.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO;
        hdr_info.header.size = std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32;
        hdr_info.header.adapterId = path.targetInfo.adapterId;
        hdr_info.header.id = path.targetInfo.id;
        let status = unsafe { DisplayConfigGetDeviceInfo(&mut hdr_info.header) };
        // evaluate this condition
        if status != ERROR_SUCCESS as i32 {
            return None;
        }

        return Some((path.targetInfo.adapterId, path.targetInfo.id, hdr_info));
    }

    None
}

/// Resolve one color-space value from one advanced-color payload.
fn color_space_from_advanced_info(
    info: &DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO,
    hdr_enabled: bool,
) -> DisplayColorSpace {
    // evaluate this condition
    if hdr_enabled {
        return DisplayColorSpace::Hdr10;
    }

    // evaluate this condition
    if info.colorEncoding == DISPLAYCONFIG_COLOR_ENCODING_RGB {
        return DisplayColorSpace::Srgb;
    }

    DisplayColorSpace::Unknown
}

/// Resolve one color-state payload for one display id.
fn monitor_color_state_by_id(device_id: &str) -> DisplayColorState {
    // return unknown state when advanced color query is unavailable
    let Some((_, _, info)) = advanced_color_target_for_device(device_id) else {
        return DisplayColorState {
            hdr_mode: DisplayHdrMode::Unknown,
            color_space: DisplayColorSpace::Unknown,
            bits_per_channel: None,
        };
    };

    // map advanced color flags to binding color state
    let flags = unsafe { info.Anonymous.value };
    let hdr_supported = (flags & ADVANCED_COLOR_SUPPORTED_BIT) != 0;
    let hdr_enabled = (flags & ADVANCED_COLOR_ENABLED_BIT) != 0;
    let hdr_mode = if hdr_supported && hdr_enabled {
        DisplayHdrMode::Hdr
    } else {
        DisplayHdrMode::Sdr
    };
    let bits_per_channel = if info.bitsPerColorChannel == 0 {
        None
    } else {
        Some(info.bitsPerColorChannel.min(u16::MAX as u32) as u16)
    };

    DisplayColorState {
        hdr_mode,
        color_space: color_space_from_advanced_info(&info, hdr_enabled),
        bits_per_channel,
    }
}

/// Apply one HDR enable state for one display id.
fn set_monitor_hdr_enabled_by_id(
    device_id: &str,
    enabled: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some((adapter_id, target_id, info)) = advanced_color_target_for_device(device_id) else {
        return Err(core_platform::not_supported(operation));
    };

    let flags = unsafe { info.Anonymous.value };
    let hdr_supported = (flags & ADVANCED_COLOR_SUPPORTED_BIT) != 0;
    // evaluate this condition
    if enabled && !hdr_supported {
        return Err(core_platform::invalid_argument(
            "mode",
            "display does not report HDR support",
        ));
    }
    // evaluate this condition
    if !hdr_supported {
        return Ok(());
    }

    let mut state = unsafe { std::mem::zeroed::<DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE>() };
    state.header.r#type = DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE;
    state.header.size = std::mem::size_of::<DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE>() as u32;
    state.header.adapterId = adapter_id;
    state.header.id = target_id;
    state.Anonymous.value = if enabled {
        SET_ADVANCED_COLOR_ENABLE_BIT
    } else {
        0
    };

    let status = unsafe { DisplayConfigSetDeviceInfo(&state.header) };
    // evaluate this condition
    if status == ERROR_SUCCESS as i32 {
        return Ok(());
    }

    Err(core::io_error_with_code(
        operation,
        "DisplayConfigSetDeviceInfo",
        status as u32,
        "failed to apply hdr mode",
    ))
}

/// Query one Win32 gamma-ramp payload from one display id.
fn read_monitor_gamma_ramp_by_id(
    device_id: &str,
    operation: &'static str,
) -> RuntimeResult<[u16; GAMMA_RAMP_TOTAL_ENTRIES]> {
    let device_wide = core_platform::wide_from_str("id", device_id)?;
    let hdc = unsafe {
        CreateDCW(
            device_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    // evaluate this condition
    if hdc == 0 {
        return Err(core::io_error(
            operation,
            "CreateDCW",
            "failed to open display device",
        ));
    }

    let mut gamma = [0u16; GAMMA_RAMP_TOTAL_ENTRIES];
    let status = unsafe { GetDeviceGammaRamp(hdc, gamma.as_mut_ptr().cast()) };
    unsafe {
        DeleteDC(hdc);
    }
    // evaluate this condition
    if status != 0 {
        return Ok(gamma);
    }

    let code = core_platform::last_error_code() as u32;
    // evaluate this condition
    if code == 0 {
        return Err(core_platform::not_supported(operation));
    }

    Err(core::io_error_with_code(
        operation,
        "GetDeviceGammaRamp",
        code,
        "failed to query display gamma ramp",
    ))
}

/// Apply one Win32 gamma-ramp payload to one display id.
fn write_monitor_gamma_ramp_by_id(
    device_id: &str,
    gamma: &[u16; GAMMA_RAMP_TOTAL_ENTRIES],
    operation: &'static str,
) -> RuntimeResult<()> {
    let device_wide = core_platform::wide_from_str("id", device_id)?;
    let hdc = unsafe {
        CreateDCW(
            device_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    // evaluate this condition
    if hdc == 0 {
        return Err(core::io_error(
            operation,
            "CreateDCW",
            "failed to open display device",
        ));
    }

    let status = unsafe { SetDeviceGammaRamp(hdc, gamma.as_ptr().cast()) };
    unsafe {
        DeleteDC(hdc);
    }
    // evaluate this condition
    if status != 0 {
        return Ok(());
    }

    let code = core_platform::last_error_code() as u32;
    // evaluate this condition
    if code == 0 {
        return Err(core_platform::not_supported(operation));
    }

    Err(core::io_error_with_code(
        operation,
        "SetDeviceGammaRamp",
        code,
        "failed to apply display gamma ramp",
    ))
}

/// Enumerate raw monitor rows from the active desktop.
fn enumerate_monitor_rows() -> RuntimeResult<Vec<MonitorRow>> {
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
    // evaluate this condition
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
    // query raw monitor rows and hdr support map
    let rows = enumerate_monitor_rows()?;
    let supports_hdr_by_device_id = display_hdr_support_map();
    let mut snapshots = Vec::with_capacity(rows.len());

    // build one snapshot per monitor row
    for (device_id, bounds, work_area, primary) in rows {
        let name = display_name_for_device(&device_id)?.unwrap_or_else(|| device_id.clone());
        let current_mode = query_display_mode(&device_id, ENUM_CURRENT_SETTINGS)?
            .unwrap_or_else(|| display_mode_from_rect(bounds));
        let orientation = query_raw_devmode(&device_id, ENUM_CURRENT_SETTINGS)?
            .map(|mode| orientation_from_devmode(&mode))
            .unwrap_or_else(|| {
                // evaluate this condition
                if current_mode.width >= current_mode.height {
                    DisplayOrientation::Landscape
                } else {
                    DisplayOrientation::Portrait
                }
            });
        let desktop_mode =
            query_display_mode(&device_id, ENUM_REGISTRY_SETTINGS)?.unwrap_or(current_mode);
        let mut modes = enumerate_display_modes(&device_id)?;
        // evaluate this condition
        if !modes.contains(&current_mode) {
            modes.push(current_mode);
        }
        // evaluate this condition
        if !modes.contains(&desktop_mode) {
            modes.push(desktop_mode);
        }

        // resolve descriptor support lanes and geometry
        let (width_mm, height_mm, scale_factor_milli) = display_metrics_for_device(&device_id)?;
        let is_builtin = display_is_builtin(&device_id)?;
        let width_px = (bounds.right - bounds.left).max(1) as u32;
        let height_px = (bounds.bottom - bounds.top).max(1) as u32;
        let work_area_width_px = (work_area.right - work_area.left).max(1) as u32;
        let work_area_height_px = (work_area.bottom - work_area.top).max(1) as u32;

        let hdr_key = device_id.to_uppercase();
        let supports_hdr = supports_hdr_by_device_id
            .get(&hdr_key)
            .copied()
            .unwrap_or(false);
        // materialize descriptor payload
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
            builtin_panel: if is_builtin {
                DisplaySupportStatus::Supported
            } else {
                DisplaySupportStatus::Unsupported
            },
            variable_refresh_support: DisplaySupportStatus::Unknown,
            hdr_support: if supports_hdr {
                DisplaySupportStatus::Supported
            } else {
                DisplaySupportStatus::Unsupported
            },
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
pub(super) fn monitor_snapshot_by_id(id: &str) -> RuntimeResult<Option<MonitorSnapshot>> {
    let snapshots = enumerate_monitor_snapshots()?;
    Ok(snapshots
        .into_iter()
        .find(|snapshot| snapshot.descriptor.id == id))
}

/// Convert one owned descriptor payload into one ABI descriptor payload.
pub(super) fn descriptor_from_owned(
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
fn monitor_snapshot_for_handle(
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

/// Apply one monitor mode through ChangeDisplaySettingsExW.
pub(super) fn apply_monitor_mode_by_id(
    id: &str,
    mode: DisplayMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    // evaluate this condition
    if mode.width == 0 || mode.height == 0 {
        return Err(core_platform::invalid_argument(
            "mode",
            "mode dimensions must be greater than zero",
        ));
    }

    let device_wide = core_platform::wide_from_str("id", id)?;

    let mut dev_mode = unsafe { std::mem::zeroed::<DEVMODEW>() };
    dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
    dev_mode.dmPelsWidth = mode.width;
    dev_mode.dmPelsHeight = mode.height;
    dev_mode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;

    // only set bit depth when the caller provided one explicit value
    if mode.bit_depth > 0 {
        dev_mode.dmBitsPerPel = u32::from(mode.bit_depth);
        dev_mode.dmFields |= DM_BITSPERPEL;
    }

    // only set refresh when the caller provided one explicit value
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

    // evaluate this condition
    if result == DISP_CHANGE_SUCCESSFUL {
        return Ok(());
    }

    // evaluate this condition
    if result == DISP_CHANGE_BADMODE {
        return Err(core_platform::invalid_argument(
            "mode",
            "requested display mode is not supported",
        ));
    }

    // evaluate this condition
    if result == DISP_CHANGE_BADPARAM || result == DISP_CHANGE_BADFLAGS {
        return Err(core_platform::invalid_argument(
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
    hwnd: HWND,
    operation: &'static str,
) -> RuntimeResult<Option<RECT>> {
    // windowed mode does not target one monitor rectangle
    if matches!(mode, WindowModeOptions::WindowWindowedModeOptions(_)) {
        return Ok(None);
    }

    // resolve borderless mode without explicit display from current window monitor
    if let WindowModeOptions::WindowBorderlessModeOptions(options) = mode
        && options.display.is_none()
        && display.is_none()
    {
        let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
        // evaluate this condition
        if monitor == 0 {
            return Ok(None);
        }

        let mut info = unsafe { std::mem::zeroed::<MONITORINFOEXW>() };
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        // evaluate this condition
        if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) } == 0 {
            return Err(core::io_error(
                operation,
                "GetMonitorInfoW",
                "failed to resolve monitor target rectangle",
            ));
        }

        return Ok(Some(info.monitorInfo.rcMonitor));
    }

    // resolve explicit display target for fullscreen/borderless placement
    let display = match mode {
        WindowModeOptions::WindowWindowedModeOptions(_) => return Ok(None),
        WindowModeOptions::WindowBorderlessModeOptions(options) => options.display.or(display),
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(options) => Some(options.display),
    }
    .ok_or_else(|| {
        core_platform::invalid_argument(
            "mode.display",
            "fullscreen mode requires one display target",
        )
    })?;

    let id = display_resource::resolve_display_id(context, display, operation)?;
    let snapshot = monitor_snapshot_by_id(&id)?.ok_or_else(|| {
        core_platform::io_not_found(
            operation,
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

/// List available displays.
pub(crate) unsafe fn monitor_list(
    context: &BindingCallContext,
    out: *mut NativeSlice<DisplayDescriptor>,
    _request: DisplayMonitorListRequest,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let descriptors = enumerate_monitor_snapshots()?
        .into_iter()
        .map(|snapshot| descriptor_from_owned(context, &snapshot.descriptor))
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
        .runtime()
        .resources
        .remove(handle.0, Some(context.engine()))
        .is_some();
    // evaluate this condition
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

/// Read the desktop-preferred mode for one opened display.
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
    // evaluate this condition
    if snapshot.modes.is_empty() {
        return Err(core_platform::io_not_found(
            "destack.display.monitor.closestMode",
            "display mode table is empty",
        ));
    }

    let mut best = snapshot.modes[0];
    let mut best_score = u64::MAX;
    // iterate this sequence
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

        // evaluate this condition
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
    // evaluate this condition
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

/// Read one monitor color-state snapshot.
pub(crate) unsafe fn monitor_color_state(
    context: &BindingCallContext,
    out: *mut DisplayColorState,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let id = display_resource::resolve_display_id(
        context,
        handle,
        "destack.display.monitor.colorState",
    )?;
    let state = monitor_color_state_by_id(&id);
    unsafe {
        *out = state;
    }

    Ok(())
}

/// Read one monitor hdr-mode value.
pub(crate) unsafe fn monitor_hdr_mode(
    context: &BindingCallContext,
    out: *mut DisplayHdrMode,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let id =
        display_resource::resolve_display_id(context, handle, "destack.display.monitor.hdrMode")?;
    let state = monitor_color_state_by_id(&id);
    unsafe {
        *out = state.hdr_mode;
    }

    Ok(())
}

/// Set one monitor hdr-mode value.
pub(crate) unsafe fn monitor_set_hdr_mode(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    mode: DisplayHdrMode,
) -> RuntimeResult<()> {
    let id = display_resource::resolve_display_id(
        context,
        handle,
        "destack.display.monitor.setHdrMode",
    )?;

    // evaluate this condition
    if mode == DisplayHdrMode::Unknown {
        return Err(core_platform::invalid_argument(
            "mode",
            "hdr mode must be one concrete mode",
        ));
    }
    // evaluate this condition
    if mode == DisplayHdrMode::System {
        return Ok(());
    }

    let enable_hdr = mode == DisplayHdrMode::Hdr;
    set_monitor_hdr_enabled_by_id(&id, enable_hdr, "destack.display.monitor.setHdrMode")?;
    event::refresh_monitor_topology_cache(context)?;

    Ok(())
}

/// Read one monitor gamma-ramp payload.
pub(crate) unsafe fn monitor_gamma_ramp(
    context: &BindingCallContext,
    out: *mut DisplayGammaRamp,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let id =
        display_resource::resolve_display_id(context, handle, "destack.display.monitor.gammaRamp")?;
    let gamma = read_monitor_gamma_ramp_by_id(&id, "destack.display.monitor.gammaRamp")?;
    let red = context.store_slice(gamma[0..GAMMA_RAMP_CHANNEL_ENTRIES].to_vec());
    let green = context
        .store_slice(gamma[GAMMA_RAMP_CHANNEL_ENTRIES..(GAMMA_RAMP_CHANNEL_ENTRIES * 2)].to_vec());
    let blue = context
        .store_slice(gamma[(GAMMA_RAMP_CHANNEL_ENTRIES * 2)..GAMMA_RAMP_TOTAL_ENTRIES].to_vec());
    unsafe {
        *out = DisplayGammaRamp { red, green, blue };
    }

    Ok(())
}

/// Set one monitor gamma-ramp payload.
pub(crate) unsafe fn monitor_set_gamma_ramp(
    context: &BindingCallContext,
    handle: resource::DisplayHandle,
    ramp: DisplayGammaRamp,
) -> RuntimeResult<()> {
    let id = display_resource::resolve_display_id(
        context,
        handle,
        "destack.display.monitor.setGammaRamp",
    )?;
    let red = unsafe { ramp.red.as_slice()? };
    let green = unsafe { ramp.green.as_slice()? };
    let blue = unsafe { ramp.blue.as_slice()? };
    // evaluate this condition
    if red.len() != GAMMA_RAMP_CHANNEL_ENTRIES
        || green.len() != GAMMA_RAMP_CHANNEL_ENTRIES
        || blue.len() != GAMMA_RAMP_CHANNEL_ENTRIES
    {
        return Err(core_platform::invalid_argument(
            "ramp",
            "gamma ramp channels must each contain exactly 256 entries",
        ));
    }

    let mut gamma = [0u16; GAMMA_RAMP_TOTAL_ENTRIES];
    gamma[0..GAMMA_RAMP_CHANNEL_ENTRIES].copy_from_slice(red);
    gamma[GAMMA_RAMP_CHANNEL_ENTRIES..(GAMMA_RAMP_CHANNEL_ENTRIES * 2)].copy_from_slice(green);
    gamma[(GAMMA_RAMP_CHANNEL_ENTRIES * 2)..GAMMA_RAMP_TOTAL_ENTRIES].copy_from_slice(blue);

    write_monitor_gamma_ramp_by_id(&id, &gamma, "destack.display.monitor.setGammaRamp")
}
