use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::device::{MidiDataFormat, MidiProtocol, MidiRecordFraming};

/// Data-format code for MIDI 1 byte-stream transport.
const MIDI_DATA_FORMAT_CODE_MIDI1_BYTES: u32 = 1;
/// Data-format code for UMP transport.
const MIDI_DATA_FORMAT_CODE_UMP: u32 = 2;
/// Protocol code for MIDI 1 semantics.
const MIDI_PROTOCOL_CODE_MIDI1: u32 = 1;
/// Protocol code for MIDI 2 semantics.
const MIDI_PROTOCOL_CODE_MIDI2: u32 = 2;
/// Framing code for one complete record.
const MIDI_RECORD_FRAMING_CODE_COMPLETE: u32 = 1;
/// Framing code for one SysEx-start record.
const MIDI_RECORD_FRAMING_CODE_START: u32 = 2;
/// Framing code for one continued SysEx record.
const MIDI_RECORD_FRAMING_CODE_CONTINUE: u32 = 3;
/// Framing code for one SysEx-end record.
const MIDI_RECORD_FRAMING_CODE_END: u32 = 4;

/// Event kind code for one port-added event.
pub(crate) const MIDI_EVENT_KIND_CODE_PORT_ADDED: u32 = 1;
/// Event kind code for one port-removed event.
pub(crate) const MIDI_EVENT_KIND_CODE_PORT_REMOVED: u32 = 2;
/// Event kind code for one port-changed event.
pub(crate) const MIDI_EVENT_KIND_CODE_PORT_CHANGED: u32 = 3;
/// Event kind code for one backend-disconnected event.
pub(crate) const MIDI_EVENT_KIND_CODE_BACKEND_DISCONNECTED: u32 = 4;
/// Direction code for one input endpoint.
pub(crate) const MIDI_PORT_DIRECTION_CODE_INPUT: u32 = 1;
/// Direction code for one output endpoint.
pub(crate) const MIDI_PORT_DIRECTION_CODE_OUTPUT: u32 = 2;

/// Return one host callback ABI data-format code.
pub(crate) fn data_format_code(data_format: MidiDataFormat) -> u32 {
    match data_format {
        MidiDataFormat::Midi1Bytes => MIDI_DATA_FORMAT_CODE_MIDI1_BYTES,
        MidiDataFormat::Ump => MIDI_DATA_FORMAT_CODE_UMP,
    }
}

/// Return one host callback ABI protocol code.
pub(crate) fn protocol_code(protocol: MidiProtocol) -> u32 {
    match protocol {
        MidiProtocol::Midi1 => MIDI_PROTOCOL_CODE_MIDI1,
        MidiProtocol::Midi2 => MIDI_PROTOCOL_CODE_MIDI2,
    }
}

/// Return one host callback ABI framing code.
pub(crate) fn framing_code(framing: MidiRecordFraming) -> u32 {
    match framing {
        MidiRecordFraming::Complete => MIDI_RECORD_FRAMING_CODE_COMPLETE,
        MidiRecordFraming::Start => MIDI_RECORD_FRAMING_CODE_START,
        MidiRecordFraming::Continue => MIDI_RECORD_FRAMING_CODE_CONTINUE,
        MidiRecordFraming::End => MIDI_RECORD_FRAMING_CODE_END,
    }
}

/// Decode one Android host callback data-format code.
pub(crate) fn decode_data_format_code(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<MidiDataFormat> {
    match code {
        MIDI_DATA_FORMAT_CODE_MIDI1_BYTES => Ok(MidiDataFormat::Midi1Bytes),
        MIDI_DATA_FORMAT_CODE_UMP => Ok(MidiDataFormat::Ump),
        _ => Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi returned unsupported data-format code {code}"
        )))
        .boxed()),
    }
}

/// Decode one optional Android host callback protocol code.
pub(crate) fn decode_protocol_code(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<Option<MidiProtocol>> {
    match code {
        0 => Ok(None),
        MIDI_PROTOCOL_CODE_MIDI1 => Ok(Some(MidiProtocol::Midi1)),
        MIDI_PROTOCOL_CODE_MIDI2 => Ok(Some(MidiProtocol::Midi2)),
        _ => Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi returned unsupported protocol code {code}"
        )))
        .boxed()),
    }
}

/// Decode one Android host callback framing code.
pub(crate) fn decode_framing_code(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<MidiRecordFraming> {
    match code {
        MIDI_RECORD_FRAMING_CODE_COMPLETE => Ok(MidiRecordFraming::Complete),
        MIDI_RECORD_FRAMING_CODE_START => Ok(MidiRecordFraming::Start),
        MIDI_RECORD_FRAMING_CODE_CONTINUE => Ok(MidiRecordFraming::Continue),
        MIDI_RECORD_FRAMING_CODE_END => Ok(MidiRecordFraming::End),
        _ => Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi returned unsupported framing code {code}"
        )))
        .boxed()),
    }
}
