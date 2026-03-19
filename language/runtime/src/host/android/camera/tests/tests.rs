use std::sync::{Arc, Mutex, OnceLock};

use crate::host::abi::HostStatus;
use crate::host::android::camera::types::{
    AndroidHostCameraCallbacks, AndroidHostCameraDeviceDescriptorHeader,
    AndroidHostCameraFrameHeader, AndroidHostCameraRecordingCapabilitiesHeader,
    AndroidHostCameraRecordingOptionsHeader, AndroidHostCameraStreamCapabilityHeader,
    AndroidHostCameraStreamConfigHeader,
};
use crate::host::android::tests::{
    callback_test_lock, register_android_bindings_camera, register_android_runtime,
};
use crate::host::core::HostQueue;
use crate::host::core::registry::HostRegistrationGuard;
use crate::runtime::{NativeSlice, NativeStringRef};

/// One deterministic Android camera device id.
pub(super) const TEST_DEVICE_ID: &str = "android.camera.device";
/// One deterministic Android camera device name.
pub(super) const TEST_DEVICE_NAME: &str = "Android Camera";
/// One deterministic Android camera manufacturer.
pub(super) const TEST_MANUFACTURER: &str = "Destack";
/// One deterministic Android camera device session id.
pub(super) const TEST_DEVICE_SESSION_ID: u64 = 401;
/// One deterministic Android camera stream session id.
pub(super) const TEST_STREAM_ID: u64 = 402;
/// One deterministic Android camera recording output path.
pub(super) const TEST_RECORDING_OUTPUT_PATH: &str = "/tmp/destack-android-camera-recording.mp4";
/// One deterministic Android camera `u64` control selector.
pub(super) const TEST_CONTROL_SELECTOR_U64: u32 = 3;
/// One deterministic Android camera `u32` control selector.
pub(super) const TEST_CONTROL_SELECTOR_U32: u32 = 4;
/// One deterministic Android camera `f64` control selector.
pub(super) const TEST_CONTROL_SELECTOR_F64: u32 = 9;
/// One deterministic Android camera `u64` control value.
pub(super) const TEST_CONTROL_VALUE_U64: u64 = 90_000_000;
/// One deterministic Android camera `u32` control value.
pub(super) const TEST_CONTROL_VALUE_U32: u32 = 320;
/// One deterministic Android camera `f64` control value.
pub(super) const TEST_CONTROL_VALUE_F64: f64 = 1.5;
/// One deterministic Android camera not-supported status.
pub(super) const TEST_STATUS_NOT_SUPPORTED: u32 = HostStatus::NotSupported.code();
/// One deterministic Android camera not-found status.
pub(super) const TEST_STATUS_NOT_FOUND: u32 = HostStatus::NotFound.code();

/// One live Android camera callback registration.
pub(super) struct AndroidCameraTestCallbacks {
    /// The runtime id bound to the callback table.
    pub runtime_id: u64,
    /// The host queue kept alive for the registration.
    _queue: Arc<HostQueue>,
    /// The host registration guard kept alive for the callback table.
    _registration: HostRegistrationGuard,
}

/// One recorded Android camera device-open call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedDeviceOpenCall {
    /// The forwarded device id.
    pub id: String,
}

/// One recorded Android camera stream-open call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedStreamOpenCall {
    /// The forwarded width in pixels.
    pub width: u32,
    /// The forwarded height in pixels.
    pub height: u32,
    /// The forwarded frame rate in milli-hertz.
    pub frame_rate_milli_hz: u32,
}

/// One recorded Android camera recording-start call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedStreamStartRecordingCall {
    /// The forwarded stream id.
    pub stream_id: u64,
    /// The forwarded output path.
    pub output_path: String,
}

/// One recorded Android camera photo-capture call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedStreamTakePhotoCall {
    /// The forwarded stream id.
    pub stream_id: u64,
    /// The forwarded timeout in nanoseconds.
    pub timeout_ns: u64,
}

