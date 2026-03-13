use crate::platform::midi::MidiBackend;
use crate::platform::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    AndroidEventResource, AndroidEventSession, AndroidInputResource, AndroidInputSession,
    AndroidOutputResource, AndroidOutputSession,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::AndroidMidi,
    input = (input_resource, AndroidInputResource, AndroidInputSession, "midi input"),
    output = (output_resource, AndroidOutputResource, AndroidOutputSession, "midi output"),
    event = (event_resource, AndroidEventResource, AndroidEventSession, "midi event")
);
