use super::core::{
    assert_camera_frame_or_would_block, with_camera_harness_context, with_camera_native_context,
    with_first_openable_local_camera_stream_native, with_first_openable_local_camera_stream_vm,
};
use crate::platform::device::{
    CameraColorSpace, CameraDynamicRange, native as device_native, vm as device_vm,
};
use crate::platform::{NativeAbiCodec, VmAbiCodec};

/// Open one listed camera, query stream metadata, and close it again.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_open_queries_stream_metadata_and_closes_first_device_when_present() {
    with_camera_native_context(|call_context| {
        with_first_openable_local_camera_stream_native(
            call_context,
            |_device_handle, stream_handle, capability| {
                unsafe {
                    device_native::destack_device_camera_stream_start(call_context, stream_handle)?;
                }

                let mut active_config = std::mem::MaybeUninit::uninit();
                unsafe {
                    device_native::destack_device_camera_stream_config(
                        call_context,
                        active_config.as_mut_ptr(),
                        stream_handle,
                    )?;
                }
                let active_config = unsafe { active_config.assume_init() };
                let active_config = unsafe { NativeAbiCodec::into_value(active_config)? };
                assert_eq!(active_config, capability.config);
                assert!(
                    capability.color_spaces.contains(
                        &active_config
                            .color_space
                            .unwrap_or(CameraColorSpace::Unknown,)
                    )
                );
                assert!(
                    capability.dynamic_ranges.contains(
                        &active_config
                            .dynamic_range
                            .unwrap_or(CameraDynamicRange::Standard,)
                    )
                );

                let mut frame_out = std::mem::MaybeUninit::uninit();
                let frame_result = unsafe {
                    device_native::destack_device_camera_stream_try_read(
                        call_context,
                        frame_out.as_mut_ptr(),
                        stream_handle,
                    )
                };
                let frame = assert_camera_frame_or_would_block(frame_result)?;
                if let Some(()) = frame.map(|_| ()) {
                    let _ = unsafe { frame_out.assume_init() };
                }

                unsafe {
                    device_native::destack_device_camera_stream_stop(call_context, stream_handle)?;
                }

                Ok(())
            },
        )
    });
}

/// Route camera open and metadata queries through the VM entrypoint when devices are present.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_vm_open_queries_stream_metadata_when_devices_are_present() {
    with_camera_harness_context(|context| {
        with_first_openable_local_camera_stream_vm(
            &context,
            |vm_context, _device_handle, stream_handle, capability| {
                device_vm::destack_device_camera_stream_start(
                    context.call_context,
                    vm_context,
                    stream_handle,
                )?;

                let active_config = device_vm::destack_device_camera_stream_config(
                    context.call_context,
                    vm_context,
                    stream_handle,
                )?;
                let active_config = VmAbiCodec::into_value(active_config, &vm_context.read())?;
                assert_eq!(active_config, capability.config);
                assert!(
                    capability.color_spaces.contains(
                        &active_config
                            .color_space
                            .unwrap_or(CameraColorSpace::Unknown,)
                    )
                );
                assert!(
                    capability.dynamic_ranges.contains(
                        &active_config
                            .dynamic_range
                            .unwrap_or(CameraDynamicRange::Standard,)
                    )
                );

                let frame = device_vm::destack_device_camera_stream_try_read(
                    context.call_context,
                    vm_context,
                    stream_handle,
                );
                let frame = assert_camera_frame_or_would_block(frame)?;
                if let Some(frame) = frame {
                    drop(frame.into_value(&vm_context.read())?);
                }

                device_vm::destack_device_camera_stream_stop(
                    context.call_context,
                    vm_context,
                    stream_handle,
                )?;

                Ok(())
            },
        )
    });
}
