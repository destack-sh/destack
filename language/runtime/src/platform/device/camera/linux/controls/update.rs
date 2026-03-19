use super::core::{unsupported_camera_binding, *};
use crate::platform::device::camera::linux::core::*;

/// Write one camera brightness value through V4L2.
pub(crate) unsafe fn destack_device_camera_stream_set_brightness(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    argument_value: f64,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setBrightness",
    )?;
    write_float_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_BRIGHTNESS,
        argument_value,
        "destack.device.camera.stream.setBrightness",
    )
}

/// Set camera contrast.
pub(crate) unsafe fn destack_device_camera_stream_set_contrast(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    argument_value: f64,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setContrast",
    )?;
    write_float_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_CONTRAST,
        argument_value,
        "destack.device.camera.stream.setContrast",
    )
}

/// Set exposure compensation.
pub(crate) unsafe fn destack_device_camera_stream_set_exposure_compensation(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    valueev: f64,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setExposureCompensation",
    )?;
    write_exposure_bias_milli_ev(
        resource.descriptor,
        (valueev * 1000.0).round() as i64,
        "destack.device.camera.stream.setExposureCompensation",
    )
}

/// Set exposure mode.
pub(crate) unsafe fn destack_device_camera_stream_set_exposure_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    mode: CameraExposureMode,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setExposureMode",
    )?;
    let value = match mode {
        CameraExposureMode::Manual => v4l2::v4l2_exposure_auto_type_V4L2_EXPOSURE_MANUAL as i64,
        CameraExposureMode::Auto | CameraExposureMode::ContinuousAuto => {
            v4l2::v4l2_exposure_auto_type_V4L2_EXPOSURE_AUTO as i64
        }
    };

    write_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_EXPOSURE_AUTO,
        value,
        "destack.device.camera.stream.setExposureMode",
    )
}

/// Set exposure time.
pub(crate) unsafe fn destack_device_camera_stream_set_exposure_time_ns(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    valuens: u64,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setExposureTimeNs",
    )?;

    if query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_EXPOSURE_AUTO,
        "destack.device.camera.stream.setExposureTimeNs",
    )?
    .is_some()
    {
        write_control_value(
            resource.descriptor,
            v4l2::V4L2_CID_EXPOSURE_AUTO,
            v4l2::v4l2_exposure_auto_type_V4L2_EXPOSURE_MANUAL as i64,
            "destack.device.camera.stream.setExposureTimeNs",
        )?;
    }

    write_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_EXPOSURE_ABSOLUTE,
        (valuens / 100_000) as i64,
        "destack.device.camera.stream.setExposureTimeNs",
    )
}

unsupported_camera_binding!(
    destack_device_camera_stream_set_focus_distance_diopters(handle: resource::CameraStreamHandle, diopters: f64),
    "destack.device.camera.stream.setFocusDistanceDiopters"
);
/// Set focus mode.
pub(crate) unsafe fn destack_device_camera_stream_set_focus_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    mode: CameraFocusMode,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setFocusMode",
    )?;
    let value = match mode {
        CameraFocusMode::Manual => 0,
        CameraFocusMode::Auto | CameraFocusMode::ContinuousAuto => 1,
    };

    write_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_FOCUS_AUTO,
        value,
        "destack.device.camera.stream.setFocusMode",
    )
}

/// Set pan angle.
pub(crate) unsafe fn destack_device_camera_stream_set_pan_degrees(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    degrees: f64,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setPanDegrees",
    )?;
    write_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_PAN_ABSOLUTE,
        (degrees * 3600.0).round() as i64,
        "destack.device.camera.stream.setPanDegrees",
    )
}

/// Set saturation.
pub(crate) unsafe fn destack_device_camera_stream_set_saturation(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    argument_value: f64,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setSaturation",
    )?;
    write_float_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_SATURATION,
        argument_value,
        "destack.device.camera.stream.setSaturation",
    )
}

/// Set sensor ISO.
pub(crate) unsafe fn destack_device_camera_stream_set_sensor_iso(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    iso: u32,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setSensorIso",
    )?;

    if query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_ISO_SENSITIVITY_AUTO,
        "destack.device.camera.stream.setSensorIso",
    )?
    .is_some()
    {
        write_control_value(
            resource.descriptor,
            v4l2::V4L2_CID_ISO_SENSITIVITY_AUTO,
            v4l2::v4l2_iso_sensitivity_auto_type_V4L2_ISO_SENSITIVITY_MANUAL as i64,
            "destack.device.camera.stream.setSensorIso",
        )?;
    }

    write_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_ISO_SENSITIVITY,
        iso as i64,
        "destack.device.camera.stream.setSensorIso",
    )
}

