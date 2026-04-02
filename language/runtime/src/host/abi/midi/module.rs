use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    module midi {
        platforms: [android];
        types: super::types::host_abi_types();
        requests {
            /// Describe host MIDI backend support.
            fn describe_backend(
                capability_flags: output(u64),
                supported_data_formats: output(u32),
                supported_protocols: output(u32),
            ) -> host_status;

            /// List host MIDI input ports.
            fn input_port_list(
                flags: u32,
                headers: slice(AndroidHostMidiPortDescriptorHeader),
                header_count_written: output(u32),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// List host MIDI output ports.
            fn output_port_list(
                flags: u32,
                headers: slice(AndroidHostMidiPortDescriptorHeader),
                header_count_written: output(u32),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Open one host MIDI input port.
            fn input_port_open(
                id: string_ref,
                data_format: u32,
                protocol: u32,
                queue_capacity: u32,
                opened_port: output(AndroidHostMidiOpenedPortHeader),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Open one host MIDI output port.
            fn output_port_open(
                id: string_ref,
                data_format: u32,
                protocol: u32,
                opened_port: output(AndroidHostMidiOpenedPortHeader),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Create one host virtual MIDI input port.
            fn input_virtual_create(
                name: string_ref,
                manufacturer: string_ref,
                model: string_ref,
                version: string_ref,
                data_format: u32,
                protocol: u32,
                queue_capacity: u32,
                opened_port: output(AndroidHostMidiOpenedPortHeader),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Create one host virtual MIDI output port.
            fn output_virtual_create(
                name: string_ref,
                manufacturer: string_ref,
                model: string_ref,
                version: string_ref,
                data_format: u32,
                protocol: u32,
                opened_port: output(AndroidHostMidiOpenedPortHeader),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Close one host MIDI input session.
            fn input_port_close(
                session_id: u64,
            ) -> host_status;

            /// Close one host MIDI output session.
            fn output_port_close(
                session_id: u64,
            ) -> host_status;

            /// Read host MIDI input records.
            fn input_read(
                session_id: u64,
                max_records: u32,
                timeout_ns: u64,
                headers: slice(AndroidHostMidiInputRecordHeader),
                record_count_written: output(u32),
                blob_bytes: slice(u8),
                blob_bytes_written: output(u32),
            ) -> host_status;

            /// Open one host MIDI event subscription.
            fn event_open(
                flags: u32,
                direction_mask: u32,
                session_id: output(u64),
            ) -> host_status;

            /// Read host MIDI topology events.
            fn event_read(
                session_id: u64,
                max_events: u32,
                timeout_ns: u64,
                headers: slice(AndroidHostMidiEventHeader),
                event_count_written: output(u32),
                string_bytes: slice(u8),
                string_bytes_written: output(u32),
            ) -> host_status;

            /// Close one host MIDI event subscription.
            fn event_close(
                session_id: u64,
            ) -> host_status;

            /// Write host MIDI output records.
            fn output_write(
                session_id: u64,
                headers: slice(AndroidHostMidiOutputRecordHeader),
                record_count: u32,
                blob_bytes: slice(u8),
                records_written: output(u32),
            ) -> host_status;
        }
        ingress {}
    }
}
