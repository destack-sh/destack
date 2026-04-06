use crate::platform::device::MidiBackend;
use crate::platform::device::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    WindowsMidiEventRepository, WindowsMidiEventResource, WindowsMidiInputRepository,
    WindowsMidiInputResource, WindowsMidiOutputRepository, WindowsMidiOutputResource,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::WindowsMidi,
    input = (
        input_resource,
        WindowsMidiInputResource,
        WindowsMidiInputRepository,
        "midi input port"
    ),
    output = (
        output_resource,
        WindowsMidiOutputResource,
        WindowsMidiOutputRepository,
        "midi output port"
    ),
    event = (
        event_resource,
        WindowsMidiEventResource,
        WindowsMidiEventRepository,
        "midi event"
    )
);
