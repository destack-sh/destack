use crate::host::android::bridge::midi::callbacks::{
    AndroidHostMidiDescribeBackendCallback, AndroidHostMidiEventCloseCallback,
    AndroidHostMidiEventOpenCallback, AndroidHostMidiEventReadCallback,
    AndroidHostMidiInputPortCloseCallback, AndroidHostMidiInputPortListCallback,
    AndroidHostMidiInputPortOpenCallback, AndroidHostMidiInputReadCallback,
    AndroidHostMidiInputVirtualCreateCallback, AndroidHostMidiOutputPortCloseCallback,
    AndroidHostMidiOutputPortListCallback, AndroidHostMidiOutputPortOpenCallback,
    AndroidHostMidiOutputVirtualCreateCallback, AndroidHostMidiOutputWriteCallback,
};

/// Fixed-size Android MIDI port descriptor header.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AndroidHostMidiPortDescriptorHeader {
    /// Offset of the stable id string inside the shared string buffer.
    pub id_offset: u32,
    /// Length of the stable id string inside the shared string buffer.
    pub id_len: u32,
    /// Offset of the optional group id string inside the shared string buffer.
    pub group_id_offset: u32,
    /// Length of the optional group id string inside the shared string buffer.
    pub group_id_len: u32,
    /// Offset of the optional backend id string inside the shared string buffer.
    pub backend_id_offset: u32,
    /// Length of the optional backend id string inside the shared string buffer.
    pub backend_id_len: u32,
    /// Offset of the endpoint name string inside the shared string buffer.
    pub name_offset: u32,
    /// Length of the endpoint name string inside the shared string buffer.
    pub name_len: u32,
    /// Offset of the optional group name string inside the shared string buffer.
    pub group_name_offset: u32,
    /// Length of the optional group name string inside the shared string buffer.
    pub group_name_len: u32,
    /// Offset of the optional manufacturer string inside the shared string buffer.
    pub manufacturer_offset: u32,
    /// Length of the optional manufacturer string inside the shared string buffer.
    pub manufacturer_len: u32,
    /// Offset of the optional model string inside the shared string buffer.
    pub model_offset: u32,
    /// Length of the optional model string inside the shared string buffer.
    pub model_len: u32,
    /// Offset of the optional version string inside the shared string buffer.
    pub version_offset: u32,
    /// Length of the optional version string inside the shared string buffer.
    pub version_len: u32,
    /// Supported transport formats for this endpoint.
    pub supported_data_formats: u32,
    /// Default transport format code, or zero when unspecified.
    pub default_data_format: u32,
    /// Supported protocol flags for this endpoint.
    pub supported_protocols: u32,
    /// Default protocol code, or zero when unspecified.
    pub default_protocol: u32,
    /// Whether the endpoint is virtual.
    pub is_virtual: u32,
    /// Whether the endpoint is currently connected.
    pub is_connected: u32,
}

/// Fixed-size Android MIDI opened-port payload.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AndroidHostMidiOpenedPortHeader {
    /// Opaque host session identifier.
    pub session_id: u64,
    /// Descriptor snapshot for the opened endpoint.
    pub descriptor: AndroidHostMidiPortDescriptorHeader,
}

/// Fixed-size Android MIDI input record header.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AndroidHostMidiInputRecordHeader {
    /// Receive timestamp in runtime monotonic nanoseconds.
    pub received_at_ns: u64,
    /// Offset of the optional source id string inside the shared blob.
    pub source_id_offset: u32,
    /// Length of the optional source id string inside the shared blob.
    pub source_id_len: u32,
    /// Offset of the record payload inside the shared blob.
    pub data_offset: u32,
    /// Length of the record payload inside the shared blob.
    pub data_len: u32,
    /// Transport data-format code.
    pub data_format: u32,
    /// Protocol code, or zero when unspecified.
    pub protocol: u32,
    /// Record framing code.
    pub framing: u32,
}

/// Fixed-size Android MIDI output record header.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AndroidHostMidiOutputRecordHeader {
    /// Scheduled send timestamp in runtime monotonic nanoseconds.
    pub send_at_ns: u64,
    /// Whether the record carries one scheduled send timestamp.
    pub has_send_at: u32,
    /// Offset of the record payload inside the shared blob.
    pub data_offset: u32,
    /// Length of the record payload inside the shared blob.
    pub data_len: u32,
    /// Transport data-format code.
    pub data_format: u32,
    /// Protocol code, or zero when unspecified.
    pub protocol: u32,
    /// Record framing code.
    pub framing: u32,
}

/// Fixed-size Android MIDI topology event header.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AndroidHostMidiEventHeader {
    /// Event timestamp in runtime monotonic nanoseconds.
    pub timestamp_ns: u64,
    /// Number of dropped events before this event.
    pub dropped_count: u64,
    /// Event kind code.
    pub kind: u32,
    /// Event direction code, or zero when absent.
    pub direction: u32,
    /// Future backend-disconnection detail flags.
    pub flags: u32,
    /// Offset of the optional removed-port runtime id string.
    pub id_offset: u32,
    /// Length of the optional removed-port runtime id string.
    pub id_len: u32,
    /// Offset of the optional removed-port group id string.
    pub group_id_offset: u32,
    /// Length of the optional removed-port group id string.
    pub group_id_len: u32,
    /// Embedded descriptor payload for added and changed events.
    pub descriptor: AndroidHostMidiPortDescriptorHeader,
}

/// Callback table for Android host MIDI interop.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AndroidHostMidiCallbacks {
    /// Callback for backend descriptor probing.
    pub describe_backend: Option<AndroidHostMidiDescribeBackendCallback>,
    /// Callback for input port enumeration.
    pub input_port_list: Option<AndroidHostMidiInputPortListCallback>,
    /// Callback for output port enumeration.
    pub output_port_list: Option<AndroidHostMidiOutputPortListCallback>,
    /// Callback for input port open.
    pub input_port_open: Option<AndroidHostMidiInputPortOpenCallback>,
    /// Callback for output port open.
    pub output_port_open: Option<AndroidHostMidiOutputPortOpenCallback>,
    /// Callback for virtual input create.
    pub input_virtual_create: Option<AndroidHostMidiInputVirtualCreateCallback>,
    /// Callback for virtual output create.
    pub output_virtual_create: Option<AndroidHostMidiOutputVirtualCreateCallback>,
    /// Callback for input port close.
    pub input_port_close: Option<AndroidHostMidiInputPortCloseCallback>,
    /// Callback for output port close.
    pub output_port_close: Option<AndroidHostMidiOutputPortCloseCallback>,
    /// Callback for input record reads.
    pub input_read: Option<AndroidHostMidiInputReadCallback>,
    /// Callback for event subscription open.
    pub event_open: Option<AndroidHostMidiEventOpenCallback>,
    /// Callback for event reads.
    pub event_read: Option<AndroidHostMidiEventReadCallback>,
    /// Callback for event close.
    pub event_close: Option<AndroidHostMidiEventCloseCallback>,
    /// Callback for output record writes.
    pub output_write: Option<AndroidHostMidiOutputWriteCallback>,
}

impl Default for AndroidHostMidiCallbacks {
    /// Build one callback table with no handlers.
    fn default() -> Self {
        Self {
            describe_backend: None,
            input_port_list: None,
            output_port_list: None,
            input_port_open: None,
            output_port_open: None,
            input_virtual_create: None,
            output_virtual_create: None,
            input_port_close: None,
            output_port_close: None,
            input_read: None,
            event_open: None,
            event_read: None,
            event_close: None,
            output_write: None,
        }
    }
}
