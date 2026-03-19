use crate::platform::device::camera::windows::core::*;
use crate::platform::device::camera::windows::metadata::*;
use crate::platform::device::camera::windows::stream::*;

/// Read one Windows camera focus mode.
pub(crate) unsafe fn destack_device_camera_stream_focus_mode(
    binding: &BindingCallContext,
    out: *mut CameraFocusMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(binding, handle, "destack.device.camera.stream.focusMode")?;
    let controller = video_device_controller(&resource, "destack.device.camera.stream.focusMode")?;
    let control = controller.FocusControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.focusMode",
            "VideoDeviceController::FocusControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.focusMode",
            "FocusControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.focusMode",
        ));
    }

    unsafe {
        out.write(focus_mode_from_windows(control.Mode().map_err(
            |error| {
                windows_camera_error(
                    "destack.device.camera.stream.focusMode",
                    "FocusControl::Mode",
                    &error,
                )
            },
        )?));
    }

    Ok(())
}

/// Write one Windows camera focus mode.
pub(crate) unsafe fn destack_device_camera_stream_set_focus_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: CameraFocusMode,
) -> RuntimeResult<()> {
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.setFocusMode")?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.setFocusMode")?;
    let control = controller.FocusControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setFocusMode",
            "VideoDeviceController::FocusControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setFocusMode",
            "FocusControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.setFocusMode",
        ));
    }

    let settings = FocusSettings::new().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setFocusMode",
            "FocusSettings::new",
            &error,
        )
    })?;
    settings
        .SetMode(match value {
            CameraFocusMode::Auto => WindowsFocusMode::Single,
            CameraFocusMode::ContinuousAuto => WindowsFocusMode::Continuous,
            CameraFocusMode::Manual => WindowsFocusMode::Manual,
        })
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.setFocusMode",
                "FocusSettings::SetMode",
                &error,
            )
        })?;
    control.Configure(&settings).map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setFocusMode",
            "FocusControl::Configure",
            &error,
        )
    })?;

    if value != CameraFocusMode::Manual {
        complete_action(
            control.FocusAsync().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.setFocusMode",
                    "FocusControl::FocusAsync",
                    &error,
                )
            })?,
            "destack.device.camera.stream.setFocusMode",
            "IAsyncAction::get",
        )?;
    }

    Ok(())
}

/// Read one Windows camera stabilization mode.
pub(crate) unsafe fn destack_device_camera_stream_stabilization_mode(
    binding: &BindingCallContext,
    out: *mut CameraStabilizationMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.stabilizationMode",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.stabilizationMode")?;
    let control = controller
        .OpticalImageStabilizationControl()
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.stabilizationMode",
                "VideoDeviceController::OpticalImageStabilizationControl",
                &error,
            )
        })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.stabilizationMode",
            "OpticalImageStabilizationControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.stabilizationMode",
        ));
    }

    unsafe {
        out.write(stabilization_mode_from_windows(control.Mode().map_err(
            |error| {
                windows_camera_error(
                    "destack.device.camera.stream.stabilizationMode",
                    "OpticalImageStabilizationControl::Mode",
                    &error,
                )
            },
        )?));
    }

    Ok(())
}

/// Write one Windows camera stabilization mode.
pub(crate) unsafe fn destack_device_camera_stream_set_stabilization_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: CameraStabilizationMode,
) -> RuntimeResult<()> {
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setStabilizationMode",
    )?;
    let controller = video_device_controller(
        &resource,
        "destack.device.camera.stream.setStabilizationMode",
    )?;
    let control = controller
        .OpticalImageStabilizationControl()
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.setStabilizationMode",
                "VideoDeviceController::OpticalImageStabilizationControl",
                &error,
            )
        })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setStabilizationMode",
            "OpticalImageStabilizationControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.setStabilizationMode",
        ));
    }

    let mode = match value {
        CameraStabilizationMode::Off => OpticalImageStabilizationMode::Off,
        CameraStabilizationMode::Standard => OpticalImageStabilizationMode::On,
        CameraStabilizationMode::HighQuality => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "value",
                "high quality stabilization is not available on this Windows camera backend",
            ))
            .boxed());
        }
    };

    control.SetMode(mode).map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setStabilizationMode",
            "OpticalImageStabilizationControl::SetMode",
            &error,
        )
    })
}

