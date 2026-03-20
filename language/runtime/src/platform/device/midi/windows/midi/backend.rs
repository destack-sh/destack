use crate::platform::core::{BackendSupport, backend_support_from_check};
use crate::platform::device::midi::core::{
    MidiBackendMetadata, midi_backend_metadata, single_midi_backend_metadata,
};
use crate::platform::device::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS,
    MIDI_BACKEND_CAP_SCHEDULED_OUTPUT, MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_BACKEND_CAP_UMP,
    MIDI_BACKEND_CAP_VIRTUAL_INPUT, MIDI_BACKEND_CAP_VIRTUAL_OUTPUT, MIDI_DATA_FORMAT_FLAG_UMP,
    MIDI_PROTOCOL_FLAG_MIDI1, MIDI_PROTOCOL_FLAG_MIDI2,
};

use super::service::{check_windows_midi_support, windows_midi_virtual_transport_available};

/// Advertised selector-row metadata for Windows MIDI Services.
const WINDOWS_MIDI_METADATA: MidiBackendMetadata = midi_backend_metadata(
    crate::platform::device::MidiBackendCapabilityFlags(
        MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0
            | MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0
            | MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0
            | MIDI_BACKEND_CAP_SCHEDULED_OUTPUT.0
            | MIDI_BACKEND_CAP_UMP.0,
    ),
    crate::platform::device::MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_UMP.0),
    crate::platform::device::MidiProtocolFlags(
        MIDI_PROTOCOL_FLAG_MIDI1.0 | MIDI_PROTOCOL_FLAG_MIDI2.0,
    ),
);

/// Return current Windows MIDI Services host support.
pub(crate) fn backend_support() -> BackendSupport {
    backend_support_from_check(
        "windows-midi",
        check_windows_midi_support("destack.device.midi.backend.support"),
    )
}

/// Return selector-row metadata for Windows MIDI Services.
pub(crate) fn backend_metadata(support: BackendSupport) -> MidiBackendMetadata {
    let virtual_transport_available = support.is_available()
        && windows_midi_virtual_transport_available("destack.device.midi.backend.list")
            .unwrap_or(false);
    let capability_flags = if virtual_transport_available {
        crate::platform::device::MidiBackendCapabilityFlags(
            WINDOWS_MIDI_METADATA.capability_flags.0
                | MIDI_BACKEND_CAP_VIRTUAL_INPUT.0
                | MIDI_BACKEND_CAP_VIRTUAL_OUTPUT.0,
        )
    } else {
        WINDOWS_MIDI_METADATA.capability_flags
    };
    let metadata = midi_backend_metadata(
        capability_flags,
        WINDOWS_MIDI_METADATA.supported_data_formats,
        WINDOWS_MIDI_METADATA.supported_protocols,
    );

    single_midi_backend_metadata(
        crate::platform::device::MidiBackend::WindowsMidi,
        crate::platform::device::MidiBackend::WindowsMidi,
        support,
        metadata,
    )
}
