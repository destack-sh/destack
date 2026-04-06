use crate::platform::device::MidiBackend;
use crate::platform::device::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    CoreMidiEventRepository, CoreMidiEventResource, CoreMidiInputRepository, CoreMidiInputResource,
    CoreMidiOutputRepository, CoreMidiOutputResource,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::CoreMIDI,
    input = (
        input_resource,
        CoreMidiInputResource,
        CoreMidiInputRepository,
        "midi input port"
    ),
    output = (
        output_resource,
        CoreMidiOutputResource,
        CoreMidiOutputRepository,
        "midi output port"
    ),
    event = (
        event_resource,
        CoreMidiEventResource,
        CoreMidiEventRepository,
        "midi event"
    )
);
