use super::color::*;
use super::core::*;
use super::exposure::*;
use super::lens::*;
use crate::diagnostic::RuntimeResult;
use crate::platform::device::{
    CameraControlPatch, CameraExposureMode, CameraFocusMode, CameraStabilizationMode,
    CameraTorchMode, CameraWhiteBalanceMode,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

/// Apply one grouped camera control patch.
pub(crate) unsafe fn destack_device_camera_stream_configure_controls(
    binding: &BindingCallContext,
    handle: resource::CameraStreamHandle,
    controls: CameraControlPatch,
) -> RuntimeResult<()> {
    let controls = unsafe { CameraControlPatch::into_value(controls)? };

    // mode-like controls
    if let Some(value) = controls.exposure_mode {
        unsafe {
            destack_device_camera_stream_set_exposure_mode(
                binding,
                handle,
                CameraExposureMode::from_value(binding, value),
            )?;
        }
    }

    if let Some(value) = controls.white_balance_mode {
        unsafe {
            destack_device_camera_stream_set_white_balance_mode(
                binding,
                handle,
                CameraWhiteBalanceMode::from_value(binding, value),
            )?;
        }
    }

    if let Some(value) = controls.focus_mode {
        unsafe {
            destack_device_camera_stream_set_focus_mode(
                binding,
                handle,
                CameraFocusMode::from_value(binding, value),
            )?;
        }
    }

    if let Some(value) = controls.stabilization_mode {
        unsafe {
            destack_device_camera_stream_set_stabilization_mode(
                binding,
                handle,
                CameraStabilizationMode::from_value(binding, value),
            )?;
        }
    }

    if let Some(value) = controls.torch_mode {
        unsafe {
            destack_device_camera_stream_set_torch_mode(
                binding,
                handle,
                CameraTorchMode::from_value(binding, value),
            )?;
        }
    }

    // numeric controls
    if let Some(value) = controls.exposure_compensation_ev {
        unsafe {
            destack_device_camera_stream_set_exposure_compensation(binding, handle, value)?;
        }
    }

    if let Some(value) = controls.exposure_time_ns {
        unsafe {
            destack_device_camera_stream_set_exposure_time_ns(binding, handle, value)?;
        }
    }

    if let Some(value) = controls.sensor_iso {
        unsafe {
            destack_device_camera_stream_set_sensor_iso(binding, handle, value)?;
        }
    }

    if let Some(value) = controls.white_balance_kelvin {
        unsafe {
            destack_device_camera_stream_set_white_balance_kelvin(binding, handle, value)?;
        }
    }

    if let Some(value) = controls.focus_distance_diopters {
        unsafe {
            destack_device_camera_stream_set_focus_distance_diopters(binding, handle, value)?;
        }
    }

    if let Some(value) = controls.brightness {
        unsafe {
            destack_device_camera_stream_set_brightness(binding, handle, value)?;
        }
    }

    if let Some(value) = controls.contrast {
        unsafe {
            destack_device_camera_stream_set_contrast(binding, handle, value)?;
        }
    }

    if let Some(value) = controls.saturation {
        unsafe {
            destack_device_camera_stream_set_saturation(binding, handle, value)?;
        }
    }

    if let Some(value) = controls.sharpness {
        unsafe {
            destack_device_camera_stream_set_sharpness(binding, handle, value)?;
        }
    }

    if let Some(value) = controls.pan_degrees {
        unsafe {
            destack_device_camera_stream_set_pan_degrees(binding, handle, value)?;
        }
    }

    if let Some(value) = controls.tilt_degrees {
        unsafe {
            destack_device_camera_stream_set_tilt_degrees(binding, handle, value)?;
        }
    }

    if let Some(value) = controls.zoom_ratio {
        unsafe {
            destack_device_camera_stream_set_zoom_ratio(binding, handle, value)?;
        }
    }

    Ok(())
}