/// Read one Windows camera torch mode.
pub(crate) unsafe fn destack_device_camera_stream_torch_mode(
    binding: &BindingCallContext,
    out: *mut CameraTorchMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(binding, handle, "destack.device.camera.stream.torchMode")?;
    let controller = video_device_controller(&resource, "destack.device.camera.stream.torchMode")?;
    let control = controller.TorchControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.torchMode",
            "VideoDeviceController::TorchControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.torchMode",
            "TorchControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.torchMode",
        ));
    }

    let mode = if control.Enabled().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.torchMode",
            "TorchControl::Enabled",
            &error,
        )
    })? {
        CameraTorchMode::On
    } else {
        CameraTorchMode::Off
    };

    unsafe {
        out.write(mode);
    }

    Ok(())
}

/// Write one Windows camera torch mode.
pub(crate) unsafe fn destack_device_camera_stream_set_torch_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: CameraTorchMode,
) -> RuntimeResult<()> {
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.setTorchMode")?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.setTorchMode")?;
    let control = controller.TorchControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setTorchMode",
            "VideoDeviceController::TorchControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setTorchMode",
            "TorchControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.setTorchMode",
        ));
    }

    match value {
        CameraTorchMode::Off => control.SetEnabled(false).map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.setTorchMode",
                "TorchControl::SetEnabled",
                &error,
            )
        }),
        CameraTorchMode::On => control.SetEnabled(true).map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.setTorchMode",
                "TorchControl::SetEnabled",
                &error,
            )
        }),
        CameraTorchMode::Auto => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "value",
            "automatic torch control is not available on this Windows camera backend",
        ))
        .boxed()),
    }
}

/// Read one Windows camera zoom ratio.
pub(crate) unsafe fn destack_device_camera_stream_zoom_ratio(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(binding, handle, "destack.device.camera.stream.zoomRatio")?;
    let controller = video_device_controller(&resource, "destack.device.camera.stream.zoomRatio")?;
    let control = controller.ZoomControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.zoomRatio",
            "VideoDeviceController::ZoomControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.zoomRatio",
            "ZoomControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.zoomRatio",
        ));
    }

    unsafe {
        out.write(control.Value().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.zoomRatio",
                "ZoomControl::Value",
                &error,
            )
        })? as f64);
    }

    Ok(())
}

/// Read one Windows camera zoom ratio range.
pub(crate) unsafe fn destack_device_camera_stream_zoom_ratio_range(
    binding: &BindingCallContext,
    out: *mut CameraZoomRatioRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.zoomRatioRange",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.zoomRatioRange")?;
    let control = controller.ZoomControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.zoomRatioRange",
            "VideoDeviceController::ZoomControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.zoomRatioRange",
            "ZoomControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.zoomRatioRange",
        ));
    }

    unsafe {
        out.write(CameraZoomRatioRange {
            minimum_ratio: control.Min().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.zoomRatioRange",
                    "ZoomControl::Min",
                    &error,
                )
            })? as f64,
            maximum_ratio: control.Max().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.zoomRatioRange",
                    "ZoomControl::Max",
                    &error,
                )
            })? as f64,
            default_ratio: control.Value().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.zoomRatioRange",
                    "ZoomControl::Value",
                    &error,
                )
            })? as f64,
            step_ratio: control.Step().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.zoomRatioRange",
                    "ZoomControl::Step",
                    &error,
                )
            })? as f64,
        });
    }

    Ok(())
}

/// Write one Windows camera zoom ratio.
pub(crate) unsafe fn destack_device_camera_stream_set_zoom_ratio(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: f64,
) -> RuntimeResult<()> {
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.setZoomRatio")?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.setZoomRatio")?;
    let control = controller.ZoomControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setZoomRatio",
            "VideoDeviceController::ZoomControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setZoomRatio",
            "ZoomControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.setZoomRatio",
        ));
    }

    let settings = ZoomSettings::new().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setZoomRatio",
            "ZoomSettings::new",
            &error,
        )
    })?;
    settings
        .SetMode(ZoomTransitionMode::Direct)
        .map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.setZoomRatio",
                "ZoomSettings::SetMode",
                &error,
            )
        })?;
    settings.SetValue(value as f32).map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setZoomRatio",
            "ZoomSettings::SetValue",
            &error,
        )
    })?;
    control.Configure(&settings).map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setZoomRatio",
            "ZoomControl::Configure",
            &error,
        )
    })
}

