use super::core::{
    with_camera_harness_context, with_camera_native_context,
    with_first_openable_local_camera_stream_native, with_first_openable_local_camera_stream_vm,
};
#[cfg(windows)]
use std::ffi::OsString;
#[cfg(target_os = "macos")]
use std::ffi::OsString;
#[cfg(target_os = "macos")]
use std::os::unix::ffi::OsStringExt;
#[cfg(windows)]
use std::os::windows::ffi::OsStringExt;
#[cfg(any(target_os = "macos", windows))]
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

#[cfg(any(target_os = "macos", windows))]
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::diagnostic::RuntimeResult;
#[cfg(any(target_os = "macos", windows))]
use crate::platform::core::monotonic_now_ns;
#[cfg(target_os = "linux")]
use crate::platform::core::monotonic_now_ns;
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use crate::platform::device::{
    CameraRecordingContainer, CameraRecordingOptions, CameraRecordingOptionsValue, CameraVideoCodec,
};
use crate::platform::device::{native as device_native, vm as device_vm};
#[cfg(any(target_os = "macos", windows))]
use crate::platform::fs::abi_generated::OsPathValue;
#[cfg(target_os = "linux")]
use crate::platform::fs::abi_generated::OsPathValue;
#[cfg(any(target_os = "macos", windows))]
use crate::platform::fs::core::os_path_from_path;
#[cfg(target_os = "linux")]
use crate::platform::fs::core::os_path_from_path;
use crate::platform::{NativeAbiCodec, VmAbiCodec};
#[cfg(any(target_os = "macos", windows))]
use crate::runtime::BindingCallContext;
#[cfg(target_os = "linux")]
use crate::runtime::BindingCallContext;

/// Build one unique recording output path for one test.
#[cfg(any(target_os = "macos", windows))]
fn unique_recording_path() -> PathBuf {
    let process_id = std::process::id();
    let timestamp_ns = monotonic_now_ns();
    let extension = recording_extension();

    std::env::temp_dir().join(format!(
        "destack-camera-recording-test-{process_id}-{timestamp_ns}.{extension}"
    ))
}

/// Return the expected recording-file extension for the current platform.
#[cfg(target_os = "macos")]
fn recording_extension() -> &'static str {
    "mov"
}

/// Return the expected recording-file extension for the current platform.
#[cfg(windows)]
fn recording_extension() -> &'static str {
    "mp4"
}

/// Return the expected recording container for the current platform.
#[cfg(target_os = "macos")]
fn expected_recording_container() -> CameraRecordingContainer {
    CameraRecordingContainer::Mov
}

/// Return the expected recording container for the current platform.
#[cfg(any(target_os = "linux", windows))]
fn expected_recording_container() -> CameraRecordingContainer {
    CameraRecordingContainer::Mp4
}

/// Return the expected recording-file extension for the current platform.
#[cfg(target_os = "linux")]
fn recording_extension() -> &'static str {
    "mp4"
}

/// Query recording state and capabilities for one opened camera stream when available.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_recording_queries_follow_one_open_stream_when_present() {
    with_camera_native_context(|call_context| {
        with_first_openable_local_camera_stream_native(
            call_context,
            |_device_handle, stream_handle, _capability| {
                // query recording state
                let mut recording_state_out = std::mem::MaybeUninit::uninit();
                unsafe {
                    device_native::destack_device_camera_stream_recording_state(
                        call_context,
                        recording_state_out.as_mut_ptr(),
                        stream_handle,
                    )?;
                }
                let recording_state = unsafe { recording_state_out.assume_init() };
                let recording_state = unsafe { NativeAbiCodec::into_value(recording_state)? };

                // query recording capabilities
                let mut recording_capabilities_out = std::mem::MaybeUninit::uninit();
                unsafe {
                    device_native::destack_device_camera_stream_recording_capabilities(
                        call_context,
                        recording_capabilities_out.as_mut_ptr(),
                        stream_handle,
                    )?;
                }
                let recording_capabilities = unsafe { recording_capabilities_out.assume_init() };
                let recording_capabilities =
                    unsafe { NativeAbiCodec::into_value(recording_capabilities)? };

                // baseline state
                assert!(!recording_state.active);
                assert!(!recording_state.paused);
                assert!(recording_state.options.is_none());
                assert!(recording_state.started_timestamp_ns.is_none());
                assert!(!recording_capabilities.audio_supported);
                assert!(recording_capabilities.audio_codecs.is_none());

                // per-target capability shape
                #[cfg(any(target_os = "macos", windows))]
                {
                    assert_eq!(recording_capabilities.containers.len(), 1);
                    assert_eq!(recording_capabilities.video_codecs.len(), 1);
                    assert!(recording_capabilities.pause_supported);
                }

                #[cfg(target_os = "linux")]
                {
                    if recording_capabilities.containers.is_empty() {
                        assert!(recording_capabilities.video_codecs.is_empty());
                    } else {
                        assert_eq!(recording_capabilities.containers.len(), 1);
                        assert_eq!(recording_capabilities.video_codecs.len(), 1);
                    }

                    assert!(!recording_capabilities.pause_supported);
                }

                Ok(())
            },
        )
    });
}