/// One recorded Android camera blocking frame-read call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedStreamReadCall {
    /// The forwarded stream id.
    pub stream_id: u64,
    /// The forwarded timeout in nanoseconds.
    pub timeout_ns: Option<u64>,
}

/// One recorded Android camera control get call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedControlGetCall {
    /// The forwarded stream id.
    pub stream_id: u64,
    /// The forwarded selector.
    pub selector: u32,
}

/// One recorded Android camera `u64` control set call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedControlSetU64Call {
    /// The forwarded stream id.
    pub stream_id: u64,
    /// The forwarded selector.
    pub selector: u32,
    /// The forwarded value.
    pub value: u64,
}

/// One recorded Android camera `u32` control set call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct RecordedControlSetU32Call {
    /// The forwarded stream id.
    pub stream_id: u64,
    /// The forwarded selector.
    pub selector: u32,
    /// The forwarded value.
    pub value: u32,
}

/// One recorded Android camera `f64` control set call.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct RecordedControlSetF64Call {
    /// The forwarded stream id.
    pub stream_id: u64,
    /// The forwarded selector.
    pub selector: u32,
    /// The forwarded value.
    pub value: f64,
}

/// One snapshot of recorded Android camera callback traffic.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct AndroidCameraTestState {
    /// The most recent device-open call.
    pub device_open: Option<RecordedDeviceOpenCall>,
    /// The most recent stream-open call.
    pub stream_open: Option<RecordedStreamOpenCall>,
    /// Closed device session ids.
    pub closed_device_sessions: Vec<u64>,
    /// Closed stream session ids.
    pub closed_stream_sessions: Vec<u64>,
    /// Started stream session ids.
    pub started_streams: Vec<u64>,
    /// Stopped stream session ids.
    pub stopped_streams: Vec<u64>,
    /// The most recent blocking frame-read call.
    pub stream_read: Option<RecordedStreamReadCall>,
    /// The most recent still-photo capture call.
    pub photo_capture: Option<RecordedStreamTakePhotoCall>,
    /// The most recent recording-start call.
    pub recording_start: Option<RecordedStreamStartRecordingCall>,
    /// Recording pause stream ids.
    pub paused_recordings: Vec<u64>,
    /// Recording resume stream ids.
    pub resumed_recordings: Vec<u64>,
    /// Recording stop stream ids.
    pub stopped_recordings: Vec<u64>,
    /// The most recent `u64` control get call.
    pub control_get_u64: Option<RecordedControlGetCall>,
    /// The most recent `u64` control set call.
    pub control_set_u64: Option<RecordedControlSetU64Call>,
    /// The most recent `u32` control get call.
    pub control_get_u32: Option<RecordedControlGetCall>,
    /// The most recent `u32` control set call.
    pub control_set_u32: Option<RecordedControlSetU32Call>,
    /// The most recent `f64` control get call.
    pub control_get_f64: Option<RecordedControlGetCall>,
    /// The most recent `f64` control set call.
    pub control_set_f64: Option<RecordedControlSetF64Call>,
    /// The most recent `f64` control range query.
    pub control_range_f64: Option<RecordedControlGetCall>,
    /// The most recent `u64` control range query.
    pub control_range_u64: Option<RecordedControlGetCall>,
    /// The most recent `u32` control range query.
    pub control_range_u32: Option<RecordedControlGetCall>,
}

/// Return the shared Android camera test state.
fn test_state() -> &'static Mutex<AndroidCameraTestState> {
    static STATE: OnceLock<Mutex<AndroidCameraTestState>> = OnceLock::new();

    STATE.get_or_init(|| Mutex::new(AndroidCameraTestState::default()))
}

/// Lock the shared Android camera test callback state.
pub(super) fn lock_test_callbacks() -> std::sync::MutexGuard<'static, ()> {
    callback_test_lock()
        .lock()
        .expect("android camera test lock should not be poisoned")
}

/// Return the current Android camera callback snapshot.
pub(super) fn recorded_test_state() -> AndroidCameraTestState {
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .clone()
}

/// Reset the shared Android camera callback state.
fn reset_test_state() {
    *test_state()
        .lock()
        .expect("android camera test state should not be poisoned") =
        AndroidCameraTestState::default();
}

