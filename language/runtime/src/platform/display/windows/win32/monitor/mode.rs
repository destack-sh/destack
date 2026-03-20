use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    CDS_FULLSCREEN, ChangeDisplaySettingsExW, DEVMODEW, DISP_CHANGE_BADFLAGS, DISP_CHANGE_BADMODE,
    DISP_CHANGE_BADPARAM, DISP_CHANGE_SUCCESSFUL, DM_BITSPERPEL, DM_DISPLAYFREQUENCY,
    DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH, DMDO_90, DMDO_180, DMDO_270, DMDO_DEFAULT,
    ENUM_CURRENT_SETTINGS, EnumDisplaySettingsExW, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST,
    MONITORINFOEXW, MonitorFromWindow,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayMode, DisplayOrientation, DisplayPixelFormat, WindowModeOptions,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::snapshot::monitor_snapshot_by_id;
use crate::platform::display::windows::win32::{
    core as display_core, resource as display_resource,
};

/// Build one DisplayMode payload from one DEVMODEW value.
pub(crate) fn display_mode_from_devmode(value: &DEVMODEW) -> DisplayMode {
    // clamp bit depth into the runtime representation
    let bit_depth = if value.dmBitsPerPel > u16::MAX as u32 {
        u16::MAX
    } else {
        value.dmBitsPerPel as u16
    };

    DisplayMode {
        width: value.dmPelsWidth,
        height: value.dmPelsHeight,
        refresh_milli_hz: value.dmDisplayFrequency.saturating_mul(1000),
        format: DisplayPixelFormat(0),
        bit_depth,
    }
}

/// Query one raw DEVMODE payload from one monitor device and mode selector.
pub(crate) fn query_raw_devmode(
    device_id: &str,
    mode_index: u32,
) -> RuntimeResult<Option<DEVMODEW>> {
    // normalize device id into win32 wide string
    let device_wide = core_platform::wide_from_str("id", device_id)?;

    // query one indexed devmode payload
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
pub(crate) fn orientation_from_devmode(value: &DEVMODEW) -> DisplayOrientation {
    // infer orientation from dimensions when the flag is absent
    let has_orientation = (value.dmFields & DM_DISPLAYORIENTATION) != 0;
    if !has_orientation {
        return if value.dmPelsWidth >= value.dmPelsHeight {
            DisplayOrientation::Landscape
        } else {
            DisplayOrientation::Portrait
        };
    }

    // map explicit win32 orientation values
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

/// Query one display mode from one monitor device and mode selector.
pub(crate) fn query_display_mode(
    device_id: &str,
    mode_index: u32,
) -> RuntimeResult<Option<DisplayMode>> {
    let Some(dev_mode) = query_raw_devmode(device_id, mode_index)? else {
        return Ok(None);
    };
    Ok(Some(display_mode_from_devmode(&dev_mode)))
}

/// Enumerate all supported display modes for one monitor device id.
pub(crate) fn enumerate_display_modes(device_id: &str) -> RuntimeResult<Vec<DisplayMode>> {
    // enumerate unique indexed display modes
    let mut modes = Vec::new();
    let mut mode_index = 0u32;

    // iterate while display settings enumeration still returns rows
    while let Some(mode) = query_display_mode(device_id, mode_index)? {
        mode_index = mode_index.saturating_add(1);

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

/// Apply one monitor mode through ChangeDisplaySettingsExW.
pub(crate) fn apply_monitor_mode_by_id(
    id: &str,
    mode: DisplayMode,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject zero sized mode requests
    if mode.width == 0 || mode.height == 0 {
        return Err(core_platform::invalid_argument(
            "mode",
            "mode dimensions must be greater than zero",
        ));
    }

    // encode the requested devmode payload
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

    // apply one fullscreen display configuration change
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

    // map well known configuration failures first
    if result == DISP_CHANGE_BADMODE {
        return Err(core_platform::invalid_argument(
            "mode",
            "requested display mode is not supported",
        ));
    }
    if result == DISP_CHANGE_BADPARAM || result == DISP_CHANGE_BADFLAGS {
        return Err(core_platform::invalid_argument(
            "mode",
            "requested display mode parameters are invalid",
        ));
    }

    Err(display_core::io_error_with_code(
        operation,
        "ChangeDisplaySettingsExW",
        result as u32,
        "failed to apply display mode",
    ))
}

/// Resolve one monitor target rectangle for one mode transition.
pub(crate) fn mode_target_rect(
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

    // resolve borderless mode without explicit display from the current window monitor
    if let WindowModeOptions::WindowBorderlessModeOptions(options) = mode
        && options.display.is_none()
        && display.is_none()
    {
        let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
        if monitor == 0 {
            return Ok(None);
        }

        let mut info = unsafe { std::mem::zeroed::<MONITORINFOEXW>() };
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) } == 0 {
            return Err(display_core::io_error(
                operation,
                "GetMonitorInfoW",
                "failed to resolve monitor target rectangle",
            ));
        }

        return Ok(Some(info.monitorInfo.rcMonitor));
    }

    // resolve explicit display target for fullscreen and borderless placement
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
