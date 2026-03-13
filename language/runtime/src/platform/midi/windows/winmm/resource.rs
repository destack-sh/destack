use crate::platform::midi::MidiBackend;
use crate::platform::midi::core::define_backend_midi_resource_accessors;

use super::core::{
    WinMmEventResource, WinMmEventSession, WinMmInputResource, WinMmInputSession,
    WinMmOutputResource, WinMmOutputSession,
};

define_backend_midi_resource_accessors!(
    vis = pub(super),
    backend = MidiBackend::WinMM,
    input = (input_resource, WinMmInputResource, WinMmInputSession, "midi input"),
    output = (output_resource, WinMmOutputResource, WinMmOutputSession, "midi output"),
    event = (event_resource, WinMmEventResource, WinMmEventSession, "midi event")
);