/// Return one native string reference for the given string.
pub(super) fn native_string_ref(value: &str) -> NativeStringRef {
    NativeStringRef {
        data: value.as_ptr().cast_mut(),
        len: value.len() as u32,
    }
}

/// Decode one native string reference into one owned Rust string.
fn decode_native_string(value: NativeStringRef) -> String {
    // treat empty references as empty strings in the fixture layer
    if value.data.is_null() || value.len == 0 {
        return String::new();
    }

    // decode the borrowed bytes as one owned utf8 string
    let bytes = unsafe { std::slice::from_raw_parts(value.data, value.len as usize) };

    std::str::from_utf8(bytes)
        .expect("android camera test strings should be utf8")
        .to_string()
}

/// Append one string to the shared output buffer and return its offset and length.
fn append_string(buffer: &mut Vec<u8>, value: &str) -> (u32, u32) {
    let offset = buffer.len() as u32;
    buffer.extend_from_slice(value.as_bytes());

    (offset, value.len() as u32)
}

/// Build one deterministic camera device header and append its strings.
fn build_device_header(buffer: &mut Vec<u8>) -> AndroidHostCameraDeviceDescriptorHeader {
    let (id_offset, id_len) = append_string(buffer, TEST_DEVICE_ID);
    let (name_offset, name_len) = append_string(buffer, TEST_DEVICE_NAME);
    let (manufacturer_offset, manufacturer_len) = append_string(buffer, TEST_MANUFACTURER);

    AndroidHostCameraDeviceDescriptorHeader {
        id_offset,
        id_len,
        group_id_offset: 0,
        group_id_len: 0,
        name_offset,
        name_len,
        manufacturer_offset,
        manufacturer_len,
        facing_mode: 3,
        depth_capable: 0,
    }
}

/// Return one deterministic camera stream config header.
pub(super) fn test_stream_config_header() -> AndroidHostCameraStreamConfigHeader {
    AndroidHostCameraStreamConfigHeader {
        width: 1920,
        height: 1080,
        frame_rate_milli_hz: 30_000,
        pixel_format: 1,
        pixel_format_family: 1,
        is_compressed: 0,
    }
}

/// Return one deterministic camera stream capability header.
fn test_stream_capability_header() -> AndroidHostCameraStreamCapabilityHeader {
    AndroidHostCameraStreamCapabilityHeader {
        config: test_stream_config_header(),
        minimum_frame_rate_milli_hz: 24_000,
        maximum_frame_rate_milli_hz: 60_000,
        color_space_flags: 1 << 1,
        dynamic_range_flags: 1 << 0,
        exposure_mode_flags: (1 << 0) | (1 << 1),
        white_balance_mode_flags: 1 << 0,
        focus_mode_flags: 1 << 0,
        stabilization_mode_flags: 1 << 0,
        torch_mode_flags: 1 << 0,
    }
}

/// Return one deterministic recording capability header.
pub(super) fn test_recording_capabilities_header() -> AndroidHostCameraRecordingCapabilitiesHeader {
    AndroidHostCameraRecordingCapabilitiesHeader {
        container_flags: 1 << 0,
        video_codec_flags: 1 << 0,
        audio_supported: 0,
        audio_codec_flags: 0,
        pause_supported: 1,
        maximum_video_bit_rate: 0,
        has_maximum_video_bit_rate: 0,
        maximum_audio_bit_rate: 0,
        has_maximum_audio_bit_rate: 0,
    }
}

/// Copy one device payload into caller buffers.
unsafe fn write_device_payload(
    devices: NativeSlice<AndroidHostCameraDeviceDescriptorHeader>,
    device_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    let mut buffer = Vec::new();
    let device = build_device_header(&mut buffer);

    // report required buffer sizes first
    if devices.data.is_null()
        || devices.len == 0
        || string_bytes.data.is_null()
        || string_bytes.len < buffer.len() as u32
    {
        unsafe {
            *device_count_written = 1;
            *string_bytes_written = buffer.len() as u32;
        }
        return HostStatus::BufferTooSmall.code();
    }

    // write the device row and string bytes
    unsafe {
        *devices.data = device;
        *device_count_written = 1;
        *string_bytes_written = buffer.len() as u32;
    }

    let output =
        unsafe { std::slice::from_raw_parts_mut(string_bytes.data, string_bytes.len as usize) };
    output[..buffer.len()].copy_from_slice(&buffer);

    HostStatus::Ok.code()
}

