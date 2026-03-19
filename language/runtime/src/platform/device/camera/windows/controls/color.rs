use crate::platform::device::camera::windows::core::*;
use crate::platform::device::camera::windows::metadata::*;
use crate::platform::device::camera::windows::stream::*;

/// Read one Windows camera brightness value.
pub(crate) unsafe fn destack_device_camera_stream_brightness(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(binding, handle, "destack.device.camera.stream.brightness")?;
    let controller = video_device_controller(&resource, "destack.device.camera.stream.brightness")?;
    let value = media_device_control_value(
        controller.Brightness().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.brightness",
                "VideoDeviceController::Brightness",
                &error,
            )
        })?,
        "destack.device.camera.stream.brightness",
        "MediaDeviceControl",
    )?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read one Windows camera brightness range.
pub(crate) unsafe fn destack_device_camera_stream_brightness_range(
    binding: &BindingCallContext,
    out: *mut CameraFloatControlRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.brightnessRange",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.brightnessRange")?;
    let range = media_device_control_range(
        controller.Brightness().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.brightnessRange",
                "VideoDeviceController::Brightness",
                &error,
            )
        })?,
        "destack.device.camera.stream.brightnessRange",
        "MediaDeviceControl",
    )?;

    unsafe {
        out.write(range);
    }

    Ok(())
}

/// Write one Windows camera brightness value.
pub(crate) unsafe fn destack_device_camera_stream_set_brightness(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: f64,
) -> RuntimeResult<()> {
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setBrightness",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.setBrightness")?;

    set_media_device_control_value(
        controller.Brightness().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.setBrightness",
                "VideoDeviceController::Brightness",
                &error,
            )
        })?,
        value,
        "destack.device.camera.stream.setBrightness",
        "MediaDeviceControl",
    )
}

/// Read one Windows camera contrast value.
pub(crate) unsafe fn destack_device_camera_stream_contrast(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(binding, handle, "destack.device.camera.stream.contrast")?;
    let controller = video_device_controller(&resource, "destack.device.camera.stream.contrast")?;
    let value = media_device_control_value(
        controller.Contrast().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.contrast",
                "VideoDeviceController::Contrast",
                &error,
            )
        })?,
        "destack.device.camera.stream.contrast",
        "MediaDeviceControl",
    )?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read one Windows camera contrast range.
pub(crate) unsafe fn destack_device_camera_stream_contrast_range(
    binding: &BindingCallContext,
    out: *mut CameraFloatControlRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.contrastRange",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.contrastRange")?;
    let range = media_device_control_range(
        controller.Contrast().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.contrastRange",
                "VideoDeviceController::Contrast",
                &error,
            )
        })?,
        "destack.device.camera.stream.contrastRange",
        "MediaDeviceControl",
    )?;

    unsafe {
        out.write(range);
    }

    Ok(())
}

/// Write one Windows camera contrast value.
pub(crate) unsafe fn destack_device_camera_stream_set_contrast(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: f64,
) -> RuntimeResult<()> {
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.setContrast")?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.setContrast")?;

    set_media_device_control_value(
        controller.Contrast().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.setContrast",
                "VideoDeviceController::Contrast",
                &error,
            )
        })?,
        value,
        "destack.device.camera.stream.setContrast",
        "MediaDeviceControl",
    )
}

/// Read one Windows camera exposure compensation value.

pub(crate) unsafe fn destack_device_camera_stream_white_balance_kelvin(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.whiteBalanceKelvin",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.whiteBalanceKelvin")?;
    let control = controller.WhiteBalanceControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.whiteBalanceKelvin",
            "VideoDeviceController::WhiteBalanceControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.whiteBalanceKelvin",
            "WhiteBalanceControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.whiteBalanceKelvin",
        ));
    }

    unsafe {
        out.write(control.Value().map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.whiteBalanceKelvin",
                "WhiteBalanceControl::Value",
                &error,
            )
        })?);
    }

    Ok(())
}

