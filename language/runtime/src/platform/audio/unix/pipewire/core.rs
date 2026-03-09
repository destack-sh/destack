use std::ffi::{CStr, CString, c_int};
use std::process::Command;
use std::ptr;
use std::sync::Arc;

use super::abi::{PipeWireApi, PipewireSampleSpec};
use super::constants::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core::codec::sample_format_bit;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, audio as audio_types, core as core_platform};

/// Return whether PipeWire backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }

    // probe one library load before probing server reachability
    let Ok(library) = load_pipewire_library() else {
        return false;
    };

    pipewire_server_available(library.as_ref())
}

/// One loaded PipeWire dynamic library payload.
#[derive(Debug)]
pub(super) struct PipeWireLibrary {
    /// Loaded dynamic-library lifetime owner.
    _library: core_platform::DynamicLibrary,
    /// Loaded PipeWire function table.
    pub(super) api: PipeWireApi,
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
) -> RuntimeResult<Arc<PipeWireLibrary>> {
    load_pipewire_library().map_err(|error| pipewire_not_supported(operation, error))
}

/// Return one PipeWire sample format selector for one runtime sample format.
pub(super) fn pipewire_sample_format(format: audio_types::AudioSampleFormat) -> Option<c_int> {
    match format {
        audio_types::AudioSampleFormat::U8 => Some(PIPEWIRE_SAMPLE_U8),
        audio_types::AudioSampleFormat::S16 => Some(if cfg!(target_endian = "little") {
            PIPEWIRE_SAMPLE_S16LE
        } else {
            PIPEWIRE_SAMPLE_S16BE
        }),
        audio_types::AudioSampleFormat::S24 => Some(if cfg!(target_endian = "little") {
            PIPEWIRE_SAMPLE_S24_32LE
        } else {
            PIPEWIRE_SAMPLE_S24_32BE
        }),
        audio_types::AudioSampleFormat::S32 => Some(if cfg!(target_endian = "little") {
            PIPEWIRE_SAMPLE_S32LE
        } else {
            PIPEWIRE_SAMPLE_S32BE
        }),
        audio_types::AudioSampleFormat::F32 => Some(if cfg!(target_endian = "little") {
            PIPEWIRE_SAMPLE_F32LE
        } else {
            PIPEWIRE_SAMPLE_F32BE
        }),
        audio_types::AudioSampleFormat::F64 => None,
    }
}

/// Return one conservative PipeWire sample-format mask.
pub(super) fn pipewire_format_mask() -> u32 {
    sample_format_bit(audio_types::AudioSampleFormat::U8)
        | sample_format_bit(audio_types::AudioSampleFormat::S16)
        | sample_format_bit(audio_types::AudioSampleFormat::S24)
        | sample_format_bit(audio_types::AudioSampleFormat::S32)
        | sample_format_bit(audio_types::AudioSampleFormat::F32)
}

/// Load one PipeWire dynamic library and required symbol table.
fn load_pipewire_library() -> Result<Arc<PipeWireLibrary>, String> {
    // try common PipeWire simple-api soname candidates in deterministic order
    let (library, api) = core_platform::load_library_with_api(
        &["libpulse-simple.so.0", "libpulse-simple.so"],
        load_pipewire_api,
    )?;

    Ok(Arc::new(PipeWireLibrary {
        _library: library,
        api,
    }))
}

/// Load one PipeWire symbol table from one open dynamic-library handle.
fn load_pipewire_api(
    library: &core_platform::DynamicLibrary,
    candidate: &str,
) -> Result<PipeWireApi, String> {
    core_platform::load_dll_api_bytes!(library, candidate, PipeWireApi {
        pa_simple_new => b"pa_simple_new\0",
        pa_simple_free => b"pa_simple_free\0",
        pa_simple_read => b"pa_simple_read\0",
        pa_simple_write => b"pa_simple_write\0",
        pa_simple_flush => b"pa_simple_flush\0",
        pa_simple_drain => b"pa_simple_drain\0",
        pa_strerror => b"pa_strerror\0",
    })
}

/// Return one human-readable PipeWire error string.
fn pipewire_error_text(code: c_int) -> String {
    let Ok(library) = load_pipewire_library() else {
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
fn pipewire_server_available(library: &PipeWireLibrary) -> bool {
    if !pipewire_pulse_server_detected() {
        return false;
    }

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
