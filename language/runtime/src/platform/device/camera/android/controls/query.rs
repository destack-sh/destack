use super::super::core::{
    AndroidCameraStreamResource, CameraControlModes, camera_control_capabilities,
    camera_stream_resource,
};
use super::common::optional_control_value;
use super::operations::*;
use crate::diagnostic::RuntimeResult;
use crate::platform::device::{
    CameraControlCapabilities, CameraControlState, CameraControlStateValue,
    CameraExposureCompensationRange, CameraExposureMode, CameraExposureTimeRange,
    CameraFloatControlRange, CameraFocusDistanceRange, CameraFocusMode, CameraSensorIsoRange,
    CameraStabilizationMode, CameraTorchMode, CameraWhiteBalanceMode, CameraWhiteBalanceRange,
};
use crate::platform::{NativeAbiCodec, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Read one grouped camera control-state snapshot.
pub(crate) unsafe fn destack_device_camera_stream_control_state(
    binding: &BindingCallContext,
    out: *mut CameraControlState,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // grouped state
    let state = CameraControlStateValue {
        exposure_mode: optional_control_value(|out| unsafe {
            destack_device_camera_stream_exposure_mode(binding, out, handle)
        })?,
        exposure_compensation_ev: optional_control_value(|out| unsafe {
            destack_device_camera_stream_exposure_compensation(binding, out, handle)
        })?,
        exposure_time_ns: optional_control_value(|out| unsafe {
            destack_device_camera_stream_exposure_time_ns(binding, out, handle)
        })?,
        sensor_iso: optional_control_value(|out| unsafe {
            destack_device_camera_stream_sensor_iso(binding, out, handle)
        })?,
        white_balance_mode: optional_control_value(|out| unsafe {
            destack_device_camera_stream_white_balance_mode(binding, out, handle)
        })?,
        white_balance_kelvin: optional_control_value(|out| unsafe {
            destack_device_camera_stream_white_balance_kelvin(binding, out, handle)
        })?,
        focus_mode: optional_control_value(|out| unsafe {
            destack_device_camera_stream_focus_mode(binding, out, handle)
        })?,
        focus_distance_diopters: optional_control_value(|out| unsafe {
            destack_device_camera_stream_focus_distance_diopters(binding, out, handle)
        })?,
        brightness: optional_control_value(|out| unsafe {
            destack_device_camera_stream_brightness(binding, out, handle)
        })?,
        contrast: optional_control_value(|out| unsafe {
            destack_device_camera_stream_contrast(binding, out, handle)
        })?,
        saturation: optional_control_value(|out| unsafe {
            destack_device_camera_stream_saturation(binding, out, handle)
        })?,
        sharpness: optional_control_value(|out| unsafe {
            destack_device_camera_stream_sharpness(binding, out, handle)
        })?,
        pan_degrees: optional_control_value(|out| unsafe {
            destack_device_camera_stream_pan_degrees(binding, out, handle)
        })?,
        tilt_degrees: optional_control_value(|out| unsafe {
            destack_device_camera_stream_tilt_degrees(binding, out, handle)
        })?,
        zoom_ratio: optional_control_value(|out| unsafe {
            destack_device_camera_stream_zoom_ratio(binding, out, handle)
        })?,
        stabilization_mode: optional_control_value(|out| unsafe {
            destack_device_camera_stream_stabilization_mode(binding, out, handle)
        })?,
        torch_mode: optional_control_value(|out| unsafe {
            destack_device_camera_stream_torch_mode(binding, out, handle)
        })?,
    };

    unsafe {
        out.write(CameraControlState::from_value(binding, state));
    }

    Ok(())
}

/// Read one grouped camera control-capability snapshot.
pub(crate) unsafe fn destack_device_camera_stream_control_capabilities(
    binding: &BindingCallContext,
    out: *mut CameraControlCapabilities,
    handle: resource::CameraStreamHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    // grouped capabilities
    let resource = camera_stream_resource::<AndroidCameraStreamResource>(
        binding,
        handle,
        "destack.device.camera.stream.controlCapabilities",
    )?;
    let mut capabilities = resource
        .capability
        .as_ref()
        .map(|capability| capability.controls.clone())
        .unwrap_or_else(|| {
            camera_control_capabilities(&CameraControlModes {
                exposure_modes: vec![CameraExposureMode::Auto],
                white_balance_modes: vec![CameraWhiteBalanceMode::Auto],
                focus_modes: vec![CameraFocusMode::Auto],
                stabilization_modes: vec![CameraStabilizationMode::Off],
                torch_modes: vec![CameraTorchMode::Off],
            })
        });

    capabilities.exposure_compensation_range =
        optional_control_value(|out: *mut CameraExposureCompensationRange| unsafe {
            destack_device_camera_stream_exposure_compensation_range(binding, out, handle)
        })?;
    capabilities.exposure_time_range =
        optional_control_value(|out: *mut CameraExposureTimeRange| unsafe {
            destack_device_camera_stream_exposure_time_range(binding, out, handle)
        })?;
    capabilities.sensor_iso_range =
        optional_control_value(|out: *mut CameraSensorIsoRange| unsafe {
            destack_device_camera_stream_sensor_iso_range(binding, out, handle)
        })?;
    capabilities.white_balance_range =
        optional_control_value(|out: *mut CameraWhiteBalanceRange| unsafe {
            destack_device_camera_stream_white_balance_range(binding, out, handle)
        })?;
    capabilities.focus_distance_range =
        optional_control_value(|out: *mut CameraFocusDistanceRange| unsafe {
            destack_device_camera_stream_focus_distance_range(binding, out, handle)
        })?;
    capabilities.brightness_range =
        optional_control_value(|out: *mut CameraFloatControlRange| unsafe {
            destack_device_camera_stream_brightness_range(binding, out, handle)
        })?;
    capabilities.contrast_range =
        optional_control_value(|out: *mut CameraFloatControlRange| unsafe {
            destack_device_camera_stream_contrast_range(binding, out, handle)
        })?;
    capabilities.saturation_range =
        optional_control_value(|out: *mut CameraFloatControlRange| unsafe {
            destack_device_camera_stream_saturation_range(binding, out, handle)
        })?;
    capabilities.sharpness_range =
        optional_control_value(|out: *mut CameraFloatControlRange| unsafe {
            destack_device_camera_stream_sharpness_range(binding, out, handle)
        })?;
    capabilities.pan_range = optional_control_value(|out| unsafe {
        destack_device_camera_stream_pan_range(binding, out, handle)
    })?;
    capabilities.tilt_range = optional_control_value(|out| unsafe {
        destack_device_camera_stream_tilt_range(binding, out, handle)
    })?;
    capabilities.zoom_ratio_range = optional_control_value(|out| unsafe {
        destack_device_camera_stream_zoom_ratio_range(binding, out, handle)
    })?;

    unsafe {
        out.write(CameraControlCapabilities::from_value(binding, capabilities));
    }

    Ok(())
}
