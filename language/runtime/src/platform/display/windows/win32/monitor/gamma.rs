use std::collections::HashMap;

use windows_sys::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO, DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
    DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO,
    DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, DisplayConfigGetDeviceInfo, DisplayConfigSetDeviceInfo,
    GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};
use windows_sys::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, LUID};
use windows_sys::Win32::Graphics::Gdi::{CreateDCW, DISPLAYCONFIG_COLOR_ENCODING_RGB, DeleteDC};
use windows_sys::Win32::UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayColorSpace, DisplayColorState, DisplayGammaRamp, DisplayHdrMode,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{GAMMA_RAMP_CHANNEL_ENTRIES, GAMMA_RAMP_TOTAL_ENTRIES};
use crate::platform::display::windows::win32::{
    core as win32_core, event, resource as display_resource,
};

/// One advanced-color flag bit indicating support.
const ADVANCED_COLOR_SUPPORTED_BIT: u32 = 0x1;
/// One advanced-color flag bit indicating active HDR output.
const ADVANCED_COLOR_ENABLED_BIT: u32 = 0x2;
/// One bit in set-advanced-color payload that enables HDR output.
const SET_ADVANCED_COLOR_ENABLE_BIT: u32 = 0x1;

/// Resolve one source-device name from one active display-config path.
fn source_name_for_path(path: &DISPLAYCONFIG_PATH_INFO) -> Option<String> {
    // query source display name for one active path
    let mut source_name = unsafe { std::mem::zeroed::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() };
    source_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
    source_name.header.size = std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
    source_name.header.adapterId = path.sourceInfo.adapterId;
    source_name.header.id = path.sourceInfo.id;

    let status = unsafe { DisplayConfigGetDeviceInfo(&mut source_name.header) };
    if status != ERROR_SUCCESS as i32 {
        return None;
    }

    let value = win32_core::utf16_buffer_to_string(&source_name.viewGdiDeviceName);
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
    if status != ERROR_SUCCESS as i32 {
        return None;
    }

    let flags = unsafe { hdr_info.Anonymous.value };
    Some((flags & ADVANCED_COLOR_SUPPORTED_BIT) != 0)
}

/// Build one hdr-support map keyed by uppercased display identifier.
pub(crate) fn display_hdr_support_map() -> HashMap<String, bool> {
    // query active display config path table with one resize retry
    let mut supports_hdr_by_device_id = HashMap::new();
    let mut attempt_count = 0u8;

    loop {
        let mut path_count = 0u32;
        let mut mode_count = 0u32;

        let count_status = unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        };
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

    loop {
        let mut path_count = 0u32;
        let mut mode_count = 0u32;
        let count_status = unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        };
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

        if query_status != ERROR_SUCCESS {
            return None;
        }

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

    for path in &paths {
        let Some(source_name) = source_name_for_path(path) else {
            continue;
        };
        if source_name.to_uppercase() != target_id {
            continue;
        }

        let mut hdr_info = unsafe { std::mem::zeroed::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() };
        hdr_info.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO;
        hdr_info.header.size = std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32;
        hdr_info.header.adapterId = path.targetInfo.adapterId;
        hdr_info.header.id = path.targetInfo.id;
        let status = unsafe { DisplayConfigGetDeviceInfo(&mut hdr_info.header) };
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
    if hdr_enabled {
        return DisplayColorSpace::Hdr10;
    }

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

    // map advanced color flags to color state
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
    if enabled && !hdr_supported {
        return Err(core_platform::invalid_argument(
            "mode",
            "display does not report HDR support",
        ));
    }
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
    if status == ERROR_SUCCESS as i32 {
        return Ok(());
    }

    Err(win32_core::io_error_with_code(
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
    if hdc == 0 {
        return Err(win32_core::io_error(
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
    if status != 0 {
        return Ok(gamma);
    }

    let code = core_platform::last_error_code() as u32;
    if code == 0 {
        return Err(core_platform::not_supported(operation));
    }

    Err(win32_core::io_error_with_code(
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
    if hdc == 0 {
        return Err(win32_core::io_error(
            operation,
            "CreateDCW",
            "failed to open display device",
        ));
    }

    let status = unsafe { SetDeviceGammaRamp(hdc, gamma.as_ptr().cast()) };
    unsafe {
        DeleteDC(hdc);
    }
    if status != 0 {
        return Ok(());
    }

    let code = core_platform::last_error_code() as u32;
    if code == 0 {
        return Err(core_platform::not_supported(operation));
    }

    Err(win32_core::io_error_with_code(
        operation,
        "SetDeviceGammaRamp",
        code,
        "failed to apply display gamma ramp",
    ))
}

/// Read one monitor color state snapshot.
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

/// Read one monitor HDR mode value.
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

/// Set one monitor HDR mode value.
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
    if mode == DisplayHdrMode::Unknown {
        return Err(core_platform::invalid_argument(
            "mode",
            "hdr mode must be one concrete mode",
        ));
    }
    if mode == DisplayHdrMode::System {
        return Ok(());
    }

    let enable_hdr = mode == DisplayHdrMode::Hdr;
    set_monitor_hdr_enabled_by_id(&id, enable_hdr, "destack.display.monitor.setHdrMode")?;
    event::refresh_monitor_topology_cache(context)?;

    Ok(())
}

/// Read one monitor gamma ramp payload.
pub(crate) unsafe fn monitor_gamma_ramp(
    context: &BindingCallContext,
    out: *mut DisplayGammaRamp,
    handle: resource::DisplayHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let id =
        display_resource::resolve_display_id(context, handle, "destack.display.monitor.gammaRamp")?;
    let gamma = read_monitor_gamma_ramp_by_id(&id, "destack.display.monitor.gammaRamp")?;
    let red = context.store_slice_copy(&gamma[0..GAMMA_RAMP_CHANNEL_ENTRIES]);
    let green = context
        .store_slice_copy(&gamma[GAMMA_RAMP_CHANNEL_ENTRIES..(GAMMA_RAMP_CHANNEL_ENTRIES * 2)]);
    let blue = context
        .store_slice_copy(&gamma[(GAMMA_RAMP_CHANNEL_ENTRIES * 2)..GAMMA_RAMP_TOTAL_ENTRIES]);
    unsafe {
        *out = DisplayGammaRamp { red, green, blue };
    }

    Ok(())
}

/// Set one monitor gamma ramp payload.
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
