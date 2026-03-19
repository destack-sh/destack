use super::core::*;

/// Read one macOS camera focus distance.
pub(crate) unsafe fn destack_device_camera_stream_focus_distance_diopters(
    _binding: &BindingCallContext,
    _out: *mut f64,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.focusDistanceDiopters",
    ))
}

/// Read one macOS camera focus distance range.
pub(crate) unsafe fn destack_device_camera_stream_focus_distance_range(
    _binding: &BindingCallContext,
    _out: *mut CameraFocusDistanceRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.focusDistanceRange",
    ))
}

/// Write one macOS camera focus distance.
pub(crate) unsafe fn destack_device_camera_stream_set_focus_distance_diopters(
    _binding: &BindingCallContext,
    _handle: resource::CameraStreamHandle,
    _value: f64,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.setFocusDistanceDiopters",
    ))
}

/// Read one macOS camera focus mode.
pub(crate) unsafe fn destack_device_camera_stream_focus_mode(
    binding: &BindingCallContext,
    out: *mut CameraFocusMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.focusMode")?;

    // query the current focus mode
    let value = dispatch_device(
        &resource,
        "destack.device.camera.stream.focusMode",
        |device| {
            if !unsafe { device.isFocusModeSupported(AVCaptureFocusMode::AutoFocus) }
                && !unsafe { device.isFocusModeSupported(AVCaptureFocusMode::ContinuousAutoFocus) }
                && !unsafe { device.isFocusModeSupported(AVCaptureFocusMode::Locked) }
            {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.focusMode",
                ));
            }

            Ok(focus_mode_from_macos(unsafe { device.focusMode() }))
        },
    )?;

    // return the public mode
    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Write one macOS camera focus mode.
pub(crate) unsafe fn destack_device_camera_stream_set_focus_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: CameraFocusMode,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.setFocusMode")?;
    let value = unsafe { CameraFocusMode::into_value(value)? };

    // apply the requested focus mode
    with_locked_device(
        &resource,
        "destack.device.camera.stream.setFocusMode",
        |device| {
            match value {
                // autofocus once
                CameraFocusMode::Auto => {
                    if !unsafe { device.isFocusModeSupported(AVCaptureFocusMode::AutoFocus) } {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "value",
                            "auto focus is not available on this macOS camera backend",
                        ))
                        .boxed());
                    }

                    unsafe {
                        device.setFocusMode(AVCaptureFocusMode::AutoFocus);
                    }
                }

                // continuous autofocus
                CameraFocusMode::ContinuousAuto => {
                    if !unsafe {
                        device.isFocusModeSupported(AVCaptureFocusMode::ContinuousAutoFocus)
                    } {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "value",
                            "continuous auto focus is not available on this macOS camera backend",
                        ))
                        .boxed());
                    }

                    unsafe {
                        device.setFocusMode(AVCaptureFocusMode::ContinuousAutoFocus);
                    }
                }

                // manual focus locking
                CameraFocusMode::Manual => {
                    if !unsafe { device.isFocusModeSupported(AVCaptureFocusMode::Locked) } {
                        return Err(unsupported_camera_control(
                            "destack.device.camera.stream.setFocusMode",
                        ));
                    }

                    unsafe {
                        device.setFocusMode(AVCaptureFocusMode::Locked);
                    }
                }
            }

            Ok(())
        },
    )
}

/// Read one macOS camera pan angle.
pub(crate) unsafe fn destack_device_camera_stream_pan_degrees(
    _binding: &BindingCallContext,
    _out: *mut f64,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.panDegrees",
    ))
}

/// Read one macOS camera pan range.
pub(crate) unsafe fn destack_device_camera_stream_pan_range(
    _binding: &BindingCallContext,
    _out: *mut CameraPanAngleRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.panRange",
    ))
}

/// Write one macOS camera pan angle.
pub(crate) unsafe fn destack_device_camera_stream_set_pan_degrees(
    _binding: &BindingCallContext,
    _handle: resource::CameraStreamHandle,
    _value: f64,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.setPanDegrees",
    ))
}

/// Read one macOS camera stabilization mode.
pub(crate) unsafe fn destack_device_camera_stream_stabilization_mode(
    binding: &BindingCallContext,
    out: *mut CameraStabilizationMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.stabilizationMode",
    )?;

    // query the current stabilization mode
    let value = dispatch_connection(
        &resource,
        "destack.device.camera.stream.stabilizationMode",
        |connection| {
            if !unsafe { connection.isVideoStabilizationSupported() } {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.stabilizationMode",
                ));
            }

            Ok(stabilization_mode_from_macos(unsafe {
                connection.activeVideoStabilizationMode()
            }))
        },
    )?;

    // return the public mode
    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Write one macOS camera stabilization mode.
pub(crate) unsafe fn destack_device_camera_stream_set_stabilization_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: CameraStabilizationMode,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.setStabilizationMode",
    )?;
    let value = unsafe { CameraStabilizationMode::into_value(value)? };

    // apply the requested stabilization mode
    dispatch_device_connection(
        &resource,
        "destack.device.camera.stream.setStabilizationMode",
        |device, connection| {
            if !unsafe { connection.isVideoStabilizationSupported() } {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.setStabilizationMode",
                ));
            }

            let format = unsafe { device.activeFormat() };
            let mode = preferred_stabilization_mode(&format, value).ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "value",
                    "requested stabilization mode is not available on this macOS camera backend",
                ))
                .boxed()
            })?;

            unsafe {
                connection.setPreferredVideoStabilizationMode(mode);
            }

            Ok(())
        },
    )
}

