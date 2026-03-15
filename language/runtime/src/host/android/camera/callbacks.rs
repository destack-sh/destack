use crate::runtime::{NativeSlice, NativeStringRef};

use super::types::{
    AndroidHostCameraDeviceDescriptorHeader, AndroidHostCameraFrameHeader,
    AndroidHostCameraStreamCapabilityHeader, AndroidHostCameraStreamConfigHeader,
};

/// Host callback for listing one slice of Android camera devices.
pub(crate) type AndroidHostCameraDeviceListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    devices: NativeSlice<AndroidHostCameraDeviceDescriptorHeader>,
    device_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32;

/// Host callback for opening one Android camera device session.
pub(crate) type AndroidHostCameraDeviceOpenCallback =
    unsafe extern "C" fn(runtime_id: u64, id: NativeStringRef, session_id: *mut u64) -> u32;

/// Host callback for closing one Android camera device session.
pub(crate) type AndroidHostCameraDeviceCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, session_id: u64) -> u32;

/// Host callback for listing one Android camera stream-config slice.
pub(crate) type AndroidHostCameraDeviceStreamConfigListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    configs: NativeSlice<AndroidHostCameraStreamConfigHeader>,
    config_count_written: *mut u32,
) -> u32;

/// Host callback for listing one Android camera stream-capability slice.
pub(crate) type AndroidHostCameraDeviceStreamCapabilityListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    capabilities: NativeSlice<AndroidHostCameraStreamCapabilityHeader>,
    capability_count_written: *mut u32,
) -> u32;

/// Host callback for opening one Android camera stream session.
pub(crate) type AndroidHostCameraStreamOpenCallback = unsafe extern "C" fn(
    runtime_id: u64,
    session_id: u64,
    config: AndroidHostCameraStreamConfigHeader,
    stream_id: *mut u64,
) -> u32;

/// Host callback for closing one Android camera stream session.
pub(crate) type AndroidHostCameraStreamCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, stream_id: u64) -> u32;

/// Host callback for starting one Android camera stream session.
pub(crate) type AndroidHostCameraStreamStartCallback =
    unsafe extern "C" fn(runtime_id: u64, stream_id: u64) -> u32;

/// Host callback for stopping one Android camera stream session.
pub(crate) type AndroidHostCameraStreamStopCallback =
    unsafe extern "C" fn(runtime_id: u64, stream_id: u64) -> u32;

/// Host callback for reading one Android camera frame.
pub(crate) type AndroidHostCameraStreamReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    stream_id: u64,
    timeout_ns: u64,
    header: *mut AndroidHostCameraFrameHeader,
    bytes: NativeSlice<u8>,
    bytes_written: *mut u32,
) -> u32;

/// Host callback for trying one non-blocking Android camera frame read.
pub(crate) type AndroidHostCameraStreamTryReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    stream_id: u64,
    header: *mut AndroidHostCameraFrameHeader,
    bytes: NativeSlice<u8>,
    bytes_written: *mut u32,
) -> u32;

/// Host callback for querying one active Android camera stream config.
pub(crate) type AndroidHostCameraStreamConfigCallback = unsafe extern "C" fn(
    runtime_id: u64,
    stream_id: u64,
    config: *mut AndroidHostCameraStreamConfigHeader,
) -> u32;

/// Host callback for reading one `u64` Android camera control value.
pub(crate) type AndroidHostCameraStreamGetU64Callback =
    unsafe extern "C" fn(runtime_id: u64, stream_id: u64, selector: u32, value: *mut u64) -> u32;

/// Host callback for writing one `u64` Android camera control value.
pub(crate) type AndroidHostCameraStreamSetU64Callback =
    unsafe extern "C" fn(runtime_id: u64, stream_id: u64, selector: u32, value: u64) -> u32;

/// Host callback for reading one `u32` Android camera control value.
pub(crate) type AndroidHostCameraStreamGetU32Callback =
    unsafe extern "C" fn(runtime_id: u64, stream_id: u64, selector: u32, value: *mut u32) -> u32;

/// Host callback for writing one `u32` Android camera control value.
pub(crate) type AndroidHostCameraStreamSetU32Callback =
    unsafe extern "C" fn(runtime_id: u64, stream_id: u64, selector: u32, value: u32) -> u32;

/// Host callback for reading one `f64` Android camera control value.
pub(crate) type AndroidHostCameraStreamGetF64Callback =
    unsafe extern "C" fn(runtime_id: u64, stream_id: u64, selector: u32, value: *mut f64) -> u32;

/// Host callback for writing one `f64` Android camera control value.
pub(crate) type AndroidHostCameraStreamSetF64Callback =
    unsafe extern "C" fn(runtime_id: u64, stream_id: u64, selector: u32, value: f64) -> u32;

/// Host callback for reading one `f64` Android camera control range.
pub(crate) type AndroidHostCameraStreamGetRangeF64Callback = unsafe extern "C" fn(
    runtime_id: u64,
    stream_id: u64,
    selector: u32,
    minimum: *mut f64,
    maximum: *mut f64,
    step: *mut f64,
) -> u32;

/// Host callback for reading one `u64` Android camera control range.
pub(crate) type AndroidHostCameraStreamGetRangeU64Callback = unsafe extern "C" fn(
    runtime_id: u64,
    stream_id: u64,
    selector: u32,
    minimum: *mut u64,
    maximum: *mut u64,
    step: *mut u64,
) -> u32;

/// Host callback for reading one `u32` Android camera control range.
pub(crate) type AndroidHostCameraStreamGetRangeU32Callback = unsafe extern "C" fn(
    runtime_id: u64,
    stream_id: u64,
    selector: u32,
    minimum: *mut u32,
    maximum: *mut u32,
    step: *mut u32,
) -> u32;
