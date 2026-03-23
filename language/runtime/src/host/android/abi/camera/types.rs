use super::callbacks::*;

/// Fixed-size Android camera recording capability header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct AndroidHostCameraRecordingCapabilitiesHeader {
    /// Supported container flags.
    pub container_flags: u32,
    /// Supported video codec flags.
    pub video_codec_flags: u32,
    /// Whether audio recording is supported.
    pub audio_supported: u32,
    /// Supported audio codec flags.
    pub audio_codec_flags: u32,
    /// Whether pause and resume are supported.
    pub pause_supported: u32,
    /// Maximum supported video bit rate in bits per second.
    pub maximum_video_bit_rate: u64,
    /// Whether the maximum video bit rate is present.
    pub has_maximum_video_bit_rate: u32,
    /// Maximum supported audio bit rate in bits per second.
    pub maximum_audio_bit_rate: u64,
    /// Whether the maximum audio bit rate is present.
    pub has_maximum_audio_bit_rate: u32,
}

/// Fixed-size Android camera recording options header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct AndroidHostCameraRecordingOptionsHeader {
    /// Preferred recording container code.
    pub container: u32,
    /// Preferred video codec code.
    pub video_codec: u32,
    /// Whether the audio-enabled flag is present.
    pub has_audio_enabled: u32,
    /// Requested audio-enabled value when present.
    pub audio_enabled: u32,
    /// Preferred audio codec code.
    pub audio_codec: u32,
    /// Requested video bit rate in bits per second.
    pub video_bit_rate: u64,
    /// Whether the video bit rate is present.
    pub has_video_bit_rate: u32,
    /// Requested audio bit rate in bits per second.
    pub audio_bit_rate: u64,
    /// Whether the audio bit rate is present.
    pub has_audio_bit_rate: u32,
    /// Requested key-frame interval in frames.
    pub key_frame_interval_frames: u32,
    /// Whether the key-frame interval is present.
    pub has_key_frame_interval_frames: u32,
    /// Requested maximum duration in nanoseconds.
    pub maximum_duration_ns: u64,
    /// Whether the maximum duration is present.
    pub has_maximum_duration_ns: u32,
    /// Requested maximum output size in bytes.
    pub maximum_bytes: u64,
    /// Whether the maximum output size is present.
    pub has_maximum_bytes: u32,
}

/// Fixed-size Android camera device descriptor header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct AndroidHostCameraDeviceDescriptorHeader {
    /// Offset of the stable id string.
    pub id_offset: u32,
    /// Length of the stable id string.
    pub id_len: u32,
    /// Offset of the optional group id string.
    pub group_id_offset: u32,
    /// Length of the optional group id string.
    pub group_id_len: u32,
    /// Offset of the display name string.
    pub name_offset: u32,
    /// Length of the display name string.
    pub name_len: u32,
    /// Offset of the manufacturer string.
    pub manufacturer_offset: u32,
    /// Length of the manufacturer string.
    pub manufacturer_len: u32,
    /// Facing-mode code.
    pub facing_mode: u32,
    /// Whether depth is supported.
    pub depth_capable: u32,
}

/// Fixed-size Android camera stream config header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct AndroidHostCameraStreamConfigHeader {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Frame rate in milli-hertz.
    pub frame_rate_milli_hz: u32,
    /// Pixel-format code.
    pub pixel_format: u32,
    /// Pixel-format family code.
    pub pixel_format_family: u32,
    /// Whether the format is compressed.
    pub is_compressed: u32,
}

/// Fixed-size Android camera stream capability header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct AndroidHostCameraStreamCapabilityHeader {
    /// Base stream configuration.
    pub config: AndroidHostCameraStreamConfigHeader,
    /// Minimum frame rate in milli-hertz.
    pub minimum_frame_rate_milli_hz: u32,
    /// Maximum frame rate in milli-hertz.
    pub maximum_frame_rate_milli_hz: u32,
    /// Supported color-space flags.
    pub color_space_flags: u32,
    /// Supported dynamic-range flags.
    pub dynamic_range_flags: u32,
    /// Supported exposure-mode flags.
    pub exposure_mode_flags: u32,
    /// Supported white-balance-mode flags.
    pub white_balance_mode_flags: u32,
    /// Supported focus-mode flags.
    pub focus_mode_flags: u32,
    /// Supported stabilization-mode flags.
    pub stabilization_mode_flags: u32,
    /// Supported torch-mode flags.
    pub torch_mode_flags: u32,
}