/// Handle one device-list callback.
unsafe extern "C" fn test_device_list(
    _runtime_id: u64,
    devices: NativeSlice<AndroidHostCameraDeviceDescriptorHeader>,
    device_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    // require writable count outputs
    if device_count_written.is_null() || string_bytes_written.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    // forward to the shared device payload helper
    unsafe {
        write_device_payload(
            devices,
            device_count_written,
            string_bytes,
            string_bytes_written,
        )
    }
}

/// Handle one device-open callback.
unsafe extern "C" fn test_device_open(
    _runtime_id: u64,
    id: NativeStringRef,
    session_id: *mut u64,
) -> u32 {
    // require one writable session output
    if session_id.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    // record the forwarded device-open request
    let mut state = test_state()
        .lock()
        .expect("android camera test state should not be poisoned");
    state.device_open = Some(RecordedDeviceOpenCall {
        id: decode_native_string(id),
    });

    // return the deterministic device session id
    unsafe {
        *session_id = TEST_DEVICE_SESSION_ID;
    }

    HostStatus::Ok.code()
}

/// Handle one device-close callback.
unsafe extern "C" fn test_device_close(_runtime_id: u64, session_id: u64) -> u32 {
    // record the closed device session id
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .closed_device_sessions
        .push(session_id);

    HostStatus::Ok.code()
}

/// Handle one stream-config-list callback.
unsafe extern "C" fn test_stream_config_list(
    _runtime_id: u64,
    _session_id: u64,
    configs: NativeSlice<AndroidHostCameraStreamConfigHeader>,
    config_count_written: *mut u32,
) -> u32 {
    // require one writable count output
    if config_count_written.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    let config = test_stream_config_header();

    // report the required row count on the sizing pass
    if configs.data.is_null() || configs.len == 0 {
        unsafe {
            *config_count_written = 1;
        }
        return HostStatus::BufferTooSmall.code();
    }

    // write the deterministic config row
    unsafe {
        *configs.data = config;
        *config_count_written = 1;
    }

    HostStatus::Ok.code()
}

/// Handle one stream-capability-list callback.
unsafe extern "C" fn test_stream_capability_list(
    _runtime_id: u64,
    _session_id: u64,
    capabilities: NativeSlice<AndroidHostCameraStreamCapabilityHeader>,
    capability_count_written: *mut u32,
) -> u32 {
    // require one writable count output
    if capability_count_written.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    let capability = test_stream_capability_header();

    // report the required row count on the sizing pass
    if capabilities.data.is_null() || capabilities.len == 0 {
        unsafe {
            *capability_count_written = 1;
        }
        return HostStatus::BufferTooSmall.code();
    }

    // write the deterministic capability row
    unsafe {
        *capabilities.data = capability;
        *capability_count_written = 1;
    }

    HostStatus::Ok.code()
}

/// Handle one stream-open callback.
unsafe extern "C" fn test_stream_open(
    _runtime_id: u64,
    _session_id: u64,
    config: AndroidHostCameraStreamConfigHeader,
    stream_id: *mut u64,
) -> u32 {
    // require one writable stream output
    if stream_id.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    // record the forwarded stream-open request
    let mut state = test_state()
        .lock()
        .expect("android camera test state should not be poisoned");
    state.stream_open = Some(RecordedStreamOpenCall {
        width: config.width,
        height: config.height,
        frame_rate_milli_hz: config.frame_rate_milli_hz,
    });

    // return the deterministic stream id
    unsafe {
        *stream_id = TEST_STREAM_ID;
    }

    HostStatus::Ok.code()
}

