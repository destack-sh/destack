use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::midi::{MidiDataFormat, MidiProtocol, MidiRecordFraming};

use super::MidiOutputRecordValue;

/// Validate one format and protocol pairing.
#[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
pub(crate) fn validate_record_shape(
    operation: &'static str,
    data_format: MidiDataFormat,
    protocol: Option<MidiProtocol>,
) -> RuntimeResult<()> {
    // reject midi2 semantics on byte-stream transport
    if matches!(protocol, Some(MidiProtocol::Midi2)) && data_format != MidiDataFormat::Ump {
        return Err(
            RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
                "protocol",
                format!("{operation}: MIDI 2 requires UMP transport"),
            ))
            .boxed(),
        );
    }

    Ok(())
}

/// Validate one outbound transport record payload.
pub(crate) fn validate_output_record_payload(
    operation: &'static str,
    record: &MidiOutputRecordValue,
) -> RuntimeResult<()> {
    // midi1 byte-stream payloads must describe one self-consistent transport record
    if record.data_format == MidiDataFormat::Midi1Bytes {
        validate_midi1_output_record_payload(operation, record.framing, &record.data)?;
    }

    // ump payloads must preserve whole big-endian words
    if record.data_format == MidiDataFormat::Ump && !record.data.len().is_multiple_of(4) {
        return Err(
            RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
                "records",
                format!("{operation}: UMP record payload must be a multiple of 4 bytes"),
            ))
            .boxed(),
        );
    }

    Ok(())
}

/// Validate one outbound MIDI 1 byte-stream transport record payload.
fn validate_midi1_output_record_payload(
    operation: &'static str,
    framing: MidiRecordFraming,
    data: &[u8],
) -> RuntimeResult<()> {
    // empty byte-stream records are never meaningful
    if data.is_empty() {
        return Err(
            RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
                "records",
                format!("{operation}: MIDI 1 byte-stream record payload must not be empty"),
            ))
            .boxed(),
        );
    }

    let starts_sysex = data.first() == Some(&0xF0);
    let ends_sysex = data.last() == Some(&0xF7);

    // sysex fragments are the only multi-record byte-stream payloads we expose
    if starts_sysex || ends_sysex || framing != MidiRecordFraming::Complete {
        return validate_sysex_fragment(operation, framing, data, starts_sysex, ends_sysex);
    }

    // complete non-sysex records must be one full canonical MIDI 1 message
    let status = data[0];
    if status < 0x80 {
        return Err(
            RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
                "records",
                format!(
                    "{operation}: complete MIDI 1 byte-stream records must start with one status byte"
                ),
            ))
            .boxed(),
        );
    }

    let expected_length = canonical_midi1_message_length(status).ok_or_else(|| {
        RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
            "records",
            format!("{operation}: unsupported MIDI 1 status byte 0x{status:02X}"),
        ))
        .boxed()
    })?;

    if data.len() != expected_length {
        return Err(
            RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
                "records",
                format!(
                    "{operation}: complete MIDI 1 byte-stream records must contain exactly one full message"
                ),
            ))
            .boxed(),
        );
    }

    // data bytes inside canonical channel/system messages must remain 7-bit clean
    if data[1..].iter().any(|byte| *byte >= 0x80) {
        return Err(
            RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
                "records",
                format!(
                    "{operation}: MIDI 1 message data bytes must remain 7-bit clean outside SysEx framing"
                ),
            ))
            .boxed(),
        );
    }

    Ok(())
}

/// Validate one outbound SysEx-fragment transport record payload.
fn validate_sysex_fragment(
    operation: &'static str,
    framing: MidiRecordFraming,
    data: &[u8],
    starts_sysex: bool,
    ends_sysex: bool,
) -> RuntimeResult<()> {
    let framing_matches = match framing {
        MidiRecordFraming::Complete => starts_sysex && ends_sysex,
        MidiRecordFraming::Start => starts_sysex && !ends_sysex,
        MidiRecordFraming::Continue => !starts_sysex && !ends_sysex,
        MidiRecordFraming::End => !starts_sysex && ends_sysex,
    };

    if !framing_matches {
        return Err(
            RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
                "records",
                format!(
                    "{operation}: MIDI 1 SysEx fragment framing does not match the record payload delimiters"
                ),
            ))
            .boxed(),
        );
    }

    let payload_range = match framing {
        MidiRecordFraming::Complete => 1..data.len() - 1,
        MidiRecordFraming::Start => 1..data.len(),
        MidiRecordFraming::Continue => 0..data.len(),
        MidiRecordFraming::End => 0..data.len() - 1,
    };

    if data[payload_range].iter().any(|byte| *byte >= 0x80) {
        return Err(
            RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
                "records",
                format!("{operation}: MIDI 1 SysEx fragment payload bytes must remain 7-bit clean"),
            ))
            .boxed(),
        );
    }

    Ok(())
}

/// Return the canonical MIDI 1 message length for one status byte.
fn canonical_midi1_message_length(status: u8) -> Option<usize> {
    match status {
        0x80..=0x8F | 0x90..=0x9F | 0xA0..=0xAF | 0xB0..=0xBF | 0xE0..=0xEF => Some(3),
        0xC0..=0xCF | 0xD0..=0xDF | 0xF1 | 0xF3 => Some(2),
        0xF2 => Some(3),
        0xF6 | 0xF8..=0xFF => Some(1),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::validate_output_record_payload;
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::platform::midi::core::MidiOutputRecordValue;
    use crate::platform::midi::{MidiDataFormat, MidiProtocol, MidiRecordFraming};

    /// Reject malformed MIDI 1 byte-stream output records.
    #[test]
    fn test_validate_output_record_payload_rejects_invalid_midi1_records() {
        let invalid_record = MidiOutputRecordValue {
            send_at_ns: None,
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: vec![0x3C, 0x40],
        };

        let error = validate_output_record_payload("destack.midi.output.write", &invalid_record)
            .expect_err("invalid MIDI 1 payload should be rejected");
        let error = error
            .platform_error()
            .expect("invalid payload should surface one platform error");

        assert_eq!(error.code, PlatformErrorCode::InvalidArgumentValue);
    }

    /// Track SysEx fragment framing against transport delimiters.
    #[test]
    fn test_validate_output_record_payload_accepts_only_matching_sysex_fragments() {
        let valid_start = MidiOutputRecordValue {
            send_at_ns: None,
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Start,
            data: vec![0xF0, 0x7D, 0x01],
        };
        validate_output_record_payload("destack.midi.output.write", &valid_start)
            .expect("valid SysEx start fragments should pass");

        let invalid_end = MidiOutputRecordValue {
            send_at_ns: None,
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::End,
            data: vec![0xF0, 0x7D, 0xF7],
        };

        let error = validate_output_record_payload("destack.midi.output.write", &invalid_end)
            .expect_err("mismatched SysEx fragment framing should be rejected");
        let error = error
            .platform_error()
            .expect("mismatched fragment should surface one platform error");

        assert_eq!(error.code, PlatformErrorCode::InvalidArgumentValue);
    }
}