/// Set sharpness.
pub(crate) unsafe fn destack_device_camera_stream_set_sharpness(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    argument_value: f64,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setSharpness",
    )?;
    write_float_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_SHARPNESS,
        argument_value,
        "destack.device.camera.stream.setSharpness",
    )
}

/// Set stabilization mode.
pub(crate) unsafe fn destack_device_camera_stream_set_stabilization_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    mode: CameraStabilizationMode,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setStabilizationMode",
    )?;
    let value = match mode {
        CameraStabilizationMode::Off => 0,
        CameraStabilizationMode::Standard => 1,
        CameraStabilizationMode::HighQuality => {
            return Err(camera_not_supported(
                "destack.device.camera.stream.setStabilizationMode",
            ));
        }
    };

    write_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_IMAGE_STABILIZATION,
        value,
        "destack.device.camera.stream.setStabilizationMode",
    )
}

/// Set tilt angle.
pub(crate) unsafe fn destack_device_camera_stream_set_tilt_degrees(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    degrees: f64,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setTiltDegrees",
    )?;
    write_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_TILT_ABSOLUTE,
        (degrees * 3600.0).round() as i64,
        "destack.device.camera.stream.setTiltDegrees",
    )
}

/// Set torch mode.
pub(crate) unsafe fn destack_device_camera_stream_set_torch_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    mode: CameraTorchMode,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setTorchMode",
    )?;
    let value = match mode {
        CameraTorchMode::Off => v4l2::v4l2_flash_led_mode_V4L2_FLASH_LED_MODE_NONE as i64,
        CameraTorchMode::On => v4l2::v4l2_flash_led_mode_V4L2_FLASH_LED_MODE_TORCH as i64,
        CameraTorchMode::Auto => v4l2::v4l2_flash_led_mode_V4L2_FLASH_LED_MODE_FLASH as i64,
    };

    write_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_FLASH_LED_MODE,
        value,
        "destack.device.camera.stream.setTorchMode",
    )
}

/// Set white balance kelvin.
pub(crate) unsafe fn destack_device_camera_stream_set_white_balance_kelvin(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    kelvin: u32,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setWhiteBalanceKelvin",
    )?;

    if query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_AUTO_WHITE_BALANCE,
        "destack.device.camera.stream.setWhiteBalanceKelvin",
    )?
    .is_some()
    {
        write_control_value(
            resource.descriptor,
            v4l2::V4L2_CID_AUTO_WHITE_BALANCE,
            0,
            "destack.device.camera.stream.setWhiteBalanceKelvin",
        )?;
    }

    write_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_WHITE_BALANCE_TEMPERATURE,
        kelvin as i64,
        "destack.device.camera.stream.setWhiteBalanceKelvin",
    )
}

/// Set white balance mode.
pub(crate) unsafe fn destack_device_camera_stream_set_white_balance_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    mode: CameraWhiteBalanceMode,
) -> RuntimeResult<()> {
    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.setWhiteBalanceMode",
    )?;
    let value = match mode {
        CameraWhiteBalanceMode::Manual => 0,
        CameraWhiteBalanceMode::Auto | CameraWhiteBalanceMode::ContinuousAuto => 1,
    };

    write_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_AUTO_WHITE_BALANCE,
        value,
        "destack.device.camera.stream.setWhiteBalanceMode",
    )
}

unsupported_camera_binding!(
    destack_device_camera_stream_set_zoom_ratio(handle: resource::CameraStreamHandle, ratio: f64),
    "destack.device.camera.stream.setZoomRatio"
);
/// Read camera sharpness.
pub(crate) unsafe fn destack_device_camera_stream_sharpness(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.sharpness",
    )?;
    let value = read_float_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_SHARPNESS,
        "destack.device.camera.stream.sharpness",
    )?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read one sharpness range.
pub(crate) unsafe fn destack_device_camera_stream_sharpness_range(
    binding: &BindingCallContext,
    out: *mut CameraFloatControlRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.sharpnessRange",
    )?;
    let range = read_float_control_range(
        resource.descriptor,
        v4l2::V4L2_CID_SHARPNESS,
        "destack.device.camera.stream.sharpnessRange",
    )?;

    unsafe {
        out.write(range);
    }

    Ok(())
}

/// Read stabilization mode.
pub(crate) unsafe fn destack_device_camera_stream_stabilization_mode(
    binding: &BindingCallContext,
    out: *mut CameraStabilizationMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.stabilizationMode",
    )?;
    let Some(_) = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_IMAGE_STABILIZATION,
        "destack.device.camera.stream.stabilizationMode",
    )?
    else {
        return Err(camera_not_supported(
            "destack.device.camera.stream.stabilizationMode",
        ));
    };
    let value = read_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_IMAGE_STABILIZATION,
        "destack.device.camera.stream.stabilizationMode",
    )?;

    unsafe {
        out.write(if value == 0 {
            CameraStabilizationMode::Off
        } else {
            CameraStabilizationMode::Standard
        });
    }

    Ok(())
}

