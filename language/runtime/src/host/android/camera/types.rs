use super::callbacks::*;

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
    /// Callback for active stream config queries.
    pub stream_config: Option<AndroidHostCameraStreamConfigCallback>,
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
