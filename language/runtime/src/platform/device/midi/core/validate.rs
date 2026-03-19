use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::device::{MidiDataFormat, MidiProtocol, MidiRecordFraming};

use super::descriptor::validate_descriptor_transport_request;
use super::{MidiOutputRecordValue, MidiPortDescriptorValue};

/// Validate one format and protocol pairing.
pub(crate) fn validate_record_shape(
    operation: &'static str,
    data_format: MidiDataFormat,
    protocol: Option<MidiProtocol>,
) -> RuntimeResult<()> {
    // reject midi2 semantics on byte-stream transport
    if matches!(protocol, Some(MidiProtocol::Midi2)) && data_format != MidiDataFormat::Ump {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "protocol",
            format!("{operation}: MIDI 2 requires UMP transport"),
        ))
        .boxed());
    }

    Ok(())
}

/// Resolve one port-open transport request from explicit options, descriptor defaults, and backend fallbacks.
pub(crate) fn resolve_open_transport(
    operation: &'static str,
    requested_data_format: Option<MidiDataFormat>,
    requested_protocol: Option<MidiProtocol>,
    default_data_format: Option<MidiDataFormat>,
    default_protocol: Option<MidiProtocol>,
    fallback_data_format: MidiDataFormat,
    fallback_protocol: Option<MidiProtocol>,
) -> RuntimeResult<(MidiDataFormat, Option<MidiProtocol>)> {
    let requested_shape_data_format = requested_data_format.unwrap_or(fallback_data_format);
    let requested_shape_protocol = requested_protocol.or(fallback_protocol);

    validate_record_shape(
        operation,
        requested_shape_data_format,
        requested_shape_protocol,
    )?;

    let data_format = requested_data_format
        .or(default_data_format)
        .unwrap_or(fallback_data_format);
    let protocol = requested_protocol
        .or(default_protocol)
        .or(fallback_protocol);

    Ok((data_format, protocol))
}

