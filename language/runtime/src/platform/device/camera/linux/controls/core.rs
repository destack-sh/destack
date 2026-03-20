use crate::platform::device::camera::linux::core::*;
use crate::platform::device::camera::linux::device::{camera_io_error, xioctl};

macro_rules! unsupported_camera_binding {
    ($name:ident ( $($argument:ident : $argument_ty:ty),* ) -> $result:ty, $binding_name:literal) => {
        pub(crate) unsafe fn $name(
            _binding: &BindingCallContext,
            $($argument : $argument_ty),*
        ) -> RuntimeResult<$result> {
            let _ = ($($argument),*);

            Err(shared_camera_not_supported($binding_name))
        }
    };
    ($name:ident ( $($argument:ident : $argument_ty:ty),* ), $binding_name:literal) => {
        pub(crate) unsafe fn $name(
            _binding: &BindingCallContext,
            $($argument : $argument_ty),*
        ) -> RuntimeResult<()> {
            let _ = ($($argument),*);

            Err(shared_camera_not_supported($binding_name))
        }
    };
}

pub(super) use unsupported_camera_binding;

/// Read one integer V4L2 control as a floating-point value.
pub(super) fn read_float_control_value(
    descriptor: RawFd,
    control_id: u32,
    operation: &'static str,
) -> RuntimeResult<f64> {
    let Some(_) = query_control_info(descriptor, control_id, operation)? else {
        return Err(camera_not_supported(operation));
    };

    Ok(read_control_value(descriptor, control_id, operation)? as f64)
}

/// Read one integer host control range as one float range.
pub(super) fn read_float_control_range(
    descriptor: RawFd,
    control_id: u32,
    operation: &'static str,
) -> RuntimeResult<CameraFloatControlRange> {
    let Some(info) = query_control_info(descriptor, control_id, operation)? else {
        return Err(camera_not_supported(operation));
    };

    Ok(float_range_from_info(info))
}

/// Write one float value into one integer host control.
pub(super) fn write_float_control_value(
    descriptor: RawFd,
    control_id: u32,
    value: f64,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(_) = query_control_info(descriptor, control_id, operation)? else {
        return Err(camera_not_supported(operation));
    };

    write_control_value(descriptor, control_id, value.round() as i64, operation)
}

/// Return whether one V4L2 capability bitset supports single-plane capture.
pub(crate) fn supports_single_plane_capture(capabilities: u32) -> bool {
    (capabilities & v4l2::V4L2_CAP_VIDEO_CAPTURE) != 0
}

/// Return whether one V4L2 capability set exposes the read interface.
pub(crate) fn supports_read_capture(capabilities: u32) -> bool {
    (capabilities & v4l2::V4L2_CAP_READWRITE) != 0
}

/// Return whether one V4L2 capability set exposes streaming capture.
pub(crate) fn supports_streaming_capture(capabilities: u32) -> bool {
    (capabilities & v4l2::V4L2_CAP_STREAMING) != 0
}

/// Build one not-supported error for one camera binding.
pub(crate) fn camera_not_supported(binding_name: &'static str) -> Box<RuntimeError> {
    shared_camera_not_supported(binding_name)
}

/// One queried V4L2 control descriptor snapshot.
#[derive(Debug, Clone, Copy)]
pub(super) struct V4l2ControlInfo {
    /// Control kind selector.
    pub(crate) kind: u32,
    /// Minimum raw host value.
    pub(crate) minimum: i64,
    /// Maximum raw host value.
    pub(crate) maximum: i64,
    /// Raw control step.
    pub(crate) step: i64,
    /// Raw default value.
    pub(crate) default: i64,
    /// V4L2 control flags.
    pub(crate) flags: u32,
}

/// Run one V4L2 ioctl and return false for unsupported requests.
pub(super) fn try_xioctl<T>(
    descriptor: RawFd,
    request: libc::c_ulong,
    value: &mut T,
    operation: &'static str,
    syscall: &'static str,
) -> RuntimeResult<bool> {
    loop {
        let status =
            unsafe { libc::ioctl(descriptor, request, value as *mut T as *mut libc::c_void) };
        if status == 0 {
            return Ok(true);
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }

        if errno == libc::EINVAL || errno == libc::ENOTTY {
            return Ok(false);
        }

        return Err(camera_io_error(operation, syscall, "camera ioctl failed"));
    }
}

