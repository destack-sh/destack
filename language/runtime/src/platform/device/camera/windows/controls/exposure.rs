use crate::platform::device::camera::windows::core::*;
use crate::platform::device::camera::windows::metadata::*;
use crate::platform::device::camera::windows::stream::*;

/// Read one Windows camera exposure compensation value.
pub(crate) unsafe fn destack_device_camera_stream_exposure_compensation(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.exposureCompensation",
    )?;
    let controller = video_device_controller(
        &resource,
        "destack.device.camera.stream.exposureCompensation",
    )?;
    let control = controller.ExposureCompensationControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.exposureCompensation",
            "VideoDeviceController::ExposureCompensationControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.exposureCompensation",
            "ExposureCompensationControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.exposureCompensation",
        ));
    }

    unsafe {
        out.write(control.Value().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.exposureCompensation",
                "ExposureCompensationControl::Value",
                &error,
            )
        })? as f64);
    }

    Ok(())
}

/// Read one Windows camera exposure compensation range.
pub(crate) unsafe fn destack_device_camera_stream_exposure_compensation_range(
    binding: &BindingCallContext,
    out: *mut CameraExposureCompensationRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.exposureCompensationRange",
    )?;
    let controller = video_device_controller(
        &resource,
        "destack.device.camera.stream.exposureCompensationRange",
    )?;
    let control = controller.ExposureCompensationControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.exposureCompensationRange",
            "VideoDeviceController::ExposureCompensationControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.exposureCompensationRange",
            "ExposureCompensationControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.exposureCompensationRange",
        ));
    }

    unsafe {
        out.write(CameraExposureCompensationRange {
            minimum_ev: control.Min().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.exposureCompensationRange",
                    "ExposureCompensationControl::Min",
                    &error,
                )
            })? as f64,
            maximum_ev: control.Max().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.exposureCompensationRange",
                    "ExposureCompensationControl::Max",
                    &error,
                )
            })? as f64,
            default_ev: control.Value().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.exposureCompensationRange",
                    "ExposureCompensationControl::Value",
                    &error,
                )
            })? as f64,
            step_ev: control.Step().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.exposureCompensationRange",
                    "ExposureCompensationControl::Step",
                    &error,
                )
            })? as f64,
            auto_supported: true,
        });
    }

    Ok(())
}

/// Write one Windows camera exposure compensation value.
pub(crate) unsafe fn destack_device_camera_stream_set_exposure_compensation(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: f64,
) -> RuntimeResult<()> {
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setExposureCompensation",
    )?;
    let controller = video_device_controller(
        &resource,
        "destack.device.camera.stream.setExposureCompensation",
    )?;
    let control = controller.ExposureCompensationControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setExposureCompensation",
            "VideoDeviceController::ExposureCompensationControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setExposureCompensation",
            "ExposureCompensationControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.setExposureCompensation",
        ));
    }

    complete_action(
        control.SetValueAsync(value as f32).map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.setExposureCompensation",
                "ExposureCompensationControl::SetValueAsync",
                &error,
            )
        })?,
        "destack.device.camera.stream.setExposureCompensation",
        "IAsyncAction::get",
    )
}

/// Read one Windows camera exposure mode.
pub(crate) unsafe fn destack_device_camera_stream_exposure_mode(
    binding: &BindingCallContext,
    out: *mut CameraExposureMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(binding, handle, "destack.device.camera.stream.exposureMode")?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.exposureMode")?;
    let control = controller.ExposureControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.exposureMode",
            "VideoDeviceController::ExposureControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.exposureMode",
            "ExposureControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.exposureMode",
        ));
    }

    let value = if control.Auto().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.exposureMode",
            "ExposureControl::Auto",
            &error,
        )
    })? {
        CameraExposureMode::Auto
    } else {
        CameraExposureMode::Manual
    };

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Write one Windows camera exposure mode.
pub(crate) unsafe fn destack_device_camera_stream_set_exposure_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: CameraExposureMode,
) -> RuntimeResult<()> {
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setExposureMode",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.setExposureMode")?;
    let control = controller.ExposureControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setExposureMode",
            "VideoDeviceController::ExposureControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setExposureMode",
            "ExposureControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.setExposureMode",
        ));
    }

    match value {
        CameraExposureMode::Auto => complete_action(
            control.SetAutoAsync(true).map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.setExposureMode",
                    "ExposureControl::SetAutoAsync",
                    &error,
                )
            })?,
            "destack.device.camera.stream.setExposureMode",
            "IAsyncAction::get",
        ),
        CameraExposureMode::Manual => complete_action(
            control.SetAutoAsync(false).map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.setExposureMode",
                    "ExposureControl::SetAutoAsync",
                    &error,
                )
            })?,
            "destack.device.camera.stream.setExposureMode",
            "IAsyncAction::get",
        ),
        CameraExposureMode::ContinuousAuto => {
            Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "value",
                "continuous auto exposure is not available on this Windows camera backend",
            ))
            .boxed())
        }
    }
}

