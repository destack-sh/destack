use crate::diagnostic::RuntimeResult;
use crate::platform::core::{BackendSupport, backend_support_from_check};
use crate::platform::midi::core::{
    MidiBackendDescriptorValue, MidiBackendMetadata, midi_backend_descriptors,
    midi_backend_metadata, midi_selector_support, resolve_midi_backend_selection,
    single_midi_backend_metadata,
};
use crate::platform::midi::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS,
    MIDI_BACKEND_CAP_SCHEDULED_OUTPUT, MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_BACKEND_CAP_UMP,
    MIDI_BACKEND_CAP_VIRTUAL_INPUT, MIDI_BACKEND_CAP_VIRTUAL_OUTPUT,
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_DATA_FORMAT_FLAG_UMP, MIDI_PROTOCOL_FLAG_MIDI1,
    MIDI_PROTOCOL_FLAG_MIDI2, MidiBackend, MidiBackendSelectionPolicy,
};
use crate::runtime::BindingCallContext;

use super::service::check_core_midi_support;

/// Host backend selectors considered by auto selection on macOS.
const PREFERRED_HOST_BACKENDS: [MidiBackend; 1] = [MidiBackend::CoreMIDI];

/// Advertised selector-row metadata for CoreMIDI.
const CORE_MIDI_METADATA: MidiBackendMetadata = midi_backend_metadata(
    crate::platform::midi::MidiBackendCapabilityFlags(
        MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0
            | MIDI_BACKEND_CAP_VIRTUAL_INPUT.0
            | MIDI_BACKEND_CAP_VIRTUAL_OUTPUT.0
            | MIDI_BACKEND_CAP_SCHEDULED_OUTPUT.0
            | MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0
            | MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0
            | MIDI_BACKEND_CAP_UMP.0,
    ),
    crate::platform::midi::MidiDataFormatFlags(
        MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0 | MIDI_DATA_FORMAT_FLAG_UMP.0,
    ),
    crate::platform::midi::MidiProtocolFlags(
        MIDI_PROTOCOL_FLAG_MIDI1.0 | MIDI_PROTOCOL_FLAG_MIDI2.0,
    ),
);

/// Return current CoreMIDI host support.
fn core_midi_backend_support() -> BackendSupport {
    backend_support_from_check(
        "coremidi",
        check_core_midi_support("destack.midi.backend.support"),
    )
}

/// Return host support for one backend selector on macOS.
pub(super) fn backend_support(backend: MidiBackend) -> BackendSupport {
    midi_selector_support(&PREFERRED_HOST_BACKENDS, backend, |backend| match backend {
        MidiBackend::CoreMIDI => Some(core_midi_backend_support()),
        _ => None,
    })
}

/// Resolve one effective host backend.
pub(super) fn resolve_backend(
    backend: MidiBackend,
    policy: MidiBackendSelectionPolicy,
    operation: &'static str,
) -> RuntimeResult<MidiBackend> {
    resolve_midi_backend_selection(
        &PREFERRED_HOST_BACKENDS,
        backend,
        policy,
        operation,
        backend_support,
    )
}

/// Return selector-row metadata for one CoreMIDI selector row.
fn backend_metadata(backend: MidiBackend, support: BackendSupport) -> MidiBackendMetadata {
    single_midi_backend_metadata(backend, MidiBackend::CoreMIDI, support, CORE_MIDI_METADATA)
}

/// Build backend descriptors for the CoreMIDI host.
pub(crate) fn midi_backend_list(
    _binding: &BindingCallContext,
) -> RuntimeResult<Vec<MidiBackendDescriptorValue>> {
    Ok(midi_backend_descriptors(
        &PREFERRED_HOST_BACKENDS,
        backend_support,
        backend_metadata,
    ))
}