/// Return whether one queried V4L2 control should be treated as unavailable.
pub(super) fn is_unavailable_control(info: &V4l2ControlInfo) -> bool {
    (info.flags & v4l2::V4L2_CTRL_FLAG_DISABLED) != 0
        || (info.flags & v4l2::V4L2_CTRL_FLAG_INACTIVE) != 0
}

/// Query one host control descriptor when available.
pub(super) fn query_control_info(
    descriptor: RawFd,
    control_id: u32,
    operation: &'static str,
) -> RuntimeResult<Option<V4l2ControlInfo>> {
    // prefer the extended descriptor form when the driver exposes it
    let mut extended = unsafe { std::mem::zeroed::<v4l2::v4l2_query_ext_ctrl>() };
    extended.id = control_id;
    if try_xioctl(
        descriptor,
        VIDIOC_QUERY_EXT_CTRL as libc::c_ulong,
        &mut extended,
        operation,
        "VIDIOC_QUERY_EXT_CTRL",
    )? {
        let info = V4l2ControlInfo {
            kind: extended.type_,
            minimum: extended.minimum,
            maximum: extended.maximum,
            step: extended.step as i64,
            default: extended.default_value,
            flags: extended.flags,
        };

        if is_unavailable_control(&info) {
            return Ok(None);
        }

        return Ok(Some(info));
    }

    // otherwise fall back to the classic descriptor form
    let mut classic = unsafe { std::mem::zeroed::<v4l2::v4l2_queryctrl>() };
    classic.id = control_id;
    if !try_xioctl(
        descriptor,
        VIDIOC_QUERYCTRL as libc::c_ulong,
        &mut classic,
        operation,
        "VIDIOC_QUERYCTRL",
    )? {
        return Ok(None);
    }

    let info = V4l2ControlInfo {
        kind: classic.type_,
        minimum: classic.minimum as i64,
        maximum: classic.maximum as i64,
        step: classic.step as i64,
        default: classic.default_value as i64,
        flags: classic.flags,
    };

    if is_unavailable_control(&info) {
        return Ok(None);
    }

    Ok(Some(info))
}

/// Read one raw host control value.
pub(super) fn read_control_value(
    descriptor: RawFd,
    control_id: u32,
    operation: &'static str,
) -> RuntimeResult<i64> {
    let mut control = v4l2::v4l2_control {
        id: control_id,
        value: 0,
    };

    xioctl(
        descriptor,
        VIDIOC_G_CTRL as libc::c_ulong,
        &mut control,
        operation,
        "VIDIOC_G_CTRL",
    )?;

    Ok(control.value as i64)
}

/// Write one raw host control value.
pub(super) fn write_control_value(
    descriptor: RawFd,
    control_id: u32,
    value: i64,
    operation: &'static str,
) -> RuntimeResult<()> {
    let value = i32::try_from(value).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "value",
            "camera control value is outside one 32-bit host range",
        ))
        .boxed()
    })?;

    let mut control = v4l2::v4l2_control {
        id: control_id,
        value,
    };

    xioctl(
        descriptor,
        VIDIOC_S_CTRL as libc::c_ulong,
        &mut control,
        operation,
        "VIDIOC_S_CTRL",
    )
}

/// Query one integer-menu value by index.
pub(super) fn query_integer_menu_value(
    descriptor: RawFd,
    control_id: u32,
    index: u32,
    operation: &'static str,
) -> RuntimeResult<Option<i64>> {
    let mut menu = unsafe { std::mem::zeroed::<v4l2::v4l2_querymenu>() };
    menu.id = control_id;
    menu.index = index;

    if !try_xioctl(
        descriptor,
        VIDIOC_QUERYMENU as libc::c_ulong,
        &mut menu,
        operation,
        "VIDIOC_QUERYMENU",
    )? {
        return Ok(None);
    }

    Ok(Some(unsafe { menu.__bindgen_anon_1.value }))
}

