use crate::platform::device::MidiBackend;
use crate::platform::device::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    JackEventRepository, JackEventResource, JackInputRepository, JackInputResource,
    JackOutputRepository, JackOutputResource,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::JackMidi,
    input = (input_resource, JackInputResource, JackInputRepository, "midi input"),
    output = (output_resource, JackOutputResource, JackOutputRepository, "midi output"),
    event = (event_resource, JackEventResource, JackEventRepository, "midi event")
);
