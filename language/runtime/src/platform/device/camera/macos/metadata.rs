#![allow(unsafe_op_in_unsafe_fn)]

use std::collections::BTreeMap;

use super::core::*;
use crate::platform::device::CameraColorSpace;

/// Return one runtime error for one AVFoundation operation.
pub(super) fn macos_camera_error(
    operation: &'static str,
    action: &'static str,
    error: impl std::fmt::Display,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        None,
        Some(operation.to_string()),
        Some(action.to_string()),
        format!("{action} failed: {error}"),
    ))
    .boxed()
}

/// Return one runtime error for one NSError.
pub(super) fn macos_camera_nserror(
    operation: &'static str,
    action: &'static str,
    error: &NSError,
) -> Box<RuntimeError> {
    macos_camera_error(operation, action, error)
}

/// Return the public descriptor identifier for one AVFoundation device.
pub(super) fn camera_descriptor_id(unique_id: &str) -> String {
    format!("{MACOS_CAMERA_DESCRIPTOR_PREFIX}{unique_id}")
}

/// Decode one public descriptor identifier into one AVFoundation unique identifier.
pub(super) fn camera_unique_id_from_descriptor_id(id: &str) -> Option<&str> {
    id.strip_prefix(MACOS_CAMERA_DESCRIPTOR_PREFIX)
}

/// Convert one host-facing mode into one public facing-mode value.
fn camera_facing_mode(device: &AVCaptureDevice) -> CameraFacingMode {
    let position = unsafe { device.position() };
    if position == AVCaptureDevicePosition::Front {
        return CameraFacingMode::User;
    }

    if position == AVCaptureDevicePosition::Back {
        return CameraFacingMode::Environment;
    }

    let transport = unsafe { device.transportType() };
    if transport != 0 {
        return CameraFacingMode::External;
    }

    CameraFacingMode::Unknown
}

/// Return whether one capture device advertises depth capture.
fn is_depth_capable_camera(device: &AVCaptureDevice) -> bool {
    let device_type = unsafe { device.deviceType() };

    &*device_type == unsafe { AVCaptureDeviceTypeBuiltInTrueDepthCamera }
        || &*device_type == unsafe { AVCaptureDeviceTypeBuiltInLiDARDepthCamera }
}

/// Convert one AVFoundation device into one public descriptor snapshot.
pub(super) fn camera_descriptor_from_device(
    device: &AVCaptureDevice,
) -> CameraDeviceDescriptorValue {
    let unique_id = unsafe { device.uniqueID() };
    let manufacturer = unsafe { device.manufacturer() };
    let localized_name = unsafe { device.localizedName() };
    let unique_id = core_platform::nsstring_to_string(unique_id.as_ref());
    let manufacturer = core_platform::nsstring_to_string(manufacturer.as_ref());

    CameraDeviceDescriptorValue {
        id: camera_descriptor_id(&unique_id),
        group_id: None,
        name: core_platform::nsstring_to_string(localized_name.as_ref()),
        manufacturer: (!manufacturer.is_empty()).then_some(manufacturer),
        facing_mode: camera_facing_mode(device),
        depth_capable: is_depth_capable_camera(device),
    }
}