/// Route recording state and capability queries through the VM when one stream is open.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_vm_recording_queries_follow_one_open_stream_when_present() {
    with_camera_harness_context(|context| {
        with_first_openable_local_camera_stream_vm(
            &context,
            |vm_context, _device_handle, stream_handle, _capability| {
                // query recording state
                let recording_state = device_vm::destack_device_camera_stream_recording_state(
                    context.call_context,
                    vm_context,
                    stream_handle,
                )?;
                let recording_state = VmAbiCodec::into_value(recording_state, vm_context)?;

                // query recording capabilities
                let recording_capabilities =
                    device_vm::destack_device_camera_stream_recording_capabilities(
                        context.call_context,
                        vm_context,
                        stream_handle,
                    )?;
                let recording_capabilities =
                    VmAbiCodec::into_value(recording_capabilities, vm_context)?;

                // baseline state
                assert!(!recording_state.active);
                assert!(!recording_state.paused);
                assert!(recording_state.options.is_none());
                assert!(recording_state.started_timestamp_ns.is_none());
                assert!(!recording_capabilities.audio_supported);
                assert!(recording_capabilities.audio_codecs.is_none());

                // per-target capability shape
                #[cfg(any(target_os = "macos", windows))]
                {
                    assert_eq!(recording_capabilities.containers.len(), 1);
                    assert_eq!(recording_capabilities.video_codecs.len(), 1);
                    assert!(recording_capabilities.pause_supported);
                }

                #[cfg(target_os = "linux")]
                {
                    if recording_capabilities.containers.is_empty() {
                        assert!(recording_capabilities.video_codecs.is_empty());
                    } else {
                        assert_eq!(recording_capabilities.containers.len(), 1);
                        assert_eq!(recording_capabilities.video_codecs.len(), 1);
                    }

                    assert!(!recording_capabilities.pause_supported);
                }

                Ok(())
            },
        )
    })
}

/// Start and stop one recording on one opened camera stream when the backend supports it.
#[cfg(any(target_os = "macos", windows))]
#[test]
fn test_device_camera_recording_start_and_stop_return_one_output_descriptor_when_present() {
    with_camera_native_context(|call_context| {
        with_first_openable_local_camera_stream_native(
            call_context,
            |_device_handle, stream_handle, _capability| {
                // start the stream first
                unsafe {
                    device_native::destack_device_camera_stream_start(call_context, stream_handle)?;
                }

                // start one recording
                let output_path = unique_recording_path();
                let options = recording_options(call_context, &output_path)?;
                unsafe {
                    device_native::destack_device_camera_stream_start_recording(
                        call_context,
                        stream_handle,
                        options,
                    )?;
                }

                // verify the active recording state
                let mut recording_state_out = std::mem::MaybeUninit::uninit();
                unsafe {
                    device_native::destack_device_camera_stream_recording_state(
                        call_context,
                        recording_state_out.as_mut_ptr(),
                        stream_handle,
                    )?;
                }
                let recording_state = unsafe { recording_state_out.assume_init() };
                let recording_state = unsafe { NativeAbiCodec::into_value(recording_state)? };
                assert!(recording_state.active);
                assert!(!recording_state.paused);
                assert!(recording_state.options.is_some());
                assert!(recording_state.started_timestamp_ns.is_some());

                // let the backend write at least one frame
                std::thread::sleep(std::time::Duration::from_millis(100));

                // stop the recording and verify the output path
                let mut recording_out = std::mem::MaybeUninit::uninit();
                unsafe {
                    device_native::destack_device_camera_stream_stop_recording(
                        call_context,
                        recording_out.as_mut_ptr(),
                        stream_handle,
                        2_000_000_000,
                    )?;
                }
                let recording = unsafe { recording_out.assume_init() };
                let recording = unsafe { NativeAbiCodec::into_value(recording)? };
                let recorded_path = path_buf_from_value(recording.path);
                assert_eq!(recorded_path, output_path);
                assert!(recorded_path.exists());
                assert_eq!(recording.container, expected_recording_container());
                assert_eq!(recording.video_codec, Some(CameraVideoCodec::H264));
                assert!(recording.audio_codec.is_none());

                // verify the inactive recording state
                let mut recording_state_out = std::mem::MaybeUninit::uninit();
                unsafe {
                    device_native::destack_device_camera_stream_recording_state(
                        call_context,
                        recording_state_out.as_mut_ptr(),
                        stream_handle,
                    )?;
                }
                let recording_state = unsafe { recording_state_out.assume_init() };
                let recording_state = unsafe { NativeAbiCodec::into_value(recording_state)? };
                assert!(!recording_state.active);
                assert!(!recording_state.paused);
                assert!(recording_state.options.is_none());
                assert!(recording_state.started_timestamp_ns.is_none());

                // cleanup
                let _ = std::fs::remove_file(recorded_path);

                Ok(())
            },
        )
    });
}

