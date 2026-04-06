use crate::platform::device::MidiBackend;
use crate::platform::device::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    WinMmEventRepository, WinMmEventResource, WinMmInputRepository, WinMmInputResource,
    WinMmOutputRepository, WinMmOutputResource,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::WinMM,
    input = (input_resource, WinMmInputResource, WinMmInputRepository, "midi input"),
    output = (output_resource, WinMmOutputResource, WinMmOutputRepository, "midi output"),
    event = (event_resource, WinMmEventResource, WinMmEventRepository, "midi event")
);
