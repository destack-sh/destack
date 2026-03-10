use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::resource;
use crate::platform::resource::ResourceKind;
use crate::runtime::BindingCallContext;

use super::core::{
    CoreMidiEventResource, CoreMidiEventSession, CoreMidiInputResource, CoreMidiInputSession,
    CoreMidiOutputResource, CoreMidiOutputSession, MIDI_EVENT_RESOURCE_LABEL,
    MIDI_INPUT_RESOURCE_LABEL, MIDI_OUTPUT_RESOURCE_LABEL,
};
use crate::platform::midi::shared::read_labeled_resource_payload;

/// Read one CoreMIDI input resource payload.
pub(super) fn input_resource(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<CoreMidiInputSession>> {
    read_labeled_resource_payload(
        binding,
        handle.0,
        ResourceKind::MidiInputPort,
        MIDI_INPUT_RESOURCE_LABEL,
        operation,
        "midi input port",
        |entry| {
            entry
                .payload_ref::<CoreMidiInputResource>()
                .map(|resource| resource.session.clone())
        },
    )
}

/// Read one CoreMIDI output resource payload.
pub(super) fn output_resource(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<CoreMidiOutputSession>> {
    read_labeled_resource_payload(
        binding,
        handle.0,
        ResourceKind::MidiOutputPort,
        MIDI_OUTPUT_RESOURCE_LABEL,
        operation,
        "midi output port",
        |entry| {
            entry
                .payload_ref::<CoreMidiOutputResource>()
                .map(|resource| resource.session.clone())
        },
    )
}

/// Read one CoreMIDI event resource payload.
pub(super) fn event_resource(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CoreMidiEventSession>>> {
    read_labeled_resource_payload(
        binding,
        handle.0,
        ResourceKind::MidiEvent,
        MIDI_EVENT_RESOURCE_LABEL,
        operation,
        "midi event",
        |entry| {
            entry
                .payload_ref::<CoreMidiEventResource>()
                .map(|resource| resource.session.clone())
        },
    )
}
