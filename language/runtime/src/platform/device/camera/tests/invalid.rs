use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::device::tests::{assert_ok_or_expected_error, with_harness_context};
use crate::platform::device::{
    CameraControlPatch, CameraControlPatchValue, CameraPhotoSettings, CameraPhotoSettingsValue,
    CameraRecordingOptions, CameraRecordingOptionsValue,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{
    CameraDeviceHandle, CameraStreamHandle, CameraWatchHandle, ResourceId,
};

/// Assert that one camera operation rejects one invalid handle.
fn assert_invalid_handle<T>(result: Result<T, Box<RuntimeError>>) -> RuntimeResult<()> {
    let result = assert_ok_or_expected_error(result, &[PlatformErrorCode::InvalidArgumentValue])?;
    assert!(result.is_none());

    Ok(())
}

/// Reject one unknown camera device handle across the public device surface.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_rejects_unknown_device_handles() {
    with_harness_context(|mut context| {
        let unknown = CameraDeviceHandle(ResourceId::local(0));

        // device operations
        assert_invalid_handle(context.destack_device_camera_device_close(unknown))?;
        assert_invalid_handle(
            context.destack_device_camera_device_stream_capability_list(unknown),
        )?;

        Ok(())
    });
}

/// Reject one unknown camera watch handle across the public watch surface.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_rejects_unknown_watch_handles() {
    with_harness_context(|mut context| {
        let unknown = CameraWatchHandle(ResourceId::local(0));

        // watch operations
        assert_invalid_handle(context.destack_device_camera_device_watch_close(unknown))?;
        assert_invalid_handle(context.destack_device_camera_device_watch_read(unknown, 1))?;
        assert_invalid_handle(context.destack_device_camera_device_watch_try_read(unknown))?;

        Ok(())
    });
}

/// Reject one unknown camera stream handle across the public stream surface.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_rejects_unknown_stream_handles() {
    with_harness_context(|mut context| {
        let unknown = CameraStreamHandle(ResourceId::local(0));
        let controls = CameraControlPatchValue {
            exposure_mode: None,
            exposure_compensation_ev: None,
            exposure_time_ns: None,
            sensor_iso: None,
            white_balance_mode: None,
            white_balance_kelvin: None,
            focus_mode: None,
            focus_distance_diopters: None,
            brightness: None,
            contrast: None,
            saturation: None,
            sharpness: None,
            pan_degrees: None,
            tilt_degrees: None,
            zoom_ratio: Some(1.0),
            stabilization_mode: None,
            torch_mode: None,
        };
        let controls = context.harness_value_from::<CameraControlPatch, _>(controls)?;
        let photo_settings = CameraPhotoSettingsValue {
            options: None,
            quality: None,
            flash_mode: None,
            red_eye_reduction_enabled: None,
        };
        let photo_settings =
            context.harness_value_from::<CameraPhotoSettings, _>(photo_settings)?;
        let recording_options = CameraRecordingOptionsValue {
            output_path: None,
            container: None,
            video_codec: None,
            audio_enabled: None,
            audio_codec: None,
            video_bit_rate: None,
            audio_bit_rate: None,
            key_frame_interval_frames: None,
            maximum_duration_ns: None,
            maximum_bytes: None,
        };
        let recording_options =
            context.harness_value_from::<CameraRecordingOptions, _>(recording_options)?;

        // stream lifecycle
        assert_invalid_handle(context.destack_device_camera_stream_close(unknown))?;
        assert_invalid_handle(context.destack_device_camera_stream_start(unknown))?;
        assert_invalid_handle(context.destack_device_camera_stream_stop(unknown))?;

        // stream data
        assert_invalid_handle(context.destack_device_camera_stream_read(unknown, 1))?;
        assert_invalid_handle(context.destack_device_camera_stream_try_read(unknown))?;
        assert_invalid_handle(context.destack_device_camera_stream_config(unknown))?;

        // control lanes
        assert_invalid_handle(context.destack_device_camera_stream_control_state(unknown))?;
        assert_invalid_handle(context.destack_device_camera_stream_control_capabilities(unknown))?;
        assert_invalid_handle(
            context.destack_device_camera_stream_configure_controls(unknown, controls),
        )?;

        // recording lanes
        assert_invalid_handle(
            context.destack_device_camera_stream_recording_capabilities(unknown),
        )?;
        assert_invalid_handle(context.destack_device_camera_stream_recording_state(unknown))?;
        assert_invalid_handle(
            context.destack_device_camera_stream_start_recording(unknown, recording_options),
        )?;
        assert_invalid_handle(context.destack_device_camera_stream_pause_recording(unknown))?;
        assert_invalid_handle(context.destack_device_camera_stream_resume_recording(unknown))?;
        assert_invalid_handle(context.destack_device_camera_stream_stop_recording(unknown, 1))?;

        // photo lanes
        assert_invalid_handle(context.destack_device_camera_stream_photo_state(unknown))?;
        assert_invalid_handle(context.destack_device_camera_stream_photo_capabilities(unknown))?;
        assert_invalid_handle(context.destack_device_camera_stream_take_photo(
            unknown,
            photo_settings,
            1,
        ))?;

        Ok(())
    });
}
