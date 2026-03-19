use crate::platform::device::MidiBackend;
use crate::platform::device::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    AlsaEventResource, AlsaEventSession, AlsaInputResource, AlsaInputSession, AlsaOutputResource,
    AlsaOutputSession,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::Alsa,
    input = (input_resource, AlsaInputResource, AlsaInputSession, "midi input"),
    output = (output_resource, AlsaOutputResource, AlsaOutputSession, "midi output"),
    event = (event_resource, AlsaEventResource, AlsaEventSession, "midi event")
);
