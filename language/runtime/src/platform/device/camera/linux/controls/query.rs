use super::core::{unsupported_camera_binding, *};
use crate::platform::device::camera::linux::core::*;

/// Read camera brightness.
pub(crate) unsafe fn destack_device_camera_stream_brightness(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.brightness",
    )?;
    let value = read_float_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_BRIGHTNESS,
        "destack.device.camera.stream.brightness",
    )?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read one brightness range.
pub(crate) unsafe fn destack_device_camera_stream_brightness_range(
    binding: &BindingCallContext,
    out: *mut CameraFloatControlRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.brightnessRange",
    )?;
    let range = read_float_control_range(
        resource.descriptor,
        v4l2::V4L2_CID_BRIGHTNESS,
        "destack.device.camera.stream.brightnessRange",
    )?;

    unsafe {
        out.write(range);
    }

    Ok(())
}

/// Read camera contrast.
pub(crate) unsafe fn destack_device_camera_stream_contrast(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.contrast",
    )?;
    let value = read_float_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_CONTRAST,
        "destack.device.camera.stream.contrast",
    )?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read one contrast range.
pub(crate) unsafe fn destack_device_camera_stream_contrast_range(
    binding: &BindingCallContext,
    out: *mut CameraFloatControlRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.contrastRange",
    )?;
    let range = read_float_control_range(
        resource.descriptor,
        v4l2::V4L2_CID_CONTRAST,
        "destack.device.camera.stream.contrastRange",
    )?;

    unsafe {
        out.write(range);
    }

    Ok(())
}

/// Read camera exposure compensation in EV.
pub(crate) unsafe fn destack_device_camera_stream_exposure_compensation(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.exposureCompensation",
    )?;
    let milli_ev = read_exposure_bias_milli_ev(
        resource.descriptor,
        "destack.device.camera.stream.exposureCompensation",
    )?;

    unsafe {
        out.write(milli_ev as f64 / 1000.0);
    }

    Ok(())
}

/// Read one exposure compensation range.
pub(crate) unsafe fn destack_device_camera_stream_exposure_compensation_range(
    binding: &BindingCallContext,
    out: *mut CameraExposureCompensationRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.exposureCompensationRange",
    )?;
    let (minimum, maximum, default, step) = exposure_bias_range_milli_ev(
        resource.descriptor,
        "destack.device.camera.stream.exposureCompensationRange",
    )?;
    let auto_supported = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_EXPOSURE_AUTO,
        "destack.device.camera.stream.exposureCompensationRange",
    )?
    .is_some();

    unsafe {
        out.write(CameraExposureCompensationRange {
            minimum_ev: minimum as f64 / 1000.0,
            maximum_ev: maximum as f64 / 1000.0,
            default_ev: default as f64 / 1000.0,
            step_ev: step as f64 / 1000.0,
            auto_supported: auto_supported,
        });
    }

    Ok(())
}

/// Read camera exposure mode.
pub(crate) unsafe fn destack_device_camera_stream_exposure_mode(
    binding: &BindingCallContext,
    out: *mut CameraExposureMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.exposureMode",
    )?;
    let mode = if query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_EXPOSURE_AUTO,
        "destack.device.camera.stream.exposureMode",
    )?
    .is_some()
    {
        match read_control_value(
            resource.descriptor,
            v4l2::V4L2_CID_EXPOSURE_AUTO,
            "destack.device.camera.stream.exposureMode",
        )? as u32
        {
            value if value == v4l2::v4l2_exposure_auto_type_V4L2_EXPOSURE_MANUAL as u32 => {
                CameraExposureMode::Manual
            }
            _ => CameraExposureMode::ContinuousAuto,
        }
    } else {
        return Err(camera_not_supported(
            "destack.device.camera.stream.exposureMode",
        ));
    };

    unsafe {
        out.write(mode);
    }

    Ok(())
}

/// Read one exposure time value.
pub(crate) unsafe fn destack_device_camera_stream_exposure_time_ns(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.exposureTimeNs",
    )?;
    let Some(_) = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_EXPOSURE_ABSOLUTE,
        "destack.device.camera.stream.exposureTimeNs",
    )?
    else {
        return Err(camera_not_supported(
            "destack.device.camera.stream.exposureTimeNs",
        ));
    };
    let value = read_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_EXPOSURE_ABSOLUTE,
        "destack.device.camera.stream.exposureTimeNs",
    )?;

    unsafe {
        out.write((value.max(0) as u64) * 100_000);
    }

    Ok(())
}

/// Read one exposure time range.
pub(crate) unsafe fn destack_device_camera_stream_exposure_time_range(
    binding: &BindingCallContext,
    out: *mut CameraExposureTimeRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.exposureTimeRange",
    )?;
    let Some(info) = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_EXPOSURE_ABSOLUTE,
        "destack.device.camera.stream.exposureTimeRange",
    )?
    else {
        return Err(camera_not_supported(
            "destack.device.camera.stream.exposureTimeRange",
        ));
    };
    let auto_supported = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_EXPOSURE_AUTO,
        "destack.device.camera.stream.exposureTimeRange",
    )?
    .is_some();

    unsafe {
        out.write(CameraExposureTimeRange {
            minimum_ns: (info.minimum.max(0) as u64) * 100_000,
            maximum_ns: (info.maximum.max(0) as u64) * 100_000,
            default_ns: (info.default.max(0) as u64) * 100_000,
            step_ns: (info.step.max(0) as u64) * 100_000,
            auto_supported: auto_supported,
        });
    }

    Ok(())
}

