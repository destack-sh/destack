use std::sync::Arc;

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::resource;
use crate::platform::resource::ResourceKind;
use crate::runtime::BindingCallContext;

use super::core::{
    MIDI_EVENT_RESOURCE_LABEL, MIDI_INPUT_RESOURCE_LABEL, MIDI_OUTPUT_RESOURCE_LABEL,
    WinRtEventResource, WinRtEventSession, WinRtInputResource, WinRtInputSession,
    WinRtOutputResource, WinRtOutputSession,
};
use crate::platform::midi::core::read_labeled_resource_payload;

/// Read one WinRT input resource payload.
pub(super) fn input_resource(
    binding: &BindingCallContext,
    handle: resource::MidiInputPortHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<WinRtInputSession>> {
    read_labeled_resource_payload(
        binding,
        handle.0,
        ResourceKind::MidiInputPort,
        MIDI_INPUT_RESOURCE_LABEL,
        operation,
        "midi input port",
        |entry| {
            entry
                .payload_ref::<WinRtInputResource>()
                .map(|resource| resource.session.clone())
        },
    )
}

/// Read one WinRT output resource payload.
pub(super) fn output_resource(
    binding: &BindingCallContext,
    handle: resource::MidiOutputPortHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<WinRtOutputSession>> {
    read_labeled_resource_payload(
        binding,
        handle.0,
        ResourceKind::MidiOutputPort,
        MIDI_OUTPUT_RESOURCE_LABEL,
        operation,
        "midi output port",
        |entry| {
            entry
                .payload_ref::<WinRtOutputResource>()
                .map(|resource| resource.session.clone())
        },
    )
}

/// Read one WinRT event resource payload.
pub(super) fn event_resource(
    binding: &BindingCallContext,
    handle: resource::MidiEventHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<WinRtEventSession>>> {
    read_labeled_resource_payload(
        binding,
        handle.0,
        ResourceKind::MidiEvent,
        MIDI_EVENT_RESOURCE_LABEL,
        operation,
        "midi event",
        |entry| {
            entry
                .payload_ref::<WinRtEventResource>()
                .map(|resource| resource.session.clone())
        },
    )
}
