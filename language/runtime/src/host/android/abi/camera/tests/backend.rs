use super::tests::{
    TEST_CONTROL_SELECTOR_F64, TEST_CONTROL_SELECTOR_U32, TEST_CONTROL_SELECTOR_U64,
    TEST_CONTROL_VALUE_F64, TEST_CONTROL_VALUE_U32, TEST_CONTROL_VALUE_U64, TEST_DEVICE_ID,
    TEST_DEVICE_NAME, TEST_DEVICE_SESSION_ID, TEST_RECORDING_OUTPUT_PATH, TEST_STATUS_NOT_FOUND,
    TEST_STATUS_NOT_SUPPORTED, TEST_STREAM_ID, lock_test_callbacks, native_string_ref,
    recorded_test_state, register_test_callbacks, test_recording_capabilities_header,
    test_stream_config_header,
};
use crate::host::android::abi::camera::ffi::{
    destack_host_android_camera_device_close, destack_host_android_camera_device_list,
    destack_host_android_camera_device_open,
    destack_host_android_camera_device_stream_capability_list,
    destack_host_android_camera_device_stream_config_list,
    destack_host_android_camera_stream_brightness,
    destack_host_android_camera_stream_brightness_range, destack_host_android_camera_stream_close,
    destack_host_android_camera_stream_config, destack_host_android_camera_stream_exposure_time_ns,
    destack_host_android_camera_stream_exposure_time_range,
    destack_host_android_camera_stream_get_f64, destack_host_android_camera_stream_get_range_f64,
    destack_host_android_camera_stream_get_range_u32,
    destack_host_android_camera_stream_get_range_u64, destack_host_android_camera_stream_get_u32,
    destack_host_android_camera_stream_get_u64, destack_host_android_camera_stream_open,
    destack_host_android_camera_stream_pause_recording, destack_host_android_camera_stream_read,
    destack_host_android_camera_stream_recording_capabilities,
    destack_host_android_camera_stream_resume_recording,
    destack_host_android_camera_stream_sensor_iso,
    destack_host_android_camera_stream_sensor_iso_range,
    destack_host_android_camera_stream_set_brightness,
    destack_host_android_camera_stream_set_exposure_time_ns,
    destack_host_android_camera_stream_set_f64, destack_host_android_camera_stream_set_sensor_iso,
    destack_host_android_camera_stream_set_u32, destack_host_android_camera_stream_set_u64,
    destack_host_android_camera_stream_start, destack_host_android_camera_stream_start_recording,
    destack_host_android_camera_stream_stop, destack_host_android_camera_stream_stop_recording,
    destack_host_android_camera_stream_take_photo, destack_host_android_camera_stream_try_read,
};
use crate::host::android::abi::camera::types::{
    AndroidHostCameraCallbacks, AndroidHostCameraDeviceDescriptorHeader,
    AndroidHostCameraFrameHeader, AndroidHostCameraRecordingCapabilitiesHeader,
    AndroidHostCameraRecordingOptionsHeader, AndroidHostCameraStreamCapabilityHeader,
    AndroidHostCameraStreamConfigHeader,
};
use crate::host::android::tests::{register_android_bindings_camera, register_android_runtime};
use crate::host::core::HostStatus;
use crate::runtime::NativeSlice;

