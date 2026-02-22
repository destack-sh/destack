use std::ffi::c_int;

use crate::platform::audio::core as audio_core;

use super::core::AlsaFormatCandidate;

/// Prefix for one ALSA playback stable id.
pub(super) const ALSA_PLAYBACK_STABLE_ID_PREFIX: &str = "alsa:playback:";
/// Prefix for one ALSA capture stable id.
pub(super) const ALSA_CAPTURE_STABLE_ID_PREFIX: &str = "alsa:capture:";
/// Prefix for one ALSA duplex stable id.
pub(super) const ALSA_DUPLEX_STABLE_ID_PREFIX: &str = "alsa:duplex:";

/// One ALSA playback stream selector.
pub(super) const ALSA_STREAM_PLAYBACK: c_int = 0;
/// One ALSA capture stream selector.
pub(super) const ALSA_STREAM_CAPTURE: c_int = 1;

/// One ALSA interleaved read and write access selector.
pub(super) const ALSA_ACCESS_RW_INTERLEAVED: c_int = 3;
/// One ALSA hardware-device type selector.
pub(super) const ALSA_PCM_TYPE_HARDWARE: c_int = 0;
/// One ALSA true value.
pub(super) const ALSA_TRUE: c_int = 1;
/// One ALSA false value.
pub(super) const ALSA_FALSE: c_int = 0;

/// One ALSA nonblocking enable value.
pub(super) const ALSA_NONBLOCK_ENABLED: c_int = 1;
/// One ALSA disabled flag value.
pub(super) const ALSA_FLAG_NONE: c_int = 0;
/// One ALSA recover call silent flag.
pub(super) const ALSA_RECOVER_SILENT: c_int = 1;
/// Worker wait timeout in milliseconds.
pub(super) const ALSA_WAIT_TIMEOUT_MILLISECONDS: c_int = 16;

/// Default preferred sample rate for conservative ALSA rows.
pub(super) const ALSA_FALLBACK_PREFERRED_SAMPLE_RATE: u32 = 48_000;
/// Default preferred period frames for conservative ALSA rows.
pub(super) const ALSA_FALLBACK_PREFERRED_PERIOD_FRAMES: u32 = 256;
/// Maximum channel count exposed by ALSA capability probing.
pub(super) const ALSA_MAX_PROBED_CHANNELS: u16 = 32;

/// One fallback hint device name when ALSA hint enumeration fails.
pub(super) const ALSA_DEFAULT_DEVICE_NAME: &str = "default";
/// One ALSA hint lookup key for one device name.
pub(super) const ALSA_HINT_NAME_KEY: &str = "NAME";
/// One ALSA hint lookup key for one device description.
pub(super) const ALSA_HINT_DESCRIPTION_KEY: &str = "DESC";
/// One ALSA hint lookup key for one I/O direction marker.
pub(super) const ALSA_HINT_IOID_KEY: &str = "IOID";
/// One ALSA hint interface selector for PCM devices.
pub(super) const ALSA_HINT_PCM_INTERFACE: &str = "pcm";

/// One stable sample-rate probe set for ALSA capability discovery.
pub(super) const ALSA_PROBED_SAMPLE_RATES: [u32; 13] = [
    8_000, 11_025, 12_000, 16_000, 22_050, 24_000, 32_000, 44_100, 48_000, 88_200, 96_000, 176_400,
    192_000,
];

/// One stable sample-format candidate for ALSA format probing.
pub(super) const ALSA_FORMAT_CANDIDATES: [AlsaFormatCandidate; 6] = [
    AlsaFormatCandidate {
        runtime_format: audio_core::AudioSampleFormat::U8,
        alsa_name: "U8",
    },
    AlsaFormatCandidate {
        runtime_format: audio_core::AudioSampleFormat::S16,
        alsa_name: "S16_LE",
    },
    AlsaFormatCandidate {
        runtime_format: audio_core::AudioSampleFormat::S24,
        alsa_name: "S24_LE",
    },
    AlsaFormatCandidate {
        runtime_format: audio_core::AudioSampleFormat::S32,
        alsa_name: "S32_LE",
    },
    AlsaFormatCandidate {
        runtime_format: audio_core::AudioSampleFormat::F32,
        alsa_name: "FLOAT_LE",
    },
    AlsaFormatCandidate {
        runtime_format: audio_core::AudioSampleFormat::F64,
        alsa_name: "FLOAT64_LE",
    },
];
