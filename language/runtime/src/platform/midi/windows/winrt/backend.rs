use crate::platform::core::{BackendSupport, backend_support_from_check};
use crate::platform::midi::core::{
    MidiBackendMetadata, midi_backend_metadata, single_midi_backend_metadata,
};
use crate::platform::midi::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS,
    MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PROTOCOL_FLAG_MIDI1,
};

use super::service::check_winrt_support;

/// Advertised selector-row metadata for WinRT MIDI.
const WINRT_METADATA: MidiBackendMetadata = midi_backend_metadata(
    crate::platform::midi::MidiBackendCapabilityFlags(
        MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0
            | MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0
            | MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0,
    ),
    crate::platform::midi::MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0),
    crate::platform::midi::MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0),
);

/// Return current WinRT host support.
pub(crate) fn backend_support() -> BackendSupport {
    backend_support_from_check("winrt", check_winrt_support("destack.midi.backend.support"))
}

/// Return selector-row metadata for WinRT MIDI.
pub(crate) fn backend_metadata(support: BackendSupport) -> MidiBackendMetadata {
    single_midi_backend_metadata(
        crate::platform::midi::MidiBackend::WinRT,
        crate::platform::midi::MidiBackend::WinRT,
        support,
        WINRT_METADATA,
    )
}
