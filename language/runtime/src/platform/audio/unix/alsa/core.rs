use std::ffi::{CStr, CString, c_int};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use super::abi::{AlsaApi, AlsaPcm};
use super::ffi::alsa_library;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, audio as audio_types, core as core_platform};

/// Return whether ALSA backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    cfg!(target_os = "linux") && alsa_library().is_some()
}

/// One loaded ALSA dynamic library payload.
#[derive(Debug)]
pub(super) struct AlsaLibrary {
    /// Loaded dynamic-library lifetime owner.
    pub(super) _library: core_platform::DynamicLibrary,
    /// Loaded ALSA function table.
    pub(super) api: AlsaApi,
}

/// One direction marker parsed from ALSA hint metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AlsaHintDirection {
    /// One playback-only hint direction marker.
    Playback,
    /// One capture-only hint direction marker.
    Capture,
}

/// One raw ALSA hint row before capability probing.
#[derive(Debug, Clone)]
pub(super) struct AlsaHintRow {
    /// ALSA PCM device name.
    pub(super) name: String,
    /// Human-readable ALSA device description.
    pub(super) description: String,
    /// Optional hint-declared direction.
    pub(super) direction: Option<AlsaHintDirection>,
}

/// One normalized ALSA stable-id lane selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AlsaDirectionLane {
    /// One playback lane.
    Playback,
    /// One capture lane.
    Capture,
    /// One duplex lane.
    Duplex,
}

/// One parsed ALSA stable-id payload.
#[derive(Debug, Clone)]
pub(super) struct ParsedAlsaStableId {
    /// Requested lane encoded in the stable-id prefix.
    pub(super) lane: AlsaDirectionLane,
    /// Playback PCM name resolved from the stable-id payload.
    pub(super) playback_name: String,
    /// Capture PCM name resolved from the stable-id payload.
    pub(super) capture_name: Option<String>,
}

/// One ALSA stream format candidate used for capability probing.
#[derive(Debug, Clone, Copy)]
pub(super) struct AlsaFormatCandidate {
    /// Runtime sample format.
    pub(super) runtime_format: audio_types::AudioSampleFormat,
    /// ALSA format token passed to `snd_pcm_format_value`.
    pub(super) alsa_name: &'static str,
}

/// One probed ALSA capability profile for one PCM lane.
#[derive(Debug, Clone)]
pub(super) struct AlsaDeviceProfile {
    /// Preferred sample rate in hertz.
    pub(super) preferred_sample_rate: u32,
    /// Minimum sample rate in hertz.
    pub(super) min_sample_rate: u32,
    /// Maximum sample rate in hertz.
    pub(super) max_sample_rate: u32,
    /// Preferred period in frames.
    pub(super) preferred_period_frames: u32,
    /// Minimum period in frames.
    pub(super) min_period_frames: u32,
    /// Maximum period in frames.
    pub(super) max_period_frames: u32,
    /// Preferred channel count.
    pub(super) preferred_channels: u16,
    /// Minimum channel count.
    pub(super) min_channels: u16,
    /// Maximum channel count.
    pub(super) max_channels: u16,
    /// Supported sample format mask.
    pub(super) format_mask: u32,
    /// Whether ALSA pause and resume is supported.
    pub(super) supports_pause: bool,
    /// Whether this PCM appears to map to one hardware endpoint.
    pub(super) supports_exclusive: bool,
}

/// One opened ALSA PCM handle.
#[derive(Debug)]
pub(super) struct AlsaPcmHandle {
    /// Raw `snd_pcm_t*` pointer.
    pub(super) raw: *mut AlsaPcm,
    /// Shared ALSA symbol table.
    pub(super) library: Arc<AlsaLibrary>,
}

impl Drop for AlsaPcmHandle {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        // close one owned pcm handle
        unsafe {
            let _ = (self.library.api.snd_pcm_close)(self.raw);
        }
    }
}

unsafe impl Send for AlsaPcmHandle {}
unsafe impl Sync for AlsaPcmHandle {}