/// Handle one stream-close callback.
unsafe extern "C" fn test_stream_close(_runtime_id: u64, stream_id: u64) -> u32 {
    // record the closed stream id
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .closed_stream_sessions
        .push(stream_id);

    HostStatus::Ok.code()
}

/// Handle one stream-start callback.
unsafe extern "C" fn test_stream_start(_runtime_id: u64, stream_id: u64) -> u32 {
    // record the started stream id
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .started_streams
        .push(stream_id);

    HostStatus::Ok.code()
}

/// Handle one stream-stop callback.
unsafe extern "C" fn test_stream_stop(_runtime_id: u64, stream_id: u64) -> u32 {
    // record the stopped stream id
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .stopped_streams
        .push(stream_id);

    HostStatus::Ok.code()
}

/// Handle one stream-config callback.
unsafe extern "C" fn test_stream_config(
    _runtime_id: u64,
    _stream_id: u64,
    config: *mut AndroidHostCameraStreamConfigHeader,
) -> u32 {
    // require one writable config output
    if config.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    // return the deterministic active config
    unsafe {
        *config = test_stream_config_header();
    }

    HostStatus::Ok.code()
}

/// Handle one recording-capability callback.
unsafe extern "C" fn test_stream_recording_capabilities(
    _runtime_id: u64,
    _stream_id: u64,
    capabilities: *mut AndroidHostCameraRecordingCapabilitiesHeader,
) -> u32 {
    // require one writable capabilities output
    if capabilities.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    // return the deterministic recording capabilities
    unsafe {
        *capabilities = test_recording_capabilities_header();
    }

    HostStatus::Ok.code()
}

/// Handle one recording-start callback.
unsafe extern "C" fn test_stream_start_recording(
    _runtime_id: u64,
    stream_id: u64,
    _options: AndroidHostCameraRecordingOptionsHeader,
    output_path: NativeStringRef,
) -> u32 {
    // record the forwarded recording request
    let mut state = test_state()
        .lock()
        .expect("android camera test state should not be poisoned");
    state.recording_start = Some(RecordedStreamStartRecordingCall {
        stream_id,
        output_path: decode_native_string(output_path),
    });

    HostStatus::Ok.code()
}

/// Handle one recording-pause callback.
unsafe extern "C" fn test_stream_pause_recording(_runtime_id: u64, stream_id: u64) -> u32 {
    // record the paused stream id
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .paused_recordings
        .push(stream_id);

    HostStatus::Ok.code()
}

/// Handle one recording-resume callback.
unsafe extern "C" fn test_stream_resume_recording(_runtime_id: u64, stream_id: u64) -> u32 {
    // record the resumed stream id
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .resumed_recordings
        .push(stream_id);

    HostStatus::Ok.code()
}

/// Handle one recording-stop callback.
unsafe extern "C" fn test_stream_stop_recording(
    _runtime_id: u64,
    stream_id: u64,
    _timeout_ns: u64,
) -> u32 {
    // record the stopped stream id
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .stopped_recordings
        .push(stream_id);

    HostStatus::Ok.code()
}

/// Handle one blocking frame read callback.
unsafe extern "C" fn test_stream_read(
    runtime_id: u64,
    stream_id: u64,
    timeout_ns: u64,
    header: *mut AndroidHostCameraFrameHeader,
    bytes: NativeSlice<u8>,
    bytes_written: *mut u32,
) -> u32 {
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .stream_read = Some(RecordedStreamReadCall {
        stream_id,
        timeout_ns: Some(timeout_ns),
    });

    unsafe { test_stream_try_read(runtime_id, stream_id, header, bytes, bytes_written) }
}