/// Query one integer-menu range and infer one linear step when possible.
pub(super) fn query_integer_menu_range(
    descriptor: RawFd,
    control_id: u32,
    info: V4l2ControlInfo,
    operation: &'static str,
) -> RuntimeResult<(i64, i64, i64, i64)> {
    let mut values = Vec::new();

    for index in info.minimum.max(0) as u32..=info.maximum.max(0) as u32 {
        let Some(value) = query_integer_menu_value(descriptor, control_id, index, operation)?
        else {
            continue;
        };

        values.push((index, value));
    }

    if values.is_empty() {
        return Err(camera_not_supported(operation));
    }

    let minimum = values.iter().map(|(_, value)| *value).min().unwrap_or(0);
    let maximum = values.iter().map(|(_, value)| *value).max().unwrap_or(0);
    let default = values
        .iter()
        .find(|(index, _)| *index as i64 == info.default)
        .map(|(_, value)| *value)
        .unwrap_or(minimum);

    let mut step = 0i64;
    if values.len() >= 2 {
        let first_step = values[1].1 - values[0].1;
        if values
            .windows(2)
            .all(|window| window[1].1 - window[0].1 == first_step)
        {
            step = first_step.abs();
        }
    }

    Ok((minimum, maximum, default, step))
}

/// Read one raw exposure-compensation value in milli-EV.
pub(super) fn read_exposure_bias_milli_ev(
    descriptor: RawFd,
    operation: &'static str,
) -> RuntimeResult<i64> {
    let Some(info) = query_control_info(descriptor, v4l2::V4L2_CID_AUTO_EXPOSURE_BIAS, operation)?
    else {
        return Err(camera_not_supported(operation));
    };

    let raw_value = read_control_value(descriptor, v4l2::V4L2_CID_AUTO_EXPOSURE_BIAS, operation)?;
    if info.kind == v4l2::v4l2_ctrl_type_V4L2_CTRL_TYPE_INTEGER_MENU as u32 {
        let Some(value) = query_integer_menu_value(
            descriptor,
            v4l2::V4L2_CID_AUTO_EXPOSURE_BIAS,
            raw_value.max(0) as u32,
            operation,
        )?
        else {
            return Err(camera_not_supported(operation));
        };

        return Ok(value);
    }

    Ok(raw_value)
}

/// Write one exposure-compensation value in milli-EV.
pub(super) fn write_exposure_bias_milli_ev(
    descriptor: RawFd,
    milli_ev: i64,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(info) = query_control_info(descriptor, v4l2::V4L2_CID_AUTO_EXPOSURE_BIAS, operation)?
    else {
        return Err(camera_not_supported(operation));
    };

    if info.kind == v4l2::v4l2_ctrl_type_V4L2_CTRL_TYPE_INTEGER_MENU as u32 {
        let mut best_index = None;
        let mut best_distance = i64::MAX;

        for index in info.minimum.max(0) as u32..=info.maximum.max(0) as u32 {
            let Some(value) = query_integer_menu_value(
                descriptor,
                v4l2::V4L2_CID_AUTO_EXPOSURE_BIAS,
                index,
                operation,
            )?
            else {
                continue;
            };

            let distance = (value - milli_ev).abs();
            if distance < best_distance {
                best_distance = distance;
                best_index = Some(index as i64);
            }
        }

        let best_index = best_index.ok_or_else(|| camera_not_supported(operation))?;
        return write_control_value(
            descriptor,
            v4l2::V4L2_CID_AUTO_EXPOSURE_BIAS,
            best_index,
            operation,
        );
    }

    write_control_value(
        descriptor,
        v4l2::V4L2_CID_AUTO_EXPOSURE_BIAS,
        milli_ev,
        operation,
    )
}

