#[cfg(feature = "generator")]
use crate::host::abi::describe::host_abi_types;

#[cfg(not(feature = "generator"))]
pub(crate) use crate::host::android::abi::midi::types::{
    AndroidHostMidiEventHeader, AndroidHostMidiInputRecordHeader, AndroidHostMidiOpenedPortHeader,
    AndroidHostMidiOutputRecordHeader, AndroidHostMidiPortDescriptorHeader,
};

#[cfg(feature = "generator")]
host_abi_types! {
    fn host_abi_types() {
        /// Fixed-size host MIDI port descriptor header.
        struct AndroidHostMidiPortDescriptorHeader {
            /// Offset of the stable id string inside the shared string buffer.
            id_offset: u32,
            /// Length of the stable id string inside the shared string buffer.
            id_len: u32,
            /// Offset of the optional group id string inside the shared string buffer.
            group_id_offset: u32,
            /// Length of the optional group id string inside the shared string buffer.
            group_id_len: u32,
            /// Offset of the optional backend id string inside the shared string buffer.
            backend_id_offset: u32,
            /// Length of the optional backend id string inside the shared string buffer.
            backend_id_len: u32,
            /// Offset of the endpoint name string inside the shared string buffer.
            name_offset: u32,
            /// Length of the endpoint name string inside the shared string buffer.
            name_len: u32,
            /// Offset of the optional group name string inside the shared string buffer.
            group_name_offset: u32,
            /// Length of the optional group name string inside the shared string buffer.
            group_name_len: u32,
            /// Offset of the optional manufacturer string inside the shared string buffer.
            manufacturer_offset: u32,
            /// Length of the optional manufacturer string inside the shared string buffer.
            manufacturer_len: u32,
            /// Offset of the optional model string inside the shared string buffer.
            model_offset: u32,
            /// Length of the optional model string inside the shared string buffer.
            model_len: u32,
            /// Offset of the optional version string inside the shared string buffer.
            version_offset: u32,
            /// Length of the optional version string inside the shared string buffer.
            version_len: u32,
            /// Supported transport formats for this endpoint.
            supported_data_formats: u32,
            /// Default transport format code, or zero when unspecified.
            default_data_format: u32,
            /// Supported protocol flags for this endpoint.
            supported_protocols: u32,
            /// Default protocol code, or zero when unspecified.
            default_protocol: u32,
            /// Whether the endpoint is virtual.
            is_virtual: u32,
            /// Whether the endpoint is currently connected.
            is_connected: u32,
        }

        /// Fixed-size host MIDI opened-port payload.
        struct AndroidHostMidiOpenedPortHeader {
            /// Opaque host session identifier.
            session_id: u64,
            /// Descriptor snapshot for the opened endpoint.
            descriptor: AndroidHostMidiPortDescriptorHeader,
        }

        /// Fixed-size host MIDI input record header.
        struct AndroidHostMidiInputRecordHeader {
            /// Receive timestamp in runtime monotonic nanoseconds.
            received_at_ns: u64,
            /// Offset of the optional source id string inside the shared blob.
            source_id_offset: u32,
            /// Length of the optional source id string inside the shared blob.
            source_id_len: u32,
            /// Offset of the record payload inside the shared blob.
            data_offset: u32,
            /// Length of the record payload inside the shared blob.
            data_len: u32,
            /// Transport data-format code.
            data_format: u32,
            /// Protocol code, or zero when unspecified.
            protocol: u32,
            /// Record framing code.
            framing: u32,
        }

        /// Fixed-size host MIDI output record header.
        struct AndroidHostMidiOutputRecordHeader {
            /// Scheduled send timestamp in runtime monotonic nanoseconds.
            send_at_ns: option(u64),
            /// Offset of the record payload inside the shared blob.
            data_offset: u32,
            /// Length of the record payload inside the shared blob.
            data_len: u32,
            /// Transport data-format code.
            data_format: u32,
            /// Protocol code, or zero when unspecified.
            protocol: u32,
            /// Record framing code.
            framing: u32,
        }

        /// Fixed-size host MIDI topology event header.
        struct AndroidHostMidiEventHeader {
            /// Event timestamp in runtime monotonic nanoseconds.
            timestamp_ns: u64,
            /// Number of dropped events before this event.
            dropped_count: u64,
            /// Event kind code.
            kind: u32,
            /// Event direction code, or zero when absent.
            direction: u32,
            /// Future backend-disconnection detail flags.
            flags: u32,
            /// Offset of the optional removed-port runtime id string.
            id_offset: u32,
            /// Length of the optional removed-port runtime id string.
            id_len: u32,
            /// Offset of the optional removed-port group id string.
            group_id_offset: u32,
            /// Length of the optional removed-port group id string.
            group_id_len: u32,
            /// Embedded descriptor payload for added and changed events.
            descriptor: AndroidHostMidiPortDescriptorHeader,
        }
    }
}
