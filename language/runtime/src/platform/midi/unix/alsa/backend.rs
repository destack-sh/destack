use super::ffi::alsa_library;
use crate::platform::core::{self as core_platform, BackendSupport, backend_support_from_check};
use crate::platform::midi::core::{
    MidiBackendMetadata, midi_backend_metadata, midi_selector_support, single_midi_backend_metadata,
};
use crate::platform::midi::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS,
    MIDI_BACKEND_CAP_SCHEDULED_OUTPUT, MIDI_BACKEND_CAP_TOPOLOGY_EVENTS,
    MIDI_BACKEND_CAP_VIRTUAL_INPUT, MIDI_BACKEND_CAP_VIRTUAL_OUTPUT,
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PROTOCOL_FLAG_MIDI1, MidiBackend,
};

/// Host backend selectors considered by Linux auto selection.
const PREFERRED_HOST_BACKENDS: [MidiBackend; 2] = [MidiBackend::Alsa, MidiBackend::JackMidi];

/// Advertised selector-row metadata for ALSA sequencer MIDI.
const ALSA_METADATA: MidiBackendMetadata = midi_backend_metadata(
    crate::platform::midi::MidiBackendCapabilityFlags(
        MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0
            | MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0
            | MIDI_BACKEND_CAP_VIRTUAL_INPUT.0
            | MIDI_BACKEND_CAP_VIRTUAL_OUTPUT.0
            | MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0
            | MIDI_BACKEND_CAP_SCHEDULED_OUTPUT.0,
    ),
    crate::platform::midi::MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0),
    crate::platform::midi::MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0),
);

/// Return host support for one backend selector on Linux.
pub(crate) fn backend_support(backend: MidiBackend) -> BackendSupport {
    midi_selector_support(&PREFERRED_HOST_BACKENDS, backend, |backend| match backend {
        MidiBackend::Alsa => Some(alsa_backend_support()),
        MidiBackend::JackMidi => Some(BackendSupport::DisabledByBuild),
        _ => None,
    })
}

/// Return current ALSA sequencer host support.
fn alsa_backend_support() -> BackendSupport {
    backend_support_from_check(
        "alsa",
        match alsa_library() {
            Some(_library) => Ok(()),
            None => Err(core_platform::backend_support_error(
                "destack.midi.backend.support",
                "alsa",
                BackendSupport::HostUnavailable,
            )),
        },
    )
}

/// Return selector-row metadata for one ALSA selector row.
pub(crate) fn backend_metadata(
    backend: MidiBackend,
    support: BackendSupport,
) -> MidiBackendMetadata {
    single_midi_backend_metadata(backend, MidiBackend::Alsa, support, ALSA_METADATA)
}
