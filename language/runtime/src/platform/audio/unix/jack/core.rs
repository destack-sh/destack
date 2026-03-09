use std::ffi::{CStr, CString, c_char, c_int, c_ulong};
use std::sync::Arc;

use super::abi::JackApi;
use super::constants::*;
#[cfg(target_os = "linux")]
use super::host::jack_available;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, audio as audio_types, core as core_platform};

/// Return whether JACK backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    // linux implementation
    #[cfg(target_os = "linux")]
    {
        jack_available()
    }

    // unsupported host
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// One loaded JACK dynamic library payload.
#[derive(Debug)]
pub(super) struct JackLibrary {
    /// Loaded dynamic-library lifetime owner.
    _library: core_platform::DynamicLibrary,
    /// Loaded JACK function table.
    pub(super) api: JackApi,
}

/// Return one contiguous channel layout enum from one channel count.
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

/// Return one JACK runtime error payload.
pub(super) fn jack_error(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Return one JACK non-supported error payload.
pub(super) fn jack_not_supported(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: {}",
        message.into()
    )))
    .boxed()
}

/// Return whether one JACK status code denotes success.
pub(super) fn jack_succeeded(status: c_int) -> bool {
    status == 0
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

/// Return one loaded JACK library or one not-supported error.
pub(super) fn require_jack_library(operation: &'static str) -> RuntimeResult<Arc<JackLibrary>> {
    load_jack_library().map_err(|error| jack_not_supported(operation, error))
}

/// Return one pointer to the nul-terminated JACK audio type string.
pub(super) fn jack_audio_type_pointer() -> *const c_char {
    JACK_AUDIO_TYPE.as_ptr().cast::<c_char>()
}

/// Load one JACK dynamic library and required symbol table.
fn load_jack_library() -> Result<Arc<JackLibrary>, String> {
    // try common JACK soname candidates in deterministic order
    let (library, api) =
        core_platform::load_library_with_api(&["libjack.so.0", "libjack.so"], load_jack_api)?;

    Ok(Arc::new(JackLibrary {
        _library: library,
        api,
    }))
}

/// Load one JACK symbol table from one open dynamic-library handle.
fn load_jack_api(
    library: &core_platform::DynamicLibrary,
    candidate: &str,
) -> Result<JackApi, String> {
    core_platform::load_dll_api_bytes!(library, candidate, JackApi {
        jack_client_open => b"jack_client_open\0",
        jack_client_close => b"jack_client_close\0",
        jack_activate => b"jack_activate\0",
        jack_deactivate => b"jack_deactivate\0",
        jack_set_process_callback => b"jack_set_process_callback\0",
        jack_set_port_registration_callback => b"jack_set_port_registration_callback\0",
        jack_set_port_connect_callback => b"jack_set_port_connect_callback\0",
        jack_on_shutdown => b"jack_on_shutdown\0",
        jack_port_register => b"jack_port_register\0",
        jack_port_name => b"jack_port_name\0",
        jack_port_get_buffer => b"jack_port_get_buffer\0",
        jack_get_ports => b"jack_get_ports\0",
        jack_connect => b"jack_connect\0",
        jack_get_sample_rate => b"jack_get_sample_rate\0",
        jack_get_buffer_size => b"jack_get_buffer_size\0",
        jack_free => b"jack_free\0",
    })
}

/// Return one opened JACK client name used for probes.
pub(super) fn probe_client_name() -> String {
    format!("destack-probe-{}", std::process::id())
}

/// Return one opened JACK client name used for streams.
pub(super) fn stream_client_name() -> String {
    format!(
        "destack-stream-{}-{}",
        std::process::id(),
        audio_core::host_monotonic_nanos()
    )
}

/// Return one static c-string view from one `char*` pointer.
pub(super) fn c_str_to_string(pointer: *const c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }

    let text = unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned();
    if text.is_empty() {
        return None;
    }

    Some(text)
}

/// Return one JACK option mask for non-invasive runtime clients.
pub(super) fn jack_client_open_options() -> u32 {
    JACK_OPTION_NO_START_SERVER
}

/// Return one JACK port flag mask for physical capture sources.
pub(super) fn capture_source_flags() -> c_ulong {
    JACK_PORT_IS_OUTPUT | JACK_PORT_IS_PHYSICAL
}

/// Return one JACK port flag mask for physical playback sinks.
pub(super) fn playback_sink_flags() -> c_ulong {
    JACK_PORT_IS_INPUT | JACK_PORT_IS_PHYSICAL
}

/// Return one JACK port registration flag mask for client input ports.
pub(super) fn client_input_port_flags() -> c_ulong {
    JACK_PORT_IS_INPUT | JACK_PORT_IS_TERMINAL
}

/// Return one JACK port registration flag mask for client output ports.
pub(super) fn client_output_port_flags() -> c_ulong {
    JACK_PORT_IS_OUTPUT | JACK_PORT_IS_TERMINAL
}
