use std::ffi::c_ulong;

use crate::platform::device::{
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PROTOCOL_FLAG_MIDI1, MidiDataFormat,
    MidiDataFormatFlags, MidiPortDirection, MidiProtocol, MidiProtocolFlags,
};

/// Stable prefix for JACK MIDI input ids.
const JACK_INPUT_ID_PREFIX: &str = "jack-midi:input:";
/// Stable prefix for JACK MIDI output ids.
const JACK_OUTPUT_ID_PREFIX: &str = "jack-midi:output:";
/// Stable prefix for JACK MIDI group ids.
const JACK_GROUP_ID_PREFIX: &str = "jack-midi:group:";
/// JACK port flag: physical port.
const JACK_PORT_IS_PHYSICAL: c_ulong = 0x4;
/// Prefix reserved for hidden Destack JACK clients.
pub(crate) const INTERNAL_CLIENT_NAME_PREFIX: &str = "destack-midi-internal-";

/// Return one exact JACK transport support tuple.
pub(crate) fn exact_transport_support() -> (
    MidiDataFormatFlags,
    Option<MidiDataFormat>,
    MidiProtocolFlags,
    Option<MidiProtocol>,
) {
    (
        MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0),
        Some(MidiDataFormat::Midi1Bytes),
        MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0),
        Some(MidiProtocol::Midi1),
    )
}

/// Return one runtime id for one JACK endpoint.
pub(crate) fn runtime_id(direction: MidiPortDirection, backend_port_name: &str) -> String {
    let prefix = match direction {
        MidiPortDirection::Input => JACK_INPUT_ID_PREFIX,
        MidiPortDirection::Output => JACK_OUTPUT_ID_PREFIX,
    };

    format!("{prefix}{backend_port_name}")
}

/// Return one stable group id from one JACK client name.
pub(crate) fn group_id(client_name: &str) -> String {
    format!("{JACK_GROUP_ID_PREFIX}{client_name}")
}

/// Split one JACK full port name into client and port names.
pub(crate) fn split_port_name(full_name: &str) -> (&str, &str) {
    match full_name.split_once(':') {
        Some((client_name, port_name)) => (client_name, port_name),
        None => ("", full_name),
    }
}

/// Return whether one JACK port is virtual.
pub(crate) fn is_virtual_port(flags: c_ulong) -> bool {
    flags & JACK_PORT_IS_PHYSICAL == 0
}

/// Return whether one JACK client name belongs to hidden Destack plumbing.
pub(crate) fn is_internal_client_name(client_name: &str) -> bool {
    client_name.starts_with(INTERNAL_CLIENT_NAME_PREFIX)
}
