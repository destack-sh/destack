use std::ffi::c_int;

/// Prefix for one AAudio playback stable id.
pub(super) const AAUDIO_PLAYBACK_STABLE_ID_PREFIX: &str = "aaudio:playback:";
/// Prefix for one AAudio capture stable id.
pub(super) const AAUDIO_CAPTURE_STABLE_ID_PREFIX: &str = "aaudio:capture:";
/// Prefix for one AAudio duplex stable id.
pub(super) const AAUDIO_DUPLEX_STABLE_ID_PREFIX: &str = "aaudio:duplex:";

/// One default AAudio endpoint name.
pub(super) const AAUDIO_DEFAULT_ENDPOINT_NAME: &str = "default";
/// One default AAudio stream name used in diagnostics.
pub(super) const AAUDIO_STREAM_NAME: &str = "destack-audio";

/// One AAudio output direction selector.
pub(super) const AAUDIO_DIRECTION_OUTPUT: c_int = 0;
/// One AAudio input direction selector.
pub(super) const AAUDIO_DIRECTION_INPUT: c_int = 1;

/// One AAudio exclusive sharing-mode selector.
pub(super) const AAUDIO_SHARING_MODE_EXCLUSIVE: c_int = 0;
/// One AAudio shared sharing-mode selector.
pub(super) const AAUDIO_SHARING_MODE_SHARED: c_int = 1;

/// One AAudio unspecified sample-format selector.
pub(super) const AAUDIO_FORMAT_UNSPECIFIED: c_int = 0;
/// One AAudio 16-bit signed PCM sample-format selector.
pub(super) const AAUDIO_FORMAT_PCM_I16: c_int = 1;
/// One AAudio 32-bit float PCM sample-format selector.
pub(super) const AAUDIO_FORMAT_PCM_FLOAT: c_int = 2;
/// One AAudio packed 24-bit PCM sample-format selector.
pub(super) const AAUDIO_FORMAT_PCM_I24_PACKED: c_int = 3;
/// One AAudio 32-bit signed PCM sample-format selector.
pub(super) const AAUDIO_FORMAT_PCM_I32: c_int = 4;

/// One AAudio low-latency performance-mode selector.
pub(super) const AAUDIO_PERFORMANCE_MODE_LOW_LATENCY: c_int = 12;

/// One AAudio success status code.
pub(super) const AAUDIO_RESULT_OK: c_int = 0;

/// One fallback preferred sample rate in hertz.
pub(super) const AAUDIO_PREFERRED_SAMPLE_RATE: u32 = 48_000;
/// One fallback minimum sample rate in hertz.
pub(super) const AAUDIO_MIN_SAMPLE_RATE: u32 = 8_000;
/// One fallback maximum sample rate in hertz.
pub(super) const AAUDIO_MAX_SAMPLE_RATE: u32 = 192_000;
/// One fallback preferred period in frames.
pub(super) const AAUDIO_PREFERRED_PERIOD_FRAMES: u32 = 256;
/// One fallback minimum period in frames.
pub(super) const AAUDIO_MIN_PERIOD_FRAMES: u32 = 64;
/// One fallback maximum period in frames.
pub(super) const AAUDIO_MAX_PERIOD_FRAMES: u32 = 8_192;
/// One fallback maximum exposed channel count.
pub(super) const AAUDIO_MAX_CHANNELS: u16 = 32;
