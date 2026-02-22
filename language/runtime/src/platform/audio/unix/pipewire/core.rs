use std::ffi::{CStr, CString, c_int, c_void};
use std::process::Command;
use std::ptr;
use std::sync::{Arc, OnceLock};

use super::abi::{PipeWireApi, PipewireSampleSpec};
use super::constants::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};

/// Return whether PipeWire backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    cfg!(target_os = "linux") && pipewire_library().is_some() && pipewire_server_available()
}

/// One process-global PipeWire dynamic library slot.
static PIPEWIRE_LIBRARY_SLOT: OnceLock<Option<Arc<PipeWireLibrary>>> = OnceLock::new();

/// One loaded PipeWire dynamic library payload.
#[derive(Debug)]
pub(super) struct PipeWireLibrary {
    /// Raw dynamic-library handle from `dlopen`.
    handle: *mut c_void,
    /// Loaded PipeWire function table.
    pub(super) api: PipeWireApi,
}

impl Drop for PipeWireLibrary {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }

        // close one open dynamic-library handle
        core_platform::close_dynamic_library(self.handle);
    }
}

unsafe impl Send for PipeWireLibrary {}
unsafe impl Sync for PipeWireLibrary {}

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

/// Return one PipeWire runtime error payload.
pub(super) fn pipewire_error(
    operation: &'static str,
    code: Option<c_int>,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    let message = message.into();
    let message = if let Some(code) = code {
        format!(
            "{message} (pulse error {code}: {})",
            pipewire_error_text(code)
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

/// Return one PipeWire non-supported error payload.
pub(super) fn pipewire_not_supported(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: {}",
        message.into()
    )))
    .boxed()
}

/// Return whether one PipeWire status code denotes success.
pub(super) fn pipewire_succeeded(status: c_int) -> bool {
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

/// Return one loaded PipeWire library or one not-supported error.
pub(super) fn require_pipewire_library(
    operation: &'static str,
) -> RuntimeResult<&'static Arc<PipeWireLibrary>> {
    pipewire_library().ok_or_else(|| {
        pipewire_not_supported(
            operation,
            "PipeWire dynamic library is unavailable on this host",
        )
    })
}

/// Return one PipeWire sample format selector for one runtime sample format.
pub(super) fn pipewire_sample_format(format: audio_core::AudioSampleFormat) -> Option<c_int> {
    match format {
        audio_core::AudioSampleFormat::U8 => Some(PIPEWIRE_SAMPLE_U8),
        audio_core::AudioSampleFormat::S16 => Some(if cfg!(target_endian = "little") {
            PIPEWIRE_SAMPLE_S16LE
        } else {
            PIPEWIRE_SAMPLE_S16BE
        }),
        audio_core::AudioSampleFormat::S24 => Some(if cfg!(target_endian = "little") {
            PIPEWIRE_SAMPLE_S24_32LE
        } else {
            PIPEWIRE_SAMPLE_S24_32BE
        }),
        audio_core::AudioSampleFormat::S32 => Some(if cfg!(target_endian = "little") {
            PIPEWIRE_SAMPLE_S32LE
        } else {
            PIPEWIRE_SAMPLE_S32BE
        }),
        audio_core::AudioSampleFormat::F32 => Some(if cfg!(target_endian = "little") {
            PIPEWIRE_SAMPLE_F32LE
        } else {
            PIPEWIRE_SAMPLE_F32BE
        }),
        audio_core::AudioSampleFormat::F64 => None,
    }
}

/// Return one conservative PipeWire sample-format mask.
pub(super) fn pipewire_format_mask() -> u32 {
    audio_core::sample_format_bit(audio_core::AudioSampleFormat::U8)
        | audio_core::sample_format_bit(audio_core::AudioSampleFormat::S16)
        | audio_core::sample_format_bit(audio_core::AudioSampleFormat::S24)
        | audio_core::sample_format_bit(audio_core::AudioSampleFormat::S32)
        | audio_core::sample_format_bit(audio_core::AudioSampleFormat::F32)
}

/// Return one pointer to one loaded PipeWire library when available.
fn pipewire_library() -> Option<&'static Arc<PipeWireLibrary>> {
    PIPEWIRE_LIBRARY_SLOT
        .get_or_init(load_pipewire_library)
        .as_ref()
}

/// Load one PipeWire dynamic library and required symbol table.
fn load_pipewire_library() -> Option<Arc<PipeWireLibrary>> {
    // try common PipeWire simple-api soname candidates in deterministic order
    for candidate in ["libpulse-simple.so.0", "libpulse-simple.so"] {
        let Some(handle) = core_platform::open_dynamic_library(candidate) else {
            continue;
        };

        // resolve all required PipeWire simple symbols
        let api = match load_pipewire_api(handle) {
            Some(api) => api,
            None => {
                core_platform::close_dynamic_library(handle);

                continue;
            }
        };

        return Some(Arc::new(PipeWireLibrary { handle, api }));
    }

    None
}

/// Load one PipeWire symbol table from one open dynamic-library handle.
fn load_pipewire_api(handle: *mut c_void) -> Option<PipeWireApi> {
    Some(PipeWireApi {
        pa_simple_new: core_platform::load_dynamic_symbol(handle, b"pa_simple_new\0")?,
        pa_simple_free: core_platform::load_dynamic_symbol(handle, b"pa_simple_free\0")?,
        pa_simple_read: core_platform::load_dynamic_symbol(handle, b"pa_simple_read\0")?,
        pa_simple_write: core_platform::load_dynamic_symbol(handle, b"pa_simple_write\0")?,
        pa_simple_flush: core_platform::load_dynamic_symbol(handle, b"pa_simple_flush\0")?,
        pa_simple_drain: core_platform::load_dynamic_symbol(handle, b"pa_simple_drain\0")?,
        pa_strerror: core_platform::load_dynamic_symbol(handle, b"pa_strerror\0")?,
    })
}

/// Return one human-readable PipeWire error string.
fn pipewire_error_text(code: c_int) -> String {
    let Some(library) = pipewire_library() else {
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

/// Return whether one PipeWire server is reachable from this process.
fn pipewire_server_available() -> bool {
    if !pipewire_pulse_server_detected() {
        return false;
    }

    let Ok(library) = require_pipewire_library("destack.audio.device.list") else {
        return false;
    };

    let Ok(application_name) = c_string(PIPEWIRE_APPLICATION_NAME, "applicationName") else {
        return false;
    };
    let Ok(stream_name) = c_string(PIPEWIRE_STREAM_NAME, "streamName") else {
        return false;
    };

    let sample_spec = PipewireSampleSpec {
        format: if cfg!(target_endian = "little") {
            PIPEWIRE_SAMPLE_S16LE
        } else {
            PIPEWIRE_SAMPLE_S16BE
        },
        rate: PIPEWIRE_PREFERRED_SAMPLE_RATE,
        channels: 2,
    };

    let mut error = 0;

    // probe one short-lived playback stream to validate server reachability
    let stream = unsafe {
        (library.api.pa_simple_new)(
            ptr::null(),
            application_name.as_ptr(),
            PIPEWIRE_STREAM_DIRECTION_PLAYBACK,
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

/// Return whether `pactl info` reports one PipeWire-backed pulse server.
fn pipewire_pulse_server_detected() -> bool {
    let output = Command::new("pactl").arg("info").output();
    let Ok(output) = output else {
        return false;
    };

    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
    stdout.contains("pipewire")
}
