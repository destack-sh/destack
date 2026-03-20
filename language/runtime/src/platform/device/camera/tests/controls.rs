use super::core::{
    with_camera_harness_context, with_camera_native_context,
    with_first_openable_local_camera_stream_native, with_first_openable_local_camera_stream_vm,
};
use crate::platform::device::tests::assert_ok_or_expected_error;
use crate::platform::device::{
    CameraExposureMode, CameraFocusMode, CameraStabilizationMode, CameraTorchMode,
    CameraWhiteBalanceMode, native as device_native, vm as device_vm,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// Assert one optional camera control query matches one advertised mode list.
fn assert_mode_query_matches_capability<T>(value: Option<T>, modes: Option<&[T]>)
where
    T: Copy + PartialEq + std::fmt::Debug,
{
    let Some(modes) = modes else {
        assert!(value.is_none());
        return;
    };

    if modes.is_empty() {
        assert!(value.is_none());
        return;
    }

    let value = value.expect("camera control query should succeed when capability is advertised");
    assert!(modes.contains(&value));
}

/// Decode one native control query result from one out pointer.
fn native_optional_control_value<T>(
    result: Option<()>,
    out: std::mem::MaybeUninit<T>,
) -> Option<T> {
    result?;

    Some(unsafe { out.assume_init() })
}

/// Query one capability-backed camera control on the first opened stream when present.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_controls_query_capability_backed_modes_for_one_open_stream_when_present() {
    with_camera_native_context(|call_context| {
        with_first_openable_local_camera_stream_native(
            call_context,
            |_device_handle, stream_handle, capability| {
                // advertised exposure modes should be queryable
                let mut exposure_mode_out = std::mem::MaybeUninit::uninit();
                let exposure_mode = assert_ok_or_expected_error(
                    unsafe {
                        device_native::destack_device_camera_stream_exposure_mode(
                            call_context,
                            exposure_mode_out.as_mut_ptr(),
                            stream_handle,
                        )
                    },
                    &[PlatformErrorCode::NotSupported],
                )?;
                let exposure_mode = native_optional_control_value(exposure_mode, exposure_mode_out);
                let exposure_modes = capability.controls.exposure_modes.as_deref();
                assert_mode_query_matches_capability::<CameraExposureMode>(
                    exposure_mode,
                    exposure_modes,
                );

                // advertised white-balance modes should be queryable
                let mut white_balance_mode_out = std::mem::MaybeUninit::uninit();
                let white_balance_mode = assert_ok_or_expected_error(
                    unsafe {
                        device_native::destack_device_camera_stream_white_balance_mode(
                            call_context,
                            white_balance_mode_out.as_mut_ptr(),
                            stream_handle,
                        )
                    },
                    &[PlatformErrorCode::NotSupported],
                )?;
                let white_balance_mode =
                    native_optional_control_value(white_balance_mode, white_balance_mode_out);
                let white_balance_modes = capability.controls.white_balance_modes.as_deref();
                assert_mode_query_matches_capability::<CameraWhiteBalanceMode>(
                    white_balance_mode,
                    white_balance_modes,
                );

                // advertised focus modes should be queryable
                let mut focus_mode_out = std::mem::MaybeUninit::uninit();
                let focus_mode = assert_ok_or_expected_error(
                    unsafe {
                        device_native::destack_device_camera_stream_focus_mode(
                            call_context,
                            focus_mode_out.as_mut_ptr(),
                            stream_handle,
                        )
                    },
                    &[PlatformErrorCode::NotSupported],
                )?;
                let focus_mode = native_optional_control_value(focus_mode, focus_mode_out);
                let focus_modes = capability.controls.focus_modes.as_deref();
                assert_mode_query_matches_capability::<CameraFocusMode>(focus_mode, focus_modes);

                // advertised torch modes should be queryable
                let mut torch_mode_out = std::mem::MaybeUninit::uninit();
                let torch_mode = assert_ok_or_expected_error(
                    unsafe {
                        device_native::destack_device_camera_stream_torch_mode(
                            call_context,
                            torch_mode_out.as_mut_ptr(),
                            stream_handle,
                        )
                    },
                    &[PlatformErrorCode::NotSupported],
                )?;
                let torch_mode = native_optional_control_value(torch_mode, torch_mode_out);
                let torch_modes = capability.controls.torch_modes.as_deref();
                assert_mode_query_matches_capability::<CameraTorchMode>(torch_mode, torch_modes);

                // advertised stabilization modes should be queryable
                let mut stabilization_mode_out = std::mem::MaybeUninit::uninit();
                let stabilization_mode = assert_ok_or_expected_error(
                    unsafe {
                        device_native::destack_device_camera_stream_stabilization_mode(
                            call_context,
                            stabilization_mode_out.as_mut_ptr(),
                            stream_handle,
                        )
                    },
                    &[PlatformErrorCode::NotSupported],
                )?;
                let stabilization_mode =
                    native_optional_control_value(stabilization_mode, stabilization_mode_out);
                let stabilization_modes = capability.controls.stabilization_modes.as_deref();
                assert_mode_query_matches_capability::<CameraStabilizationMode>(
                    stabilization_mode,
                    stabilization_modes,
                );

                Ok(())
            },
        )
    });
}

