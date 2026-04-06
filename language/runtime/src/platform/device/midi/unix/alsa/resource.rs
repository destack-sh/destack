use crate::platform::device::MidiBackend;
use crate::platform::device::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    AlsaEventRepository, AlsaEventResource, AlsaInputRepository, AlsaInputResource,
    AlsaOutputRepository, AlsaOutputResource,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::Alsa,
    input = (input_resource, AlsaInputResource, AlsaInputRepository, "midi input"),
    output = (output_resource, AlsaOutputResource, AlsaOutputRepository, "midi output"),
    event = (event_resource, AlsaEventResource, AlsaEventRepository, "midi event")
);
