use crate::platform::core::{BackendSupport, backend_support_from_check};
use crate::platform::midi::core::{
    MidiBackendMetadata, midi_backend_metadata, single_midi_backend_metadata,
};
use crate::platform::midi::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS,
    MIDI_BACKEND_CAP_SCHEDULED_OUTPUT, MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_BACKEND_CAP_UMP,
    MIDI_DATA_FORMAT_FLAG_UMP, MIDI_PROTOCOL_FLAG_MIDI1, MIDI_PROTOCOL_FLAG_MIDI2,
};

use super::service::check_windows_midi_support;

/// Advertised selector-row metadata for Windows MIDI Services.
const WINDOWS_MIDI_METADATA: MidiBackendMetadata = midi_backend_metadata(
    crate::platform::midi::MidiBackendCapabilityFlags(
        MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0
            | MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0
            | MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0
            | MIDI_BACKEND_CAP_SCHEDULED_OUTPUT.0
            | MIDI_BACKEND_CAP_UMP.0,
    ),
    crate::platform::midi::MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_UMP.0),
    crate::platform::midi::MidiProtocolFlags(
        MIDI_PROTOCOL_FLAG_MIDI1.0 | MIDI_PROTOCOL_FLAG_MIDI2.0,
    ),
);

/// Return current Windows MIDI Services host support.
pub(crate) fn backend_support() -> BackendSupport {
    backend_support_from_check(
        "windows-midi",
        check_windows_midi_support("destack.midi.backend.support"),
    )
}

/// Return selector-row metadata for Windows MIDI Services.
pub(crate) fn backend_metadata(support: BackendSupport) -> MidiBackendMetadata {
    single_midi_backend_metadata(
        crate::platform::midi::MidiBackend::WindowsMidi,
        crate::platform::midi::MidiBackend::WindowsMidi,
        support,
        WINDOWS_MIDI_METADATA,
    )
}
