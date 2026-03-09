use crate::platform::audio as audio_types;
use windows_sys::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;
use windows_sys::core::{GUID, HRESULT};

pub(super) const IID_IMM_DEVICE_ENUMERATOR: GUID =
    GUID::from_u128(0xa95664d2_9614_4f35_a746_de8db63617e6);
pub(super) const IID_IMM_NOTIFICATION_CLIENT: GUID =
    GUID::from_u128(0x7991eec9_7e89_4d85_8390_6c703cec60c0);
pub(super) const IID_IAUDIO_CLIENT: GUID = GUID::from_u128(0x1cb9ad4c_dbfa_4c32_b178_c2f568a703b2);
pub(super) const IID_IAUDIO_RENDER_CLIENT: GUID =
    GUID::from_u128(0xf294acfc_3146_4483_a7bf_addca7c260e2);
pub(super) const IID_IAUDIO_CAPTURE_CLIENT: GUID =
    GUID::from_u128(0xc8adbd64_e71e_48a0_a4de_185c395cd317);
/// The device friendly-name property key.
pub(super) const DEVICE_FRIENDLY_NAME_PROPERTY_KEY: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 14,
};
/// The endpoint form-factor property key.
pub(super) const ENDPOINT_FORM_FACTOR_PROPERTY_KEY: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0x1da5d803_d492_4edd_8c23_e0c0ffee7f0e),
    pid: 0,
};

/// WAVE_FORMAT_EXTENSIBLE tag value.
pub(super) const WAVE_FORMAT_EXTENSIBLE_TAG: u16 = 0xfffe;
/// WAVEFORMATEX extra bytes required for one WAVEFORMATEXTENSIBLE payload.
pub(super) const WAVE_FORMAT_EXTENSIBLE_EXTRA_BYTES: u16 = 22;
/// PCM subtype guid for extensible wave formats.
pub(super) const WAVE_SUBTYPE_PCM: GUID = GUID::from_u128(0x00000001_0000_0010_8000_00aa00389b71);
/// IEEE float subtype guid for extensible wave formats.
pub(super) const WAVE_SUBTYPE_IEEE_FLOAT: GUID =
    GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);

/// Known successful HRESULT values for exact and closest-match format queries.
pub(super) const HRESULT_OK: HRESULT = 0;

/// Prefix for one encoded WASAPI duplex stable id.
pub(super) const DUPLEX_STABLE_ID_PREFIX: &str = "wasapi:duplex:";
/// Maximum probed channel count for descriptor capability scans.
pub(super) const WASAPI_MAX_PROBED_CHANNELS: u16 = 8;
/// Maximum number of handles waited in one stream worker cycle.
pub(super) const MAX_EVENT_WAIT_HANDLES: usize = 2;
/// Fallback minimum advertised sample rate.
pub(super) const DEFAULT_MIN_SAMPLE_RATE: u32 = 8_000;
/// Fallback maximum advertised sample rate.
pub(super) const DEFAULT_MAX_SAMPLE_RATE: u32 = 192_000;
/// Fallback preferred period in frames.
pub(super) const DEFAULT_PREFERRED_PERIOD_FRAMES: u32 = 256;
/// Fallback maximum period in frames.
pub(super) const DEFAULT_MAX_PERIOD_FRAMES: u32 = 8_192;
/// Default low-latency poll timeout in milliseconds.
pub(super) const MIN_WAIT_TIMEOUT_MILLISECONDS: u32 = 1;
/// The shared sample-rate probe set used for descriptor capability discovery.
pub(super) const PROBED_SAMPLE_RATES: [u32; 13] = [
    8_000, 11_025, 12_000, 16_000, 22_050, 24_000, 32_000, 44_100, 48_000, 88_200, 96_000, 176_400,
    192_000,
];
/// The shared sample-format probe set used for descriptor capability discovery.
pub(super) const PROBED_SAMPLE_FORMATS: [audio_types::AudioSampleFormat; 5] = [
    audio_types::AudioSampleFormat::U8,
    audio_types::AudioSampleFormat::S16,
    audio_types::AudioSampleFormat::S24,
    audio_types::AudioSampleFormat::S32,
    audio_types::AudioSampleFormat::F32,
];
/// The QPC scale denominator for 100ns timestamps.
pub(super) const HUNDRED_NANOS_PER_SECOND: u128 = 10_000_000;
/// The nanosecond conversion factor from 100ns units.
pub(super) const NANOS_PER_HUNDRED_NANOS: u64 = 100;
