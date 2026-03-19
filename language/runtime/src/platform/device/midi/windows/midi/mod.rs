mod abi;
mod backend;
mod core;
mod descriptor;
mod event;
mod input;
mod output;
mod resource;
#[allow(
    dead_code,
    missing_debug_implementations,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unreachable_pub
)]
mod sdk;
mod service;

pub(crate) use backend::{backend_metadata, backend_support};
pub(crate) use event::{
    midi_event_close, midi_event_open, midi_event_read, midi_event_read_batch, midi_event_try_read,
    midi_event_try_read_batch,
};
pub(crate) use input::{
    midi_input_port_close, midi_input_port_descriptor, midi_input_port_list, midi_input_port_open,
    midi_input_read, midi_input_read_batch, midi_input_try_read, midi_input_try_read_batch,
    midi_input_virtual_create,
};
pub(crate) use output::{
    midi_output_port_close, midi_output_port_descriptor, midi_output_port_list,
    midi_output_port_open, midi_output_virtual_create, midi_output_write,
};
pub(crate) use service::{WindowsMidiService, windows_midi_service};
