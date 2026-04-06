use crate::platform::device::MidiBackend;
use crate::platform::device::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    WinRtEventRepository, WinRtEventResource, WinRtInputRepository, WinRtInputResource,
    WinRtOutputRepository, WinRtOutputResource,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::WinRT,
    input = (input_resource, WinRtInputResource, WinRtInputRepository, "midi input port"),
    output = (
        output_resource,
        WinRtOutputResource,
        WinRtOutputRepository,
        "midi output port"
    ),
    event = (event_resource, WinRtEventResource, WinRtEventRepository, "midi event")
);