/// Read camera tilt angle.
pub(crate) unsafe fn destack_device_camera_stream_tilt_degrees(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.tiltDegrees",
    )?;
    let value = read_float_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_TILT_ABSOLUTE,
        "destack.device.camera.stream.tiltDegrees",
    )?;

    unsafe {
        out.write(value / 3600.0);
    }

    Ok(())
}

/// Read one tilt range.
pub(crate) unsafe fn destack_device_camera_stream_tilt_range(
    binding: &BindingCallContext,
    out: *mut CameraTiltAngleRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.tiltRange",
    )?;
    let Some(info) = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_TILT_ABSOLUTE,
        "destack.device.camera.stream.tiltRange",
    )?
    else {
        return Err(camera_not_supported(
            "destack.device.camera.stream.tiltRange",
        ));
    };

    unsafe {
        out.write(CameraTiltAngleRange {
            minimum_degrees: info.minimum as f64 / 3600.0,
            maximum_degrees: info.maximum as f64 / 3600.0,
            default_degrees: info.default as f64 / 3600.0,
            step_degrees: info.step as f64 / 3600.0,
        });
    }

    Ok(())
}

/// Read torch mode.
pub(crate) unsafe fn destack_device_camera_stream_torch_mode(
    binding: &BindingCallContext,
    out: *mut CameraTorchMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.torchMode",
    )?;
    let value = read_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_FLASH_LED_MODE,
        "destack.device.camera.stream.torchMode",
    )?;

    let mode = match value as u32 {
        value if value == v4l2::v4l2_flash_led_mode_V4L2_FLASH_LED_MODE_TORCH as u32 => {
            CameraTorchMode::On
        }
        value if value == v4l2::v4l2_flash_led_mode_V4L2_FLASH_LED_MODE_FLASH as u32 => {
            CameraTorchMode::Auto
        }
        _ => CameraTorchMode::Off,
    };

    unsafe {
        out.write(mode);
    }

    Ok(())
}

/// Read white balance kelvin.
pub(crate) unsafe fn destack_device_camera_stream_white_balance_kelvin(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.whiteBalanceKelvin",
    )?;
    let value = read_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_WHITE_BALANCE_TEMPERATURE,
        "destack.device.camera.stream.whiteBalanceKelvin",
    )?;

    unsafe {
        out.write(value.max(0) as u32);
    }

    Ok(())
}

/// Read white balance mode.
pub(crate) unsafe fn destack_device_camera_stream_white_balance_mode(
    binding: &BindingCallContext,
    out: *mut CameraWhiteBalanceMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.whiteBalanceMode",
    )?;
    let value = read_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_AUTO_WHITE_BALANCE,
        "destack.device.camera.stream.whiteBalanceMode",
    )?;

    unsafe {
        out.write(if value != 0 {
            CameraWhiteBalanceMode::ContinuousAuto
        } else {
            CameraWhiteBalanceMode::Manual
        });
    }

    Ok(())
}

/// Read one white balance range.
pub(crate) unsafe fn destack_device_camera_stream_white_balance_range(
    binding: &BindingCallContext,
    out: *mut CameraWhiteBalanceRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.whiteBalanceRange",
    )?;
    let Some(info) = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_WHITE_BALANCE_TEMPERATURE,
        "destack.device.camera.stream.whiteBalanceRange",
    )?
    else {
        return Err(camera_not_supported(
            "destack.device.camera.stream.whiteBalanceRange",
        ));
    };
    let auto_supported = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_AUTO_WHITE_BALANCE,
        "destack.device.camera.stream.whiteBalanceRange",
    )?
    .is_some();

    unsafe {
        out.write(CameraWhiteBalanceRange {
            minimum_kelvin: info.minimum.max(0) as u32,
            maximum_kelvin: info.maximum.max(0) as u32,
            default_kelvin: info.default.max(0) as u32,
            step_kelvin: info.step.max(0) as u32,
            auto_supported: auto_supported,
        });
    }

    Ok(())
}
unsupported_camera_binding!(
    destack_device_camera_stream_zoom_ratio(out: *mut f64, handle: resource::CameraStreamHandle),
    "destack.device.camera.stream.zoomRatio"
);
unsupported_camera_binding!(
    destack_device_camera_stream_zoom_ratio_range(
        out: *mut CameraZoomRatioRange,
        handle: resource::CameraStreamHandle
    ),
    "destack.device.camera.stream.zoomRatioRange"
);