/// Start and stop one recording through the VM camera path when the backend supports it.
#[cfg(any(target_os = "macos", windows))]
#[test]
fn test_device_camera_vm_recording_start_and_stop_return_one_output_descriptor_when_present() {
    with_camera_harness_context(|context| {
        with_first_openable_local_camera_stream_vm(
            &context,
            |vm_context, _device_handle, stream_handle, _capability| {
                // start the stream first
                device_vm::destack_device_camera_stream_start(
                    context.call_context,
                    vm_context,
                    stream_handle,
                )?;

                // start one recording
                let output_path = unique_recording_path();
                let options = recording_options_value(context.call_context, &output_path)?;
                let options = VmAbiCodec::from_value(vm_context, options)?;
                device_vm::destack_device_camera_stream_start_recording(
                    context.call_context,
                    vm_context,
                    stream_handle,
                    options,
                )?;

                // let the backend write at least one frame
                std::thread::sleep(std::time::Duration::from_millis(100));

                // stop the recording and verify the output path
                let recording = device_vm::destack_device_camera_stream_stop_recording(
                    context.call_context,
                    vm_context,
                    stream_handle,
                    2_000_000_000,
                )?;
                let recording = VmAbiCodec::into_value(recording, vm_context)?;
                let recorded_path = path_buf_from_value(recording.path);
                assert_eq!(recorded_path, output_path);
                assert!(recorded_path.exists());
                assert_eq!(recording.container, expected_recording_container());
                assert_eq!(recording.video_codec, Some(CameraVideoCodec::H264));
                assert!(recording.audio_codec.is_none());

                // cleanup
                let _ = std::fs::remove_file(recorded_path);

                Ok(())
            },
        )
    });
}

/// Start and stop one Linux recording when the ffmpeg recording pipeline is available.
#[cfg(target_os = "linux")]
#[test]
fn test_device_camera_linux_recording_start_and_stop_return_one_output_descriptor_when_present() {
    with_camera_native_context(|call_context| {
        with_first_openable_local_camera_stream_native(
            call_context,
            |_device_handle, stream_handle, _capability| {
                // query recording capabilities first
                let mut capabilities_out = std::mem::MaybeUninit::uninit();
                unsafe {
                    device_native::destack_device_camera_stream_recording_capabilities(
                        call_context,
                        capabilities_out.as_mut_ptr(),
                        stream_handle,
                    )?;
                }
                let capabilities = unsafe { capabilities_out.assume_init() };
                let capabilities = unsafe { NativeAbiCodec::into_value(capabilities)? };
                if capabilities.containers.is_empty() {
                    return Ok(());
                }

                // start the stream first
                unsafe {
                    device_native::destack_device_camera_stream_start(call_context, stream_handle)?;
                }

                // start one recording
                let output_path = unique_recording_path();
                let options = recording_options(call_context, &output_path)?;
                unsafe {
                    device_native::destack_device_camera_stream_start_recording(
                        call_context,
                        stream_handle,
                        options,
                    )?;
                }

                // let the backend write at least one frame
                std::thread::sleep(std::time::Duration::from_millis(250));

                // stop the recording and verify the output path
                let mut recording_out = std::mem::MaybeUninit::uninit();
                unsafe {
                    device_native::destack_device_camera_stream_stop_recording(
                        call_context,
                        recording_out.as_mut_ptr(),
                        stream_handle,
                        5_000_000_000,
                    )?;
                }
                let recording = unsafe { recording_out.assume_init() };
                let recording = unsafe { NativeAbiCodec::into_value(recording)? };
                let recorded_path = path_buf_from_value(recording.path);
                assert_eq!(recorded_path, output_path);
                assert!(recorded_path.exists());
                assert_eq!(recording.container, expected_recording_container());
                assert_eq!(recording.video_codec, Some(CameraVideoCodec::H264));
                assert!(recording.audio_codec.is_none());

                // cleanup
                let _ = std::fs::remove_file(recorded_path);

                Ok(())
            },
        )
    });
}

