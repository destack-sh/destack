use std::ffi::{CStr, c_int};
use std::sync::Arc;

use super::abi::AAudioApi;
use super::constants::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, audio as audio_types, core as core_platform};

/// Return whether AAudio backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    cfg!(target_os = "android") && load_aaudio_library().is_ok()
}

/// One loaded AAudio dynamic library payload.
#[derive(Debug)]
pub(super) struct AAudioLibrary {
    /// Loaded dynamic-library lifetime owner.
    _library: core_platform::DynamicLibrary,
    /// Loaded AAudio function table.
    pub(super) api: AAudioApi,
}

/// Return one normalized channel layout from one channel count.
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

/// Return one AAudio runtime error payload.
pub(super) fn aaudio_error(
    operation: &'static str,
    code: Option<c_int>,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    let message = message.into();
    let message = if let Some(code) = code {
        format!(
            "{message} (aaudio status {code}: {})",
            aaudio_status_text(code)
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

/// Return one AAudio non-supported error payload.
pub(super) fn aaudio_not_supported(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: {}",
        message.into()
    )))
    .boxed()
}

/// Return whether one AAudio status code denotes success.
pub(super) fn aaudio_succeeded(status: c_int) -> bool {
    status >= AAUDIO_RESULT_OK
}

/// Return one loaded AAudio library or one not-supported error.
pub(super) fn require_aaudio_library(operation: &'static str) -> RuntimeResult<Arc<AAudioLibrary>> {
    load_aaudio_library().map_err(|error| aaudio_not_supported(operation, error))
}

/// Return one AAudio sample format for one runtime format.
pub(super) fn aaudio_sample_format(format: audio_types::AudioSampleFormat) -> Option<c_int> {
    match format {
        audio_types::AudioSampleFormat::S16 => Some(AAUDIO_FORMAT_PCM_I16),
        audio_types::AudioSampleFormat::S24 => Some(AAUDIO_FORMAT_PCM_I24_PACKED),
        audio_types::AudioSampleFormat::S32 => Some(AAUDIO_FORMAT_PCM_I32),
        audio_types::AudioSampleFormat::F32 => Some(AAUDIO_FORMAT_PCM_FLOAT),
        audio_types::AudioSampleFormat::U8 | audio_types::AudioSampleFormat::F64 => None,
    }
}

/// Return one runtime sample format for one AAudio format selector.
pub(super) fn runtime_sample_format(
    aaudio_format: c_int,
) -> Option<audio_types::AudioSampleFormat> {
    match aaudio_format {
        AAUDIO_FORMAT_PCM_I16 => Some(audio_types::AudioSampleFormat::S16),
        AAUDIO_FORMAT_PCM_I24_PACKED => Some(audio_types::AudioSampleFormat::S24),
        AAUDIO_FORMAT_PCM_I32 => Some(audio_types::AudioSampleFormat::S32),
        AAUDIO_FORMAT_PCM_FLOAT => Some(audio_types::AudioSampleFormat::F32),
        AAUDIO_FORMAT_UNSPECIFIED => None,
        _ => None,
    }
}

/// Return one conservative AAudio sample-format mask.
pub(super) fn aaudio_format_mask() -> u32 {
    audio_core::sample_format_bit(audio_types::AudioSampleFormat::S16)
        | audio_core::sample_format_bit(audio_types::AudioSampleFormat::S24)
        | audio_core::sample_format_bit(audio_types::AudioSampleFormat::S32)
        | audio_core::sample_format_bit(audio_types::AudioSampleFormat::F32)
}

/// Return one AAudio sharing-mode selector for one runtime share mode.
pub(super) fn aaudio_sharing_mode(mode: audio_types::AudioShareMode) -> c_int {
    match mode {
        audio_types::AudioShareMode::Shared => AAUDIO_SHARING_MODE_SHARED,
        audio_types::AudioShareMode::Exclusive => AAUDIO_SHARING_MODE_EXCLUSIVE,
    }
}

/// Return one AAudio read and write timeout in nanoseconds for one worker period.
pub(super) fn io_timeout_nanoseconds(period: std::time::Duration) -> i64 {
    period.as_nanos().min(i64::MAX as u128) as i64
}

/// Load one AAudio dynamic library and required symbol table.
fn load_aaudio_library() -> Result<Arc<AAudioLibrary>, String> {
    // try common AAudio soname candidates in deterministic order
    let (library, api) = core_platform::load_library_with_api(&["libaaudio.so"], load_aaudio_api)?;

    Ok(Arc::new(AAudioLibrary {
        _library: library,
        api,
    }))
}

/// Load one AAudio symbol table from one open dynamic-library handle.
fn load_aaudio_api(
    library: &core_platform::DynamicLibrary,
    candidate: &str,
) -> Result<AAudioApi, String> {
    core_platform::load_dll_api_bytes!(library, candidate, AAudioApi {
        create_stream_builder => b"AAudio_createStreamBuilder\0",
        stream_builder_delete => b"AAudioStreamBuilder_delete\0",
        stream_builder_set_direction => b"AAudioStreamBuilder_setDirection\0",
        stream_builder_set_sample_rate => b"AAudioStreamBuilder_setSampleRate\0",
        stream_builder_set_channel_count => b"AAudioStreamBuilder_setChannelCount\0",
        stream_builder_set_format => b"AAudioStreamBuilder_setFormat\0",
        stream_builder_set_sharing_mode => b"AAudioStreamBuilder_setSharingMode\0",
        stream_builder_set_performance_mode => b"AAudioStreamBuilder_setPerformanceMode\0",
        stream_builder_set_buffer_capacity_frames => b"AAudioStreamBuilder_setBufferCapacityInFrames\0",
        stream_builder_open_stream => b"AAudioStreamBuilder_openStream\0",
        stream_close => b"AAudioStream_close\0",
        stream_request_start => b"AAudioStream_requestStart\0",
        stream_request_pause => b"AAudioStream_requestPause\0",
        stream_request_stop => b"AAudioStream_requestStop\0",
        stream_request_flush => b"AAudioStream_requestFlush\0",
        stream_read => b"AAudioStream_read\0",
        stream_write => b"AAudioStream_write\0",
        stream_get_sample_rate => b"AAudioStream_getSampleRate\0",
        stream_get_channel_count => b"AAudioStream_getChannelCount\0",
        stream_get_frames_per_burst => b"AAudioStream_getFramesPerBurst\0",
        stream_get_format => b"AAudioStream_getFormat\0",
        stream_get_sharing_mode => b"AAudioStream_getSharingMode\0",
        convert_result_to_text => b"AAudio_convertResultToText\0",
    })
}

/// Return one human-readable AAudio status text.
fn aaudio_status_text(status: c_int) -> String {
    let Ok(library) = load_aaudio_library() else {
        return String::from("unknown status");
    };

    let pointer = unsafe { (library.api.convert_result_to_text)(status) };
    if pointer.is_null() {
        return String::from("unknown status");
    }

    unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned()
}
