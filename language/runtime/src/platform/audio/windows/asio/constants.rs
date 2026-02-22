/// ASIO host registry root.
pub(super) const ASIO_REGISTRY_PATH: &str = "SOFTWARE\\ASIO";
/// Prefix for one ASIO playback stable id.
pub(super) const ASIO_PLAYBACK_STABLE_ID_PREFIX: &str = "asio:playback:";
/// Prefix for one ASIO capture stable id.
pub(super) const ASIO_CAPTURE_STABLE_ID_PREFIX: &str = "asio:capture:";
/// Prefix for one ASIO duplex stable id.
pub(super) const ASIO_DUPLEX_STABLE_ID_PREFIX: &str = "asio:duplex:";
/// Maximum length of one ASIO driver display string.
pub(super) const ASIO_MAX_DRIVER_NAME_BYTES: usize = 32;
/// Maximum length of one ASIO driver error string.
pub(super) const ASIO_MAX_ERROR_MESSAGE_BYTES: usize = 124;
/// Maximum probed channel count exposed to descriptor rows.
pub(super) const ASIO_MAX_PROBED_CHANNELS: u16 = 64;
/// Fallback minimum sample rate for conservative descriptors.
pub(super) const ASIO_FALLBACK_MIN_SAMPLE_RATE: u32 = 8_000;
/// Fallback maximum sample rate for conservative descriptors.
pub(super) const ASIO_FALLBACK_MAX_SAMPLE_RATE: u32 = 384_000;
/// Fallback preferred period when probing fails.
pub(super) const ASIO_FALLBACK_PREFERRED_PERIOD_FRAMES: u32 = 256;
/// Fallback minimum period when probing fails.
pub(super) const ASIO_FALLBACK_MIN_PERIOD_FRAMES: u32 = 64;
/// Fallback maximum period when probing fails.
pub(super) const ASIO_FALLBACK_MAX_PERIOD_FRAMES: u32 = 8_192;

/// ASIO success return code.
pub(super) const ASE_OK: i32 = 0;
/// ASIO not-present return code.
pub(super) const ASE_NOT_PRESENT: i32 = -1000;

/// One ASIO true value.
pub(super) const ASIO_TRUE: i32 = 1;
/// One ASIO false value.
pub(super) const ASIO_FALSE: i32 = 0;

/// One ASIO selector-supported message id.
pub(super) const K_ASIO_SELECTOR_SUPPORTED: i32 = 1;
/// One ASIO engine-version message id.
pub(super) const K_ASIO_ENGINE_VERSION: i32 = 2;
/// One ASIO reset-request message id.
pub(super) const K_ASIO_RESET_REQUEST: i32 = 3;
/// One ASIO resync-request message id.
pub(super) const K_ASIO_RESYNC_REQUEST: i32 = 5;
/// One ASIO latency-changed message id.
pub(super) const K_ASIO_LATENCIES_CHANGED: i32 = 6;
/// One ASIO supports-time-info message id.
pub(super) const K_ASIO_SUPPORTS_TIME_INFO: i32 = 7;
/// One ASIO supports-time-code message id.
pub(super) const K_ASIO_SUPPORTS_TIME_CODE: i32 = 8;

/// One ASIO 16-bit little-endian sample encoding.
pub(super) const ASIO_ST_INT16_LSB: i32 = 16;
/// One ASIO 24-bit little-endian sample encoding.
pub(super) const ASIO_ST_INT24_LSB: i32 = 17;
/// One ASIO 32-bit little-endian sample encoding.
pub(super) const ASIO_ST_INT32_LSB: i32 = 18;
/// One ASIO 32-bit little-endian float sample encoding.
pub(super) const ASIO_ST_FLOAT32_LSB: i32 = 19;
/// One ASIO 64-bit little-endian float sample encoding.
pub(super) const ASIO_ST_FLOAT64_LSB: i32 = 20;
/// One ASIO 32-bit aligned little-endian 24-bit encoding.
pub(super) const ASIO_ST_INT32_LSB24: i32 = 27;
/// One ASIO 16-bit big-endian sample encoding.
pub(super) const ASIO_ST_INT16_MSB: i32 = 0;
/// One ASIO 24-bit big-endian sample encoding.
pub(super) const ASIO_ST_INT24_MSB: i32 = 1;
/// One ASIO 32-bit big-endian sample encoding.
pub(super) const ASIO_ST_INT32_MSB: i32 = 2;
/// One ASIO 32-bit big-endian float sample encoding.
pub(super) const ASIO_ST_FLOAT32_MSB: i32 = 3;
/// One ASIO 64-bit big-endian float sample encoding.
pub(super) const ASIO_ST_FLOAT64_MSB: i32 = 4;
/// One ASIO 32-bit aligned big-endian 24-bit encoding.
pub(super) const ASIO_ST_INT32_MSB24: i32 = 11;

/// One stable sample-rate probe set for ASIO capability discovery.
pub(super) const ASIO_PROBED_SAMPLE_RATES: [u32; 13] = [
    8_000, 11_025, 12_000, 16_000, 22_050, 24_000, 32_000, 44_100, 48_000, 88_200, 96_000, 176_400,
    192_000,
];