/// Handle one nonblocking frame read callback.
unsafe extern "C" fn test_stream_try_read(
    _runtime_id: u64,
    _stream_id: u64,
    header: *mut AndroidHostCameraFrameHeader,
    bytes: NativeSlice<u8>,
    bytes_written: *mut u32,
) -> u32 {
    // require writable frame outputs
    if header.is_null() || bytes_written.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    let payload = [1u8, 2, 3, 4];

    // report the required payload size on short buffers
    if bytes.data.is_null() || bytes.len < payload.len() as u32 {
        unsafe {
            *bytes_written = payload.len() as u32;
        }
        return HostStatus::BufferTooSmall.code();
    }

    // write the deterministic frame metadata
    unsafe {
        *header = AndroidHostCameraFrameHeader {
            timestamp_ns: 55,
            sequence: 7,
            width: 1920,
            height: 1080,
            pixel_format: 1,
            pixel_format_family: 1,
            is_compressed: 0,
            color_space: 2,
            plane_count: 1,
            bytes_len: payload.len() as u32,
        };
        *bytes_written = payload.len() as u32;
    }

    // copy the deterministic payload bytes
    let output = unsafe { std::slice::from_raw_parts_mut(bytes.data, bytes.len as usize) };
    output[..payload.len()].copy_from_slice(&payload);

    HostStatus::Ok.code()
}

/// Handle one `u64` camera control get callback.
unsafe extern "C" fn test_stream_get_u64(
    _runtime_id: u64,
    stream_id: u64,
    selector: u32,
    value: *mut u64,
) -> u32 {
    if value.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .control_get_u64 = Some(RecordedControlGetCall {
        stream_id,
        selector,
    });

    unsafe {
        *value = TEST_CONTROL_VALUE_U64;
    }

    HostStatus::Ok.code()
}

/// Handle one `u64` camera control set callback.
unsafe extern "C" fn test_stream_set_u64(
    _runtime_id: u64,
    stream_id: u64,
    selector: u32,
    value: u64,
) -> u32 {
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .control_set_u64 = Some(RecordedControlSetU64Call {
        stream_id,
        selector,
        value,
    });

    HostStatus::Ok.code()
}

/// Handle one `u32` camera control get callback.
unsafe extern "C" fn test_stream_get_u32(
    _runtime_id: u64,
    stream_id: u64,
    selector: u32,
    value: *mut u32,
) -> u32 {
    if value.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .control_get_u32 = Some(RecordedControlGetCall {
        stream_id,
        selector,
    });

    unsafe {
        *value = TEST_CONTROL_VALUE_U32;
    }

    HostStatus::Ok.code()
}

/// Handle one `u32` camera control set callback.
unsafe extern "C" fn test_stream_set_u32(
    _runtime_id: u64,
    stream_id: u64,
    selector: u32,
    value: u32,
) -> u32 {
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .control_set_u32 = Some(RecordedControlSetU32Call {
        stream_id,
        selector,
        value,
    });

    HostStatus::Ok.code()
}

/// Handle one `f64` camera control get callback.
unsafe extern "C" fn test_stream_get_f64(
    _runtime_id: u64,
    stream_id: u64,
    selector: u32,
    value: *mut f64,
) -> u32 {
    if value.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .control_get_f64 = Some(RecordedControlGetCall {
        stream_id,
        selector,
    });

    unsafe {
        *value = TEST_CONTROL_VALUE_F64;
    }

    HostStatus::Ok.code()
}

/// Handle one `f64` camera control set callback.
unsafe extern "C" fn test_stream_set_f64(
    _runtime_id: u64,
    stream_id: u64,
    selector: u32,
    value: f64,
) -> u32 {
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .control_set_f64 = Some(RecordedControlSetF64Call {
        stream_id,
        selector,
        value,
    });

    HostStatus::Ok.code()
}

/// Handle one `f64` camera control range callback.
unsafe extern "C" fn test_stream_get_range_f64(
    _runtime_id: u64,
    stream_id: u64,
    selector: u32,
    minimum: *mut f64,
    maximum: *mut f64,
    step: *mut f64,
) -> u32 {
    if minimum.is_null() || maximum.is_null() || step.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .control_range_f64 = Some(RecordedControlGetCall {
        stream_id,
        selector,
    });

    unsafe {
        *minimum = 0.5;
        *maximum = 2.5;
        *step = 0.25;
    }

    HostStatus::Ok.code()
}