/// Read one macOS camera tilt angle.
pub(crate) unsafe fn destack_device_camera_stream_tilt_degrees(
    _binding: &BindingCallContext,
    _out: *mut f64,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.tiltDegrees",
    ))
}

/// Read one macOS camera tilt range.
pub(crate) unsafe fn destack_device_camera_stream_tilt_range(
    _binding: &BindingCallContext,
    _out: *mut CameraTiltAngleRange,
    _handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.tiltRange",
    ))
}

/// Write one macOS camera tilt angle.
pub(crate) unsafe fn destack_device_camera_stream_set_tilt_degrees(
    _binding: &BindingCallContext,
    _handle: resource::CameraStreamHandle,
    _value: f64,
) -> RuntimeResult<()> {
    Err(unsupported_camera_control(
        "destack.device.camera.stream.setTiltDegrees",
    ))
}

/// Read one macOS camera torch mode.
pub(crate) unsafe fn destack_device_camera_stream_torch_mode(
    binding: &BindingCallContext,
    out: *mut CameraTorchMode,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.torchMode")?;

    // query the current torch mode
    let value = dispatch_device(
        &resource,
        "destack.device.camera.stream.torchMode",
        |device| {
            if !unsafe { device.hasTorch() && device.isTorchAvailable() } {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.torchMode",
                ));
            }

            Ok(torch_mode_from_macos(unsafe { device.torchMode() }))
        },
    )?;

    // return the public mode
    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Write one macOS camera torch mode.
pub(crate) unsafe fn destack_device_camera_stream_set_torch_mode(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: CameraTorchMode,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.setTorchMode")?;
    let value = unsafe { CameraTorchMode::into_value(value)? };

    // apply the requested torch mode
    with_locked_device(
        &resource,
        "destack.device.camera.stream.setTorchMode",
        |device| {
            if !unsafe { device.hasTorch() && device.isTorchAvailable() } {
                return Err(unsupported_camera_control(
                    "destack.device.camera.stream.setTorchMode",
                ));
            }

            match value {
                CameraTorchMode::Off => unsafe {
                    device.setTorchMode(AVCaptureTorchMode::Off);
                },
                CameraTorchMode::On => {
                    if !unsafe { device.isTorchModeSupported(AVCaptureTorchMode::On) } {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "value",
                            "torch on is not available on this macOS camera backend",
                        ))
                        .boxed());
                    }

                    unsafe {
                        device.setTorchMode(AVCaptureTorchMode::On);
                    }
                }
                CameraTorchMode::Auto => {
                    if !unsafe { device.isTorchModeSupported(AVCaptureTorchMode::Auto) } {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "value",
                            "automatic torch is not available on this macOS camera backend",
                        ))
                        .boxed());
                    }

                    unsafe {
                        device.setTorchMode(AVCaptureTorchMode::Auto);
                    }
                }
            }

            Ok(())
        },
    )
}

/// Read one macOS camera zoom ratio.
pub(crate) unsafe fn destack_device_camera_stream_zoom_ratio(
    binding: &BindingCallContext,
    out: *mut f64,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.zoomRatio")?;

    // read the current zoom factor
    let value = dispatch_device(
        &resource,
        "destack.device.camera.stream.zoomRatio",
        |device| Ok(unsafe { device.videoZoomFactor() }),
    )?;

    // return the public zoom factor
    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Read one macOS camera zoom ratio range.
pub(crate) unsafe fn destack_device_camera_stream_zoom_ratio_range(
    binding: &BindingCallContext,
    out: *mut CameraZoomRatioRange,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // resolve the selected stream
    let resource = stream_resource(
        binding,
        handle,
        "destack.device.camera.stream.zoomRatioRange",
    )?;

    // read the supported zoom range
    let range = dispatch_device(
        &resource,
        "destack.device.camera.stream.zoomRatioRange",
        |device| {
            Ok(CameraZoomRatioRange {
                minimum_ratio: unsafe { device.minAvailableVideoZoomFactor() },
                maximum_ratio: unsafe { device.maxAvailableVideoZoomFactor() },
                default_ratio: 1.0,
                step_ratio: 0.0,
            })
        },
    )?;

    // return the public range
    unsafe {
        out.write(range);
    }

    Ok(())
}

/// Write one macOS camera zoom ratio.
pub(crate) unsafe fn destack_device_camera_stream_set_zoom_ratio(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    value: f64,
) -> RuntimeResult<()> {
    // resolve the selected stream
    let resource = stream_resource(binding, handle, "destack.device.camera.stream.setZoomRatio")?;
    let value = value as f32;

    // apply the requested zoom factor
    with_locked_device(
        &resource,
        "destack.device.camera.stream.setZoomRatio",
        |device| {
            // validate the requested zoom factor
            let minimum_ratio = unsafe { device.minAvailableVideoZoomFactor() };
            let maximum_ratio = unsafe { device.maxAvailableVideoZoomFactor() };
            let value = value as f64;
            if value < minimum_ratio || value > maximum_ratio {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "value",
                    "camera zoom ratio is outside the supported range",
                ))
                .boxed());
            }

            unsafe {
                device.setVideoZoomFactor(value);
            }

            Ok(())
        },
    )
}
