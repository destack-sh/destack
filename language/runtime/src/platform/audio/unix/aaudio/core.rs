use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::{Arc, OnceLock};

use super::abi::{AAudioApi, AAudioStream, AAudioStreamBuilder};
use super::constants::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};

/// Return whether AAudio backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    cfg!(target_os = "android") && aaudio_library().is_some()
}

/// One process-global AAudio dynamic library slot.
static AAUDIO_LIBRARY_SLOT: OnceLock<Option<Arc<AAudioLibrary>>> = OnceLock::new();

/// One loaded AAudio dynamic library payload.
#[derive(Debug)]
pub(super) struct AAudioLibrary {
    /// Raw dynamic-library handle from `dlopen`.
    handle: *mut c_void,
    /// Loaded AAudio function table.
    pub(super) api: AAudioApi,
}

impl Drop for AAudioLibrary {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }

        // close one open dynamic-library handle
        core_platform::close_dynamic_library(self.handle);
    }
}

unsafe impl Send for AAudioLibrary {}
unsafe impl Sync for AAudioLibrary {}

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
pub(super) fn require_aaudio_library(
    operation: &'static str,
) -> RuntimeResult<&'static Arc<AAudioLibrary>> {
    aaudio_library().ok_or_else(|| {
        aaudio_not_supported(
            operation,
            "AAudio dynamic library is unavailable on this host",
        )
    })
}

/// Return one AAudio sample format for one runtime format.
pub(super) fn aaudio_sample_format(format: audio_core::AudioSampleFormat) -> Option<c_int> {
    match format {
        audio_core::AudioSampleFormat::S16 => Some(AAUDIO_FORMAT_PCM_I16),
        audio_core::AudioSampleFormat::S24 => Some(AAUDIO_FORMAT_PCM_I24_PACKED),
        audio_core::AudioSampleFormat::S32 => Some(AAUDIO_FORMAT_PCM_I32),
        audio_core::AudioSampleFormat::F32 => Some(AAUDIO_FORMAT_PCM_FLOAT),
        audio_core::AudioSampleFormat::U8 | audio_core::AudioSampleFormat::F64 => None,
    }
}

/// Return one runtime sample format for one AAudio format selector.
pub(super) fn runtime_sample_format(aaudio_format: c_int) -> Option<audio_core::AudioSampleFormat> {
    match aaudio_format {
        AAUDIO_FORMAT_PCM_I16 => Some(audio_core::AudioSampleFormat::S16),
        AAUDIO_FORMAT_PCM_I24_PACKED => Some(audio_core::AudioSampleFormat::S24),
        AAUDIO_FORMAT_PCM_I32 => Some(audio_core::AudioSampleFormat::S32),
        AAUDIO_FORMAT_PCM_FLOAT => Some(audio_core::AudioSampleFormat::F32),
        AAUDIO_FORMAT_UNSPECIFIED => None,
        _ => None,
    }
}

/// Return one conservative AAudio sample-format mask.
pub(super) fn aaudio_format_mask() -> u32 {
    audio_core::sample_format_bit(audio_core::AudioSampleFormat::S16)
        | audio_core::sample_format_bit(audio_core::AudioSampleFormat::S24)
        | audio_core::sample_format_bit(audio_core::AudioSampleFormat::S32)
        | audio_core::sample_format_bit(audio_core::AudioSampleFormat::F32)
}

/// Return one AAudio sharing-mode selector for one runtime share mode.
pub(super) fn aaudio_sharing_mode(mode: audio_core::AudioShareMode) -> c_int {
    match mode {
        audio_core::AudioShareMode::Shared => AAUDIO_SHARING_MODE_SHARED,
        audio_core::AudioShareMode::Exclusive => AAUDIO_SHARING_MODE_EXCLUSIVE,
    }
}

/// Return one AAudio read and write timeout in nanoseconds for one worker period.
pub(super) fn io_timeout_nanoseconds(period: std::time::Duration) -> i64 {
    period.as_nanos().min(i64::MAX as u128) as i64
}

/// Return one pointer to one loaded AAudio library when available.
fn aaudio_library() -> Option<&'static Arc<AAudioLibrary>> {
    AAUDIO_LIBRARY_SLOT
        .get_or_init(load_aaudio_library)
        .as_ref()
}

