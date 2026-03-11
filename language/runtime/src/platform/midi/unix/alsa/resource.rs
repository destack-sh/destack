use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::midi::core::{read_labeled_resource_payload, remove_labeled_resource};
use crate::platform::resource;
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::runtime::BindingCallContext;

use super::core::{
    AlsaEventResource, AlsaEventSession, AlsaInputResource, AlsaInputSession, AlsaOutputResource,
    AlsaOutputSession, MIDI_EVENT_RESOURCE_LABEL, MIDI_INPUT_RESOURCE_LABEL,
    MIDI_OUTPUT_RESOURCE_LABEL,
};

/// Return one shared ALSA input session from one opened resource handle.
pub(super) fn input_resource(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AlsaInputSession>> {
    read_labeled_resource_payload(
        binding,
        handle.0,
        ResourceKind::MidiInputPort,
        MIDI_INPUT_RESOURCE_LABEL,
        operation,
        "midi input",
        |entry: &ResourceEntry| {
            entry
                .payload_ref::<AlsaInputResource>()
                .map(|payload| payload.session.clone())
        },
    )
}

/// Return one shared ALSA output session from one opened resource handle.
pub(super) fn output_resource(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<AlsaOutputSession>> {
    read_labeled_resource_payload(
        binding,
        handle.0,
        ResourceKind::MidiOutputPort,
        MIDI_OUTPUT_RESOURCE_LABEL,
        operation,
        "midi output",
        |entry: &ResourceEntry| {
            entry
                .payload_ref::<AlsaOutputResource>()
                .map(|payload| payload.session.clone())
        },
    )
}

/// Return one shared ALSA event session from one opened resource handle.
pub(super) fn event_resource(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<AlsaEventSession>>> {
    read_labeled_resource_payload(
        binding,
        handle.0,
        ResourceKind::MidiEvent,
        MIDI_EVENT_RESOURCE_LABEL,
        operation,
        "midi event",
        |entry: &ResourceEntry| {
            entry
                .payload_ref::<AlsaEventResource>()
                .map(|payload| payload.session.clone())
        },
    )
}

/// Remove one ALSA input handle.
pub(super) fn remove_input_resource(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    remove_labeled_resource(binding, handle.0, operation, "midi input")
}

/// Remove one ALSA output handle.
pub(super) fn remove_output_resource(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    remove_labeled_resource(binding, handle.0, operation, "midi output")
}

/// Remove one ALSA event handle.
pub(super) fn remove_event_resource(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    remove_labeled_resource(binding, handle.0, operation, "midi event")
}