/// Read one unsupported Windows camera focus-distance control.
pub(crate) unsafe fn destack_device_camera_stream_focus_distance_diopters(
    _binding: &BindingCallContext,
    _out: *mut f64,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.focusDistanceDiopters",
    ))
}

/// Read one unsupported Windows camera focus-distance range.
pub(crate) unsafe fn destack_device_camera_stream_focus_distance_range(
    _binding: &BindingCallContext,
    _out: *mut CameraFocusDistanceRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.focusDistanceRange",
    ))
}

/// Write one unsupported Windows camera focus-distance value.
pub(crate) unsafe fn destack_device_camera_stream_set_focus_distance_diopters(
    _binding: &BindingCallContext,
    _handle: resource::CameraStreamHandle,
    _value: f64,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.setFocusDistanceDiopters",
    ))
}

/// Read one unsupported Windows camera saturation control.
pub(crate) unsafe fn destack_device_camera_stream_saturation(
    _binding: &BindingCallContext,
    _out: *mut f64,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.saturation",
    ))
}

/// Read one unsupported Windows camera saturation range.
pub(crate) unsafe fn destack_device_camera_stream_saturation_range(
    _binding: &BindingCallContext,
    _out: *mut CameraFloatControlRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.saturationRange",
    ))
}

/// Write one unsupported Windows camera saturation value.
pub(crate) unsafe fn destack_device_camera_stream_set_saturation(
    _binding: &BindingCallContext,
    _handle: resource::CameraStreamHandle,
    _value: f64,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.setSaturation",
    ))
}

/// Read one unsupported Windows camera sharpness control.
pub(crate) unsafe fn destack_device_camera_stream_sharpness(
    _binding: &BindingCallContext,
    _out: *mut f64,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.sharpness",
    ))
}

/// Read one unsupported Windows camera sharpness range.
pub(crate) unsafe fn destack_device_camera_stream_sharpness_range(
    _binding: &BindingCallContext,
    _out: *mut CameraFloatControlRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.sharpnessRange",
    ))
}

/// Write one unsupported Windows camera sharpness value.
pub(crate) unsafe fn destack_device_camera_stream_set_sharpness(
    _binding: &BindingCallContext,
    _handle: resource::CameraStreamHandle,
    _value: f64,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.setSharpness",
    ))
}

/// Read one unsupported Windows camera pan control.
pub(crate) unsafe fn destack_device_camera_stream_pan_degrees(
    _binding: &BindingCallContext,
    _out: *mut f64,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.panDegrees",
    ))
}

/// Read one unsupported Windows camera pan range.
pub(crate) unsafe fn destack_device_camera_stream_pan_range(
    _binding: &BindingCallContext,
    _out: *mut CameraPanAngleRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.panRange",
    ))
}

/// Write one unsupported Windows camera pan value.
pub(crate) unsafe fn destack_device_camera_stream_set_pan_degrees(
    _binding: &BindingCallContext,
    _handle: resource::CameraStreamHandle,
    _value: f64,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.setPanDegrees",
    ))
}

/// Read one unsupported Windows camera tilt control.
pub(crate) unsafe fn destack_device_camera_stream_tilt_degrees(
    _binding: &BindingCallContext,
    _out: *mut f64,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.tiltDegrees",
    ))
}

/// Read one unsupported Windows camera tilt range.
pub(crate) unsafe fn destack_device_camera_stream_tilt_range(
    _binding: &BindingCallContext,
    _out: *mut CameraTiltAngleRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.tiltRange",
    ))
}

/// Write one unsupported Windows camera tilt value.
pub(crate) unsafe fn destack_device_camera_stream_set_tilt_degrees(
    _binding: &BindingCallContext,
    _handle: resource::CameraStreamHandle,
    _value: f64,
) -> RuntimeResult<()> {
    Err(unsupported_windows_camera_control(
        "destack.device.camera.stream.setTiltDegrees",
    ))
}
