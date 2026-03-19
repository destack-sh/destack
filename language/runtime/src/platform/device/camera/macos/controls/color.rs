use super::core::*;

/// Clamp one white-balance gain set into the device-supported range.
fn clamped_white_balance_gains(
    device: &AVCaptureDevice,
    values: AVCaptureWhiteBalanceTemperatureAndTintValues,
) -> objc2_av_foundation::AVCaptureWhiteBalanceGains {
    let gains = unsafe { device.deviceWhiteBalanceGainsForTemperatureAndTintValues(values) };
    let maximum_gain = unsafe { device.maxWhiteBalanceGain() };

    objc2_av_foundation::AVCaptureWhiteBalanceGains {
        redGain: gains.redGain.clamp(1.0, maximum_gain),
        greenGain: gains.greenGain.clamp(1.0, maximum_gain),
        blueGain: gains.blueGain.clamp(1.0, maximum_gain),
    }
}

/// Read one macOS camera brightness value.
pub(crate) unsafe fn destack_device_camera_stream_brightness(
    _binding: &BindingCallContext,
    _out: *mut f64,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.brightness",
    ))
}

/// Read one macOS camera brightness range.
pub(crate) unsafe fn destack_device_camera_stream_brightness_range(
    _binding: &BindingCallContext,
    _out: *mut CameraFloatControlRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.brightnessRange",
    ))
}

/// Write one macOS camera brightness value.
pub(crate) unsafe fn destack_device_camera_stream_set_brightness(
    _binding: &BindingCallContext,
    _handle: resource::CameraStreamHandle,
    _value: f64,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.setBrightness",
    ))
}

/// Read one macOS camera contrast value.
pub(crate) unsafe fn destack_device_camera_stream_contrast(
    _binding: &BindingCallContext,
    _out: *mut f64,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.contrast",
    ))
}

/// Read one macOS camera contrast range.
pub(crate) unsafe fn destack_device_camera_stream_contrast_range(
    _binding: &BindingCallContext,
    _out: *mut CameraFloatControlRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.contrastRange",
    ))
}

/// Write one macOS camera contrast value.
pub(crate) unsafe fn destack_device_camera_stream_set_contrast(
    _binding: &BindingCallContext,
    _handle: resource::CameraStreamHandle,
    _value: f64,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.setContrast",
    ))
}

/// Read one macOS camera saturation value.
pub(crate) unsafe fn destack_device_camera_stream_saturation(
    _binding: &BindingCallContext,
    _out: *mut f64,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.saturation",
    ))
}

/// Read one macOS camera saturation range.
pub(crate) unsafe fn destack_device_camera_stream_saturation_range(
    _binding: &BindingCallContext,
    _out: *mut CameraFloatControlRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.saturationRange",
    ))
}

/// Write one macOS camera saturation value.
pub(crate) unsafe fn destack_device_camera_stream_set_saturation(
    _binding: &BindingCallContext,
    _handle: resource::CameraStreamHandle,
    _value: f64,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.setSaturation",
    ))
}

/// Read one macOS camera sharpness value.
pub(crate) unsafe fn destack_device_camera_stream_sharpness(
    _binding: &BindingCallContext,
    _out: *mut f64,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.sharpness",
    ))
}

/// Read one macOS camera sharpness range.
pub(crate) unsafe fn destack_device_camera_stream_sharpness_range(
    _binding: &BindingCallContext,
    _out: *mut CameraFloatControlRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.sharpnessRange",
    ))
}

/// Write one macOS camera sharpness value.
pub(crate) unsafe fn destack_device_camera_stream_set_sharpness(
    _binding: &BindingCallContext,
    _handle: resource::CameraStreamHandle,
    _value: f64,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.setSharpness",
    ))
}

/// Read one macOS camera white-balance mode.
pub(crate) unsafe fn destack_device_camera_stream_white_balance_mode(
    binding: &BindingCallContext,
    out: *mut CameraWhiteBalanceMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.whiteBalanceMode",
    )?;

    // query the current white-balance mode
    let mode = dispatch_device(
        &resource,
        "destack.device.camera.stream.whiteBalanceMode",
        |device| {
            if !unsafe {
                device.isWhiteBalanceModeSupported(AVCaptureWhiteBalanceMode::AutoWhiteBalance)
            } && !unsafe {
                device.isWhiteBalanceModeSupported(
                    AVCaptureWhiteBalanceMode::ContinuousAutoWhiteBalance,
                )
            } && !unsafe {
                device.isWhiteBalanceModeSupported(AVCaptureWhiteBalanceMode::Locked)
            } {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.whiteBalanceMode",
                ));
            }

            Ok(white_balance_mode_from_macos(unsafe {
                device.whiteBalanceMode()
            }))
        },
    )?;

    // return the public mode
    unsafe {
        out.write(mode);
    }

    Ok(())
}

