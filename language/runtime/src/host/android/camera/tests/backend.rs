use super::tests::{
    TEST_DEVICE_ID, TEST_DEVICE_NAME, TEST_DEVICE_SESSION_ID, TEST_STATUS_NOT_FOUND,
    TEST_STATUS_NOT_SUPPORTED, TEST_STREAM_ID, lock_test_callbacks, native_string_ref,
    recorded_test_state, register_test_callbacks, test_stream_config_header,
};
use crate::host::abi::HostStatus;
use crate::host::android::camera::ffi::{
    destack_host_android_camera_device_close, destack_host_android_camera_device_list,
    destack_host_android_camera_device_open,
    destack_host_android_camera_device_stream_capability_list,
    destack_host_android_camera_device_stream_config_list,
    destack_host_android_camera_stream_close, destack_host_android_camera_stream_config,
    destack_host_android_camera_stream_open, destack_host_android_camera_stream_start,
    destack_host_android_camera_stream_stop, destack_host_android_camera_stream_try_read,
};
use crate::host::android::camera::types::{
    AndroidHostCameraCallbacks, AndroidHostCameraDeviceDescriptorHeader,
    AndroidHostCameraFrameHeader, AndroidHostCameraStreamCapabilityHeader,
    AndroidHostCameraStreamConfigHeader,
};
use crate::host::android::tests::{register_android_bindings_camera, register_android_runtime};
use crate::runtime::NativeSlice;

/// Return not-supported when Android camera callbacks are not registered.
#[test]
fn test_default_callbacks_return_not_supported() {
    let _lock = lock_test_callbacks();
    let (_queue, _registration, runtime_id) = register_android_runtime();
    let mut device_count_written = 0u32;
    let mut string_bytes_written = 0u32;

    let status = unsafe {
        destack_host_android_camera_device_list(
            runtime_id,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut device_count_written,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut string_bytes_written,
        )
    };

    assert_eq!(status, TEST_STATUS_NOT_SUPPORTED);
}

/// Reject Android camera callback registration for one unknown runtime.
#[test]
fn test_register_bindings_rejects_unknown_runtime() {
    let _lock = lock_test_callbacks();

    let status = register_android_bindings_camera(0, AndroidHostCameraCallbacks::default());

    assert_eq!(status, TEST_STATUS_NOT_FOUND);
}

/// Route camera device listing through the registered callback table.
#[test]
fn test_register_bindings_routes_device_list() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut header = AndroidHostCameraDeviceDescriptorHeader::default();
    let mut device_count_written = 0u32;
    let mut string_bytes_written = 0u32;

    // verify camera device listing honors the two-pass buffer contract
    let status = unsafe {
        destack_host_android_camera_device_list(
            callbacks.runtime_id,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut device_count_written,
            NativeSlice {
                data: std::ptr::null_mut(),
                len: 0,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::BufferTooSmall.code());
    assert_eq!(device_count_written, 1);
    assert!(string_bytes_written > 0);

    let mut string_bytes = vec![0u8; string_bytes_written as usize];
    let status = unsafe {
        destack_host_android_camera_device_list(
            callbacks.runtime_id,
            NativeSlice {
                data: &mut header,
                len: 1,
            },
            &mut device_count_written,
            NativeSlice {
                data: string_bytes.as_mut_ptr(),
                len: string_bytes.len() as u32,
            },
            &mut string_bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    // verify the returned row decodes to the deterministic device fixture
    let name_start = header.name_offset as usize;
    let name_end = name_start + header.name_len as usize;
    assert_eq!(
        std::str::from_utf8(&string_bytes[name_start..name_end]).unwrap(),
        TEST_DEVICE_NAME
    );
}

/// Route camera open and basic stream lifecycle through the registered callback table.
#[test]
fn test_register_bindings_routes_stream_lifecycle() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut device_session_id = 0u64;
    let mut config = AndroidHostCameraStreamConfigHeader::default();
    let mut capability = AndroidHostCameraStreamCapabilityHeader::default();
    let mut config_count_written = 0u32;
    let mut capability_count_written = 0u32;
    let mut stream_id = 0u64;
    let mut frame = AndroidHostCameraFrameHeader::default();
    let mut bytes_written = 0u32;
    let mut frame_bytes = vec![0u8; 8];

    // verify device open forwards the requested id
    let status = unsafe {
        destack_host_android_camera_device_open(
            callbacks.runtime_id,
            native_string_ref(TEST_DEVICE_ID),
            &mut device_session_id,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(device_session_id, TEST_DEVICE_SESSION_ID);

    let state = recorded_test_state();
    let device_open = state.device_open.expect("device open should be recorded");
    assert_eq!(device_open.id, TEST_DEVICE_ID);

    // verify stream metadata enumeration routes through the callback table
    let status = unsafe {
        destack_host_android_camera_device_stream_config_list(
            callbacks.runtime_id,
            device_session_id,
            NativeSlice {
                data: &mut config,
                len: 1,
            },
            &mut config_count_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(config_count_written, 1);
    assert_eq!(config, test_stream_config_header());

    let status = unsafe {
        destack_host_android_camera_device_stream_capability_list(
            callbacks.runtime_id,
            device_session_id,
            NativeSlice {
                data: &mut capability,
                len: 1,
            },
            &mut capability_count_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(capability_count_written, 1);
    assert_eq!(capability.config, test_stream_config_header());

    // verify stream open and basic lifecycle calls route through the callback table
    let status = unsafe {
        destack_host_android_camera_stream_open(
            callbacks.runtime_id,
            device_session_id,
            test_stream_config_header(),
            &mut stream_id,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(stream_id, TEST_STREAM_ID);

    let state = recorded_test_state();
    let stream_open = state.stream_open.expect("stream open should be recorded");
    assert_eq!(stream_open.width, 1920);
    assert_eq!(stream_open.height, 1080);
    assert_eq!(stream_open.frame_rate_milli_hz, 30_000);

    let status =
        unsafe { destack_host_android_camera_stream_start(callbacks.runtime_id, stream_id) };
    assert_eq!(status, HostStatus::Ok.code());
    let status = unsafe {
        destack_host_android_camera_stream_config(callbacks.runtime_id, stream_id, &mut config)
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(config, test_stream_config_header());

    let status = unsafe {
        destack_host_android_camera_stream_try_read(
            callbacks.runtime_id,
            stream_id,
            &mut frame,
            NativeSlice {
                data: frame_bytes.as_mut_ptr(),
                len: frame_bytes.len() as u32,
            },
            &mut bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(frame.sequence, 7);
    assert_eq!(bytes_written, 4);
    assert_eq!(&frame_bytes[..4], &[1, 2, 3, 4]);

    let status =
        unsafe { destack_host_android_camera_stream_stop(callbacks.runtime_id, stream_id) };
    assert_eq!(status, HostStatus::Ok.code());
    let status =
        unsafe { destack_host_android_camera_stream_close(callbacks.runtime_id, stream_id) };
    assert_eq!(status, HostStatus::Ok.code());
    let status = unsafe {
        destack_host_android_camera_device_close(callbacks.runtime_id, device_session_id)
    };
    assert_eq!(status, HostStatus::Ok.code());

    let state = recorded_test_state();
    assert_eq!(state.started_streams, vec![TEST_STREAM_ID]);
    assert_eq!(state.stopped_streams, vec![TEST_STREAM_ID]);
    assert_eq!(state.closed_stream_sessions, vec![TEST_STREAM_ID]);
    assert_eq!(state.closed_device_sessions, vec![TEST_DEVICE_SESSION_ID]);
}
