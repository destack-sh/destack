/// Prefix for one OpenSL ES playback stable id.
pub(super) const OPENSLES_PLAYBACK_STABLE_ID_PREFIX: &str = "opensles:playback:";
/// Prefix for one OpenSL ES capture stable id.
pub(super) const OPENSLES_CAPTURE_STABLE_ID_PREFIX: &str = "opensles:capture:";
/// Prefix for one OpenSL ES duplex stable id.
pub(super) const OPENSLES_DUPLEX_STABLE_ID_PREFIX: &str = "opensles:duplex:";

/// One default OpenSL ES endpoint name.
pub(super) const OPENSLES_DEFAULT_ENDPOINT_NAME: &str = "default";
/// One stream name used for diagnostics.
pub(super) const OPENSLES_STREAM_NAME: &str = "destack-audio";
/// One OpenSL ES queue depth for stream transfer.
pub(super) const OPENSLES_QUEUE_DEPTH: usize = 3;

/// One fallback preferred sample rate in hertz.
pub(super) const OPENSLES_PREFERRED_SAMPLE_RATE: u32 = 48_000;
/// One fallback minimum sample rate in hertz.
pub(super) const OPENSLES_MIN_SAMPLE_RATE: u32 = 8_000;
/// One fallback maximum sample rate in hertz.
pub(super) const OPENSLES_MAX_SAMPLE_RATE: u32 = 192_000;
/// One fallback preferred period in frames.
pub(super) const OPENSLES_PREFERRED_PERIOD_FRAMES: u32 = 256;
/// One fallback minimum period in frames.
pub(super) const OPENSLES_MIN_PERIOD_FRAMES: u32 = 64;
/// One fallback maximum period in frames.
pub(super) const OPENSLES_MAX_PERIOD_FRAMES: u32 = 8_192;
/// One fallback maximum exposed channel count.
pub(super) const OPENSLES_MAX_CHANNELS: u16 = 2;

/// One OpenSL ES success status code.
pub(super) const SL_RESULT_SUCCESS: SLresult = 0;
/// One OpenSL ES false value.
pub(super) const SL_BOOLEAN_FALSE: SLboolean = 0;
/// One OpenSL ES true value.
pub(super) const SL_BOOLEAN_TRUE: SLboolean = 1;

/// One OpenSL ES I/O device locator tag.
pub(super) const SL_DATALOCATOR_IODEVICE: SLuint32 = 0x00000003;
/// One OpenSL ES output-mix locator tag.
pub(super) const SL_DATALOCATOR_OUTPUTMIX: SLuint32 = 0x00000004;
/// One OpenSL ES Android simple-buffer-queue locator tag.
pub(super) const SL_DATALOCATOR_ANDROIDSIMPLEBUFFERQUEUE: SLuint32 = 0x8000_07BD;

/// One OpenSL ES PCM format tag.
pub(super) const SL_DATAFORMAT_PCM: SLuint32 = 0x00000002;
/// One OpenSL ES little-endian byte order marker.
pub(super) const SL_BYTEORDER_LITTLEENDIAN: SLuint32 = 0x00000002;

/// One OpenSL ES audio-input device type selector.
pub(super) const SL_IODEVICE_AUDIOINPUT: SLuint32 = 0x00000001;
/// One OpenSL ES default audio-input device id.
pub(super) const SL_DEFAULTDEVICEID_AUDIOINPUT: SLuint32 = 0xFFFF_FFFF;

/// One OpenSL ES 16-bit PCM sample-format selector.
pub(super) const SL_PCMSAMPLEFORMAT_FIXED_16: SLuint32 = 0x0000_0010;

/// One OpenSL ES speaker mask bit: front-left.
pub(super) const SL_SPEAKER_FRONT_LEFT: SLuint32 = 0x0000_0001;
/// One OpenSL ES speaker mask bit: front-right.
pub(super) const SL_SPEAKER_FRONT_RIGHT: SLuint32 = 0x0000_0002;
/// One OpenSL ES speaker mask bit: front-center.
pub(super) const SL_SPEAKER_FRONT_CENTER: SLuint32 = 0x0000_0004;

/// One OpenSL ES play-state selector: stopped.
pub(super) const SL_PLAYSTATE_STOPPED: SLuint32 = 0x0000_0001;
/// One OpenSL ES play-state selector: paused.
pub(super) const SL_PLAYSTATE_PAUSED: SLuint32 = 0x0000_0002;
/// One OpenSL ES play-state selector: playing.
pub(super) const SL_PLAYSTATE_PLAYING: SLuint32 = 0x0000_0003;

/// One OpenSL ES record-state selector: stopped.
pub(super) const SL_RECORDSTATE_STOPPED: SLuint32 = 0x0000_0001;
/// One OpenSL ES record-state selector: paused.
pub(super) const SL_RECORDSTATE_PAUSED: SLuint32 = 0x0000_0002;
/// One OpenSL ES record-state selector: recording.
pub(super) const SL_RECORDSTATE_RECORDING: SLuint32 = 0x0000_0003;
