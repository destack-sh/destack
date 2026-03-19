use crate::platform::device::camera::android::codec::*;

/// Encode one camera exposure mode into the Android host selector code.
fn encode_exposure_mode(mode: CameraExposureMode) -> u32 {
    match mode {
        CameraExposureMode::Auto => 1,
        CameraExposureMode::ContinuousAuto => 2,
        CameraExposureMode::Manual => 3,
    }
}

/// Encode one camera white-balance mode into the Android host selector code.
fn encode_white_balance_mode(mode: CameraWhiteBalanceMode) -> u32 {
    match mode {
        CameraWhiteBalanceMode::Auto => 1,
        CameraWhiteBalanceMode::ContinuousAuto => 2,
        CameraWhiteBalanceMode::Manual => 3,
    }
}

/// Encode one camera focus mode into the Android host selector code.
fn encode_focus_mode(mode: CameraFocusMode) -> u32 {
    match mode {
        CameraFocusMode::Auto => 1,
        CameraFocusMode::ContinuousAuto => 2,
        CameraFocusMode::Manual => 3,
    }
}

/// Encode one camera stabilization mode into the Android host selector code.
fn encode_stabilization_mode(mode: CameraStabilizationMode) -> u32 {
    match mode {
        CameraStabilizationMode::Off => 1,
        CameraStabilizationMode::Standard => 2,
        CameraStabilizationMode::HighQuality => 3,
    }
}

/// Encode one camera torch mode into the Android host selector code.
fn encode_torch_mode(mode: CameraTorchMode) -> u32 {
    match mode {
        CameraTorchMode::Off => 1,
        CameraTorchMode::On => 2,
        CameraTorchMode::Auto => 3,
    }
}

