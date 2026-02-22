use std::ffi::c_int;

/// Prefix for one PipeWire playback stable id.
pub(super) const PIPEWIRE_PLAYBACK_STABLE_ID_PREFIX: &str = "pipewire:playback:";
/// Prefix for one PipeWire capture stable id.
pub(super) const PIPEWIRE_CAPTURE_STABLE_ID_PREFIX: &str = "pipewire:capture:";
/// Prefix for one PipeWire duplex stable id.
pub(super) const PIPEWIRE_DUPLEX_STABLE_ID_PREFIX: &str = "pipewire:duplex:";
/// Prefix for one PipeWire loopback stable id.
pub(super) const PIPEWIRE_LOOPBACK_STABLE_ID_PREFIX: &str = "pipewire:loopback:";

/// PipeWire stream direction selector: playback.
pub(super) const PIPEWIRE_STREAM_DIRECTION_PLAYBACK: c_int = 1;
/// PipeWire stream direction selector: capture.
pub(super) const PIPEWIRE_STREAM_DIRECTION_CAPTURE: c_int = 2;

/// PipeWire sample format selector: unsigned 8-bit PCM.
pub(super) const PIPEWIRE_SAMPLE_U8: c_int = 0;
/// PipeWire sample format selector: signed 16-bit little-endian PCM.
pub(super) const PIPEWIRE_SAMPLE_S16LE: c_int = 3;
/// PipeWire sample format selector: signed 16-bit big-endian PCM.
pub(super) const PIPEWIRE_SAMPLE_S16BE: c_int = 4;
/// PipeWire sample format selector: 32-bit float little-endian PCM.
pub(super) const PIPEWIRE_SAMPLE_F32LE: c_int = 5;
/// PipeWire sample format selector: 32-bit float big-endian PCM.
pub(super) const PIPEWIRE_SAMPLE_F32BE: c_int = 6;
/// PipeWire sample format selector: signed 32-bit little-endian PCM.
pub(super) const PIPEWIRE_SAMPLE_S32LE: c_int = 7;
/// PipeWire sample format selector: signed 32-bit big-endian PCM.
pub(super) const PIPEWIRE_SAMPLE_S32BE: c_int = 8;
/// PipeWire sample format selector: packed signed 24-bit in 32-bit lane, little-endian.
pub(super) const PIPEWIRE_SAMPLE_S24_32LE: c_int = 11;
/// PipeWire sample format selector: packed signed 24-bit in 32-bit lane, big-endian.
pub(super) const PIPEWIRE_SAMPLE_S24_32BE: c_int = 12;

/// One fallback preferred sample rate in hertz.
pub(super) const PIPEWIRE_PREFERRED_SAMPLE_RATE: u32 = 48_000;
/// One fallback minimum sample rate in hertz.
pub(super) const PIPEWIRE_MIN_SAMPLE_RATE: u32 = 8_000;
/// One fallback maximum sample rate in hertz.
pub(super) const PIPEWIRE_MAX_SAMPLE_RATE: u32 = 192_000;
/// One fallback preferred period in frames.
pub(super) const PIPEWIRE_PREFERRED_PERIOD_FRAMES: u32 = 256;
/// One fallback minimum period in frames.
pub(super) const PIPEWIRE_MIN_PERIOD_FRAMES: u32 = 64;
/// One fallback maximum period in frames.
pub(super) const PIPEWIRE_MAX_PERIOD_FRAMES: u32 = 8_192;
/// One fallback maximum exposed channel count.
pub(super) const PIPEWIRE_MAX_CHANNELS: u16 = 32;

/// One default PipeWire device name.
pub(super) const PIPEWIRE_DEFAULT_DEVICE_NAME: &str = "default";
/// One default PipeWire server stream name.
pub(super) const PIPEWIRE_STREAM_NAME: &str = "destack-audio";
/// One default PipeWire client application name.
pub(super) const PIPEWIRE_APPLICATION_NAME: &str = "destack-runtime";
