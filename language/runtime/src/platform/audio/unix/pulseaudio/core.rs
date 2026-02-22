use std::ffi::{CStr, CString, c_int, c_void};
use std::ptr;
use std::sync::{Arc, OnceLock};

use super::abi::{PulseAudioApi, PulseSampleSpec};
use super::constants::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};

/// Return whether PulseAudio backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    cfg!(target_os = "linux") && pulseaudio_library().is_some() && pulseaudio_server_available()
}

/// One process-global PulseAudio dynamic library slot.
static PULSEAUDIO_LIBRARY_SLOT: OnceLock<Option<Arc<PulseAudioLibrary>>> = OnceLock::new();

/// One loaded PulseAudio dynamic library payload.
#[derive(Debug)]
pub(super) struct PulseAudioLibrary {
    /// Raw dynamic-library handle from `dlopen`.
    handle: *mut c_void,
    /// Loaded PulseAudio function table.
    pub(super) api: PulseAudioApi,
}

impl Drop for PulseAudioLibrary {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }

        // close one open dynamic-library handle
        core_platform::close_dynamic_library(self.handle);
    }
}

unsafe impl Send for PulseAudioLibrary {}
unsafe impl Sync for PulseAudioLibrary {}

/// Return one normalized channel layout from one channel count.
pub(super) fn channel_layout(channel_count: u16) -> audio_core::AudioChannelLayout {
    match channel_count {
        1 => audio_core::AudioChannelLayout::Mono,
        2 => audio_core::AudioChannelLayout::Stereo,
        4 => audio_core::AudioChannelLayout::Quad,
        5 => audio_core::AudioChannelLayout::Surround41,
        6 => audio_core::AudioChannelLayout::Surround51,
        7 => audio_core::AudioChannelLayout::Surround61,
        8 => audio_core::AudioChannelLayout::Surround71,
        _ => audio_core::AudioChannelLayout::Unknown,
    }
}

/// Return one contiguous channel mask from one channel count.
pub(super) fn channel_mask(channel_count: u16) -> u64 {
    if channel_count == 0 {
        return 0;
    }

    if channel_count >= 64 {
        return u64::MAX;
    }

    (1u64 << channel_count) - 1
}

/// Return one PulseAudio runtime error payload.
pub(super) fn pulseaudio_error(
    operation: &'static str,
    code: Option<c_int>,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    let message = message.into();
    let message = if let Some(code) = code {
        format!(
            "{message} (pulse error {code}: {})",
            pulseaudio_error_text(code)
        )
    } else {
        message
    };

    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        message,
    ))
    .boxed()
}

/// Return one PulseAudio non-supported error payload.
pub(super) fn pulseaudio_not_supported(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: {}",
        message.into()
    )))
    .boxed()
}

/// Return whether one PulseAudio status code denotes success.
pub(super) fn pulseaudio_succeeded(status: c_int) -> bool {
    status >= 0
}

/// Return one c-string built from one UTF-8 payload.
pub(super) fn c_string(value: &str, field: &'static str) -> RuntimeResult<CString> {
    CString::new(value).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "value must not contain interior NUL bytes",
        ))
        .boxed()
    })
}

/// Return one loaded PulseAudio library or one not-supported error.
pub(super) fn require_pulseaudio_library(
    operation: &'static str,
) -> RuntimeResult<&'static Arc<PulseAudioLibrary>> {
    pulseaudio_library().ok_or_else(|| {
        pulseaudio_not_supported(
            operation,
            "PulseAudio dynamic library is unavailable on this host",
        )
    })
}

/// Return one PulseAudio sample format selector for one runtime sample format.
pub(super) fn pulseaudio_sample_format(format: audio_core::AudioSampleFormat) -> Option<c_int> {
    match format {
        audio_core::AudioSampleFormat::U8 => Some(PULSEAUDIO_SAMPLE_U8),
        audio_core::AudioSampleFormat::S16 => Some(if cfg!(target_endian = "little") {
            PULSEAUDIO_SAMPLE_S16LE
        } else {
            PULSEAUDIO_SAMPLE_S16BE
        }),
        audio_core::AudioSampleFormat::S24 => Some(if cfg!(target_endian = "little") {
            PULSEAUDIO_SAMPLE_S24_32LE
        } else {
            PULSEAUDIO_SAMPLE_S24_32BE
        }),
        audio_core::AudioSampleFormat::S32 => Some(if cfg!(target_endian = "little") {
            PULSEAUDIO_SAMPLE_S32LE
        } else {
            PULSEAUDIO_SAMPLE_S32BE
        }),
        audio_core::AudioSampleFormat::F32 => Some(if cfg!(target_endian = "little") {
            PULSEAUDIO_SAMPLE_F32LE
        } else {
            PULSEAUDIO_SAMPLE_F32BE
        }),
        audio_core::AudioSampleFormat::F64 => None,
    }
}

