use crate::platform::device::{
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_DATA_FORMAT_FLAG_UMP, MIDI_PROTOCOL_FLAG_MIDI1,
    MIDI_PROTOCOL_FLAG_MIDI2, MidiDataFormat, MidiDataFormatFlags, MidiProtocol, MidiProtocolFlags,
};

use super::super::abi::{K_MIDI_PROTOCOL_1_0, K_MIDI_PROTOCOL_2_0, MIDIProtocolID};

/// Return the CoreMIDI protocol id for one selected protocol.
pub(crate) fn selected_protocol_id(
    protocol: Option<MidiProtocol>,
    data_format: MidiDataFormat,
) -> MIDIProtocolID {
    match (data_format, protocol) {
        (MidiDataFormat::Ump, Some(MidiProtocol::Midi2)) => K_MIDI_PROTOCOL_2_0,
        (MidiDataFormat::Ump, _) => K_MIDI_PROTOCOL_1_0,
        (MidiDataFormat::Midi1Bytes, Some(MidiProtocol::Midi2)) => K_MIDI_PROTOCOL_2_0,
        (MidiDataFormat::Midi1Bytes, _) => K_MIDI_PROTOCOL_1_0,
    }
}

/// Return the flag bit for one data format.
pub(crate) fn data_format_flag(data_format: MidiDataFormat) -> MidiDataFormatFlags {
    match data_format {
        MidiDataFormat::Midi1Bytes => MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES,
        MidiDataFormat::Ump => MIDI_DATA_FORMAT_FLAG_UMP,
    }
}

/// Return the flag bit for one protocol.
pub(crate) fn protocol_flag(protocol: MidiProtocol) -> MidiProtocolFlags {
    match protocol {
        MidiProtocol::Midi1 => MIDI_PROTOCOL_FLAG_MIDI1,
        MidiProtocol::Midi2 => MIDI_PROTOCOL_FLAG_MIDI2,
    }
}

/// Return one exact transport-support tuple for one selected shape.
pub(crate) fn exact_transport_support(
    data_format: MidiDataFormat,
    protocol: MidiProtocol,
) -> (
    MidiDataFormatFlags,
    Option<MidiDataFormat>,
    MidiProtocolFlags,
    Option<MidiProtocol>,
) {
    (
        data_format_flag(data_format),
        Some(data_format),
        protocol_flag(protocol),
        Some(protocol),
    )
}
