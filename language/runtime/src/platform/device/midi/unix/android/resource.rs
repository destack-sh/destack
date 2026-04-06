use crate::platform::device::MidiBackend;
use crate::platform::device::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    AndroidEventRepository, AndroidEventResource, AndroidInputRepository, AndroidInputResource,
    AndroidOutputRepository, AndroidOutputResource,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::AndroidMidi,
    input = (input_resource, AndroidInputResource, AndroidInputRepository, "midi input"),
    output = (output_resource, AndroidOutputResource, AndroidOutputRepository, "midi output"),
    event = (event_resource, AndroidEventResource, AndroidEventRepository, "midi event")
);
