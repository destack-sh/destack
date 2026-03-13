use crate::platform::core::{BackendSupport, backend_support_from_check};
use crate::platform::midi::core::{
    MidiBackendMetadata, midi_backend_metadata, single_midi_backend_metadata,
};
use crate::platform::midi::{
    MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS, MIDI_BACKEND_CAP_TOPOLOGY_EVENTS,
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PROTOCOL_FLAG_MIDI1,
};

use super::service::check_winmm_support;

/// Advertised selector-row metadata for WinMM.
const WINMM_METADATA: MidiBackendMetadata = midi_backend_metadata(
    crate::platform::midi::MidiBackendCapabilityFlags(
        MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0 | MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0,
    ),
    crate::platform::midi::MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0),
    crate::platform::midi::MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0),
);

/// Return current WinMM host support.
pub(crate) fn backend_support() -> BackendSupport {
    backend_support_from_check("winmm", check_winmm_support("destack.midi.backend.support"))
}

/// Return selector-row metadata for WinMM.
pub(crate) fn backend_metadata(support: BackendSupport) -> MidiBackendMetadata {
    single_midi_backend_metadata(
        crate::platform::midi::MidiBackend::WinMM,
        crate::platform::midi::MidiBackend::WinMM,
        support,
        WINMM_METADATA,
    )
}