/// Start and stop one Linux recording through the VM path when the ffmpeg pipeline is available.
#[cfg(target_os = "linux")]
#[test]
fn test_device_camera_vm_linux_recording_start_and_stop_return_one_output_descriptor_when_present()
{
    with_camera_harness_context(|context| {
        with_first_openable_local_camera_stream_vm(
            &context,
            |vm_context, _device_handle, stream_handle, _capability| {
                // query recording capabilities first
                let capabilities = device_vm::destack_device_camera_stream_recording_capabilities(
                    context.call_context,
                    vm_context,
                    stream_handle,
                )?;
                let capabilities = VmAbiCodec::into_value(capabilities, vm_context)?;
                if capabilities.containers.is_empty() {
                    return Ok(());
                }

                // start the stream first
                device_vm::destack_device_camera_stream_start(
                    context.call_context,
                    vm_context,
                    stream_handle,
                )?;

                // start one recording
                let output_path = unique_recording_path();
                let options = recording_options_value(context.call_context, &output_path)?;
                let options = VmAbiCodec::from_value(vm_context, options)?;
                device_vm::destack_device_camera_stream_start_recording(
                    context.call_context,
                    vm_context,
                    stream_handle,
                    options,
                )?;

                // let the backend write at least one frame
                std::thread::sleep(std::time::Duration::from_millis(250));

                // stop the recording and verify the output path
                let recording = device_vm::destack_device_camera_stream_stop_recording(
                    context.call_context,
                    vm_context,
                    stream_handle,
                    5_000_000_000,
                )?;
                let recording = VmAbiCodec::into_value(recording, vm_context)?;
                let recorded_path = path_buf_from_value(recording.path);
                assert_eq!(recorded_path, output_path);
                assert!(recorded_path.exists());
                assert_eq!(recording.container, expected_recording_container());
                assert_eq!(recording.video_codec, Some(CameraVideoCodec::H264));
                assert!(recording.audio_codec.is_none());

                // cleanup
                let _ = std::fs::remove_file(recorded_path);

                Ok(())
            },
        )
    });
}

/// Build one recording options payload for one output path.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
fn recording_options_value(
    call_context: &BindingCallContext,
    path: &Path,
) -> RuntimeResult<CameraRecordingOptionsValue> {
    Ok(CameraRecordingOptionsValue {
        output_path: Some(unsafe { os_path_from_path(call_context, path).into_value()? }),
        container: None,
        video_codec: None,
        audio_enabled: Some(false),
        audio_codec: None,
        video_bit_rate: None,
        audio_bit_rate: None,
        key_frame_interval_frames: None,
        maximum_duration_ns: None,
        maximum_bytes: None,
    })
}

/// Build one recording options payload for one output path.
#[cfg(any(target_os = "macos", windows))]
fn recording_options(
    call_context: &BindingCallContext,
    path: &Path,
) -> RuntimeResult<CameraRecordingOptions> {
    let options = recording_options_value(call_context, path)?;

    Ok(CameraRecordingOptions::from_value(call_context, options))
}

/// Build one recording options payload for one output path.
#[cfg(target_os = "linux")]
fn recording_options(
    call_context: &BindingCallContext,
    path: &Path,
) -> RuntimeResult<CameraRecordingOptions> {
    let options = recording_options_value(call_context, path)?;

    Ok(CameraRecordingOptions::from_value(call_context, options))
}

/// Decode one recording-path value into one host path.
#[cfg(target_os = "macos")]
fn path_buf_from_value(path: OsPathValue) -> PathBuf {
    match path {
        OsPathValue::OsPathBytes(path) => PathBuf::from(OsString::from_vec(path.bytes.0)),
        OsPathValue::OsPathUtf16(path) => {
            PathBuf::from(String::from_utf16(&path.utf16.0).expect("utf16 path"))
        }
    }
}

/// Decode one recording-path value into one host path.
#[cfg(windows)]
fn path_buf_from_value(path: OsPathValue) -> PathBuf {
    match path {
        OsPathValue::OsPathBytes(path) => {
            PathBuf::from(String::from_utf8(path.bytes.0).expect("utf8 path"))
        }
        OsPathValue::OsPathUtf16(path) => PathBuf::from(OsString::from_wide(&path.utf16.0)),
    }
}

/// Decode one recording-path value into one host path.
#[cfg(target_os = "linux")]
fn path_buf_from_value(path: OsPathValue) -> PathBuf {
    match path {
        OsPathValue::OsPathBytes(path) => {
            PathBuf::from(String::from_utf8(path.bytes.0).expect("utf8 path"))
        }
        OsPathValue::OsPathUtf16(path) => {
            PathBuf::from(String::from_utf16(&path.utf16.0).expect("utf16 path"))
        }
    }
}
