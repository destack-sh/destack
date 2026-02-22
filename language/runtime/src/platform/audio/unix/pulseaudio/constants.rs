use std::ffi::c_int;

/// Prefix for one PulseAudio playback stable id.
pub(super) const PULSEAUDIO_PLAYBACK_STABLE_ID_PREFIX: &str = "pulseaudio:playback:";
/// Prefix for one PulseAudio capture stable id.
pub(super) const PULSEAUDIO_CAPTURE_STABLE_ID_PREFIX: &str = "pulseaudio:capture:";
/// Prefix for one PulseAudio duplex stable id.
pub(super) const PULSEAUDIO_DUPLEX_STABLE_ID_PREFIX: &str = "pulseaudio:duplex:";
/// Prefix for one PulseAudio loopback stable id.
pub(super) const PULSEAUDIO_LOOPBACK_STABLE_ID_PREFIX: &str = "pulseaudio:loopback:";

/// PulseAudio stream direction selector: playback.
pub(super) const PULSEAUDIO_STREAM_DIRECTION_PLAYBACK: c_int = 1;
/// PulseAudio stream direction selector: capture.
pub(super) const PULSEAUDIO_STREAM_DIRECTION_CAPTURE: c_int = 2;

/// PulseAudio sample format selector: unsigned 8-bit PCM.
pub(super) const PULSEAUDIO_SAMPLE_U8: c_int = 0;
/// PulseAudio sample format selector: signed 16-bit little-endian PCM.
pub(super) const PULSEAUDIO_SAMPLE_S16LE: c_int = 3;
/// PulseAudio sample format selector: signed 16-bit big-endian PCM.
pub(super) const PULSEAUDIO_SAMPLE_S16BE: c_int = 4;
/// PulseAudio sample format selector: 32-bit float little-endian PCM.
pub(super) const PULSEAUDIO_SAMPLE_F32LE: c_int = 5;
/// PulseAudio sample format selector: 32-bit float big-endian PCM.
pub(super) const PULSEAUDIO_SAMPLE_F32BE: c_int = 6;
/// PulseAudio sample format selector: signed 32-bit little-endian PCM.
pub(super) const PULSEAUDIO_SAMPLE_S32LE: c_int = 7;
/// PulseAudio sample format selector: signed 32-bit big-endian PCM.
pub(super) const PULSEAUDIO_SAMPLE_S32BE: c_int = 8;
/// PulseAudio sample format selector: packed signed 24-bit in 32-bit lane, little-endian.
pub(super) const PULSEAUDIO_SAMPLE_S24_32LE: c_int = 11;
/// PulseAudio sample format selector: packed signed 24-bit in 32-bit lane, big-endian.
pub(super) const PULSEAUDIO_SAMPLE_S24_32BE: c_int = 12;

/// One fallback preferred sample rate in hertz.
pub(super) const PULSEAUDIO_PREFERRED_SAMPLE_RATE: u32 = 48_000;
/// One fallback minimum sample rate in hertz.
pub(super) const PULSEAUDIO_MIN_SAMPLE_RATE: u32 = 8_000;
/// One fallback maximum sample rate in hertz.
pub(super) const PULSEAUDIO_MAX_SAMPLE_RATE: u32 = 192_000;
/// One fallback preferred period in frames.
pub(super) const PULSEAUDIO_PREFERRED_PERIOD_FRAMES: u32 = 256;
/// One fallback minimum period in frames.
pub(super) const PULSEAUDIO_MIN_PERIOD_FRAMES: u32 = 64;
/// One fallback maximum period in frames.
pub(super) const PULSEAUDIO_MAX_PERIOD_FRAMES: u32 = 8_192;
/// One fallback maximum exposed channel count.
pub(super) const PULSEAUDIO_MAX_CHANNELS: u16 = 32;

/// One default PulseAudio device name.
pub(super) const PULSEAUDIO_DEFAULT_DEVICE_NAME: &str = "default";
/// One default PulseAudio server stream name.
pub(super) const PULSEAUDIO_STREAM_NAME: &str = "destack-audio";
/// One default PulseAudio client application name.
pub(super) const PULSEAUDIO_APPLICATION_NAME: &str = "destack-runtime";