/// Decode one utf8 string from one caller-owned byte buffer.
fn decode_string(bytes: &[u8], offset: u32, len: u32) -> &str {
    let start = offset as usize;
    let end = start + len as usize;

    std::str::from_utf8(&bytes[start..end]).expect("android camera fixture strings should decode")
}

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
    assert_eq!(
        decode_string(&string_bytes, header.name_offset, header.name_len),
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

    let status = unsafe {
        destack_host_android_camera_stream_take_photo(
            callbacks.runtime_id,
            stream_id,
            123,
            &mut frame,
            NativeSlice {
                data: frame_bytes.as_mut_ptr(),
                len: frame_bytes.len() as u32,
            },
            &mut bytes_written,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

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
    assert_eq!(
        state.photo_capture,
        Some(super::tests::RecordedStreamTakePhotoCall {
            stream_id: TEST_STREAM_ID,
            timeout_ns: 123,
        })
    );
    assert_eq!(state.closed_stream_sessions, vec![TEST_STREAM_ID]);
    assert_eq!(state.closed_device_sessions, vec![TEST_DEVICE_SESSION_ID]);
}

/// Route blocking frame reads through the registered callback table.
#[test]
fn test_register_bindings_routes_blocking_stream_read() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut frame = AndroidHostCameraFrameHeader::default();
    let mut bytes_written = 0u32;
    let mut frame_bytes = vec![0u8; 8];

    let status = unsafe {
        destack_host_android_camera_stream_read(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            456,
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

    let state = recorded_test_state();
    let stream_read = state.stream_read.expect("stream read should be recorded");
    assert_eq!(stream_read.stream_id, TEST_STREAM_ID);
    assert_eq!(stream_read.timeout_ns, Some(456));
}

/// Route camera recording callbacks through the registered callback table.
#[test]
fn test_register_bindings_routes_recording_lifecycle() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut capabilities = AndroidHostCameraRecordingCapabilitiesHeader::default();

    // verify recording capabilities route through the callback table
    let status = unsafe {
        destack_host_android_camera_stream_recording_capabilities(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            &mut capabilities,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(capabilities, test_recording_capabilities_header());

    // verify recording lifecycle routes through the callback table
    let status = unsafe {
        destack_host_android_camera_stream_start_recording(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            AndroidHostCameraRecordingOptionsHeader::default(),
            native_string_ref(TEST_RECORDING_OUTPUT_PATH),
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_camera_stream_pause_recording(callbacks.runtime_id, TEST_STREAM_ID)
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_camera_stream_resume_recording(callbacks.runtime_id, TEST_STREAM_ID)
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_camera_stream_stop_recording(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            2_000_000_000,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    // verify the recorded callback traffic
    let state = recorded_test_state();
    let recording_start = state
        .recording_start
        .expect("recording start should be recorded");
    assert_eq!(recording_start.stream_id, TEST_STREAM_ID);
    assert_eq!(recording_start.output_path, TEST_RECORDING_OUTPUT_PATH);
    assert_eq!(state.paused_recordings, vec![TEST_STREAM_ID]);
    assert_eq!(state.resumed_recordings, vec![TEST_STREAM_ID]);
    assert_eq!(state.stopped_recordings, vec![TEST_STREAM_ID]);
}

/// Route generic and aliased camera controls through the registered callback table.
#[test]
fn test_register_bindings_routes_control_callbacks_and_aliases() {
    let _lock = lock_test_callbacks();
    let callbacks = register_test_callbacks();
    let mut value_u64 = 0u64;
    let mut value_u32 = 0u32;
    let mut value_f64 = 0.0f64;
    let mut min_u64 = 0u64;
    let mut max_u64 = 0u64;
    let mut step_u64 = 0u64;
    let mut min_u32 = 0u32;
    let mut max_u32 = 0u32;
    let mut step_u32 = 0u32;
    let mut min_f64 = 0.0f64;
    let mut max_f64 = 0.0f64;
    let mut step_f64 = 0.0f64;

    // verify the generic selector lanes route through the registered callbacks
    let status = unsafe {
        destack_host_android_camera_stream_get_u64(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            TEST_CONTROL_SELECTOR_U64,
            &mut value_u64,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(value_u64, TEST_CONTROL_VALUE_U64);

    let status = unsafe {
        destack_host_android_camera_stream_set_u64(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            TEST_CONTROL_SELECTOR_U64,
            TEST_CONTROL_VALUE_U64,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_camera_stream_get_range_u64(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            TEST_CONTROL_SELECTOR_U64,
            &mut min_u64,
            &mut max_u64,
            &mut step_u64,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!((min_u64, max_u64, step_u64), (1, TEST_CONTROL_VALUE_U64, 1));

    let status = unsafe {
        destack_host_android_camera_stream_get_u32(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            TEST_CONTROL_SELECTOR_U32,
            &mut value_u32,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(value_u32, TEST_CONTROL_VALUE_U32);

    let status = unsafe {
        destack_host_android_camera_stream_set_u32(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            TEST_CONTROL_SELECTOR_U32,
            TEST_CONTROL_VALUE_U32,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_camera_stream_get_range_u32(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            TEST_CONTROL_SELECTOR_U32,
            &mut min_u32,
            &mut max_u32,
            &mut step_u32,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(
        (min_u32, max_u32, step_u32),
        (100, TEST_CONTROL_VALUE_U32, 10)
    );

    let status = unsafe {
        destack_host_android_camera_stream_get_f64(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            TEST_CONTROL_SELECTOR_F64,
            &mut value_f64,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(value_f64, TEST_CONTROL_VALUE_F64);

    let status = unsafe {
        destack_host_android_camera_stream_set_f64(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            TEST_CONTROL_SELECTOR_F64,
            TEST_CONTROL_VALUE_F64,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_camera_stream_get_range_f64(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            TEST_CONTROL_SELECTOR_F64,
            &mut min_f64,
            &mut max_f64,
            &mut step_f64,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!((min_f64, max_f64, step_f64), (0.5, 2.5, 0.25));

    // verify representative alias shims forward the expected selector lanes
    let status = unsafe {
        destack_host_android_camera_stream_exposure_time_ns(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            &mut value_u64,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(value_u64, TEST_CONTROL_VALUE_U64);

    let status = unsafe {
        destack_host_android_camera_stream_set_exposure_time_ns(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            TEST_CONTROL_VALUE_U64,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_camera_stream_exposure_time_range(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            &mut min_u64,
            &mut max_u64,
            &mut step_u64,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_camera_stream_sensor_iso(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            &mut value_u32,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(value_u32, TEST_CONTROL_VALUE_U32);

    let status = unsafe {
        destack_host_android_camera_stream_set_sensor_iso(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            TEST_CONTROL_VALUE_U32,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_camera_stream_sensor_iso_range(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            &mut min_u32,
            &mut max_u32,
            &mut step_u32,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_camera_stream_brightness(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            &mut value_f64,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());
    assert_eq!(value_f64, TEST_CONTROL_VALUE_F64);

    let status = unsafe {
        destack_host_android_camera_stream_set_brightness(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            TEST_CONTROL_VALUE_F64,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let status = unsafe {
        destack_host_android_camera_stream_brightness_range(
            callbacks.runtime_id,
            TEST_STREAM_ID,
            &mut min_f64,
            &mut max_f64,
            &mut step_f64,
        )
    };
    assert_eq!(status, HostStatus::Ok.code());

    let state = recorded_test_state();

    let control_get_u64 = state
        .control_get_u64
        .expect("u64 control get should be recorded");
    assert_eq!(control_get_u64.stream_id, TEST_STREAM_ID);
    assert_eq!(control_get_u64.selector, TEST_CONTROL_SELECTOR_U64);

    let control_set_u64 = state
        .control_set_u64
        .expect("u64 control set should be recorded");
    assert_eq!(control_set_u64.stream_id, TEST_STREAM_ID);
    assert_eq!(control_set_u64.selector, TEST_CONTROL_SELECTOR_U64);
    assert_eq!(control_set_u64.value, TEST_CONTROL_VALUE_U64);

    let control_get_u32 = state
        .control_get_u32
        .expect("u32 control get should be recorded");
    assert_eq!(control_get_u32.stream_id, TEST_STREAM_ID);
    assert_eq!(control_get_u32.selector, TEST_CONTROL_SELECTOR_U32);

    let control_set_u32 = state
        .control_set_u32
        .expect("u32 control set should be recorded");
    assert_eq!(control_set_u32.stream_id, TEST_STREAM_ID);
    assert_eq!(control_set_u32.selector, TEST_CONTROL_SELECTOR_U32);
    assert_eq!(control_set_u32.value, TEST_CONTROL_VALUE_U32);

    let control_get_f64 = state
        .control_get_f64
        .expect("f64 control get should be recorded");
    assert_eq!(control_get_f64.stream_id, TEST_STREAM_ID);
    assert_eq!(control_get_f64.selector, TEST_CONTROL_SELECTOR_F64);

    let control_set_f64 = state
        .control_set_f64
        .expect("f64 control set should be recorded");
    assert_eq!(control_set_f64.stream_id, TEST_STREAM_ID);
    assert_eq!(control_set_f64.selector, TEST_CONTROL_SELECTOR_F64);
    assert_eq!(control_set_f64.value, TEST_CONTROL_VALUE_F64);

    let control_range_u64 = state
        .control_range_u64
        .expect("u64 control range should be recorded");
    assert_eq!(control_range_u64.stream_id, TEST_STREAM_ID);
    assert_eq!(control_range_u64.selector, TEST_CONTROL_SELECTOR_U64);

    let control_range_u32 = state
        .control_range_u32
        .expect("u32 control range should be recorded");
    assert_eq!(control_range_u32.stream_id, TEST_STREAM_ID);
    assert_eq!(control_range_u32.selector, TEST_CONTROL_SELECTOR_U32);

    let control_range_f64 = state
        .control_range_f64
        .expect("f64 control range should be recorded");
    assert_eq!(control_range_f64.stream_id, TEST_STREAM_ID);
    assert_eq!(control_range_f64.selector, TEST_CONTROL_SELECTOR_F64);
}