/// Resolve one descriptor-backed transport request and reject unsupported shapes.
pub(crate) fn resolve_descriptor_open_transport(
    operation: &'static str,
    descriptor: &MidiPortDescriptorValue,
    requested_data_format: Option<MidiDataFormat>,
    requested_protocol: Option<MidiProtocol>,
    fallback_data_format: MidiDataFormat,
    fallback_protocol: Option<MidiProtocol>,
) -> RuntimeResult<(MidiDataFormat, Option<MidiProtocol>)> {
    let (data_format, protocol) = resolve_open_transport(
        operation,
        requested_data_format,
        requested_protocol,
        descriptor.default_data_format,
        descriptor.default_protocol,
        fallback_data_format,
        fallback_protocol,
    )?;

    validate_descriptor_transport_request(operation, descriptor, data_format, protocol)?;

    Ok((data_format, protocol))
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
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "records",
            format!("{operation}: UMP record payload must be a multiple of 4 bytes"),
        ))
        .boxed());
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
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "records",
            format!("{operation}: MIDI 1 byte-stream record payload must not be empty"),
        ))
        .boxed());
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
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "records",
            format!(
                "{operation}: complete MIDI 1 byte-stream records must start with one status byte"
            ),
        ))
        .boxed());
    }

    let expected_length = canonical_midi1_message_length(status).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "records",
            format!("{operation}: unsupported MIDI 1 status byte 0x{status:02X}"),
        ))
        .boxed()
    })?;

    if data.len() != expected_length {
        return Err(
            RuntimeError::from(PlatformError::invalid_argument_value(
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
            RuntimeError::from(PlatformError::invalid_argument_value(
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
            RuntimeError::from(PlatformError::invalid_argument_value(
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
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "records",
            format!("{operation}: MIDI 1 SysEx fragment payload bytes must remain 7-bit clean"),
        ))
        .boxed());
    }

    Ok(())
}

/// Return the canonical MIDI 1 message length for one status byte.
pub(crate) fn canonical_midi1_message_length(status: u8) -> Option<usize> {
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
    use super::{
        resolve_descriptor_open_transport, resolve_open_transport, validate_output_record_payload,
    };
    use crate::platform::device::midi::core::{MidiOutputRecordValue, MidiPortDescriptorValue};
    use crate::platform::device::{
        MidiBackend, MidiDataFormat, MidiDataFormatFlags, MidiProtocol, MidiProtocolFlags,
        MidiRecordFraming,
    };
    use crate::platform::diagnostic::PlatformErrorCode;

    /// Build one descriptor row for transport resolution tests.
    fn test_descriptor(
        supported_data_formats: MidiDataFormatFlags,
        default_data_format: Option<MidiDataFormat>,
        supported_protocols: MidiProtocolFlags,
        default_protocol: Option<MidiProtocol>,
    ) -> MidiPortDescriptorValue {
        MidiPortDescriptorValue {
            backend: MidiBackend::CoreMIDI,
            id: "test:input:0".to_string(),
            group_id: None,
            backend_id: Some("0".to_string()),
            name: "Test Input".to_string(),
            group_name: None,
            manufacturer: None,
            model: None,
            version: None,
            supported_data_formats,
            default_data_format,
            supported_protocols,
            default_protocol,
            is_virtual: false,
            is_connected: true,
        }
    }

    /// Reject malformed MIDI 1 byte-stream output records.
    #[test]
    fn test_validate_output_record_payload_rejects_invalid_midi1_records() {
        let invalid_record = MidiOutputRecordValue {
            send_at_ns: None,
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: vec![0x3C, 0x40].into(),
        };

        let error =
            validate_output_record_payload("destack.device.midi.output.write", &invalid_record)
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
            data: vec![0xF0, 0x7D, 0x01].into(),
        };
        validate_output_record_payload("destack.device.midi.output.write", &valid_start)
            .expect("valid SysEx start fragments should pass");

        let invalid_end = MidiOutputRecordValue {
            send_at_ns: None,
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::End,
            data: vec![0xF0, 0x7D, 0xF7].into(),
        };

        let error =
            validate_output_record_payload("destack.device.midi.output.write", &invalid_end)
                .expect_err("mismatched SysEx fragment framing should be rejected");
        let error = error
            .platform_error()
            .expect("mismatched fragment should surface one platform error");

        assert_eq!(error.code, PlatformErrorCode::InvalidArgumentValue);
    }

    /// Accept one complete SysEx message as one valid Web MIDI byte-stream payload.
    #[test]
    fn test_validate_output_record_payload_accepts_complete_sysex_messages() {
        let valid_sysex = MidiOutputRecordValue {
            send_at_ns: None,
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: vec![0xF0, 0x7D, 0x01, 0x02, 0xF7].into(),
        };

        validate_output_record_payload("destack.device.midi.output.write", &valid_sysex)
            .expect("complete SysEx messages should pass payload validation");
    }

    /// Reject concatenated byte-stream messages inside one low-level output record.
    #[test]
    fn test_validate_output_record_payload_rejects_multiple_messages_in_one_complete_record() {
        let invalid_record = MidiOutputRecordValue {
            send_at_ns: None,
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: vec![0x90, 0x3C, 0x40, 0x80, 0x3C, 0x00].into(),
        };

        let error =
            validate_output_record_payload("destack.device.midi.output.write", &invalid_record)
                .expect_err("low-level records should not pack multiple MIDI messages together");
        let error = error
            .platform_error()
            .expect("invalid payload should surface one platform error");

        assert_eq!(error.code, PlatformErrorCode::InvalidArgumentValue);
    }

    /// Resolve explicit open requests against descriptor defaults and backend fallbacks.
    #[test]
    fn test_resolve_open_transport_prefers_requested_then_descriptor_then_backend_defaults() {
        let resolved_transport = resolve_open_transport(
            "destack.device.midi.input.port.open",
            None,
            None,
            Some(MidiDataFormat::Ump),
            Some(MidiProtocol::Midi2),
            MidiDataFormat::Midi1Bytes,
            Some(MidiProtocol::Midi1),
        )
        .expect("descriptor defaults should resolve the transport");

        assert_eq!(
            resolved_transport,
            (MidiDataFormat::Ump, Some(MidiProtocol::Midi2)),
        );
    }

    /// Reject incompatible explicit open requests before descriptor defaults can hide the mismatch.
    #[test]
    fn test_resolve_open_transport_rejects_explicit_midi2_requests_without_ump_transport() {
        let error = resolve_open_transport(
            "destack.device.midi.input.port.open",
            None,
            Some(MidiProtocol::Midi2),
            Some(MidiDataFormat::Ump),
            Some(MidiProtocol::Midi2),
            MidiDataFormat::Midi1Bytes,
            None,
        )
        .expect_err("backend fallback transport should not hide one invalid explicit request");
        let error = error
            .platform_error()
            .expect("invalid transport requests should surface one platform error");

        assert_eq!(error.code, PlatformErrorCode::InvalidArgumentValue);
    }

    /// Resolve descriptor-backed open requests through one shared validation path.
    #[test]
    fn test_resolve_descriptor_open_transport_prefers_descriptor_defaults() {
        let descriptor = test_descriptor(
            MidiDataFormatFlags(0b10),
            Some(MidiDataFormat::Ump),
            MidiProtocolFlags(0b10),
            Some(MidiProtocol::Midi2),
        );

        let resolved_transport = resolve_descriptor_open_transport(
            "destack.device.midi.input.port.open",
            &descriptor,
            None,
            None,
            MidiDataFormat::Midi1Bytes,
            Some(MidiProtocol::Midi1),
        )
        .expect("descriptor defaults should resolve one supported transport");

        assert_eq!(
            resolved_transport,
            (MidiDataFormat::Ump, Some(MidiProtocol::Midi2)),
        );
    }

    /// Reject descriptor-backed requests that ask for unsupported transports.
    #[test]
    fn test_resolve_descriptor_open_transport_rejects_unsupported_descriptor_shape() {
        let descriptor = test_descriptor(
            MidiDataFormatFlags(0b01),
            Some(MidiDataFormat::Midi1Bytes),
            MidiProtocolFlags(0b01),
            Some(MidiProtocol::Midi1),
        );

        let error = resolve_descriptor_open_transport(
            "destack.device.midi.output.port.open",
            &descriptor,
            Some(MidiDataFormat::Ump),
            Some(MidiProtocol::Midi2),
            MidiDataFormat::Midi1Bytes,
            Some(MidiProtocol::Midi1),
        )
        .expect_err("unsupported descriptor transport requests should fail loudly");
        let error = error
            .platform_error()
            .expect("unsupported descriptor requests should surface one platform error");

        assert_eq!(error.code, PlatformErrorCode::InvalidArgumentValue);
    }
}