/// Read one Windows camera exposure time.
pub(crate) unsafe fn destack_device_camera_stream_exposure_time_ns(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.exposureTimeNs",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.exposureTimeNs")?;
    let control = controller.ExposureControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.exposureTimeNs",
            "VideoDeviceController::ExposureControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.exposureTimeNs",
            "ExposureControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.exposureTimeNs",
        ));
    }

    unsafe {
        out.write(time_span_ns(control.Value().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.exposureTimeNs",
                "ExposureControl::Value",
                &error,
            )
        })?));
    }

    Ok(())
}

/// Read one Windows camera exposure time range.
pub(crate) unsafe fn destack_device_camera_stream_exposure_time_range(
    binding: &BindingCallContext,
    out: *mut CameraExposureTimeRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.exposureTimeRange",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.exposureTimeRange")?;
    let control = controller.ExposureControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.exposureTimeRange",
            "VideoDeviceController::ExposureControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.exposureTimeRange",
            "ExposureControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.exposureTimeRange",
        ));
    }

    unsafe {
        out.write(CameraExposureTimeRange {
            minimum_ns: time_span_ns(control.Min().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.exposureTimeRange",
                    "ExposureControl::Min",
                    &error,
                )
            })?),
            maximum_ns: time_span_ns(control.Max().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.exposureTimeRange",
                    "ExposureControl::Max",
                    &error,
                )
            })?),
            default_ns: time_span_ns(control.Value().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.exposureTimeRange",
                    "ExposureControl::Value",
                    &error,
                )
            })?),
            step_ns: time_span_ns(control.Step().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.exposureTimeRange",
                    "ExposureControl::Step",
                    &error,
                )
            })?),
            auto_supported: true,
        });
    }

    Ok(())
}

/// Write one Windows camera exposure time.
pub(crate) unsafe fn destack_device_camera_stream_set_exposure_time_ns(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: u64,
) -> RuntimeResult<()> {
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setExposureTimeNs",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.setExposureTimeNs")?;
    let control = controller.ExposureControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setExposureTimeNs",
            "VideoDeviceController::ExposureControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setExposureTimeNs",
            "ExposureControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.setExposureTimeNs",
        ));
    }

    complete_action(
        control
            .SetValueAsync(windows::Foundation::TimeSpan {
                Duration: i64::try_from(value / 100).unwrap_or(i64::MAX),
            })
            .map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.setExposureTimeNs",
                    "ExposureControl::SetValueAsync",
                    &error,
                )
            })?,
        "destack.device.camera.stream.setExposureTimeNs",
        "IAsyncAction::get",
    )
}

/// Read one Windows camera sensor ISO value.
pub(crate) unsafe fn destack_device_camera_stream_sensor_iso(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(binding, handle, "destack.device.camera.stream.sensorIso")?;
    let controller = video_device_controller(&resource, "destack.device.camera.stream.sensorIso")?;
    let control = controller.IsoSpeedControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.sensorIso",
            "VideoDeviceController::IsoSpeedControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.sensorIso",
            "IsoSpeedControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.sensorIso",
        ));
    }

    unsafe {
        out.write(control.Value().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.sensorIso",
                "IsoSpeedControl::Value",
                &error,
            )
        })?);
    }

    Ok(())
}

/// Read one Windows camera sensor ISO range.
pub(crate) unsafe fn destack_device_camera_stream_sensor_iso_range(
    binding: &BindingCallContext,
    out: *mut CameraSensorIsoRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.sensorIsoRange",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.sensorIsoRange")?;
    let control = controller.IsoSpeedControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.sensorIsoRange",
            "VideoDeviceController::IsoSpeedControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.sensorIsoRange",
            "IsoSpeedControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.sensorIsoRange",
        ));
    }

    unsafe {
        out.write(CameraSensorIsoRange {
            minimum: control.Min().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.sensorIsoRange",
                    "IsoSpeedControl::Min",
                    &error,
                )
            })?,
            maximum: control.Max().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.sensorIsoRange",
                    "IsoSpeedControl::Max",
                    &error,
                )
            })?,
            default: control.Value().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.sensorIsoRange",
                    "IsoSpeedControl::Value",
                    &error,
                )
            })?,
            step: control.Step().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.sensorIsoRange",
                    "IsoSpeedControl::Step",
                    &error,
                )
            })?,
            auto_supported: true,
        });
    }

    Ok(())
}

/// Write one Windows camera sensor ISO value.
pub(crate) unsafe fn destack_device_camera_stream_set_sensor_iso(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: u32,
) -> RuntimeResult<()> {
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.setSensorIso")?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.setSensorIso")?;
    let control = controller.IsoSpeedControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setSensorIso",
            "VideoDeviceController::IsoSpeedControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setSensorIso",
            "IsoSpeedControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.setSensorIso",
        ));
    }

    complete_action(
        control.SetValueAsync(value).map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.setSensorIso",
                "IsoSpeedControl::SetValueAsync",
                &error,
            )
        })?,
        "destack.device.camera.stream.setSensorIso",
        "IAsyncAction::get",
    )
}