/// Handle one `u64` camera control range callback.
unsafe extern "C" fn test_stream_get_range_u64(
    _runtime_id: u64,
    stream_id: u64,
    selector: u32,
    minimum: *mut u64,
    maximum: *mut u64,
    step: *mut u64,
) -> u32 {
    if minimum.is_null() || maximum.is_null() || step.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .control_range_u64 = Some(RecordedControlGetCall {
        stream_id,
        selector,
    });

    unsafe {
        *minimum = 1;
        *maximum = TEST_CONTROL_VALUE_U64;
        *step = 1;
    }

    HostStatus::Ok.code()
}

/// Handle one `u32` camera control range callback.
unsafe extern "C" fn test_stream_get_range_u32(
    _runtime_id: u64,
    stream_id: u64,
    selector: u32,
    minimum: *mut u32,
    maximum: *mut u32,
    step: *mut u32,
) -> u32 {
    if minimum.is_null() || maximum.is_null() || step.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .control_range_u32 = Some(RecordedControlGetCall {
        stream_id,
        selector,
    });

    unsafe {
        *minimum = 100;
        *maximum = TEST_CONTROL_VALUE_U32;
        *step = 10;
    }

    HostStatus::Ok.code()
}

/// Handle one still-photo capture callback.
unsafe extern "C" fn test_stream_take_photo(
    _runtime_id: u64,
    stream_id: u64,
    timeout_ns: u64,
    header: *mut AndroidHostCameraFrameHeader,
    bytes: NativeSlice<u8>,
    bytes_written: *mut u32,
) -> u32 {
    // record the forwarded photo-capture request
    test_state()
        .lock()
        .expect("android camera test state should not be poisoned")
        .photo_capture = Some(RecordedStreamTakePhotoCall {
        stream_id,
        timeout_ns,
    });

    // reuse the deterministic frame payload for still-photo capture
    unsafe { test_stream_try_read(0, stream_id, header, bytes, bytes_written) }
}

/// Build one complete Android camera callback table.
fn test_callbacks() -> AndroidHostCameraCallbacks {
    AndroidHostCameraCallbacks {
        device_list: Some(test_device_list),
        device_open: Some(test_device_open),
        device_close: Some(test_device_close),
        stream_config_list: Some(test_stream_config_list),
        stream_capability_list: Some(test_stream_capability_list),
        stream_open: Some(test_stream_open),
        stream_close: Some(test_stream_close),
        stream_start: Some(test_stream_start),
        stream_stop: Some(test_stream_stop),
        stream_read: Some(test_stream_read),
        stream_try_read: Some(test_stream_try_read),
        stream_take_photo: Some(test_stream_take_photo),
        stream_config: Some(test_stream_config),
        stream_recording_capabilities: Some(test_stream_recording_capabilities),
        stream_start_recording: Some(test_stream_start_recording),
        stream_pause_recording: Some(test_stream_pause_recording),
        stream_resume_recording: Some(test_stream_resume_recording),
        stream_stop_recording: Some(test_stream_stop_recording),
        stream_get_u64: Some(test_stream_get_u64),
        stream_set_u64: Some(test_stream_set_u64),
        stream_get_u32: Some(test_stream_get_u32),
        stream_set_u32: Some(test_stream_set_u32),
        stream_get_f64: Some(test_stream_get_f64),
        stream_set_f64: Some(test_stream_set_f64),
        stream_get_range_f64: Some(test_stream_get_range_f64),
        stream_get_range_u64: Some(test_stream_get_range_u64),
        stream_get_range_u32: Some(test_stream_get_range_u32),
    }
}

/// Register the deterministic Android camera callback table.
pub(super) fn register_test_callbacks() -> AndroidCameraTestCallbacks {
    // register one temporary android runtime and reset the recorded state
    let (queue, registration, runtime_id) = register_android_runtime();
    reset_test_state();

    // install the deterministic callback table
    let status = register_android_bindings_camera(runtime_id, test_callbacks());
    assert_eq!(status, HostStatus::Ok.code());

    AndroidCameraTestCallbacks {
        runtime_id,
        _queue: queue,
        _registration: registration,
    }
}