/// Return one conservative PulseAudio sample-format mask.
pub(super) fn pulseaudio_format_mask() -> u32 {
    audio_core::sample_format_bit(audio_core::AudioSampleFormat::U8)
        | audio_core::sample_format_bit(audio_core::AudioSampleFormat::S16)
        | audio_core::sample_format_bit(audio_core::AudioSampleFormat::S24)
        | audio_core::sample_format_bit(audio_core::AudioSampleFormat::S32)
        | audio_core::sample_format_bit(audio_core::AudioSampleFormat::F32)
}

/// Return one pointer to one loaded PulseAudio library when available.
fn pulseaudio_library() -> Option<&'static Arc<PulseAudioLibrary>> {
    PULSEAUDIO_LIBRARY_SLOT
        .get_or_init(load_pulseaudio_library)
        .as_ref()
}

/// Load one PulseAudio dynamic library and required symbol table.
fn load_pulseaudio_library() -> Option<Arc<PulseAudioLibrary>> {
    // try common PulseAudio simple-api soname candidates in deterministic order
    for candidate in ["libpulse-simple.so.0", "libpulse-simple.so"] {
        let Some(handle) = core_platform::open_dynamic_library(candidate) else {
            continue;
        };

        // resolve all required PulseAudio simple symbols
        let api = match load_pulseaudio_api(handle) {
            Some(api) => api,
            None => {
                core_platform::close_dynamic_library(handle);

                continue;
            }
        };

        return Some(Arc::new(PulseAudioLibrary { handle, api }));
    }

    None
}

/// Load one PulseAudio symbol table from one open dynamic-library handle.
fn load_pulseaudio_api(handle: *mut c_void) -> Option<PulseAudioApi> {
    Some(PulseAudioApi {
        pa_simple_new: core_platform::load_dynamic_symbol(handle, b"pa_simple_new\0")?,
        pa_simple_free: core_platform::load_dynamic_symbol(handle, b"pa_simple_free\0")?,
        pa_simple_read: core_platform::load_dynamic_symbol(handle, b"pa_simple_read\0")?,
        pa_simple_write: core_platform::load_dynamic_symbol(handle, b"pa_simple_write\0")?,
        pa_simple_flush: core_platform::load_dynamic_symbol(handle, b"pa_simple_flush\0")?,
        pa_simple_drain: core_platform::load_dynamic_symbol(handle, b"pa_simple_drain\0")?,
        pa_strerror: core_platform::load_dynamic_symbol(handle, b"pa_strerror\0")?,
    })
}

/// Return one human-readable PulseAudio error string.
fn pulseaudio_error_text(code: c_int) -> String {
    let Some(library) = pulseaudio_library() else {
        return String::from("unknown error");
    };

    let pointer = unsafe { (library.api.pa_strerror)(code) };
    if pointer.is_null() {
        return String::from("unknown error");
    }

    unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned()
}

/// Return whether one PulseAudio server is reachable from this process.
fn pulseaudio_server_available() -> bool {
    let Ok(library) = require_pulseaudio_library("destack.audio.device.list") else {
        return false;
    };

    let Ok(application_name) = c_string(PULSEAUDIO_APPLICATION_NAME, "applicationName") else {
        return false;
    };
    let Ok(stream_name) = c_string(PULSEAUDIO_STREAM_NAME, "streamName") else {
        return false;
    };

    let sample_spec = PulseSampleSpec {
        format: if cfg!(target_endian = "little") {
            PULSEAUDIO_SAMPLE_S16LE
        } else {
            PULSEAUDIO_SAMPLE_S16BE
        },
        rate: PULSEAUDIO_PREFERRED_SAMPLE_RATE,
        channels: 2,
    };

    let mut error = 0;

    // probe one short-lived playback stream to validate server reachability
    let stream = unsafe {
        (library.api.pa_simple_new)(
            ptr::null(),
            application_name.as_ptr(),
            PULSEAUDIO_STREAM_DIRECTION_PLAYBACK,
            ptr::null(),
            stream_name.as_ptr(),
            &sample_spec,
            ptr::null(),
            ptr::null(),
            &mut error,
        )
    };
    if stream.is_null() {
        return false;
    }

    // close one successful probe stream immediately
    unsafe {
        (library.api.pa_simple_free)(stream);
    }

    true
}
