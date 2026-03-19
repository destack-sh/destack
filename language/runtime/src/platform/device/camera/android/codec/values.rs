use super::*;

/// Decode one camera facing-mode code.
fn decode_facing_mode(code: u32, operation: &'static str) -> RuntimeResult<CameraFacingMode> {
    match code {
        1 => Ok(CameraFacingMode::Unknown),
        2 => Ok(CameraFacingMode::User),
        3 => Ok(CameraFacingMode::Environment),
        4 => Ok(CameraFacingMode::Left),
        5 => Ok(CameraFacingMode::Right),
        6 => Ok(CameraFacingMode::External),
        _ => Err(invalid_data(
            operation,
            format!("android host returned one unknown camera facing mode code {code}"),
        )),
    }
}

/// Decode one camera pixel-format code.
pub(crate) fn decode_pixel_format(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<CameraPixelFormat> {
    match code {
        1 => Ok(CameraPixelFormat::Bgra8),
        2 => Ok(CameraPixelFormat::Rgba8),
        3 => Ok(CameraPixelFormat::Yuv420),
        4 => Ok(CameraPixelFormat::Jpeg),
        _ => Err(invalid_data(
            operation,
            format!("android host returned one unknown camera pixel-format code {code}"),
        )),
    }
}

/// Decode one camera pixel-format-family code.
pub(crate) fn decode_pixel_format_family(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<CameraPixelFormatFamily> {
    match code {
        1 => Ok(CameraPixelFormatFamily::PackedRgb),
        2 => Ok(CameraPixelFormatFamily::PlanarYuv),
        3 => Ok(CameraPixelFormatFamily::Encoded),
        _ => Err(invalid_data(
            operation,
            format!("android host returned one unknown camera pixel-format-family code {code}"),
        )),
    }
}

/// Decode one camera color-space code.
pub(crate) fn decode_color_space(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<CameraColorSpace> {
    match code {
        1 => Ok(CameraColorSpace::Unknown),
        2 => Ok(CameraColorSpace::Srgb),
        3 => Ok(CameraColorSpace::Bt601),
        4 => Ok(CameraColorSpace::Bt709),
        5 => Ok(CameraColorSpace::Bt2020),
        _ => Err(invalid_data(
            operation,
            format!("android host returned one unknown camera color-space code {code}"),
        )),
    }
}

/// Decode one camera exposure-mode code.
pub(crate) fn decode_exposure_mode(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<CameraExposureMode> {
    match code {
        1 => Ok(CameraExposureMode::Auto),
        2 => Ok(CameraExposureMode::ContinuousAuto),
        3 => Ok(CameraExposureMode::Manual),
        _ => Err(invalid_data(
            operation,
            format!("android host returned one unknown camera exposure-mode code {code}"),
        )),
    }
}

/// Decode one camera white-balance-mode code.
pub(crate) fn decode_white_balance_mode(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<CameraWhiteBalanceMode> {
    match code {
        1 => Ok(CameraWhiteBalanceMode::Auto),
        2 => Ok(CameraWhiteBalanceMode::ContinuousAuto),
        3 => Ok(CameraWhiteBalanceMode::Manual),
        _ => Err(invalid_data(
            operation,
            format!("android host returned one unknown camera white-balance-mode code {code}"),
        )),
    }
}

/// Decode one camera focus-mode code.
pub(crate) fn decode_focus_mode(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<CameraFocusMode> {
    match code {
        1 => Ok(CameraFocusMode::Auto),
        2 => Ok(CameraFocusMode::ContinuousAuto),
        3 => Ok(CameraFocusMode::Manual),
        _ => Err(invalid_data(
            operation,
            format!("android host returned one unknown camera focus-mode code {code}"),
        )),
    }
}

/// Decode one camera stabilization-mode code.
pub(crate) fn decode_stabilization_mode(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<CameraStabilizationMode> {
    match code {
        1 => Ok(CameraStabilizationMode::Off),
        2 => Ok(CameraStabilizationMode::Standard),
        3 => Ok(CameraStabilizationMode::HighQuality),
        _ => Err(invalid_data(
            operation,
            format!("android host returned one unknown camera stabilization-mode code {code}"),
        )),
    }
}

/// Decode one camera torch-mode code.
pub(crate) fn decode_torch_mode(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<CameraTorchMode> {
    match code {
        1 => Ok(CameraTorchMode::Off),
        2 => Ok(CameraTorchMode::On),
        3 => Ok(CameraTorchMode::Auto),
        _ => Err(invalid_data(
            operation,
            format!("android host returned one unknown camera torch-mode code {code}"),
        )),
    }
}

/// Decode one camera device descriptor header.
pub(crate) fn decode_camera_device_descriptor(
    header: &AndroidHostCameraDeviceDescriptorHeader,
    string_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<CameraDeviceDescriptorValue> {
    Ok(CameraDeviceDescriptorValue {
        id: decode_required_string(
            string_bytes,
            header.id_offset,
            header.id_len,
            "id",
            operation,
        )?,
        group_id: decode_optional_string(
            string_bytes,
            header.group_id_offset,
            header.group_id_len,
            "groupId",
            operation,
        )?,
        name: decode_required_string(
            string_bytes,
            header.name_offset,
            header.name_len,
            "name",
            operation,
        )?,
        manufacturer: decode_optional_string(
            string_bytes,
            header.manufacturer_offset,
            header.manufacturer_len,
            "manufacturer",
            operation,
        )?,
        facing_mode: decode_facing_mode(header.facing_mode, operation)?,
        depth_capable: header.depth_capable != 0,
    })
}

/// Decode one camera stream config header.
pub(crate) fn decode_camera_stream_config(
    header: &AndroidHostCameraStreamConfigHeader,
    operation: &'static str,
) -> RuntimeResult<CameraStreamConfigValue> {
    Ok(CameraStreamConfigValue {
        width: header.width,
        height: header.height,
        frame_rate_milli_hz: header.frame_rate_milli_hz,
        pixel_format: CameraPixelFormatDescriptorValue {
            format: decode_pixel_format(header.pixel_format, operation)?,
            family: decode_pixel_format_family(header.pixel_format_family, operation)?,
            compressed: header.is_compressed != 0,
        },
        color_space: None,
        dynamic_range: None,
    })
}

/// Decode one color-space flag payload.
fn decode_color_spaces(flags: u32) -> Vec<CameraColorSpace> {
    let mut values = Vec::new();

    if flags & CAMERA_COLOR_SPACE_SRGB != 0 {
        values.push(CameraColorSpace::Srgb);
    }
    if flags & CAMERA_COLOR_SPACE_BT601 != 0 {
        values.push(CameraColorSpace::Bt601);
    }
    if flags & CAMERA_COLOR_SPACE_BT709 != 0 {
        values.push(CameraColorSpace::Bt709);
    }
    if flags & CAMERA_COLOR_SPACE_BT2020 != 0 {
        values.push(CameraColorSpace::Bt2020);
    }

    if values.is_empty() {
        values.push(CameraColorSpace::Unknown);
    }

    values
}

/// Decode one dynamic-range flag payload.
fn decode_dynamic_ranges(flags: u32) -> Vec<CameraDynamicRange> {
    let mut values = Vec::new();

    if flags & CAMERA_DYNAMIC_RANGE_STANDARD != 0 {
        values.push(CameraDynamicRange::Standard);
    }
    if flags & CAMERA_DYNAMIC_RANGE_HDR10 != 0 {
        values.push(CameraDynamicRange::Hdr10);
    }
    if flags & CAMERA_DYNAMIC_RANGE_HLG != 0 {
        values.push(CameraDynamicRange::Hlg);
    }

    if values.is_empty() {
        values.push(CameraDynamicRange::Standard);
    }

    values
}

/// Decode one exposure-mode flag payload.
fn decode_exposure_modes(flags: u32) -> Vec<CameraExposureMode> {
    let mut values = Vec::new();

    if flags & CAMERA_EXPOSURE_MODE_AUTO != 0 {
        values.push(CameraExposureMode::Auto);
    }
    if flags & CAMERA_EXPOSURE_MODE_CONTINUOUS_AUTO != 0 {
        values.push(CameraExposureMode::ContinuousAuto);
    }
    if flags & CAMERA_EXPOSURE_MODE_MANUAL != 0 {
        values.push(CameraExposureMode::Manual);
    }

    if values.is_empty() {
        values.push(CameraExposureMode::Auto);
    }

    values
}

/// Decode one white-balance-mode flag payload.
fn decode_white_balance_modes(flags: u32) -> Vec<CameraWhiteBalanceMode> {
    let mut values = Vec::new();

    if flags & CAMERA_WHITE_BALANCE_MODE_AUTO != 0 {
        values.push(CameraWhiteBalanceMode::Auto);
    }
    if flags & CAMERA_WHITE_BALANCE_MODE_CONTINUOUS_AUTO != 0 {
        values.push(CameraWhiteBalanceMode::ContinuousAuto);
    }
    if flags & CAMERA_WHITE_BALANCE_MODE_MANUAL != 0 {
        values.push(CameraWhiteBalanceMode::Manual);
    }

    if values.is_empty() {
        values.push(CameraWhiteBalanceMode::Auto);
    }

    values
}

/// Decode one focus-mode flag payload.
fn decode_focus_modes(flags: u32) -> Vec<CameraFocusMode> {
    let mut values = Vec::new();

    if flags & CAMERA_FOCUS_MODE_AUTO != 0 {
        values.push(CameraFocusMode::Auto);
    }
    if flags & CAMERA_FOCUS_MODE_CONTINUOUS_AUTO != 0 {
        values.push(CameraFocusMode::ContinuousAuto);
    }
    if flags & CAMERA_FOCUS_MODE_MANUAL != 0 {
        values.push(CameraFocusMode::Manual);
    }

    if values.is_empty() {
        values.push(CameraFocusMode::Auto);
    }

    values
}

/// Decode one stabilization-mode flag payload.
fn decode_stabilization_modes(flags: u32) -> Vec<CameraStabilizationMode> {
    let mut values = Vec::new();

    if flags & CAMERA_STABILIZATION_MODE_OFF != 0 {
        values.push(CameraStabilizationMode::Off);
    }
    if flags & CAMERA_STABILIZATION_MODE_STANDARD != 0 {
        values.push(CameraStabilizationMode::Standard);
    }
    if flags & CAMERA_STABILIZATION_MODE_HIGH_QUALITY != 0 {
        values.push(CameraStabilizationMode::HighQuality);
    }

    if values.is_empty() {
        values.push(CameraStabilizationMode::Off);
    }

    values
}

/// Decode one torch-mode flag payload.
fn decode_torch_modes(flags: u32) -> Vec<CameraTorchMode> {
    let mut values = Vec::new();

    if flags & CAMERA_TORCH_MODE_OFF != 0 {
        values.push(CameraTorchMode::Off);
    }
    if flags & CAMERA_TORCH_MODE_ON != 0 {
        values.push(CameraTorchMode::On);
    }
    if flags & CAMERA_TORCH_MODE_AUTO != 0 {
        values.push(CameraTorchMode::Auto);
    }

    if values.is_empty() {
        values.push(CameraTorchMode::Off);
    }

    values
}

/// Decode one camera stream capability header.
pub(crate) fn decode_camera_stream_capability(
    header: &AndroidHostCameraStreamCapabilityHeader,
    operation: &'static str,
) -> RuntimeResult<CameraStreamCapabilityValue> {
    let control_modes = CameraControlModes {
        exposure_modes: decode_exposure_modes(header.exposure_mode_flags),
        white_balance_modes: decode_white_balance_modes(header.white_balance_mode_flags),
        focus_modes: decode_focus_modes(header.focus_mode_flags),
        stabilization_modes: decode_stabilization_modes(header.stabilization_mode_flags),
        torch_modes: decode_torch_modes(header.torch_mode_flags),
    };

    Ok(CameraStreamCapabilityValue {
        config: decode_camera_stream_config(&header.config, operation)?,
        minimum_frame_rate_milli_hz: header.minimum_frame_rate_milli_hz,
        maximum_frame_rate_milli_hz: header.maximum_frame_rate_milli_hz,
        color_spaces: decode_color_spaces(header.color_space_flags),
        dynamic_ranges: decode_dynamic_ranges(header.dynamic_range_flags),
        controls: camera_control_capabilities(&control_modes),
    })
}

/// Return whether one capability supports host auto exposure.
pub(crate) fn supports_auto_exposure(capability: &Option<CameraStreamCapabilityValue>) -> bool {
    let Some(capability) = capability.as_ref() else {
        return false;
    };

    capability
        .controls
        .exposure_modes
        .as_ref()
        .is_some_and(|modes| modes.contains(&CameraExposureMode::Auto))
        || capability
            .controls
            .exposure_modes
            .as_ref()
            .is_some_and(|modes| modes.contains(&CameraExposureMode::ContinuousAuto))
}

/// Return whether one capability supports host auto white balance.
pub(crate) fn supports_auto_white_balance(
    capability: &Option<CameraStreamCapabilityValue>,
) -> bool {
    let Some(capability) = capability.as_ref() else {
        return false;
    };

    capability
        .controls
        .white_balance_modes
        .as_ref()
        .is_some_and(|modes| modes.contains(&CameraWhiteBalanceMode::Auto))
        || capability
            .controls
            .white_balance_modes
            .as_ref()
            .is_some_and(|modes| modes.contains(&CameraWhiteBalanceMode::ContinuousAuto))
}

/// Return whether one capability supports host auto focus.
pub(crate) fn supports_auto_focus(capability: &Option<CameraStreamCapabilityValue>) -> bool {
    let Some(capability) = capability.as_ref() else {
        return false;
    };

    capability
        .controls
        .focus_modes
        .as_ref()
        .is_some_and(|modes| modes.contains(&CameraFocusMode::Auto))
        || capability
            .controls
            .focus_modes
            .as_ref()
            .is_some_and(|modes| modes.contains(&CameraFocusMode::ContinuousAuto))
}