/// Fixed-size Android camera frame header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct AndroidHostCameraFrameHeader {
    /// Frame timestamp in monotonic nanoseconds.
    pub timestamp_ns: u64,
    /// Frame sequence number.
    pub sequence: u64,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Pixel-format code.
    pub pixel_format: u32,
    /// Pixel-format family code.
    pub pixel_format_family: u32,
    /// Whether the format is compressed.
    pub is_compressed: u32,
    /// Color-space code.
    pub color_space: u32,
    /// Plane count.
    pub plane_count: u32,
    /// Frame byte length.
    pub bytes_len: u32,
}

/// Callback table for Android host camera interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostCameraCallbacks {
    /// Callback for device listing.
    pub device_list: Option<AndroidHostCameraDeviceListCallback>,
    /// Callback for device open.
    pub device_open: Option<AndroidHostCameraDeviceOpenCallback>,
    /// Callback for device close.
    pub device_close: Option<AndroidHostCameraDeviceCloseCallback>,
    /// Callback for stream-config listing.
    pub stream_config_list: Option<AndroidHostCameraDeviceStreamConfigListCallback>,
    /// Callback for stream-capability listing.
    pub stream_capability_list: Option<AndroidHostCameraDeviceStreamCapabilityListCallback>,
    /// Callback for stream open.
    pub stream_open: Option<AndroidHostCameraStreamOpenCallback>,
    /// Callback for stream close.
    pub stream_close: Option<AndroidHostCameraStreamCloseCallback>,
    /// Callback for stream start.
    pub stream_start: Option<AndroidHostCameraStreamStartCallback>,
    /// Callback for stream stop.
    pub stream_stop: Option<AndroidHostCameraStreamStopCallback>,
    /// Callback for blocking frame reads.
    pub stream_read: Option<AndroidHostCameraStreamReadCallback>,
    /// Callback for nonblocking frame reads.
    pub stream_try_read: Option<AndroidHostCameraStreamTryReadCallback>,
    /// Callback for still-photo capture.
    pub stream_take_photo: Option<AndroidHostCameraStreamTakePhotoCallback>,
    /// Callback for active stream config queries.
    pub stream_config: Option<AndroidHostCameraStreamConfigCallback>,
    /// Callback for recording capability queries.
    pub stream_recording_capabilities: Option<AndroidHostCameraStreamRecordingCapabilitiesCallback>,
    /// Callback for recording start.
    pub stream_start_recording: Option<AndroidHostCameraStreamStartRecordingCallback>,
    /// Callback for recording pause.
    pub stream_pause_recording: Option<AndroidHostCameraStreamPauseRecordingCallback>,
    /// Callback for recording resume.
    pub stream_resume_recording: Option<AndroidHostCameraStreamResumeRecordingCallback>,
    /// Callback for recording stop.
    pub stream_stop_recording: Option<AndroidHostCameraStreamStopRecordingCallback>,
    /// Callback for `u64` control reads.
    pub stream_get_u64: Option<AndroidHostCameraStreamGetU64Callback>,
    /// Callback for `u64` control writes.
    pub stream_set_u64: Option<AndroidHostCameraStreamSetU64Callback>,
    /// Callback for `u32` control reads.
    pub stream_get_u32: Option<AndroidHostCameraStreamGetU32Callback>,
    /// Callback for `u32` control writes.
    pub stream_set_u32: Option<AndroidHostCameraStreamSetU32Callback>,
    /// Callback for `f64` control reads.
    pub stream_get_f64: Option<AndroidHostCameraStreamGetF64Callback>,
    /// Callback for `f64` control writes.
    pub stream_set_f64: Option<AndroidHostCameraStreamSetF64Callback>,
    /// Callback for `f64` control range queries.
    pub stream_get_range_f64: Option<AndroidHostCameraStreamGetRangeF64Callback>,
    /// Callback for `u64` control range queries.
    pub stream_get_range_u64: Option<AndroidHostCameraStreamGetRangeU64Callback>,
    /// Callback for `u32` control range queries.
    pub stream_get_range_u32: Option<AndroidHostCameraStreamGetRangeU32Callback>,
}
