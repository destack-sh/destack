#[cfg(feature = "generator")]
use crate::host::abi::describe::host_abi_types;

#[cfg(not(feature = "generator"))]
pub(crate) use crate::host::android::abi::camera::types::{
    AndroidHostCameraDeviceDescriptorHeader, AndroidHostCameraFrameHeader,
    AndroidHostCameraRecordingCapabilitiesHeader, AndroidHostCameraRecordingOptionsHeader,
    AndroidHostCameraStreamCapabilityHeader, AndroidHostCameraStreamConfigHeader,
};

#[cfg(feature = "generator")]
host_abi_types! {
    fn host_abi_types() {
        /// Fixed-size host camera recording capability header.
        struct AndroidHostCameraRecordingCapabilitiesHeader {
            /// Supported container flags.
            container_flags: u32,
            /// Supported video codec flags.
            video_codec_flags: u32,
            /// Whether audio recording is supported.
            audio_supported: u32,
            /// Supported audio codec flags.
            audio_codec_flags: u32,
            /// Whether pause and resume are supported.
            pause_supported: u32,
            /// Maximum supported video bit rate in bits per second.
            maximum_video_bit_rate: option(u64),
            /// Maximum supported audio bit rate in bits per second.
            maximum_audio_bit_rate: option(u64),
        }

        /// Fixed-size host camera recording options header.
        struct AndroidHostCameraRecordingOptionsHeader {
            /// Preferred recording container code.
            container: u32,
            /// Preferred video codec code.
            video_codec: u32,
            /// Requested audio-enabled value when present.
            audio_enabled: option(u32),
            /// Preferred audio codec code.
            audio_codec: u32,
            /// Requested video bit rate in bits per second.
            video_bit_rate: option(u64),
            /// Requested audio bit rate in bits per second.
            audio_bit_rate: option(u64),
            /// Requested key-frame interval in frames.
            key_frame_interval_frames: option(u32),
            /// Requested maximum duration in nanoseconds.
            maximum_duration_ns: option(u64),
            /// Requested maximum output size in bytes.
            maximum_bytes: option(u64),
        }

        /// Fixed-size host camera device descriptor header.
        struct AndroidHostCameraDeviceDescriptorHeader {
            /// Offset of the stable id string.
            id_offset: u32,
            /// Length of the stable id string.
            id_len: u32,
            /// Offset of the optional group id string.
            group_id_offset: u32,
            /// Length of the optional group id string.
            group_id_len: u32,
            /// Offset of the display name string.
            name_offset: u32,
            /// Length of the display name string.
            name_len: u32,
            /// Offset of the manufacturer string.
            manufacturer_offset: u32,
            /// Length of the manufacturer string.
            manufacturer_len: u32,
            /// Facing-mode code.
            facing_mode: u32,
            /// Whether depth is supported.
            depth_capable: u32,
        }

        /// Fixed-size host camera stream config header.
        struct AndroidHostCameraStreamConfigHeader {
            /// Frame width in pixels.
            width: u32,
            /// Frame height in pixels.
            height: u32,
            /// Frame rate in milli-hertz.
            frame_rate_milli_hz: u32,
            /// Pixel-format code.
            pixel_format: u32,
            /// Pixel-format family code.
            pixel_format_family: u32,
            /// Whether the format is compressed.
            is_compressed: u32,
        }

        /// Fixed-size host camera stream capability header.
        struct AndroidHostCameraStreamCapabilityHeader {
            /// Base stream configuration.
            config: AndroidHostCameraStreamConfigHeader,
            /// Minimum frame rate in milli-hertz.
            minimum_frame_rate_milli_hz: u32,
            /// Maximum frame rate in milli-hertz.
            maximum_frame_rate_milli_hz: u32,
            /// Supported color-space flags.
            color_space_flags: u32,
            /// Supported dynamic-range flags.
            dynamic_range_flags: u32,
            /// Supported exposure-mode flags.
            exposure_mode_flags: u32,
            /// Supported white-balance-mode flags.
            white_balance_mode_flags: u32,
            /// Supported focus-mode flags.
            focus_mode_flags: u32,
            /// Supported stabilization-mode flags.
            stabilization_mode_flags: u32,
            /// Supported torch-mode flags.
            torch_mode_flags: u32,
        }

        /// Fixed-size host camera frame header.
        struct AndroidHostCameraFrameHeader {
            /// Frame timestamp in monotonic nanoseconds.
            timestamp_ns: u64,
            /// Frame sequence number.
            sequence: u64,
            /// Width in pixels.
            width: u32,
            /// Height in pixels.
            height: u32,
            /// Pixel-format code.
            pixel_format: u32,
            /// Pixel-format family code.
            pixel_format_family: u32,
            /// Whether the format is compressed.
            is_compressed: u32,
            /// Color-space code.
            color_space: u32,
            /// Plane count.
            plane_count: u32,
            /// Frame byte length.
            bytes_len: u32,
        }
    }
}
