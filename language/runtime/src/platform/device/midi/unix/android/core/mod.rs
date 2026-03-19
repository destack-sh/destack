mod bridge;
mod codec;
mod session;

pub(crate) use crate::platform::device::midi::core::snapshot_key;
pub(crate) use bridge::{
    close_event_session, close_input_session, close_output_session, create_virtual_input_session,
    create_virtual_output_session, describe_backend, open_event_session, open_input_session,
    open_output_session, read_input_port_descriptors, read_input_records, read_native_events,
    read_output_port_descriptors, write_output_records,
};
pub(crate) use codec::{
    MIDI_EVENT_KIND_CODE_BACKEND_DISCONNECTED, MIDI_EVENT_KIND_CODE_PORT_ADDED,
    MIDI_EVENT_KIND_CODE_PORT_CHANGED, MIDI_EVENT_KIND_CODE_PORT_REMOVED,
    MIDI_PORT_DIRECTION_CODE_INPUT, MIDI_PORT_DIRECTION_CODE_OUTPUT, data_format_code,
    decode_data_format_code, decode_framing_code, decode_protocol_code, framing_code,
    protocol_code,
};
pub(crate) use session::{
    AndroidBackendDescription, AndroidEventDeliveryKind, AndroidEventResource, AndroidEventSession,
    AndroidInputResource, AndroidInputSession, AndroidOutputResource, AndroidOutputSession,
    SnapshotKey, insert_event_resource, insert_input_resource, insert_output_resource,
};
