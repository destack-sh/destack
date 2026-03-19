use crate::platform::device::MidiBackend;
use crate::platform::device::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    WindowsMidiEventResource, WindowsMidiEventSession, WindowsMidiInputResource,
    WindowsMidiInputSession, WindowsMidiOutputResource, WindowsMidiOutputSession,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::WindowsMidi,
    input = (
        input_resource,
        WindowsMidiInputResource,
        WindowsMidiInputSession,
        "midi input port"
    ),
    output = (
        output_resource,
        WindowsMidiOutputResource,
        WindowsMidiOutputSession,
        "midi output port"
    ),
    event = (
        event_resource,
        WindowsMidiEventResource,
        WindowsMidiEventSession,
        "midi event"
    )
);
