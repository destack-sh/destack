use std::ffi::{c_ulong, c_void};
use std::ptr;

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

use super::abi::JackClient;
use super::core::{
    JackLibrary, c_str_to_string, c_string, capture_source_flags, jack_audio_type_pointer,
    jack_client_open_options, jack_error, jack_not_supported, playback_sink_flags,
    probe_client_name, require_jack_library,
};

/// One probed JACK host endpoint snapshot.
#[derive(Debug, Clone)]
pub(super) struct JackEndpointSnapshot {
    /// Physical playback sink port names.
    pub(super) playback_sinks: Vec<String>,
    /// Physical capture source port names.
    pub(super) capture_sources: Vec<String>,
    /// JACK graph sample rate in hertz.
    pub(super) sample_rate: u32,
    /// JACK graph period size in frames.
    pub(super) period_frames: u32,
}

/// Return whether JACK endpoint rows are currently available.
pub(super) fn jack_available() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }

    probe_jack_endpoints("destack.audio.device.list").is_ok_and(|snapshot| {
        !snapshot.playback_sinks.is_empty() || !snapshot.capture_sources.is_empty()
    })
}

/// Probe one JACK endpoint snapshot via one short-lived client.
pub(super) fn probe_jack_endpoints(operation: &'static str) -> RuntimeResult<JackEndpointSnapshot> {
    let library = require_jack_library(operation)?;
    let client_name = probe_client_name();
    let client = open_jack_client(&library, operation, &client_name)?;

    // query graph-level sample rate and period size while one probe client is active
    let sample_rate = unsafe { (library.api.jack_get_sample_rate)(client) }.max(1);
    let period_frames = unsafe { (library.api.jack_get_buffer_size)(client) }
        .max(audio_core::MIN_STREAM_PERIOD_FRAMES);

    // enumerate one stable list of physical capture source ports
    let capture_sources = list_ports(&library, client, capture_source_flags(), operation)?;

    // enumerate one stable list of physical playback sink ports
    let playback_sinks = list_ports(&library, client, playback_sink_flags(), operation)?;

    close_jack_client(&library, client);

    Ok(JackEndpointSnapshot {
        playback_sinks,
        capture_sources,
        sample_rate,
        period_frames,
    })
}

/// Open one JACK client for one operation.
pub(super) fn open_jack_client(
    library: &JackLibrary,
    operation: &'static str,
    client_name: &str,
) -> RuntimeResult<*mut JackClient> {
    let client_name = c_string(client_name, "clientName")?;
    let mut status = 0u32;

    // open one non-invasive client that does not auto-start jackd
    let client = unsafe {
        (library.api.jack_client_open)(
            client_name.as_ptr(),
            jack_client_open_options(),
            &mut status,
        )
    };
    if client.is_null() {
        return Err(jack_not_supported(
            operation,
            format!("failed to open JACK client (status 0x{status:08x})"),
        ));
    }

    Ok(client)
}

/// Close one JACK client handle.
pub(super) fn close_jack_client(library: &JackLibrary, client: *mut JackClient) {
    if client.is_null() {
        return;
    }

    // close one JACK client handle when it exists
    unsafe {
        let _ = (library.api.jack_client_close)(client);
    }
}

/// Enumerate one JACK port-name set by one flag selector.
pub(super) fn list_ports(
    library: &JackLibrary,
    client: *mut JackClient,
    flags: c_ulong,
    operation: &'static str,
) -> RuntimeResult<Vec<String>> {
    // query one null-terminated JACK port-name array
    let port_array = unsafe {
        (library.api.jack_get_ports)(client, ptr::null(), jack_audio_type_pointer(), flags)
    };
    if port_array.is_null() {
        return Ok(Vec::new());
    }

    let mut ports = Vec::new();
    let mut index = 0usize;

    // decode one null-terminated C string array into Rust strings
    loop {
        let port_pointer = unsafe { *port_array.add(index) };
        if port_pointer.is_null() {
            break;
        }

        let Some(port_name) = c_str_to_string(port_pointer) else {
            unsafe {
                (library.api.jack_free)(port_array.cast::<c_void>());
            }

            return Err(jack_error(operation, "failed to decode JACK port name"));
        };

        ports.push(port_name);
        index = index.saturating_add(1);
    }

    // release one JACK-allocated array payload
    unsafe {
        (library.api.jack_free)(port_array.cast::<c_void>());
    }

    Ok(ports)
}