/// One opened ALSA stream runtime payload.
#[derive(Debug)]
pub(super) struct AlsaStreamRuntime {
    /// Effective sample rate in hertz.
    pub(super) sample_rate: u32,
    /// Effective channel count.
    pub(super) channels: u16,
    /// Effective period in frames.
    pub(super) period_frames: u32,
    /// Bytes per interleaved frame.
    pub(super) frame_bytes: usize,
    /// Effective runtime sample format.
    pub(super) format: audio_types::AudioSampleFormat,
    /// Period pacing duration for idle sleeps.
    pub(super) poll_period: Duration,
    /// Whether pause and resume is supported by all opened PCM lanes.
    pub(super) supports_pause: bool,
    /// Playback lane handle when opened.
    pub(super) playback_pcm: Option<Arc<AlsaPcmHandle>>,
    /// Capture lane handle when opened.
    pub(super) capture_pcm: Option<Arc<AlsaPcmHandle>>,
    /// Weak link to stream host state for callback workers.
    pub(super) stream_state: Mutex<Weak<audio_core::AudioStreamHostState>>,
}

unsafe impl Send for AlsaStreamRuntime {}
unsafe impl Sync for AlsaStreamRuntime {}

/// One host-operations payload for one ALSA stream host state.
#[derive(Debug)]
pub(super) struct AlsaHostStreamOps {
    /// Shared ALSA runtime payload.
    pub(super) runtime: Arc<AlsaStreamRuntime>,
}

/// Build one ALSA runtime I/O error from one status code and message.
pub(super) fn alsa_error(
    operation: &'static str,
    status: c_int,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    let description = if let Some(library) = alsa_library() {
        let error_text = unsafe {
            let pointer = (library.api.snd_strerror)(status);
            if pointer.is_null() {
                String::from("unknown ALSA error")
            } else {
                CStr::from_ptr(pointer).to_string_lossy().into_owned()
            }
        };

        format!("{} (alsa status {status}: {error_text})", message.into())
    } else {
        format!("{} (alsa status {status})", message.into())
    };

    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        description,
    ))
    .boxed()
}

/// Return one transport label for one ALSA device name.
pub(super) fn transport_from_device_name(device_name: &str) -> &'static str {
    if device_name.starts_with("hw:") {
        return "alsa-hardware";
    }

    if device_name.starts_with("plughw:") {
        return "alsa-plugin";
    }

    if device_name.starts_with("pipewire") {
        return "pipewire";
    }

    if device_name.starts_with("jack") {
        return "jack";
    }

    if device_name.starts_with("pulse") {
        return "pulseaudio";
    }

    "alsa"
}

/// Return one channel layout enum from one channel count.
pub(super) fn channel_layout(channel_count: u16) -> audio_types::AudioChannelLayout {
    match channel_count {
        1 => audio_types::AudioChannelLayout::Mono,
        2 => audio_types::AudioChannelLayout::Stereo,
        4 => audio_types::AudioChannelLayout::Quad,
        5 => audio_types::AudioChannelLayout::Surround41,
        6 => audio_types::AudioChannelLayout::Surround51,
        7 => audio_types::AudioChannelLayout::Surround61,
        8 => audio_types::AudioChannelLayout::Surround71,
        _ => audio_types::AudioChannelLayout::Unknown,
    }
}

/// Return one contiguous channel mask for one channel count.
pub(super) fn channel_mask(channel_count: u16) -> u64 {
    if channel_count == 0 {
        return 0;
    }

    if channel_count >= 64 {
        return u64::MAX;
    }

    (1u64 << channel_count) - 1
}

/// Build one c-string from one UTF-8 payload.
pub(super) fn c_string(text: &str, field: &'static str) -> RuntimeResult<CString> {
    CString::new(text).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "value must not contain interior NUL bytes",
        ))
        .boxed()
    })
}

/// Return whether one ALSA status code denotes success.
pub(super) fn alsa_succeeded(status: c_int) -> bool {
    status >= 0
}
