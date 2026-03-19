use super::core::*;

/// Read one macOS camera exposure compensation value.
pub(crate) unsafe fn destack_device_camera_stream_exposure_compensation(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.exposureCompensation",
    )?;

    // read the current exposure bias
    let value = dispatch_device(
        &resource,
        "destack.device.camera.stream.exposureCompensation",
        |device| Ok(unsafe { device.exposureTargetBias() } as f64),
    )?;

    // return the public value
    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read one macOS camera exposure compensation range.
pub(crate) unsafe fn destack_device_camera_stream_exposure_compensation_range(
    binding: &BindingCallContext,
    out: *mut CameraExposureCompensationRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.exposureCompensationRange",
    )?;

    // read the supported exposure-bias range
    let range = dispatch_device(
        &resource,
        "destack.device.camera.stream.exposureCompensationRange",
        |device| {
            Ok(CameraExposureCompensationRange {
                minimum_ev: unsafe { device.minExposureTargetBias() } as f64,
                maximum_ev: unsafe { device.maxExposureTargetBias() } as f64,
                default_ev: 0.0,
                step_ev: 0.0,
                auto_supported: unsafe {
                    device.isExposureModeSupported(AVCaptureExposureMode::AutoExpose)
                } || unsafe {
                    device.isExposureModeSupported(AVCaptureExposureMode::ContinuousAutoExposure)
                },
            })
        },
    )?;

    // return the public range
    unsafe {
        out.write(range);
    }

    Ok(())
}

/// Write one macOS camera exposure compensation value.
pub(crate) unsafe fn destack_device_camera_stream_set_exposure_compensation(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: f64,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setExposureCompensation",
    )?;
    let value = value as f32;

    // apply the requested exposure bias
    with_locked_device(
        &resource,
        "destack.device.camera.stream.setExposureCompensation",
        |device| {
            // validate the requested bias
            let minimum_bias = unsafe { device.minExposureTargetBias() };
            let maximum_bias = unsafe { device.maxExposureTargetBias() };
            if value < minimum_bias || value > maximum_bias {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "value",
                    "camera exposure compensation is outside the supported range",
                ))
                .boxed());
            }

            unsafe { device.setExposureTargetBias_completionHandler(value, None) };

            Ok(())
        },
    )
}

/// Read one macOS camera exposure mode.
pub(crate) unsafe fn destack_device_camera_stream_exposure_mode(
    binding: &BindingCallContext,
    out: *mut CameraExposureMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.exposureMode")?;

    // query the current exposure mode
    let value = dispatch_device(
        &resource,
        "destack.device.camera.stream.exposureMode",
        |device| {
            if !unsafe { device.isExposureModeSupported(AVCaptureExposureMode::AutoExpose) }
                && !unsafe {
                    device.isExposureModeSupported(AVCaptureExposureMode::ContinuousAutoExposure)
                }
                && !unsafe { device.isExposureModeSupported(AVCaptureExposureMode::Locked) }
                && !unsafe { device.isExposureModeSupported(AVCaptureExposureMode::Custom) }
            {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.exposureMode",
                ));
            }

            Ok(exposure_mode_from_macos(unsafe { device.exposureMode() }))
        },
    )?;

    // return the public mode
    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Write one macOS camera exposure mode.
pub(crate) unsafe fn destack_device_camera_stream_set_exposure_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: CameraExposureMode,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setExposureMode",
    )?;
    let value = unsafe { CameraExposureMode::into_value(value)? };

    // apply the requested exposure mode
    with_locked_device(
        &resource,
        "destack.device.camera.stream.setExposureMode",
        |device| {
            match value {
                // auto once
                CameraExposureMode::Auto => {
                    if !unsafe { device.isExposureModeSupported(AVCaptureExposureMode::AutoExpose) }
                    {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "value",
                            "auto exposure is not available on this macOS camera backend",
                        ))
                        .boxed());
                    }

                    unsafe {
                        device.setExposureMode(AVCaptureExposureMode::AutoExpose);
                    }
                }

                // continuous auto
                CameraExposureMode::ContinuousAuto => {
                    if !unsafe {
                        device
                            .isExposureModeSupported(AVCaptureExposureMode::ContinuousAutoExposure)
                    } {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "value",
                            "continuous auto exposure is not available on this macOS camera backend",
                        ))
                        .boxed());
                    }

                    unsafe {
                        device.setExposureMode(AVCaptureExposureMode::ContinuousAutoExposure);
                    }
                }

                // manual locking
                CameraExposureMode::Manual => {
                    if unsafe { device.isExposureModeSupported(AVCaptureExposureMode::Locked) } {
                        unsafe {
                            device.setExposureMode(AVCaptureExposureMode::Locked);
                        }
                    } else if unsafe {
                        device.isExposureModeSupported(AVCaptureExposureMode::Custom)
                    } {
                        unsafe {
                            device.setExposureModeCustomWithDuration_ISO_completionHandler(
                                AVCaptureExposureDurationCurrent,
                                AVCaptureISOCurrent,
                                None,
                            );
                        }
                    } else {
                        return Err(unsupported_camera_control(
                            "destack.device.camera.stream.setExposureMode",
                        ));
                    }
                }
            }

            Ok(())
        },
    )
}

/// Read one macOS camera exposure time.
pub(crate) unsafe fn destack_device_camera_stream_exposure_time_ns(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.exposureTimeNs",
    )?;

    // read the current exposure duration
    let value = dispatch_device(
        &resource,
        "destack.device.camera.stream.exposureTimeNs",
        |device| {
            if !unsafe { device.isExposureModeSupported(AVCaptureExposureMode::Locked) }
                && !unsafe { device.isExposureModeSupported(AVCaptureExposureMode::Custom) }
            {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.exposureTimeNs",
                ));
            }

            Ok(camera_duration_ns(unsafe { device.exposureDuration() }))
        },
    )?;

    // return the public duration
    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read one macOS camera exposure time range.