/// Route one capability-backed camera control through the VM entrypoint when one stream is present.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_vm_controls_query_capability_backed_modes_for_one_open_stream_when_present() {
    with_camera_harness_context(|context| {
        with_first_openable_local_camera_stream_vm(
            &context,
            |vm_context, _device_handle, stream_handle, capability| {
                // advertised exposure modes should be queryable
                let exposure_mode = assert_ok_or_expected_error(
                    device_vm::destack_device_camera_stream_exposure_mode(
                        context.call_context,
                        vm_context,
                        stream_handle,
                    ),
                    &[PlatformErrorCode::NotSupported],
                )?;
                let exposure_modes = capability.controls.exposure_modes.as_deref();
                assert_mode_query_matches_capability::<CameraExposureMode>(
                    exposure_mode,
                    exposure_modes,
                );

                // advertised white-balance modes should be queryable
                let white_balance_mode = assert_ok_or_expected_error(
                    device_vm::destack_device_camera_stream_white_balance_mode(
                        context.call_context,
                        vm_context,
                        stream_handle,
                    ),
                    &[PlatformErrorCode::NotSupported],
                )?;
                let white_balance_modes = capability.controls.white_balance_modes.as_deref();
                assert_mode_query_matches_capability::<CameraWhiteBalanceMode>(
                    white_balance_mode,
                    white_balance_modes,
                );

                // advertised focus modes should be queryable
                let focus_mode = assert_ok_or_expected_error(
                    device_vm::destack_device_camera_stream_focus_mode(
                        context.call_context,
                        vm_context,
                        stream_handle,
                    ),
                    &[PlatformErrorCode::NotSupported],
                )?;
                let focus_modes = capability.controls.focus_modes.as_deref();
                assert_mode_query_matches_capability::<CameraFocusMode>(focus_mode, focus_modes);

                // advertised torch modes should be queryable
                let torch_mode = assert_ok_or_expected_error(
                    device_vm::destack_device_camera_stream_torch_mode(
                        context.call_context,
                        vm_context,
                        stream_handle,
                    ),
                    &[PlatformErrorCode::NotSupported],
                )?;
                let torch_modes = capability.controls.torch_modes.as_deref();
                assert_mode_query_matches_capability::<CameraTorchMode>(torch_mode, torch_modes);

                // advertised stabilization modes should be queryable
                let stabilization_mode = assert_ok_or_expected_error(
                    device_vm::destack_device_camera_stream_stabilization_mode(
                        context.call_context,
                        vm_context,
                        stream_handle,
                    ),
                    &[PlatformErrorCode::NotSupported],
                )?;
                let stabilization_modes = capability.controls.stabilization_modes.as_deref();
                assert_mode_query_matches_capability::<CameraStabilizationMode>(
                    stabilization_mode,
                    stabilization_modes,
                );

                Ok(())
            },
        )
    });
}
