use std::ffi::{c_char, c_int, c_void};

/// One opaque AAudio stream-builder payload.
#[repr(C)]
pub(super) struct AAudioStreamBuilder {
    /// Opaque bytes owned by AAudio.
    _opaque: [u8; 0],
}

/// One opaque AAudio stream payload.
#[repr(C)]
pub(super) struct AAudioStream {
    /// Opaque bytes owned by AAudio.
    _opaque: [u8; 0],
}

/// One loaded AAudio symbol table.
#[derive(Debug)]
pub(super) struct AAudioApi {
    /// `AAudio_createStreamBuilder` function pointer.
    pub(super) create_stream_builder: unsafe extern "C" fn(*mut *mut AAudioStreamBuilder) -> c_int,
    /// `AAudioStreamBuilder_delete` function pointer.
    pub(super) stream_builder_delete: unsafe extern "C" fn(*mut AAudioStreamBuilder) -> c_int,
    /// `AAudioStreamBuilder_setDirection` function pointer.
    pub(super) stream_builder_set_direction: unsafe extern "C" fn(*mut AAudioStreamBuilder, c_int),
    /// `AAudioStreamBuilder_setSampleRate` function pointer.
    pub(super) stream_builder_set_sample_rate:
        unsafe extern "C" fn(*mut AAudioStreamBuilder, c_int),
    /// `AAudioStreamBuilder_setChannelCount` function pointer.
    pub(super) stream_builder_set_channel_count:
        unsafe extern "C" fn(*mut AAudioStreamBuilder, c_int),
    /// `AAudioStreamBuilder_setFormat` function pointer.
    pub(super) stream_builder_set_format: unsafe extern "C" fn(*mut AAudioStreamBuilder, c_int),
    /// `AAudioStreamBuilder_setSharingMode` function pointer.
    pub(super) stream_builder_set_sharing_mode:
        unsafe extern "C" fn(*mut AAudioStreamBuilder, c_int),
    /// `AAudioStreamBuilder_setPerformanceMode` function pointer.
    pub(super) stream_builder_set_performance_mode:
        unsafe extern "C" fn(*mut AAudioStreamBuilder, c_int),
    /// `AAudioStreamBuilder_setBufferCapacityInFrames` function pointer.
    pub(super) stream_builder_set_buffer_capacity_frames:
        unsafe extern "C" fn(*mut AAudioStreamBuilder, c_int),
    /// `AAudioStreamBuilder_openStream` function pointer.
    pub(super) stream_builder_open_stream:
        unsafe extern "C" fn(*mut AAudioStreamBuilder, *mut *mut AAudioStream) -> c_int,
    /// `AAudioStream_close` function pointer.
    pub(super) stream_close: unsafe extern "C" fn(*mut AAudioStream) -> c_int,
    /// `AAudioStream_requestStart` function pointer.
    pub(super) stream_request_start: unsafe extern "C" fn(*mut AAudioStream) -> c_int,
    /// `AAudioStream_requestPause` function pointer.
    pub(super) stream_request_pause: unsafe extern "C" fn(*mut AAudioStream) -> c_int,
    /// `AAudioStream_requestStop` function pointer.
    pub(super) stream_request_stop: unsafe extern "C" fn(*mut AAudioStream) -> c_int,
    /// `AAudioStream_requestFlush` function pointer.
    pub(super) stream_request_flush: unsafe extern "C" fn(*mut AAudioStream) -> c_int,
    /// `AAudioStream_read` function pointer.
    pub(super) stream_read:
        unsafe extern "C" fn(*mut AAudioStream, *mut c_void, c_int, i64) -> c_int,
    /// `AAudioStream_write` function pointer.
    pub(super) stream_write:
        unsafe extern "C" fn(*mut AAudioStream, *const c_void, c_int, i64) -> c_int,
    /// `AAudioStream_getSampleRate` function pointer.
    pub(super) stream_get_sample_rate: unsafe extern "C" fn(*const AAudioStream) -> c_int,
    /// `AAudioStream_getChannelCount` function pointer.
    pub(super) stream_get_channel_count: unsafe extern "C" fn(*const AAudioStream) -> c_int,
    /// `AAudioStream_getFramesPerBurst` function pointer.
    pub(super) stream_get_frames_per_burst: unsafe extern "C" fn(*const AAudioStream) -> c_int,
    /// `AAudioStream_getFormat` function pointer.
    pub(super) stream_get_format: unsafe extern "C" fn(*const AAudioStream) -> c_int,
    /// `AAudioStream_getSharingMode` function pointer.
    pub(super) stream_get_sharing_mode: unsafe extern "C" fn(*const AAudioStream) -> c_int,
    /// `AAudio_convertResultToText` function pointer.
    pub(super) convert_result_to_text: unsafe extern "C" fn(c_int) -> *const c_char,
}