unsupported_camera_binding!(
    destack_device_camera_stream_focus_distance_diopters(out: *mut f64, handle: resource::CameraStreamHandle),
    "destack.device.camera.stream.focusDistanceDiopters"
);
unsupported_camera_binding!(
    destack_device_camera_stream_focus_distance_range(
        out: *mut CameraFocusDistanceRange,
        handle: resource::CameraStreamHandle
    ),
    "destack.device.camera.stream.focusDistanceRange"
);
/// Read camera focus mode.
pub(crate) unsafe fn destack_device_camera_stream_focus_mode(
    binding: &BindingCallContext,
    out: *mut CameraFocusMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.focusMode",
    )?;
    let Some(_) = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_FOCUS_AUTO,
        "destack.device.camera.stream.focusMode",
    )?
    else {
        return Err(camera_not_supported(
            "destack.device.camera.stream.focusMode",
        ));
    };
    let value = read_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_FOCUS_AUTO,
        "destack.device.camera.stream.focusMode",
    )?;

    unsafe {
        out.write(if value != 0 {
            CameraFocusMode::ContinuousAuto
        } else {
            CameraFocusMode::Manual
        });
    }

    Ok(())
}

/// Read camera pan angle.
pub(crate) unsafe fn destack_device_camera_stream_pan_degrees(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.panDegrees",
    )?;
    let value = read_float_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_PAN_ABSOLUTE,
        "destack.device.camera.stream.panDegrees",
    )?;

    unsafe {
        out.write(value / 3600.0);
    }

    Ok(())
}

/// Read one pan range.
pub(crate) unsafe fn destack_device_camera_stream_pan_range(
    binding: &BindingCallContext,
    out: *mut CameraPanAngleRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.panRange",
    )?;
    let Some(info) = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_PAN_ABSOLUTE,
        "destack.device.camera.stream.panRange",
    )?
    else {
        return Err(camera_not_supported(
            "destack.device.camera.stream.panRange",
        ));
    };

    unsafe {
        out.write(CameraPanAngleRange {
            minimum_degrees: info.minimum as f64 / 3600.0,
            maximum_degrees: info.maximum as f64 / 3600.0,
            default_degrees: info.default as f64 / 3600.0,
            step_degrees: info.step as f64 / 3600.0,
        });
    }

    Ok(())
}

/// Read camera saturation.
pub(crate) unsafe fn destack_device_camera_stream_saturation(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.saturation",
    )?;
    let value = read_float_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_SATURATION,
        "destack.device.camera.stream.saturation",
    )?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read one saturation range.
pub(crate) unsafe fn destack_device_camera_stream_saturation_range(
    binding: &BindingCallContext,
    out: *mut CameraFloatControlRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.saturationRange",
    )?;
    let range = read_float_control_range(
        resource.descriptor,
        v4l2::V4L2_CID_SATURATION,
        "destack.device.camera.stream.saturationRange",
    )?;

    unsafe {
        out.write(range);
    }

    Ok(())
}

/// Read camera sensor ISO.
pub(crate) unsafe fn destack_device_camera_stream_sensor_iso(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.sensorIso",
    )?;
    let Some(_) = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_ISO_SENSITIVITY,
        "destack.device.camera.stream.sensorIso",
    )?
    else {
        return Err(camera_not_supported(
            "destack.device.camera.stream.sensorIso",
        ));
    };
    let value = read_control_value(
        resource.descriptor,
        v4l2::V4L2_CID_ISO_SENSITIVITY,
        "destack.device.camera.stream.sensorIso",
    )?;

    unsafe {
        out.write(value.max(0) as u32);
    }

    Ok(())
}

/// Read one sensor ISO range.
pub(crate) unsafe fn destack_device_camera_stream_sensor_iso_range(
    binding: &BindingCallContext,
    out: *mut CameraSensorIsoRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = camera_stream_resource::<LinuxCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.sensorIsoRange",
    )?;
    let Some(info) = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_ISO_SENSITIVITY,
        "destack.device.camera.stream.sensorIsoRange",
    )?
    else {
        return Err(camera_not_supported(
            "destack.device.camera.stream.sensorIsoRange",
        ));
    };
    let auto_supported = query_control_info(
        resource.descriptor,
        v4l2::V4L2_CID_ISO_SENSITIVITY_AUTO,
        "destack.device.camera.stream.sensorIsoRange",
    )?
    .is_some();

    unsafe {
        out.write(CameraSensorIsoRange {
            minimum: info.minimum.max(0) as u32,
            maximum: info.maximum.max(0) as u32,
            default: info.default.max(0) as u32,
            step: info.step.max(0) as u32,
            auto_supported: auto_supported,
        });
    }

    Ok(())
}
