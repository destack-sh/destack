use crate::platform::core::{BackendSupport, backend_support_from_check};
use crate::platform::device::midi::core::{
    MidiBackendMetadata, midi_backend_metadata, single_midi_backend_metadata,
};
use crate::platform::device::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS,
    MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_BACKEND_CAP_VIRTUAL_INPUT,
    MIDI_BACKEND_CAP_VIRTUAL_OUTPUT, MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PROTOCOL_FLAG_MIDI1,
    MidiBackend,
};

use super::core::check_jack_support;

/// Advertised selector-row metadata for JACK MIDI.
const JACK_METADATA: MidiBackendMetadata = midi_backend_metadata(
    crate::platform::device::MidiBackendCapabilityFlags(
        MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0
            | MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0
            | MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0
            | MIDI_BACKEND_CAP_VIRTUAL_INPUT.0
            | MIDI_BACKEND_CAP_VIRTUAL_OUTPUT.0,
    ),
    crate::platform::device::MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0),
    crate::platform::device::MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0),
);

/// Return current JACK host support.
pub(crate) fn backend_support() -> BackendSupport {
    backend_support_from_check(
        "jack-midi",
        check_jack_support("destack.device.midi.backend.support"),
    )
}

/// Return selector-row metadata for JACK MIDI.
pub(crate) fn backend_metadata(support: BackendSupport) -> MidiBackendMetadata {
    single_midi_backend_metadata(
        MidiBackend::JackMidi,
        MidiBackend::JackMidi,
        support,
        JACK_METADATA,
    )
}