/// Resolve one Android camera stream resource.
fn android_camera_stream_resource(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AndroidCameraStreamResource>> {
    camera_stream_resource::<AndroidCameraStreamResource>(binding, handle, operation)
}

/// Build one float control range value.
fn float_range(minimum: f64, maximum: f64, step: f64) -> CameraFloatControlRange {
    CameraFloatControlRange {
        minimum,
        maximum,
        default: minimum,
        step,
    }
}

/// Build one float control range result.
fn float_range_result(
    _binding: &BindingCallContext,
    _stream_id: u64,
    minimum: f64,
    maximum: f64,
    step: f64,
    _capability: &Option<CameraStreamCapabilityValue>,
) -> RuntimeResult<CameraFloatControlRange> {
    Ok(float_range(minimum, maximum, step))
}

macro_rules! camera_scalar_getter {
    ($(#[$meta:meta])* $name:ident, $selector:expr, $ty:ty, $reader:ident, $converter:expr) => {
        $(#[$meta])*
        pub(crate) unsafe fn $name(
            binding: &BindingCallContext,
            out: *mut $ty,
            handle: resource::CameraStreamHandle,
        ) -> RuntimeResult<()> {
            core_platform::ensure_out(out, "out")?;

            // stream resource
            let resource = android_camera_stream_resource(binding, handle, stringify!($name))?;

            // host read
            let raw = $reader(binding, resource.stream_id, $selector, stringify!($name))?;
            unsafe {
                out.write(($converter)(raw, &resource.capability, stringify!($name))?);
            }

            Ok(())
        }
    };
}

macro_rules! camera_scalar_setter {
    ($(#[$meta:meta])* $name:ident, $selector:expr, $ty:ty, $writer:ident, $converter:expr) => {
        $(#[$meta])*
        pub(crate) unsafe fn $name(
            binding: &BindingCallContext,
            handle: resource::CameraStreamHandle,
            value: $ty,
        ) -> RuntimeResult<()> {
            // stream resource
            let resource = android_camera_stream_resource(binding, handle, stringify!($name))?;

            // host write
            let value = ($converter)(value, &resource.capability, stringify!($name))?;
            $writer(binding, resource.stream_id, $selector, value, stringify!($name))
        }
    };
}

macro_rules! camera_range_getter {
    ($(#[$meta:meta])* $name:ident, $selector:expr, $range_ty:ty, $reader:ident, $builder:expr) => {
        $(#[$meta])*
        pub(crate) unsafe fn $name(
            binding: &BindingCallContext,
            out: *mut $range_ty,
            handle: resource::CameraStreamHandle,
        ) -> RuntimeResult<()> {
            core_platform::ensure_out(out, "out")?;

            // stream resource
            let resource = android_camera_stream_resource(binding, handle, stringify!($name))?;

            // host range
            let (minimum, maximum, step) =
                $reader(binding, resource.stream_id, $selector, stringify!($name))?;
            unsafe {
                out.write(($builder)(
                    binding,
                    resource.stream_id,
                    minimum,
                    maximum,
                    step,
                    &resource.capability,
                )?);
            }

            Ok(())
        }
    };
}

/// Convert one raw `f64` control value without modification.
fn passthrough_f64(
    value: f64,
    _capability: &Option<CameraStreamCapabilityValue>,
    _operation: &'static str,
) -> RuntimeResult<f64> {
    Ok(value)
}

/// Convert one raw `u64` control value without modification.
fn passthrough_u64(
    value: u64,
    _capability: &Option<CameraStreamCapabilityValue>,
    _operation: &'static str,
) -> RuntimeResult<u64> {
    Ok(value)
}

/// Convert one raw `u32` control value without modification.
fn passthrough_u32(
    value: u32,
    _capability: &Option<CameraStreamCapabilityValue>,
    _operation: &'static str,
) -> RuntimeResult<u32> {
    Ok(value)
}

/// Convert one exposure-mode code into the public enum.
fn decode_exposure_control(
    value: u32,
    _capability: &Option<CameraStreamCapabilityValue>,
    operation: &'static str,
) -> RuntimeResult<CameraExposureMode> {
    decode_exposure_mode(value, operation)
}

/// Convert one white-balance-mode code into the public enum.
fn decode_white_balance_control(
    value: u32,
    _capability: &Option<CameraStreamCapabilityValue>,
    operation: &'static str,
) -> RuntimeResult<CameraWhiteBalanceMode> {
    decode_white_balance_mode(value, operation)
}

/// Convert one focus-mode code into the public enum.
fn decode_focus_control(
    value: u32,
    _capability: &Option<CameraStreamCapabilityValue>,
    operation: &'static str,
) -> RuntimeResult<CameraFocusMode> {
    decode_focus_mode(value, operation)
}

/// Convert one stabilization-mode code into the public enum.
fn decode_stabilization_control(
    value: u32,
    _capability: &Option<CameraStreamCapabilityValue>,
    operation: &'static str,
) -> RuntimeResult<CameraStabilizationMode> {
    decode_stabilization_mode(value, operation)
}

/// Convert one torch-mode code into the public enum.
fn decode_torch_control(
    value: u32,
    _capability: &Option<CameraStreamCapabilityValue>,
    operation: &'static str,
) -> RuntimeResult<CameraTorchMode> {
    decode_torch_mode(value, operation)
}

/// Convert one exposure mode into the Android host code.
fn encode_exposure_control(
    value: CameraExposureMode,
    _capability: &Option<CameraStreamCapabilityValue>,
    _operation: &'static str,
) -> RuntimeResult<u32> {
    Ok(encode_exposure_mode(value))
}

/// Convert one white-balance mode into the Android host code.
fn encode_white_balance_control(
    value: CameraWhiteBalanceMode,
    _capability: &Option<CameraStreamCapabilityValue>,
    _operation: &'static str,
) -> RuntimeResult<u32> {
    Ok(encode_white_balance_mode(value))
}

/// Convert one focus mode into the Android host code.
fn encode_focus_control(
    value: CameraFocusMode,
    _capability: &Option<CameraStreamCapabilityValue>,
    _operation: &'static str,
) -> RuntimeResult<u32> {
    Ok(encode_focus_mode(value))
}

/// Convert one stabilization mode into the Android host code.
fn encode_stabilization_control(
    value: CameraStabilizationMode,
    _capability: &Option<CameraStreamCapabilityValue>,
    _operation: &'static str,
) -> RuntimeResult<u32> {
    Ok(encode_stabilization_mode(value))
}

/// Convert one torch mode into the Android host code.
fn encode_torch_control(
    value: CameraTorchMode,
    _capability: &Option<CameraStreamCapabilityValue>,
    _operation: &'static str,
) -> RuntimeResult<u32> {
    Ok(encode_torch_mode(value))
}

/// Build one exposure-compensation range.
fn exposure_compensation_range(
    binding: &BindingCallContext,
    stream_id: u64,
    minimum: f64,
    maximum: f64,
    step: f64,
    capability: &Option<CameraStreamCapabilityValue>,
) -> RuntimeResult<CameraExposureCompensationRange> {
    let default_ev = camera_get_f64(
        binding,
        stream_id,
        CAMERA_CONTROL_EXPOSURE_COMPENSATION,
        "destack.device.camera.stream.exposureCompensationRange",
    )?;

    Ok(CameraExposureCompensationRange {
        minimum_ev: minimum,
        maximum_ev: maximum,
        default_ev,
        step_ev: step,
        auto_supported: supports_auto_exposure(capability),
    })
}

/// Build one exposure-time range.
fn exposure_time_range(
    binding: &BindingCallContext,
    stream_id: u64,
    minimum: u64,
    maximum: u64,
    step: u64,
    capability: &Option<CameraStreamCapabilityValue>,
) -> RuntimeResult<CameraExposureTimeRange> {
    let default_ns = camera_get_u64(
        binding,
        stream_id,
        CAMERA_CONTROL_EXPOSURE_TIME_NS,
        "destack.device.camera.stream.exposureTimeRange",
    )?;

    Ok(CameraExposureTimeRange {
        minimum_ns: minimum,
        maximum_ns: maximum,
        default_ns,
        step_ns: step,
        auto_supported: supports_auto_exposure(capability),
    })
}

/// Build one sensor-iso range.
fn sensor_iso_range(
    binding: &BindingCallContext,
    stream_id: u64,
    minimum: u32,
    maximum: u32,
    step: u32,
    capability: &Option<CameraStreamCapabilityValue>,
) -> RuntimeResult<CameraSensorIsoRange> {
    let default = camera_get_u32(
        binding,
        stream_id,
        CAMERA_CONTROL_SENSOR_ISO,
        "destack.device.camera.stream.sensorIsoRange",
    )?;

    Ok(CameraSensorIsoRange {
        minimum,
        maximum,
        default,
        step,
        auto_supported: supports_auto_exposure(capability),
    })
}

/// Build one white-balance range.
fn white_balance_range(
    binding: &BindingCallContext,
    stream_id: u64,
    minimum: u32,
    maximum: u32,
    step: u32,
    capability: &Option<CameraStreamCapabilityValue>,
) -> RuntimeResult<CameraWhiteBalanceRange> {
    let default_kelvin = camera_get_u32(
        binding,
        stream_id,
        CAMERA_CONTROL_WHITE_BALANCE_KELVIN,
        "destack.device.camera.stream.whiteBalanceRange",
    )?;

    Ok(CameraWhiteBalanceRange {
        minimum_kelvin: minimum,
        maximum_kelvin: maximum,
        default_kelvin,
        step_kelvin: step,
        auto_supported: supports_auto_white_balance(capability),
    })
}

/// Build one focus-distance range.
fn focus_distance_range(
    binding: &BindingCallContext,
    stream_id: u64,
    minimum: f64,
    maximum: f64,
    step: f64,
    capability: &Option<CameraStreamCapabilityValue>,
) -> RuntimeResult<CameraFocusDistanceRange> {
    let default_diopters = camera_get_f64(
        binding,
        stream_id,
        CAMERA_CONTROL_FOCUS_DISTANCE_DIOPTERS,
        "destack.device.camera.stream.focusDistanceRange",
    )?;

    Ok(CameraFocusDistanceRange {
        minimum_diopters: minimum,
        maximum_diopters: maximum,
        default_diopters,
        step_diopters: step,
        auto_supported: supports_auto_focus(capability),
    })
}

/// Build one pan-angle range.
fn pan_range(
    binding: &BindingCallContext,
    stream_id: u64,
    minimum: f64,
    maximum: f64,
    step: f64,
    _capability: &Option<CameraStreamCapabilityValue>,
) -> RuntimeResult<CameraPanAngleRange> {
    let default_degrees = camera_get_f64(
        binding,
        stream_id,
        CAMERA_CONTROL_PAN_DEGREES,
        "destack.device.camera.stream.panRange",
    )?;

    Ok(CameraPanAngleRange {
        minimum_degrees: minimum,
        maximum_degrees: maximum,
        default_degrees,
        step_degrees: step,
    })
}

/// Build one tilt-angle range.
fn tilt_range(
    binding: &BindingCallContext,
    stream_id: u64,
    minimum: f64,
    maximum: f64,
    step: f64,
    _capability: &Option<CameraStreamCapabilityValue>,
) -> RuntimeResult<CameraTiltAngleRange> {
    let default_degrees = camera_get_f64(
        binding,
        stream_id,
        CAMERA_CONTROL_TILT_DEGREES,
        "destack.device.camera.stream.tiltRange",
    )?;

    Ok(CameraTiltAngleRange {
        minimum_degrees: minimum,
        maximum_degrees: maximum,
        default_degrees,
        step_degrees: step,
    })
}

/// Build one zoom-ratio range.
fn zoom_ratio_range(
    binding: &BindingCallContext,
    stream_id: u64,
    minimum: f64,
    maximum: f64,
    step: f64,
    _capability: &Option<CameraStreamCapabilityValue>,
) -> RuntimeResult<CameraZoomRatioRange> {
    let default_ratio = camera_get_f64(
        binding,
        stream_id,
        CAMERA_CONTROL_ZOOM_RATIO,
        "destack.device.camera.stream.zoomRatioRange",
    )?;

    Ok(CameraZoomRatioRange {
        minimum_ratio: minimum,
        maximum_ratio: maximum,
        default_ratio,
        step_ratio: step,
    })
}

camera_scalar_getter!(
    /// Read one Android camera exposure compensation value.
    destack_device_camera_stream_exposure_compensation,
    CAMERA_CONTROL_EXPOSURE_COMPENSATION,
    f64,
    camera_get_f64,
    passthrough_f64
);
camera_scalar_setter!(
    /// Write one Android camera exposure compensation value.
    destack_device_camera_stream_set_exposure_compensation,
    CAMERA_CONTROL_EXPOSURE_COMPENSATION,
    f64,
    camera_set_f64,
    passthrough_f64
);
camera_range_getter!(
    /// Read one Android camera exposure compensation range.
    destack_device_camera_stream_exposure_compensation_range,
    CAMERA_CONTROL_EXPOSURE_COMPENSATION,
    CameraExposureCompensationRange,
    camera_range_f64,
    exposure_compensation_range
);

camera_scalar_getter!(
    /// Read one Android camera exposure mode.
    destack_device_camera_stream_exposure_mode,
    CAMERA_CONTROL_EXPOSURE_MODE,
    CameraExposureMode,
    camera_get_u32,
    decode_exposure_control
);
camera_scalar_setter!(
    /// Write one Android camera exposure mode.
    destack_device_camera_stream_set_exposure_mode,
    CAMERA_CONTROL_EXPOSURE_MODE,
    CameraExposureMode,
    camera_set_u32,
    encode_exposure_control
);

camera_scalar_getter!(
    /// Read one Android camera exposure time.
    destack_device_camera_stream_exposure_time_ns,
    CAMERA_CONTROL_EXPOSURE_TIME_NS,
    u64,
    camera_get_u64,
    passthrough_u64
);
camera_scalar_setter!(
    /// Write one Android camera exposure time.
    destack_device_camera_stream_set_exposure_time_ns,
    CAMERA_CONTROL_EXPOSURE_TIME_NS,
    u64,
    camera_set_u64,
    passthrough_u64
);
camera_range_getter!(
    /// Read one Android camera exposure time range.
    destack_device_camera_stream_exposure_time_range,
    CAMERA_CONTROL_EXPOSURE_TIME_NS,
    CameraExposureTimeRange,
    camera_range_u64,
    exposure_time_range
);

camera_scalar_getter!(
    /// Read one Android camera sensor ISO value.
    destack_device_camera_stream_sensor_iso,
    CAMERA_CONTROL_SENSOR_ISO,
    u32,
    camera_get_u32,
    passthrough_u32
);
camera_scalar_setter!(
    /// Write one Android camera sensor ISO value.
    destack_device_camera_stream_set_sensor_iso,
    CAMERA_CONTROL_SENSOR_ISO,
    u32,
    camera_set_u32,
    passthrough_u32
);
camera_range_getter!(
    /// Read one Android camera sensor ISO range.
    destack_device_camera_stream_sensor_iso_range,
    CAMERA_CONTROL_SENSOR_ISO,
    CameraSensorIsoRange,
    camera_range_u32,
    sensor_iso_range
);

camera_scalar_getter!(
    /// Read one Android camera white-balance kelvin value.
    destack_device_camera_stream_white_balance_kelvin,
    CAMERA_CONTROL_WHITE_BALANCE_KELVIN,
    u32,
    camera_get_u32,
    passthrough_u32
);
camera_scalar_setter!(
    /// Write one Android camera white-balance kelvin value.
    destack_device_camera_stream_set_white_balance_kelvin,
    CAMERA_CONTROL_WHITE_BALANCE_KELVIN,
    u32,
    camera_set_u32,
    passthrough_u32
);
camera_range_getter!(
    /// Read one Android camera white-balance range.
    destack_device_camera_stream_white_balance_range,
    CAMERA_CONTROL_WHITE_BALANCE_KELVIN,
    CameraWhiteBalanceRange,
    camera_range_u32,
    white_balance_range
);

camera_scalar_getter!(
    /// Read one Android camera white-balance mode.
    destack_device_camera_stream_white_balance_mode,
    CAMERA_CONTROL_WHITE_BALANCE_MODE,
    CameraWhiteBalanceMode,
    camera_get_u32,
    decode_white_balance_control
);
camera_scalar_setter!(
    /// Write one Android camera white-balance mode.
    destack_device_camera_stream_set_white_balance_mode,
    CAMERA_CONTROL_WHITE_BALANCE_MODE,
    CameraWhiteBalanceMode,
    camera_set_u32,
    encode_white_balance_control
);

camera_scalar_getter!(
    /// Read one Android camera focus distance.
    destack_device_camera_stream_focus_distance_diopters,
    CAMERA_CONTROL_FOCUS_DISTANCE_DIOPTERS,
    f64,
    camera_get_f64,
    passthrough_f64
);
camera_scalar_setter!(
    /// Write one Android camera focus distance.
    destack_device_camera_stream_set_focus_distance_diopters,
    CAMERA_CONTROL_FOCUS_DISTANCE_DIOPTERS,
    f64,
    camera_set_f64,
    passthrough_f64
);
camera_range_getter!(
    /// Read one Android camera focus distance range.
    destack_device_camera_stream_focus_distance_range,
    CAMERA_CONTROL_FOCUS_DISTANCE_DIOPTERS,
    CameraFocusDistanceRange,
    camera_range_f64,
    focus_distance_range
);

camera_scalar_getter!(
    /// Read one Android camera focus mode.
    destack_device_camera_stream_focus_mode,
    CAMERA_CONTROL_FOCUS_MODE,
    CameraFocusMode,
    camera_get_u32,
    decode_focus_control
);
camera_scalar_setter!(
    /// Write one Android camera focus mode.
    destack_device_camera_stream_set_focus_mode,
    CAMERA_CONTROL_FOCUS_MODE,
    CameraFocusMode,
    camera_set_u32,
    encode_focus_control
);

camera_scalar_getter!(
    /// Read one Android camera brightness value.
    destack_device_camera_stream_brightness,
    CAMERA_CONTROL_BRIGHTNESS,
    f64,
    camera_get_f64,
    passthrough_f64
);
camera_scalar_setter!(
    /// Write one Android camera brightness value.
    destack_device_camera_stream_set_brightness,
    CAMERA_CONTROL_BRIGHTNESS,
    f64,
    camera_set_f64,
    passthrough_f64
);
camera_range_getter!(
    /// Read one Android camera brightness range.
    destack_device_camera_stream_brightness_range,
    CAMERA_CONTROL_BRIGHTNESS,
    CameraFloatControlRange,
    camera_range_f64,
    float_range_result
);

camera_scalar_getter!(
    /// Read one Android camera contrast value.
    destack_device_camera_stream_contrast,
    CAMERA_CONTROL_CONTRAST,
    f64,
    camera_get_f64,
    passthrough_f64
);
camera_scalar_setter!(
    /// Write one Android camera contrast value.
    destack_device_camera_stream_set_contrast,
    CAMERA_CONTROL_CONTRAST,
    f64,
    camera_set_f64,
    passthrough_f64
);
camera_range_getter!(
    /// Read one Android camera contrast range.
    destack_device_camera_stream_contrast_range,
    CAMERA_CONTROL_CONTRAST,
    CameraFloatControlRange,
    camera_range_f64,
    float_range_result
);

camera_scalar_getter!(
    /// Read one Android camera saturation value.
    destack_device_camera_stream_saturation,
    CAMERA_CONTROL_SATURATION,
    f64,
    camera_get_f64,
    passthrough_f64
);
camera_scalar_setter!(
    /// Write one Android camera saturation value.
    destack_device_camera_stream_set_saturation,
    CAMERA_CONTROL_SATURATION,
    f64,
    camera_set_f64,
    passthrough_f64
);
camera_range_getter!(
    /// Read one Android camera saturation range.
    destack_device_camera_stream_saturation_range,
    CAMERA_CONTROL_SATURATION,
    CameraFloatControlRange,
    camera_range_f64,
    float_range_result
);

camera_scalar_getter!(
    /// Read one Android camera sharpness value.
    destack_device_camera_stream_sharpness,
    CAMERA_CONTROL_SHARPNESS,
    f64,
    camera_get_f64,
    passthrough_f64
);
camera_scalar_setter!(
    /// Write one Android camera sharpness value.
    destack_device_camera_stream_set_sharpness,
    CAMERA_CONTROL_SHARPNESS,
    f64,
    camera_set_f64,
    passthrough_f64
);
camera_range_getter!(
    /// Read one Android camera sharpness range.
    destack_device_camera_stream_sharpness_range,
    CAMERA_CONTROL_SHARPNESS,
    CameraFloatControlRange,
    camera_range_f64,
    float_range_result
);

camera_scalar_getter!(
    /// Read one Android camera pan angle.
    destack_device_camera_stream_pan_degrees,
    CAMERA_CONTROL_PAN_DEGREES,
    f64,
    camera_get_f64,
    passthrough_f64
);
camera_scalar_setter!(
    /// Write one Android camera pan angle.
    destack_device_camera_stream_set_pan_degrees,
    CAMERA_CONTROL_PAN_DEGREES,
    f64,
    camera_set_f64,
    passthrough_f64
);
camera_range_getter!(
    /// Read one Android camera pan angle range.
    destack_device_camera_stream_pan_range,
    CAMERA_CONTROL_PAN_DEGREES,
    CameraPanAngleRange,
    camera_range_f64,
    pan_range
);

camera_scalar_getter!(
    /// Read one Android camera tilt angle.
    destack_device_camera_stream_tilt_degrees,
    CAMERA_CONTROL_TILT_DEGREES,
    f64,
    camera_get_f64,
    passthrough_f64
);
camera_scalar_setter!(
    /// Write one Android camera tilt angle.
    destack_device_camera_stream_set_tilt_degrees,
    CAMERA_CONTROL_TILT_DEGREES,
    f64,
    camera_set_f64,
    passthrough_f64
);
camera_range_getter!(
    /// Read one Android camera tilt angle range.
    destack_device_camera_stream_tilt_range,
    CAMERA_CONTROL_TILT_DEGREES,
    CameraTiltAngleRange,
    camera_range_f64,
    tilt_range
);

camera_scalar_getter!(
    /// Read one Android camera zoom ratio.
    destack_device_camera_stream_zoom_ratio,
    CAMERA_CONTROL_ZOOM_RATIO,
    f64,
    camera_get_f64,
    passthrough_f64
);
camera_scalar_setter!(
    /// Write one Android camera zoom ratio.
    destack_device_camera_stream_set_zoom_ratio,
    CAMERA_CONTROL_ZOOM_RATIO,
    f64,
    camera_set_f64,
    passthrough_f64
);
camera_range_getter!(
    /// Read one Android camera zoom ratio range.
    destack_device_camera_stream_zoom_ratio_range,
    CAMERA_CONTROL_ZOOM_RATIO,
    CameraZoomRatioRange,
    camera_range_f64,
    zoom_ratio_range
);

camera_scalar_getter!(
    /// Read one Android camera stabilization mode.
    destack_device_camera_stream_stabilization_mode,
    CAMERA_CONTROL_STABILIZATION_MODE,
    CameraStabilizationMode,
    camera_get_u32,
    decode_stabilization_control
);
camera_scalar_setter!(
    /// Write one Android camera stabilization mode.
    destack_device_camera_stream_set_stabilization_mode,
    CAMERA_CONTROL_STABILIZATION_MODE,
    CameraStabilizationMode,
    camera_set_u32,
    encode_stabilization_control
);

camera_scalar_getter!(
    /// Read one Android camera torch mode.
    destack_device_camera_stream_torch_mode,
    CAMERA_CONTROL_TORCH_MODE,
    CameraTorchMode,
    camera_get_u32,
    decode_torch_control
);
camera_scalar_setter!(
    /// Write one Android camera torch mode.
    destack_device_camera_stream_set_torch_mode,
    CAMERA_CONTROL_TORCH_MODE,
    CameraTorchMode,
    camera_set_u32,
    encode_torch_control
);