/// Load one AAudio dynamic library and required symbol table.
fn load_aaudio_library() -> Option<Arc<AAudioLibrary>> {
    // try common AAudio soname candidates in deterministic order
    for candidate in ["libaaudio.so"] {
        let Some(handle) = core_platform::open_dynamic_library(candidate) else {
            continue;
        };

        // resolve all required AAudio symbols
        let api = match load_aaudio_api(handle) {
            Some(api) => api,
            None => {
                core_platform::close_dynamic_library(handle);

                continue;
            }
        };

        return Some(Arc::new(AAudioLibrary { handle, api }));
    }

    None
}

/// Load one AAudio symbol table from one open dynamic-library handle.
fn load_aaudio_api(handle: *mut c_void) -> Option<AAudioApi> {
    Some(AAudioApi {
        create_stream_builder: core_platform::load_dynamic_symbol(
            handle,
            b"AAudio_createStreamBuilder\0",
        )?,
        stream_builder_delete: core_platform::load_dynamic_symbol(
            handle,
            b"AAudioStreamBuilder_delete\0",
        )?,
        stream_builder_set_direction: core_platform::load_dynamic_symbol(
            handle,
            b"AAudioStreamBuilder_setDirection\0",
        )?,
        stream_builder_set_sample_rate: load_symbol(
            handle,
            b"AAudioStreamBuilder_setSampleRate\0",
        )?,
        stream_builder_set_channel_count: load_symbol(
            handle,
            b"AAudioStreamBuilder_setChannelCount\0",
        )?,
        stream_builder_set_format: load_symbol(handle, b"AAudioStreamBuilder_setFormat\0")?,
        stream_builder_set_sharing_mode: load_symbol(
            handle,
            b"AAudioStreamBuilder_setSharingMode\0",
        )?,
        stream_builder_set_performance_mode: load_symbol(
            handle,
            b"AAudioStreamBuilder_setPerformanceMode\0",
        )?,
        stream_builder_set_buffer_capacity_frames: load_symbol(
            handle,
            b"AAudioStreamBuilder_setBufferCapacityInFrames\0",
        )?,
        stream_builder_open_stream: load_symbol(handle, b"AAudioStreamBuilder_openStream\0")?,
        stream_close: load_symbol(handle, b"AAudioStream_close\0")?,
        stream_request_start: load_symbol(handle, b"AAudioStream_requestStart\0")?,
        stream_request_pause: load_symbol(handle, b"AAudioStream_requestPause\0")?,
        stream_request_stop: load_symbol(handle, b"AAudioStream_requestStop\0")?,
        stream_request_flush: load_symbol(handle, b"AAudioStream_requestFlush\0")?,
        stream_read: core_platform::load_dynamic_symbol(handle, b"AAudioStream_read\0")?,
        stream_write: core_platform::load_dynamic_symbol(handle, b"AAudioStream_write\0")?,
        stream_get_sample_rate: core_platform::load_dynamic_symbol(
            handle,
            b"AAudioStream_getSampleRate\0",
        )?,
        stream_get_channel_count: core_platform::load_dynamic_symbol(
            handle,
            b"AAudioStream_getChannelCount\0",
        )?,
        stream_get_frames_per_burst: core_platform::load_dynamic_symbol(
            handle,
            b"AAudioStream_getFramesPerBurst\0",
        )?,
        stream_get_format: core_platform::load_dynamic_symbol(handle, b"AAudioStream_getFormat\0")?,
        stream_get_sharing_mode: core_platform::load_dynamic_symbol(
            handle,
            b"AAudioStream_getSharingMode\0",
        )?,
        convert_result_to_text: core_platform::load_dynamic_symbol(
            handle,
            b"AAudio_convertResultToText\0",
        )?,
    })
}

/// Load one typed symbol from one dynamic-library handle.
fn load_symbol<T>(handle: *mut c_void, name: &[u8]) -> Option<T>
where
    T: Copy,
{
    core_platform::load_dynamic_symbol(handle, name)
}

/// Return one human-readable AAudio status text.
fn aaudio_status_text(status: c_int) -> String {
    let Some(library) = aaudio_library() else {
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
