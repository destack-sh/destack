use crate::platform::midi::MidiBackend;
use crate::platform::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    JackEventResource, JackEventSession, JackInputResource, JackInputSession, JackOutputResource,
    JackOutputSession,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::JackMidi,
    input = (input_resource, JackInputResource, JackInputSession, "midi input"),
    output = (output_resource, JackOutputResource, JackOutputSession, "midi output"),
    event = (event_resource, JackEventResource, JackEventSession, "midi event")
);
