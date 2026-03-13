use crate::platform::midi::MidiBackend;
use crate::platform::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    WinRtEventResource, WinRtEventSession, WinRtInputResource, WinRtInputSession,
    WinRtOutputResource, WinRtOutputSession,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::WinRT,
    input = (input_resource, WinRtInputResource, WinRtInputSession, "midi input port"),
    output = (
        output_resource,
        WinRtOutputResource,
        WinRtOutputSession,
        "midi output port"
    ),
    event = (event_resource, WinRtEventResource, WinRtEventSession, "midi event")
);