pub(crate) unsafe fn destack_device_camera_stream_exposure_time_range(
    binding: &BindingCallContext,
    out: *mut CameraExposureTimeRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.exposureTimeRange",
    )?;

    // read the current format's exposure limits
    let range = dispatch_device(
        &resource,
        "destack.device.camera.stream.exposureTimeRange",
        |device| {
            let format = unsafe { device.activeFormat() };

            Ok(CameraExposureTimeRange {
                minimum_ns: camera_duration_ns(unsafe { format.minExposureDuration() }),
                maximum_ns: camera_duration_ns(unsafe { format.maxExposureDuration() }),
                default_ns: camera_duration_ns(unsafe { device.exposureDuration() }),
                step_ns: 0,
                auto_supported: unsafe {
                    device.isExposureModeSupported(AVCaptureExposureMode::AutoExpose)
                } || unsafe {
                    device.isExposureModeSupported(AVCaptureExposureMode::ContinuousAutoExposure)
                },
            })
        },
    )?;

    // return the public range
    unsafe {
        out.write(range);
    }

    Ok(())
}

/// Write one macOS camera exposure time.
pub(crate) unsafe fn destack_device_camera_stream_set_exposure_time_ns(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: u64,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setExposureTimeNs",
    )?;
    let duration = camera_duration_from_ns(value);

    // apply one custom exposure duration
    with_locked_device(
        &resource,
        "destack.device.camera.stream.setExposureTimeNs",
        |device| {
            if !unsafe { device.isExposureModeSupported(AVCaptureExposureMode::Custom) } {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.setExposureTimeNs",
                ));
            }

            // validate the requested duration
            let format = unsafe { device.activeFormat() };
            let minimum_duration = unsafe { format.minExposureDuration() };
            let maximum_duration = unsafe { format.maxExposureDuration() };
            let requested_ns = camera_duration_ns(duration);
            let minimum_ns = camera_duration_ns(minimum_duration);
            let maximum_ns = camera_duration_ns(maximum_duration);
            if requested_ns < minimum_ns || requested_ns > maximum_ns {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "value",
                    "camera exposure time is outside the supported range",
                ))
                .boxed());
            }

            unsafe {
                device.setExposureModeCustomWithDuration_ISO_completionHandler(
                    duration,
                    AVCaptureISOCurrent,
                    None,
                );
            }

            Ok(())
        },
    )
}

/// Read one macOS camera sensor ISO value.
pub(crate) unsafe fn destack_device_camera_stream_sensor_iso(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.sensorIso")?;

    // read the current ISO value
    let value = dispatch_device(
        &resource,
        "destack.device.camera.stream.sensorIso",
        |device| {
            if !unsafe { device.isExposureModeSupported(AVCaptureExposureMode::Custom) }
                && !unsafe { device.isExposureModeSupported(AVCaptureExposureMode::Locked) }
            {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.sensorIso",
                ));
            }

            let value = unsafe { device.ISO() }.round().clamp(0.0, u32::MAX as f32) as u32;

            Ok(value)
        },
    )?;

    // return the public ISO value
    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read one macOS camera sensor ISO range.
pub(crate) unsafe fn destack_device_camera_stream_sensor_iso_range(
    binding: &BindingCallContext,
    out: *mut CameraSensorIsoRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.sensorIsoRange",
    )?;

    // read the current format's ISO limits
    let range = dispatch_device(
        &resource,
        "destack.device.camera.stream.sensorIsoRange",
        |device| {
            let format = unsafe { device.activeFormat() };

            Ok(CameraSensorIsoRange {
                minimum: unsafe { format.minISO() }
                    .round()
                    .clamp(0.0, u32::MAX as f32) as u32,
                maximum: unsafe { format.maxISO() }
                    .round()
                    .clamp(0.0, u32::MAX as f32) as u32,
                default: unsafe { device.ISO() }.round().clamp(0.0, u32::MAX as f32) as u32,
                step: 0,
                auto_supported: unsafe {
                    device.isExposureModeSupported(AVCaptureExposureMode::AutoExpose)
                } || unsafe {
                    device.isExposureModeSupported(AVCaptureExposureMode::ContinuousAutoExposure)
                },
            })
        },
    )?;

    // return the public range
    unsafe {
        out.write(range);
    }

    Ok(())
}

/// Write one macOS camera sensor ISO value.
pub(crate) unsafe fn destack_device_camera_stream_set_sensor_iso(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: u32,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.setSensorIso")?;
    let iso = value as f32;

    // apply one custom ISO value
    with_locked_device(
        &resource,
        "destack.device.camera.stream.setSensorIso",
        |device| {
            if !unsafe { device.isExposureModeSupported(AVCaptureExposureMode::Custom) } {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.setSensorIso",
                ));
            }

            // validate the requested ISO
            let format = unsafe { device.activeFormat() };
            let minimum_iso = unsafe { format.minISO() };
            let maximum_iso = unsafe { format.maxISO() };
            if iso < minimum_iso || iso > maximum_iso {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "value",
                    "camera ISO is outside the supported range",
                ))
                .boxed());
            }

            unsafe {
                device.setExposureModeCustomWithDuration_ISO_completionHandler(
                    AVCaptureExposureDurationCurrent,
                    iso,
                    None,
                );
            }

            Ok(())
        },
    )
}
