use super::core::{
    with_camera_harness_context, with_camera_native_context,
    with_first_openable_local_camera_stream_native, with_first_openable_local_camera_stream_vm,
};
use crate::platform::device::tests::assert_ok_or_expected_error;
use crate::platform::device::{
    CameraStabilizationMode, CameraTorchMode, native as device_native, vm as device_vm,
};
use crate::platform::diagnostic::PlatformErrorCode;

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
                let exposure_modes = capability
                    .controls
                    .exposure_modes
                    .expect("camera exposure modes should be present");
                let exposure_modes = exposure_modes.as_slice();
                if !exposure_modes.is_empty() {
                    assert!(exposure_mode.is_some());
                }

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
                let white_balance_modes = capability
                    .controls
                    .white_balance_modes
                    .expect("camera white balance modes should be present");
                let white_balance_modes = white_balance_modes.as_slice();
                if !white_balance_modes.is_empty() {
                    assert!(white_balance_mode.is_some());
                }

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
                let focus_modes = capability
                    .controls
                    .focus_modes
                    .expect("camera focus modes should be present");
                let focus_modes = focus_modes.as_slice();
                if !focus_modes.is_empty() {
                    assert!(focus_mode.is_some());
                }

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
                let torch_modes = capability
                    .controls
                    .torch_modes
                    .expect("camera torch modes should be present");
                let torch_modes = torch_modes.as_slice();
                if torch_modes.iter().any(|mode| *mode != CameraTorchMode::Off) {
                    assert!(torch_mode.is_some());
                }

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
                let stabilization_modes = capability
                    .controls
                    .stabilization_modes
                    .expect("camera stabilization modes should be present");
                let stabilization_modes = stabilization_modes.as_slice();
                if stabilization_modes
                    .iter()
                    .any(|mode| *mode != CameraStabilizationMode::Off)
                {
                    assert!(stabilization_mode.is_some());
                }

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
                let exposure_modes = capability
                    .controls
                    .exposure_modes
                    .expect("camera exposure modes should be present");
                if !exposure_modes.is_empty() {
                    assert!(exposure_mode.is_some());
                }

                // advertised white-balance modes should be queryable
                let white_balance_mode = assert_ok_or_expected_error(
                    device_vm::destack_device_camera_stream_white_balance_mode(
                        context.call_context,
                        vm_context,
                        stream_handle,
                    ),
                    &[PlatformErrorCode::NotSupported],
                )?;
                let white_balance_modes = capability
                    .controls
                    .white_balance_modes
                    .expect("camera white balance modes should be present");
                if !white_balance_modes.is_empty() {
                    assert!(white_balance_mode.is_some());
                }

                // advertised focus modes should be queryable
                let focus_mode = assert_ok_or_expected_error(
                    device_vm::destack_device_camera_stream_focus_mode(
                        context.call_context,
                        vm_context,
                        stream_handle,
                    ),
                    &[PlatformErrorCode::NotSupported],
                )?;
                let focus_modes = capability
                    .controls
                    .focus_modes
                    .expect("camera focus modes should be present");
                if !focus_modes.is_empty() {
                    assert!(focus_mode.is_some());
                }

                // advertised torch modes should be queryable
                let torch_mode = assert_ok_or_expected_error(
                    device_vm::destack_device_camera_stream_torch_mode(
                        context.call_context,
                        vm_context,
                        stream_handle,
                    ),
                    &[PlatformErrorCode::NotSupported],
                )?;
                let torch_modes = capability
                    .controls
                    .torch_modes
                    .expect("camera torch modes should be present");
                if torch_modes.iter().any(|mode| *mode != CameraTorchMode::Off) {
                    assert!(torch_mode.is_some());
                }

                // advertised stabilization modes should be queryable
                let stabilization_mode = assert_ok_or_expected_error(
                    device_vm::destack_device_camera_stream_stabilization_mode(
                        context.call_context,
                        vm_context,
                        stream_handle,
                    ),
                    &[PlatformErrorCode::NotSupported],
                )?;
                let stabilization_modes = capability
                    .controls
                    .stabilization_modes
                    .expect("camera stabilization modes should be present");
                if stabilization_modes
                    .iter()
                    .any(|mode| *mode != CameraStabilizationMode::Off)
                {
                    assert!(stabilization_mode.is_some());
                }

                Ok(())
            },
        )
    });
}
