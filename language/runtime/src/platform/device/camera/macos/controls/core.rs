pub(super) use crate::platform::device::camera::macos::core::*;
pub(super) use crate::platform::device::camera::macos::metadata::*;
use std::panic::AssertUnwindSafe;

/// Resolve one opened macOS camera stream resource.
pub(super) fn stream_resource(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<MacosCameraStreamResource>> {
    camera_stream_resource::<MacosCameraStreamResource>(binding, handle, operation)
}

/// Build one not-supported error for one macOS camera control.
pub(super) fn unsupported_camera_control(operation: &'static str) -> Box<RuntimeError> {
    camera_not_supported(operation)
}

/// Convert one `CMTime` into nanoseconds.
pub(super) fn camera_duration_ns(duration: CMTime) -> u64 {
    let seconds = unsafe { duration.seconds() };
    if !seconds.is_finite() || seconds.is_sign_negative() {
        return 0;
    }

    let nanoseconds = seconds * 1_000_000_000.0;

    nanoseconds.clamp(0.0, u64::MAX as f64) as u64
}

/// Convert one nanosecond duration into one `CMTime`.
pub(super) fn camera_duration_from_ns(nanoseconds: u64) -> CMTime {
    let seconds = nanoseconds as f64 / 1_000_000_000.0;

    unsafe { CMTime::with_seconds(seconds, CAMERA_FRAME_TIME_SCALE) }
}

/// Clamp one kelvin value into one practical camera range.
pub(super) fn camera_kelvin_value(temperature: f32) -> u32 {
    if !temperature.is_finite() {
        return 0;
    }

    temperature.round().clamp(0.0, u32::MAX as f32) as u32
}

/// Catch one Objective-C exception from the macOS control path.
fn catch_control_exception<R>(
    operation: &'static str,
    action_name: &'static str,
    callback: impl FnOnce() -> RuntimeResult<R>,
) -> RuntimeResult<R> {
    exception::catch(AssertUnwindSafe(callback)).map_err(|exception| {
        let message = exception
            .map(|exception| exception.to_string())
            .unwrap_or_else(|| String::from("unknown Objective-C exception"));

        core_platform::io_operation_error(
            operation,
            None,
            format!("{action_name} raised Objective-C exception: {message}"),
        )
    })?
}

/// Dispatch one read-only closure on the stream's camera queue.
pub(super) fn dispatch_device<F, R>(
    resource: &MacosCameraStreamResource,
    operation: &'static str,
    callback: F,
) -> RuntimeResult<R>
where
    F: FnOnce(&AVCaptureDevice) -> RuntimeResult<R> + Send,
    R: Send,
{
    resource
        .device
        .dispatch_on(resource.callback_queue.as_ref(), |device| {
            catch_control_exception(operation, "AVCaptureDevice operation", || callback(device))
        })
}

/// Dispatch one read-only closure on the stream's video connection queue.
pub(super) fn dispatch_connection<F, R>(
    resource: &MacosCameraStreamResource,
    operation: &'static str,
    callback: F,
) -> RuntimeResult<R>
where
    F: FnOnce(&AVCaptureConnection) -> RuntimeResult<R> + Send,
    R: Send,
{
    resource
        .connection
        .dispatch_on(resource.callback_queue.as_ref(), |connection| {
            catch_control_exception(operation, "AVCaptureConnection operation", || {
                callback(connection)
            })
        })
}

/// Dispatch one closure with both the live device and video connection.
pub(super) fn dispatch_device_connection<F, R>(
    resource: &MacosCameraStreamResource,
    operation: &'static str,
    callback: F,
) -> RuntimeResult<R>
where
    F: FnOnce(&AVCaptureDevice, &AVCaptureConnection) -> RuntimeResult<R> + Send,
    R: Send,
{
    resource
        .device
        .dispatch_on(resource.callback_queue.as_ref(), |device| {
            catch_control_exception(
                operation,
                "AVCaptureDevice and AVCaptureConnection operation",
                || {
                    // shared queue access
                    let connection = unsafe { resource.connection.get_unchecked() };

                    callback(device, connection)
                },
            )
        })
}

/// Lock one live AVFoundation device for configuration while running one closure.
pub(super) fn with_locked_device<F, R>(
    resource: &MacosCameraStreamResource,
    operation: &'static str,
    callback: F,
) -> RuntimeResult<R>
where
    F: FnOnce(&AVCaptureDevice) -> RuntimeResult<R> + Send,
    R: Send,
{
    dispatch_device(resource, operation, |device| {
        // configuration lock
        unsafe { device.lockForConfiguration() }.map_err(|error| {
            macos_camera_nserror(operation, "AVCaptureDevice::lockForConfiguration", &error)
        })?;

        // configuration body
        let result = callback(device);

        // configuration unlock
        unsafe {
            device.unlockForConfiguration();
        }

        result
    })
}

/// Convert one AVFoundation exposure mode into one public value.
pub(super) fn exposure_mode_from_macos(mode: AVCaptureExposureMode) -> CameraExposureMode {
    if mode == AVCaptureExposureMode::ContinuousAutoExposure {
        return CameraExposureMode::ContinuousAuto;
    }

    if mode == AVCaptureExposureMode::AutoExpose {
        return CameraExposureMode::Auto;
    }

    CameraExposureMode::Manual
}

/// Convert one AVFoundation white-balance mode into one public value.
pub(super) fn white_balance_mode_from_macos(
    mode: AVCaptureWhiteBalanceMode,
) -> CameraWhiteBalanceMode {
    if mode == AVCaptureWhiteBalanceMode::ContinuousAutoWhiteBalance {
        return CameraWhiteBalanceMode::ContinuousAuto;
    }

    if mode == AVCaptureWhiteBalanceMode::AutoWhiteBalance {
        return CameraWhiteBalanceMode::Auto;
    }

    CameraWhiteBalanceMode::Manual
}

/// Convert one AVFoundation focus mode into one public value.
pub(super) fn focus_mode_from_macos(mode: AVCaptureFocusMode) -> CameraFocusMode {
    if mode == AVCaptureFocusMode::ContinuousAutoFocus {
        return CameraFocusMode::ContinuousAuto;
    }

    if mode == AVCaptureFocusMode::AutoFocus {
        return CameraFocusMode::Auto;
    }

    CameraFocusMode::Manual
}

/// Convert one AVFoundation torch mode into one public value.
pub(super) fn torch_mode_from_macos(mode: AVCaptureTorchMode) -> CameraTorchMode {
    if mode == AVCaptureTorchMode::Auto {
        return CameraTorchMode::Auto;
    }

    if mode == AVCaptureTorchMode::On {
        return CameraTorchMode::On;
    }

    CameraTorchMode::Off
}

/// Convert one AVFoundation stabilization mode into one public value.
pub(super) fn stabilization_mode_from_macos(
    mode: AVCaptureVideoStabilizationMode,
) -> CameraStabilizationMode {
    if mode == AVCaptureVideoStabilizationMode::CinematicExtended
        || mode == AVCaptureVideoStabilizationMode::CinematicExtendedEnhanced
    {
        return CameraStabilizationMode::HighQuality;
    }

    if mode != AVCaptureVideoStabilizationMode::Off {
        return CameraStabilizationMode::Standard;
    }

    CameraStabilizationMode::Off
}

/// Choose one AVFoundation stabilization mode for one requested public value.
pub(super) fn preferred_stabilization_mode(
    format: &AVCaptureDeviceFormat,
    value: CameraStabilizationMode,
) -> Option<AVCaptureVideoStabilizationMode> {
    // explicit off
    if value == CameraStabilizationMode::Off {
        return Some(AVCaptureVideoStabilizationMode::Off);
    }

    // higher quality first
    if value == CameraStabilizationMode::HighQuality {
        if unsafe {
            format.isVideoStabilizationModeSupported(
                AVCaptureVideoStabilizationMode::CinematicExtendedEnhanced,
            )
        } {
            return Some(AVCaptureVideoStabilizationMode::CinematicExtendedEnhanced);
        }

        if unsafe {
            format.isVideoStabilizationModeSupported(
                AVCaptureVideoStabilizationMode::CinematicExtended,
            )
        } {
            return Some(AVCaptureVideoStabilizationMode::CinematicExtended);
        }

        return None;
    }

    // standard quality fallbacks
    if unsafe {
        format.isVideoStabilizationModeSupported(AVCaptureVideoStabilizationMode::Standard)
    } {
        return Some(AVCaptureVideoStabilizationMode::Standard);
    }

    if unsafe {
        format.isVideoStabilizationModeSupported(AVCaptureVideoStabilizationMode::Cinematic)
    } {
        return Some(AVCaptureVideoStabilizationMode::Cinematic);
    }

    if unsafe {
        format.isVideoStabilizationModeSupported(AVCaptureVideoStabilizationMode::PreviewOptimized)
    } {
        return Some(AVCaptureVideoStabilizationMode::PreviewOptimized);
    }

    if unsafe {
        format.isVideoStabilizationModeSupported(AVCaptureVideoStabilizationMode::LowLatency)
    } {
        return Some(AVCaptureVideoStabilizationMode::LowLatency);
    }

    if unsafe { format.isVideoStabilizationModeSupported(AVCaptureVideoStabilizationMode::Auto) } {
        return Some(AVCaptureVideoStabilizationMode::Auto);
    }

    None
}