/// Read one Windows camera white-balance range.
pub(crate) unsafe fn destack_device_camera_stream_white_balance_range(
    binding: &BindingCallContext,
    out: *mut CameraWhiteBalanceRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.whiteBalanceRange",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.whiteBalanceRange")?;
    let control = controller.WhiteBalanceControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.whiteBalanceRange",
            "VideoDeviceController::WhiteBalanceControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.whiteBalanceRange",
            "WhiteBalanceControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.whiteBalanceRange",
        ));
    }

    unsafe {
        out.write(CameraWhiteBalanceRange {
            minimum_kelvin: control.Min().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.whiteBalanceRange",
                    "WhiteBalanceControl::Min",
                    &error,
                )
            })?,
            maximum_kelvin: control.Max().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.whiteBalanceRange",
                    "WhiteBalanceControl::Max",
                    &error,
                )
            })?,
            default_kelvin: control.Value().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.whiteBalanceRange",
                    "WhiteBalanceControl::Value",
                    &error,
                )
            })?,
            step_kelvin: control.Step().map_err(|error| {
                windows_camera_error(
                    "destack.device.camera.stream.whiteBalanceRange",
                    "WhiteBalanceControl::Step",
                    &error,
                )
            })?,
            auto_supported: true,
        });
    }

    Ok(())
}

/// Write one Windows camera white-balance kelvin value.
pub(crate) unsafe fn destack_device_camera_stream_set_white_balance_kelvin(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: u32,
) -> RuntimeResult<()> {
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setWhiteBalanceKelvin",
    )?;
    let controller = video_device_controller(
        &resource,
        "destack.device.camera.stream.setWhiteBalanceKelvin",
    )?;
    let control = controller.WhiteBalanceControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setWhiteBalanceKelvin",
            "VideoDeviceController::WhiteBalanceControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setWhiteBalanceKelvin",
            "WhiteBalanceControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.setWhiteBalanceKelvin",
        ));
    }

    complete_action(
        control.SetValueAsync(value).map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.setWhiteBalanceKelvin",
                "WhiteBalanceControl::SetValueAsync",
                &error,
            )
        })?,
        "destack.device.camera.stream.setWhiteBalanceKelvin",
        "IAsyncAction::get",
    )
}

/// Read one Windows camera white-balance mode.
pub(crate) unsafe fn destack_device_camera_stream_white_balance_mode(
    binding: &BindingCallContext,
    out: *mut CameraWhiteBalanceMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.whiteBalanceMode",
    )?;
    let controller =
        video_device_controller(&resource, "destack.device.camera.stream.whiteBalanceMode")?;
    let control = controller.WhiteBalanceControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.whiteBalanceMode",
            "VideoDeviceController::WhiteBalanceControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.whiteBalanceMode",
            "WhiteBalanceControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.whiteBalanceMode",
        ));
    }

    unsafe {
        out.write(white_balance_mode_from_preset(control.Preset().map_err(
            |error| {
                windows_camera_error(
                    "destack.device.camera.stream.whiteBalanceMode",
                    "WhiteBalanceControl::Preset",
                    &error,
                )
            },
        )?));
    }

    Ok(())
}

/// Write one Windows camera white-balance mode.
pub(crate) unsafe fn destack_device_camera_stream_set_white_balance_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: CameraWhiteBalanceMode,
) -> RuntimeResult<()> {
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setWhiteBalanceMode",
    )?;
    let controller = video_device_controller(
        &resource,
        "destack.device.camera.stream.setWhiteBalanceMode",
    )?;
    let control = controller.WhiteBalanceControl().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setWhiteBalanceMode",
            "VideoDeviceController::WhiteBalanceControl",
            &error,
        )
    })?;
    if !control.Supported().map_err(|error| {
        windows_camera_error(
            "destack.device.camera.stream.setWhiteBalanceMode",
            "WhiteBalanceControl::Supported",
            &error,
        )
    })? {
        return Err(unsupported_windows_camera_control(
            "destack.device.camera.stream.setWhiteBalanceMode",
        ));
    }

    let preset = match value {
        CameraWhiteBalanceMode::Auto => ColorTemperaturePreset::Auto,
        CameraWhiteBalanceMode::Manual => ColorTemperaturePreset::Manual,
        CameraWhiteBalanceMode::ContinuousAuto => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "value",
                "continuous auto white balance is not available on this Windows camera backend",
            ))
            .boxed());
        }
    };

    complete_action(
        control.SetPresetAsync(preset).map_err(|error| {
            windows_camera_error(
                "destack.device.camera.stream.setWhiteBalanceMode",
                "WhiteBalanceControl::SetPresetAsync",
                &error,
            )
        })?,
        "destack.device.camera.stream.setWhiteBalanceMode",
        "IAsyncAction::get",
    )
}
