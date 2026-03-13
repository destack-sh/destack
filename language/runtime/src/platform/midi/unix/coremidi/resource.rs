use crate::platform::midi::MidiBackend;
use crate::platform::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    CoreMidiEventResource, CoreMidiEventSession, CoreMidiInputResource, CoreMidiInputSession,
    CoreMidiOutputResource, CoreMidiOutputSession,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::CoreMIDI,
    input = (
        input_resource,
        CoreMidiInputResource,
        CoreMidiInputSession,
        "midi input port"
    ),
    output = (
        output_resource,
        CoreMidiOutputResource,
        CoreMidiOutputSession,
        "midi output port"
    ),
    event = (
        event_resource,
        CoreMidiEventResource,
        CoreMidiEventSession,
        "midi event"
    )
);