/// Query one exposure-compensation range in milli-EV.
pub(super) fn exposure_bias_range_milli_ev(
    descriptor: RawFd,
    operation: &'static str,
) -> RuntimeResult<(i64, i64, i64, i64)> {
    let Some(info) = query_control_info(descriptor, v4l2::V4L2_CID_AUTO_EXPOSURE_BIAS, operation)?
    else {
        return Err(camera_not_supported(operation));
    };

    if info.kind == v4l2::v4l2_ctrl_type_V4L2_CTRL_TYPE_INTEGER_MENU as u32 {
        return query_integer_menu_range(
            descriptor,
            v4l2::V4L2_CID_AUTO_EXPOSURE_BIAS,
            info,
            operation,
        );
    }

    Ok((info.minimum, info.maximum, info.default, info.step))
}

/// Build one float control range from one integer host control.
pub(super) fn float_range_from_info(info: V4l2ControlInfo) -> CameraFloatControlRange {
    CameraFloatControlRange {
        minimum: info.minimum as f64,
        maximum: info.maximum as f64,
        default: info.default as f64,
        step: info.step as f64,
    }
}

/// Query one supported mode snapshot for one endpoint descriptor.
pub(crate) fn query_camera_control_modes(descriptor: RawFd) -> RuntimeResult<CameraControlModes> {
    let mut exposure_modes = Vec::new();
    let mut white_balance_modes = Vec::new();
    let mut focus_modes = Vec::new();
    let mut stabilization_modes = Vec::new();
    let mut torch_modes = Vec::new();

    // exposure modes
    if query_control_info(
        descriptor,
        v4l2::V4L2_CID_EXPOSURE_AUTO,
        "destack.device.camera.device.streamCapabilityList",
    )?
    .is_some()
    {
        exposure_modes.push(CameraExposureMode::ContinuousAuto);
    }
    if query_control_info(
        descriptor,
        v4l2::V4L2_CID_EXPOSURE_ABSOLUTE,
        "destack.device.camera.device.streamCapabilityList",
    )?
    .is_some()
    {
        exposure_modes.push(CameraExposureMode::Manual);
    }
    // white balance modes
    if query_control_info(
        descriptor,
        v4l2::V4L2_CID_AUTO_WHITE_BALANCE,
        "destack.device.camera.device.streamCapabilityList",
    )?
    .is_some()
    {
        white_balance_modes.push(CameraWhiteBalanceMode::ContinuousAuto);
    }
    if query_control_info(
        descriptor,
        v4l2::V4L2_CID_WHITE_BALANCE_TEMPERATURE,
        "destack.device.camera.device.streamCapabilityList",
    )?
    .is_some()
    {
        white_balance_modes.push(CameraWhiteBalanceMode::Manual);
    }
    // focus modes
    if query_control_info(
        descriptor,
        v4l2::V4L2_CID_FOCUS_AUTO,
        "destack.device.camera.device.streamCapabilityList",
    )?
    .is_some()
    {
        focus_modes.push(CameraFocusMode::ContinuousAuto);
    }
    if query_control_info(
        descriptor,
        v4l2::V4L2_CID_FOCUS_ABSOLUTE,
        "destack.device.camera.device.streamCapabilityList",
    )?
    .is_some()
    {
        focus_modes.push(CameraFocusMode::Manual);
    }
    // stabilization modes
    if query_control_info(
        descriptor,
        v4l2::V4L2_CID_IMAGE_STABILIZATION,
        "destack.device.camera.device.streamCapabilityList",
    )?
    .is_some()
    {
        stabilization_modes.push(CameraStabilizationMode::Off);
        stabilization_modes.push(CameraStabilizationMode::Standard);
    }

    // torch modes
    if query_control_info(
        descriptor,
        v4l2::V4L2_CID_FLASH_LED_MODE,
        "destack.device.camera.device.streamCapabilityList",
    )?
    .is_some()
    {
        torch_modes.push(CameraTorchMode::Off);
        torch_modes.push(CameraTorchMode::On);
        torch_modes.push(CameraTorchMode::Auto);
    }

    Ok(CameraControlModes {
        exposure_modes,
        white_balance_modes,
        focus_modes,
        stabilization_modes,
        torch_modes,
    })
}