/// Collect the current macOS camera descriptor snapshot keyed by stable id.
pub(super) fn camera_descriptor_snapshot(
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, CameraDeviceDescriptorValue>> {
    let devices = camera_devices(operation)?;
    let mut snapshot = BTreeMap::new();

    // descriptor scan
    for device in devices.iter() {
        let descriptor = camera_descriptor_from_device(&device);
        snapshot.insert(descriptor.id.clone(), descriptor);
    }

    Ok(snapshot)
}

/// Return the device types we want AVFoundation to discover.
fn camera_device_types() -> Retained<NSArray<AVCaptureDeviceType>> {
    NSArray::from_slice(&[
        unsafe { AVCaptureDeviceTypeBuiltInWideAngleCamera },
        unsafe { AVCaptureDeviceTypeBuiltInTelephotoCamera },
        unsafe { AVCaptureDeviceTypeBuiltInUltraWideCamera },
        unsafe { AVCaptureDeviceTypeBuiltInDualCamera },
        unsafe { AVCaptureDeviceTypeBuiltInDualWideCamera },
        unsafe { AVCaptureDeviceTypeBuiltInTripleCamera },
        unsafe { AVCaptureDeviceTypeBuiltInTrueDepthCamera },
        unsafe { AVCaptureDeviceTypeBuiltInLiDARDepthCamera },
        unsafe { AVCaptureDeviceTypeExternal },
        unsafe { AVCaptureDeviceTypeContinuityCamera },
    ])
}

/// Return one AVFoundation discovery session for video devices.
fn camera_discovery_session(
    operation: &'static str,
) -> RuntimeResult<Retained<AVCaptureDeviceDiscoverySession>> {
    let media_type =
        unsafe { AVMediaTypeVideo }.ok_or_else(|| core_platform::not_supported(operation))?;

    Ok(unsafe {
        AVCaptureDeviceDiscoverySession::discoverySessionWithDeviceTypes_mediaType_position(
            &camera_device_types(),
            Some(media_type),
            AVCaptureDevicePosition::Unspecified,
        )
    })
}

/// Return one shared callback queue for one opened camera stream.
pub(super) fn camera_callback_queue() -> DispatchRetained<DispatchQueue> {
    core_platform::apple_serial_dispatch_queue(
        MACOS_CAMERA_QUEUE_LABEL,
        Some(core_platform::apple_dispatch_queue()),
    )
}

/// Convert one frame-rate value into milli-hertz.
fn frame_rate_to_milli_hz(frame_rate: f64) -> u32 {
    if !frame_rate.is_finite() || frame_rate <= 0.0 {
        return 0;
    }

    let milli_hz = (frame_rate * 1000.0).round();
    milli_hz.clamp(0.0, u32::MAX as f64) as u32
}

/// Collect one practical frame-rate selection set for one host frame-rate range.
fn extend_frame_rates(frame_rates: &mut BTreeSet<u32>, range: &AVFrameRateRange) {
    let minimum = frame_rate_to_milli_hz(unsafe { range.minFrameRate() });
    let maximum = frame_rate_to_milli_hz(unsafe { range.maxFrameRate() });
    if minimum != 0 {
        frame_rates.insert(minimum);
    }
    if maximum != 0 {
        frame_rates.insert(maximum);
    }

    let minimum_fps = unsafe { range.minFrameRate() };
    let maximum_fps = unsafe { range.maxFrameRate() };
    let minimum_integer = minimum_fps.ceil() as i64;
    let maximum_integer = maximum_fps.floor() as i64;
    if maximum_integer < minimum_integer {
        return;
    }

    if maximum_integer - minimum_integer > 240 {
        return;
    }

    for frame_rate in minimum_integer..=maximum_integer {
        if frame_rate <= 0 {
            continue;
        }

        frame_rates.insert((frame_rate as u32).saturating_mul(1000));
    }
}

/// Build one control-mode snapshot from one AVFoundation device.
pub(super) fn camera_control_modes(device: &AVCaptureDevice) -> CameraControlModes {
    let mut modes = empty_camera_control_modes();

    if unsafe {
        device.isExposureModeSupported(objc2_av_foundation::AVCaptureExposureMode::AutoExpose)
    } {
        modes.exposure_modes.push(CameraExposureMode::Auto);
    }
    if unsafe {
        device.isExposureModeSupported(
            objc2_av_foundation::AVCaptureExposureMode::ContinuousAutoExposure,
        )
    } {
        modes
            .exposure_modes
            .push(CameraExposureMode::ContinuousAuto);
    }
    if unsafe { device.isExposureModeSupported(objc2_av_foundation::AVCaptureExposureMode::Custom) }
        || unsafe {
            device.isExposureModeSupported(objc2_av_foundation::AVCaptureExposureMode::Locked)
        }
    {
        modes.exposure_modes.push(CameraExposureMode::Manual);
    }
    if unsafe {
        device.isWhiteBalanceModeSupported(
            objc2_av_foundation::AVCaptureWhiteBalanceMode::AutoWhiteBalance,
        )
    } {
        modes.white_balance_modes.push(CameraWhiteBalanceMode::Auto);
    }
    if unsafe {
        device.isWhiteBalanceModeSupported(
            objc2_av_foundation::AVCaptureWhiteBalanceMode::ContinuousAutoWhiteBalance,
        )
    } {
        modes
            .white_balance_modes
            .push(CameraWhiteBalanceMode::ContinuousAuto);
    }
    if unsafe {
        device.isWhiteBalanceModeSupported(objc2_av_foundation::AVCaptureWhiteBalanceMode::Locked)
    } {
        modes
            .white_balance_modes
            .push(CameraWhiteBalanceMode::Manual);
    }
    if unsafe { device.isFocusModeSupported(objc2_av_foundation::AVCaptureFocusMode::AutoFocus) } {
        modes.focus_modes.push(CameraFocusMode::Auto);
    }
    if unsafe {
        device.isFocusModeSupported(objc2_av_foundation::AVCaptureFocusMode::ContinuousAutoFocus)
    } {
        modes.focus_modes.push(CameraFocusMode::ContinuousAuto);
    }
    if unsafe { device.isFocusModeSupported(objc2_av_foundation::AVCaptureFocusMode::Locked) } {
        modes.focus_modes.push(CameraFocusMode::Manual);
    }
    if unsafe { device.hasTorch() && device.isTorchAvailable() } {
        modes.torch_modes.clear();
        modes.torch_modes.push(CameraTorchMode::Off);

        if unsafe { device.isTorchModeSupported(objc2_av_foundation::AVCaptureTorchMode::On) } {
            modes.torch_modes.push(CameraTorchMode::On);
        }
        if unsafe { device.isTorchModeSupported(objc2_av_foundation::AVCaptureTorchMode::Auto) } {
            modes.torch_modes.push(CameraTorchMode::Auto);
        }
    }

    modes
}

/// Return the stabilization modes one format can support.
fn stabilization_modes_for_format(format: &AVCaptureDeviceFormat) -> Vec<CameraStabilizationMode> {
    let mut modes = Vec::new();
    let has_standard_mode = unsafe {
        format.isVideoStabilizationModeSupported(AVCaptureVideoStabilizationMode::Standard)
    } || unsafe {
        format.isVideoStabilizationModeSupported(AVCaptureVideoStabilizationMode::Cinematic)
    } || unsafe {
        format.isVideoStabilizationModeSupported(AVCaptureVideoStabilizationMode::PreviewOptimized)
    } || unsafe {
        format.isVideoStabilizationModeSupported(AVCaptureVideoStabilizationMode::LowLatency)
    } || unsafe {
        format.isVideoStabilizationModeSupported(AVCaptureVideoStabilizationMode::Auto)
    };
    let has_high_quality_mode = unsafe {
        format.isVideoStabilizationModeSupported(AVCaptureVideoStabilizationMode::CinematicExtended)
    } || unsafe {
        format.isVideoStabilizationModeSupported(
            AVCaptureVideoStabilizationMode::CinematicExtendedEnhanced,
        )
    };

    if !has_standard_mode && !has_high_quality_mode {
        return modes;
    }

    modes.push(CameraStabilizationMode::Off);

    if has_standard_mode {
        modes.push(CameraStabilizationMode::Standard);
    }

    if has_high_quality_mode {
        modes.push(CameraStabilizationMode::HighQuality);
    }

    modes
}

/// Enumerate one camera format into public configs and capabilities.
fn enumerate_format_configs(
    format: &AVCaptureDeviceFormat,
    control_modes: &CameraControlModes,
) -> Vec<(CameraStreamConfigValue, CameraStreamCapabilityValue)> {
    let description = unsafe { format.formatDescription() };
    let dimensions = unsafe { CMVideoFormatDescriptionGetDimensions(&description) };
    if dimensions.width <= 0 || dimensions.height <= 0 {
        return Vec::new();
    }

    let frame_ranges = unsafe { format.videoSupportedFrameRateRanges() };
    let mut frame_rates = BTreeSet::new();
    let mut minimum_rate = u32::MAX;
    let mut maximum_rate = 0u32;

    for frame_range in frame_ranges.iter() {
        extend_frame_rates(&mut frame_rates, &frame_range);

        let current_minimum = frame_rate_to_milli_hz(unsafe { frame_range.minFrameRate() });
        let current_maximum = frame_rate_to_milli_hz(unsafe { frame_range.maxFrameRate() });
        if current_minimum != 0 {
            minimum_rate = minimum_rate.min(current_minimum);
        }
        maximum_rate = maximum_rate.max(current_maximum);
    }

    if frame_rates.is_empty() {
        return Vec::new();
    }

    let minimum_rate = if minimum_rate == u32::MAX {
        *frame_rates.first().unwrap_or(&0)
    } else {
        minimum_rate
    };
    let maximum_rate = maximum_rate.max(*frame_rates.last().unwrap_or(&0));
    let width = dimensions.width as u32;
    let height = dimensions.height as u32;
    let pixel_format = bgra_camera_pixel_format_descriptor();
    let mut results = Vec::new();

    for frame_rate_milli_hz in frame_rates {
        let config = CameraStreamConfigValue {
            width,
            height,
            frame_rate_milli_hz,
            pixel_format,
            color_space: None,
            dynamic_range: None,
        };
        let mut controls = camera_control_capabilities(control_modes);
        controls.stabilization_modes = Some(stabilization_modes_for_format(format));

        let capability = CameraStreamCapabilityValue {
            minimum_frame_rate_milli_hz: minimum_rate,
            maximum_frame_rate_milli_hz: maximum_rate,
            controls,
            ..basic_camera_stream_capability(&config, control_modes)
        };

        results.push((config, capability));
    }

    results
}

/// Enumerate one AVFoundation device into public configs and capabilities.
fn enumerate_device_streams(
    device: &AVCaptureDevice,
) -> (
    Vec<CameraStreamConfigValue>,
    Vec<CameraStreamCapabilityValue>,
) {
    let control_modes = camera_control_modes(device);
    let mut configs = Vec::new();
    let mut capabilities = Vec::new();

    for format in unsafe { device.formats() }.iter() {
        for (config, capability) in enumerate_format_configs(&format, &control_modes) {
            if configs.iter().any(|existing| existing == &config) {
                continue;
            }

            configs.push(config);
            capabilities.push(capability);
        }
    }

    (configs, capabilities)
}

/// Build one cached descriptor-info snapshot from one AVFoundation device.
pub(super) fn camera_descriptor_info_from_device(
    device: &AVCaptureDevice,
) -> MacosCameraDescriptorInfo {
    let unique_id = unsafe { device.uniqueID() };
    let (configs, capabilities) = enumerate_device_streams(device);

    MacosCameraDescriptorInfo {
        unique_id: core_platform::nsstring_to_string(unique_id.as_ref()),
        configs,
        capabilities,
    }
}

/// Return all currently discoverable AVFoundation camera devices.
pub(super) fn camera_devices(
    operation: &'static str,
) -> RuntimeResult<Retained<NSArray<AVCaptureDevice>>> {
    Ok(unsafe { camera_discovery_session(operation)?.devices() })
}

/// Find one AVFoundation camera device by public identifier.
pub(super) fn camera_device_by_id(
    id: &str,
    operation: &'static str,
) -> RuntimeResult<Retained<AVCaptureDevice>> {
    let unique_id = camera_unique_id_from_descriptor_id(id)
        .ok_or_else(|| core_platform::invalid_argument("id", "camera identifier is invalid"))?;

    for device in camera_devices(operation)?.iter() {
        let device_unique_id = unsafe { device.uniqueID() };
        let device_unique_id = core_platform::nsstring_to_string(device_unique_id.as_ref());
        if device_unique_id == unique_id {
            return Ok(device.retain());
        }
    }

    Err(core_platform::io_not_found(
        operation,
        "unknown camera device",
    ))
}

/// Find one AVFoundation format that matches one selected public config.
pub(super) fn camera_format_for_config(
    device: &AVCaptureDevice,
    selected_config: &CameraStreamConfigValue,
) -> Option<Retained<AVCaptureDeviceFormat>> {
    let control_modes = camera_control_modes(device);

    for format in unsafe { device.formats() }.iter() {
        for (config, _) in enumerate_format_configs(&format, &control_modes) {
            if &config == selected_config {
                return Some(format.retain());
            }
        }
    }

    None
}

/// Convert one selected frame-rate into one AVFoundation frame duration.
fn frame_duration_from_config(config: &CameraStreamConfigValue) -> CMTime {
    let frame_rate = config.frame_rate_milli_hz as f64 / 1000.0;

    unsafe { CMTime::with_seconds(1.0 / frame_rate, CAMERA_FRAME_TIME_SCALE) }
}

/// Build one BGRA output-settings dictionary.
pub(super) fn camera_output_settings()
-> Retained<NSDictionary<objc2_foundation::NSString, AnyObject>> {
    unsafe {
        NSDictionary::from_retained_objects::<objc2_foundation::NSString>(
            &[kCVPixelBufferPixelFormatTypeKey.as_ref()],
            &[Retained::cast_unchecked::<AnyObject>(NSNumber::new_u32(
                kCVPixelFormatType_32BGRA,
            ))],
        )
    }
}

/// Convert one CMSampleBuffer timestamp into one runtime timestamp.
fn sample_timestamp_ns(sample_buffer: &CMSampleBuffer) -> u64 {
    let timestamp = unsafe { sample_buffer.presentation_time_stamp() };
    if timestamp == unsafe { kCMTimeInvalid } {
        return core_platform::monotonic_now_ns();
    }

    let seconds = unsafe { timestamp.seconds() };
    if !seconds.is_finite() || seconds.is_sign_negative() {
        return core_platform::monotonic_now_ns();
    }

    let nanos = seconds * 1_000_000_000.0;
    nanos.clamp(0.0, u64::MAX as f64) as u64
}

/// Copy one BGRA pixel buffer into one owned byte vector.
fn copy_pixel_buffer_bytes(pixel_buffer: &CVPixelBuffer) -> Vec<u8> {
    unsafe {
        let _ = CVPixelBufferLockBaseAddress(pixel_buffer, CVPixelBufferLockFlags::ReadOnly);
    }

    let bytes = unsafe {
        let address = CVPixelBufferGetBaseAddress(pixel_buffer);
        if address.is_null() {
            Vec::new()
        } else {
            let length = CVPixelBufferGetDataSize(pixel_buffer);
            std::slice::from_raw_parts(address.cast::<u8>(), length).to_vec()
        }
    };

    unsafe {
        let _ = CVPixelBufferUnlockBaseAddress(pixel_buffer, CVPixelBufferLockFlags::ReadOnly);
    }

    bytes
}

/// Convert one captured AVFoundation photo into one public still-photo payload.
pub(super) fn camera_photo_value_from_photo(
    photo: &AVCapturePhoto,
    operation: &'static str,
) -> RuntimeResult<CameraPhotoValue> {
    let pixel_buffer = unsafe { photo.pixelBuffer() }.ok_or_else(|| {
        core_platform::io_operation_error(
            operation,
            None,
            "camera photo did not contain one pixel buffer",
        )
    })?;
    let bytes = copy_pixel_buffer_bytes(&pixel_buffer);
    if bytes.is_empty() {
        return Err(core_platform::io_operation_error(
            operation,
            None,
            "camera photo buffer was empty",
        ));
    }

    let row_stride = CVPixelBufferGetBytesPerRow(&pixel_buffer) as u32;
    let width = CVPixelBufferGetWidth(&pixel_buffer) as u32;
    let height = CVPixelBufferGetHeight(&pixel_buffer) as u32;

    Ok(CameraPhotoValue {
        timestamp_ns: core_platform::monotonic_now_ns(),
        width,
        height,
        pixel_format: bgra_camera_pixel_format_descriptor(),
        color_space: CameraColorSpace::Unknown,
        dynamic_range: CameraDynamicRange::Standard,
        planes: vec![CameraPlaneLayoutValue {
            offset_bytes: 0,
            length_bytes: bytes.len() as u32,
            row_stride_bytes: row_stride,
            pixel_stride_bytes: 4,
        }],
        metadata: empty_camera_metadata(),
        bytes,
    })
}

/// Convert one sample buffer into one public camera frame.
pub(super) fn camera_frame_value_from_sample_buffer(
    sample_buffer: &CMSampleBuffer,
    config: &CameraStreamConfigValue,
    state: &Mutex<CameraStreamState>,
    operation: &'static str,
) -> RuntimeResult<CameraFrameValue> {
    let image_buffer = unsafe { sample_buffer.image_buffer() }.ok_or_else(|| {
        core_platform::io_operation_error(operation, None, "camera frame did not contain pixels")
    })?;
    let pixel_buffer = unsafe {
        &*(std::ptr::from_ref(image_buffer.as_ref()) as *const _ as *const CVPixelBuffer)
    };
    let bytes = copy_pixel_buffer_bytes(pixel_buffer);
    if bytes.is_empty() {
        return Err(core_platform::io_operation_error(
            operation,
            None,
            "camera frame buffer was empty",
        ));
    }

    let row_stride = CVPixelBufferGetBytesPerRow(pixel_buffer);
    let width = CVPixelBufferGetWidth(pixel_buffer) as u32;
    let height = CVPixelBufferGetHeight(pixel_buffer) as u32;
    let mut planes = frame_plane_layouts(config, bytes.len());
    if let Some(plane) = planes.first_mut() {
        plane.row_stride_bytes = row_stride.min(u32::MAX as usize) as u32;
    }

    Ok(CameraFrameValue {
        timestamp_ns: sample_timestamp_ns(sample_buffer),
        sequence: {
            let mut state = state.lock();
            let sequence = state.next_sequence;
            state.next_sequence = state.next_sequence.saturating_add(1);

            sequence
        },
        width,
        height,
        pixel_format: bgra_camera_pixel_format_descriptor(),
        color_space: CameraColorSpace::Unknown,
        dynamic_range: CameraDynamicRange::Standard,
        planes,
        metadata: empty_camera_metadata(),
        bytes,
    })
}

/// Configure one AVFoundation device for one selected format and frame rate.
pub(super) fn configure_camera_device(
    device: &AVCaptureDevice,
    format: &AVCaptureDeviceFormat,
    config: &CameraStreamConfigValue,
    operation: &'static str,
) -> RuntimeResult<()> {
    unsafe { device.lockForConfiguration() }.map_err(|error| {
        macos_camera_nserror(operation, "AVCaptureDevice::lockForConfiguration", &error)
    })?;

    // selected format and cadence
    unsafe {
        device.setActiveFormat(format);

        let duration = frame_duration_from_config(config);
        device.setActiveVideoMinFrameDuration(duration);
        device.setActiveVideoMaxFrameDuration(duration);

        device.unlockForConfiguration();
    }

    Ok(())
}

/// Read one camera frame from one running stream.
pub(super) fn read_camera_frame(
    resource: &MacosCameraStreamResource,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<CameraFrameValue> {
    if !resource.state.lock().is_started {
        return Err(core_platform::io_would_block(
            operation,
            "camera stream is not started",
        ));
    }

    resource
        .queue
        .pop_with_timeout_or_else(std::time::Duration::from_nanos(timeout_ns), || {
            Err(core_platform::io_would_block(
                operation,
                "camera frame is not ready",
            ))
        })
}

/// Try reading one camera frame without blocking.
pub(super) fn try_read_camera_frame(
    resource: &MacosCameraStreamResource,
    operation: &'static str,
) -> RuntimeResult<CameraFrameValue> {
    if !resource.state.lock().is_started {
        return Err(core_platform::io_would_block(
            operation,
            "camera stream is not started",
        ));
    }

    resource.queue.try_pop_or_else(|| {
        Err(core_platform::io_would_block(
            operation,
            "camera frame is not ready",
        ))
    })
}
