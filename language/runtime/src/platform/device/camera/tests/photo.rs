use super::core::{
    with_camera_harness_context, with_camera_native_context,
    with_first_openable_local_camera_stream_native, with_first_openable_local_camera_stream_vm,
};
use crate::platform::device::{
    CameraPhotoSettings, CameraPhotoSettingsValue, native as device_native, vm as device_vm,
};
use crate::platform::{NativeAbiCodec, VmAbiCodec};

/// Query still-photo state and capabilities for one opened camera stream when available.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_photo_queries_follow_one_open_stream_config_when_present() {
    with_camera_native_context(|call_context| {
        with_first_openable_local_camera_stream_native(
            call_context,
            |_device_handle, stream_handle, capability| {
                let config = capability.config;

                // query still-photo state
                let mut photo_state_out = std::mem::MaybeUninit::uninit();
                unsafe {
                    device_native::destack_device_camera_stream_photo_state(
                        call_context,
                        photo_state_out.as_mut_ptr(),
                        stream_handle,
                    )?;
                }
                let photo_state = unsafe { photo_state_out.assume_init() };
                let photo_state = unsafe { NativeAbiCodec::into_value(photo_state)? };

                // query still-photo capabilities
                let mut photo_capabilities_out = std::mem::MaybeUninit::uninit();
                unsafe {
                    device_native::destack_device_camera_stream_photo_capabilities(
                        call_context,
                        photo_capabilities_out.as_mut_ptr(),
                        stream_handle,
                    )?;
                }
                let photo_capabilities = unsafe { photo_capabilities_out.assume_init() };
                let photo_capabilities = unsafe { NativeAbiCodec::into_value(photo_capabilities)? };

                // assert that still-photo config tracks the opened stream config
                let state_options = photo_state
                    .options
                    .expect("photo state should expose options");
                assert_eq!(state_options.width, config.width);
                assert_eq!(state_options.height, config.height);
                assert_eq!(state_options.pixel_format, config.pixel_format);

                let capability_options = photo_capabilities.options;
                assert_eq!(capability_options.len(), 1);
                assert_eq!(capability_options[0].width, config.width);
                assert_eq!(capability_options[0].height, config.height);
                assert_eq!(capability_options[0].pixel_format, config.pixel_format);
                assert!(photo_capabilities.quality_range.is_none());
                assert!(photo_capabilities.flash_modes.is_none());
                assert!(photo_capabilities.red_eye_reduction.is_none());
                assert!(photo_state.quality.is_none());
                assert!(photo_state.flash_mode.is_none());
                assert!(photo_state.red_eye_reduction_enabled.is_none());

                Ok(())
            },
        )
    });
}

/// Capture one still photo through the native camera path when one stream is open.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_take_photo_returns_one_photo_for_one_open_stream_when_present() {
    with_camera_native_context(|call_context| {
        with_first_openable_local_camera_stream_native(
            call_context,
            |_device_handle, stream_handle, capability| {
                // running stream
                unsafe {
                    device_native::destack_device_camera_stream_start(call_context, stream_handle)?;
                }

                // photo capture
                let settings = CameraPhotoSettingsValue {
                    options: None,
                    quality: None,
                    flash_mode: None,
                    red_eye_reduction_enabled: None,
                };
                let settings =
                    <CameraPhotoSettings as NativeAbiCodec>::from_value(call_context, settings);
                let mut photo_out = std::mem::MaybeUninit::uninit();
                unsafe {
                    device_native::destack_device_camera_stream_take_photo(
                        call_context,
                        photo_out.as_mut_ptr(),
                        stream_handle,
                        settings,
                        5_000_000_000,
                    )?;
                }
                let photo = unsafe { photo_out.assume_init() };
                let photo = unsafe { NativeAbiCodec::into_value(photo)? };

                // photo shape
                assert_eq!(photo.width, capability.config.width);
                assert_eq!(photo.height, capability.config.height);
                assert_eq!(photo.pixel_format, capability.config.pixel_format);
                assert!(!photo.bytes.is_empty());

                // stream teardown
                unsafe {
                    device_native::destack_device_camera_stream_stop(call_context, stream_handle)?;
                }

                Ok(())
            },
        )
    });
}

/// Route still-photo state and capability queries through the VM when one stream is open.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_vm_photo_queries_follow_one_open_stream_config_when_present() {
    with_camera_harness_context(|context| {
        with_first_openable_local_camera_stream_vm(
            &context,
            |vm_context, _device_handle, stream_handle, capability| {
                let config = capability.config;

                // query still-photo state
                let photo_state = device_vm::destack_device_camera_stream_photo_state(
                    context.call_context,
                    vm_context,
                    stream_handle,
                )?;
                let photo_state = VmAbiCodec::into_value(photo_state, vm_context)?;

                // query still-photo capabilities
                let photo_capabilities =
                    device_vm::destack_device_camera_stream_photo_capabilities(
                        context.call_context,
                        vm_context,
                        stream_handle,
                    )?;
                let photo_capabilities = VmAbiCodec::into_value(photo_capabilities, vm_context)?;

                // assert that still-photo config tracks the opened stream config
                let state_options = photo_state
                    .options
                    .expect("photo state should expose options");
                assert_eq!(state_options.width, config.width);
                assert_eq!(state_options.height, config.height);
                assert_eq!(state_options.pixel_format, config.pixel_format);

                assert_eq!(photo_capabilities.options.len(), 1);
                assert_eq!(photo_capabilities.options[0].width, config.width);
                assert_eq!(photo_capabilities.options[0].height, config.height);
                assert_eq!(
                    photo_capabilities.options[0].pixel_format,
                    config.pixel_format
                );
                assert!(photo_capabilities.quality_range.is_none());
                assert!(photo_capabilities.flash_modes.is_none());
                assert!(photo_capabilities.red_eye_reduction.is_none());
                assert!(photo_state.quality.is_none());
                assert!(photo_state.flash_mode.is_none());
                assert!(photo_state.red_eye_reduction_enabled.is_none());

                Ok(())
            },
        )
    })
}

/// Capture one still photo through the VM camera path when one stream is open.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_vm_take_photo_returns_one_photo_for_one_open_stream_when_present() {
    with_camera_harness_context(|context| {
        with_first_openable_local_camera_stream_vm(
            &context,
            |vm_context, _device_handle, stream_handle, capability| {
                // running stream
                device_vm::destack_device_camera_stream_start(
                    context.call_context,
                    vm_context,
                    stream_handle,
                )?;

                // photo capture
                let settings = CameraPhotoSettingsValue {
                    options: None,
                    quality: None,
                    flash_mode: None,
                    red_eye_reduction_enabled: None,
                };
                let settings =
                    <CameraPhotoSettings as VmAbiCodec>::from_value(vm_context, settings)?;
                let photo = device_vm::destack_device_camera_stream_take_photo(
                    context.call_context,
                    vm_context,
                    stream_handle,
                    settings,
                    5_000_000_000,
                )?;
                let photo = VmAbiCodec::into_value(photo, vm_context)?;

                // photo shape
                assert_eq!(photo.width, capability.config.width);
                assert_eq!(photo.height, capability.config.height);
                assert_eq!(photo.pixel_format, capability.config.pixel_format);
                assert!(!photo.bytes.is_empty());

                // stream teardown
                device_vm::destack_device_camera_stream_stop(
                    context.call_context,
                    vm_context,
                    stream_handle,
                )?;

                Ok(())
            },
        )
    })
}
