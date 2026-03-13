mod callbacks;
mod ffi;
#[cfg(test)]
mod tests;
mod types;

pub use callbacks::{
    AndroidHostMidiDescribeBackendCallback, AndroidHostMidiEventCloseCallback,
    AndroidHostMidiEventOpenCallback, AndroidHostMidiEventReadCallback,
    AndroidHostMidiInputPortCloseCallback, AndroidHostMidiInputPortListCallback,
    AndroidHostMidiInputPortOpenCallback, AndroidHostMidiInputReadCallback,
    AndroidHostMidiInputVirtualCreateCallback, AndroidHostMidiOutputFlushCallback,
    AndroidHostMidiOutputPortCloseCallback, AndroidHostMidiOutputPortListCallback,
    AndroidHostMidiOutputPortOpenCallback, AndroidHostMidiOutputVirtualCreateCallback,
    AndroidHostMidiOutputWriteCallback,
};
pub use ffi::{
    destack_host_android_midi_describe_backend, destack_host_android_midi_event_close,
    destack_host_android_midi_event_open, destack_host_android_midi_event_read,
    destack_host_android_midi_input_port_close, destack_host_android_midi_input_port_list,
    destack_host_android_midi_input_port_open, destack_host_android_midi_input_read,
    destack_host_android_midi_input_virtual_create, destack_host_android_midi_output_flush,
    destack_host_android_midi_output_port_close, destack_host_android_midi_output_port_list,
    destack_host_android_midi_output_port_open, destack_host_android_midi_output_virtual_create,
    destack_host_android_midi_output_write,
};
pub use types::{
    AndroidHostMidiCallbacks, AndroidHostMidiEventHeader, AndroidHostMidiInputRecordHeader,
    AndroidHostMidiOpenedPortHeader, AndroidHostMidiOutputRecordHeader,
    AndroidHostMidiPortDescriptorHeader,
};