/// Write one macOS camera white-balance mode.
pub(crate) unsafe fn destack_device_camera_stream_set_white_balance_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: CameraWhiteBalanceMode,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setWhiteBalanceMode",
    )?;
    let value = unsafe { CameraWhiteBalanceMode::into_value(value)? };

    // apply the requested mode
    with_locked_device(
        &resource,
        "destack.device.camera.stream.setWhiteBalanceMode",
        |device| {
            match value {
                // auto once
                CameraWhiteBalanceMode::Auto => {
                    if !unsafe {
                        device.isWhiteBalanceModeSupported(
                            AVCaptureWhiteBalanceMode::AutoWhiteBalance,
                        )
                    } {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "value",
                            "auto white balance is not available on this macOS camera backend",
                        ))
                        .boxed());
                    }

                    unsafe {
                        device.setWhiteBalanceMode(AVCaptureWhiteBalanceMode::AutoWhiteBalance);
                    }
                }

                // continuous auto
                CameraWhiteBalanceMode::ContinuousAuto => {
                    if !unsafe {
                        device.isWhiteBalanceModeSupported(
                            AVCaptureWhiteBalanceMode::ContinuousAutoWhiteBalance,
                        )
                    } {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "value",
                            "continuous auto white balance is not available on this macOS camera backend",
                        ))
                        .boxed());
                    }

                    unsafe {
                        device.setWhiteBalanceMode(
                            AVCaptureWhiteBalanceMode::ContinuousAutoWhiteBalance,
                        );
                    }
                }

                // lock the current white balance
                CameraWhiteBalanceMode::Manual => {
                    if !unsafe {
                        device.isWhiteBalanceModeSupported(AVCaptureWhiteBalanceMode::Locked)
                    } {
                        return Err(unsupported_camera_control(
                            "destack.device.camera.stream.setWhiteBalanceMode",
                        ));
                    }

                    unsafe {
                        device.setWhiteBalanceMode(AVCaptureWhiteBalanceMode::Locked);
                    }
                }
            }

            Ok(())
        },
    )
}

/// Read one macOS camera white-balance value in kelvin.
pub(crate) unsafe fn destack_device_camera_stream_white_balance_kelvin(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.whiteBalanceKelvin",
    )?;

    // convert the current gains into one temperature value
    let value = dispatch_device(
        &resource,
        "destack.device.camera.stream.whiteBalanceKelvin",
        |device| {
            if !unsafe { device.isWhiteBalanceModeSupported(AVCaptureWhiteBalanceMode::Locked) } {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.whiteBalanceKelvin",
                ));
            }

            let gains = unsafe { device.deviceWhiteBalanceGains() };
            let value = unsafe { device.temperatureAndTintValuesForDeviceWhiteBalanceGains(gains) };

            Ok(camera_kelvin_value(value.temperature))
        },
    )?;

    // return the public kelvin value
    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read one macOS camera white-balance range.
pub(crate) unsafe fn destack_device_camera_stream_white_balance_range(
    _binding: &BindingCallContext,
    _out: *mut CameraWhiteBalanceRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.whiteBalanceRange",
    ))
}

/// Write one macOS camera white-balance value in kelvin.
pub(crate) unsafe fn destack_device_camera_stream_set_white_balance_kelvin(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: u32,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setWhiteBalanceKelvin",
    )?;
    let temperature = value as f32;

    // apply one manual white-balance target
    with_locked_device(
        &resource,
        "destack.device.camera.stream.setWhiteBalanceKelvin",
        |device| {
            if !unsafe { device.isWhiteBalanceModeSupported(AVCaptureWhiteBalanceMode::Locked) }
                || !unsafe { device.isLockingWhiteBalanceWithCustomDeviceGainsSupported() }
            {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.setWhiteBalanceKelvin",
                ));
            }

            let values = AVCaptureWhiteBalanceTemperatureAndTintValues {
                temperature,
                tint: 0.0,
            };
            let gains = clamped_white_balance_gains(device, values);

            unsafe {
                device.setWhiteBalanceModeLockedWithDeviceWhiteBalanceGains_completionHandler(
                    gains, None,
                );
            }

            Ok(())
        },
    )
}
