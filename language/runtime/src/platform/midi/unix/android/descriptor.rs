use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::{
    AndroidHostMidiEventHeader, AndroidHostMidiInputRecordHeader, AndroidHostMidiOpenedPortHeader,
    AndroidHostMidiOutputRecordHeader, AndroidHostMidiPortDescriptorHeader,
};
use crate::platform::PlatformError;
use crate::platform::midi::core::{
    MidiEventMetadataValue, MidiEventValue, MidiInputRecordValue, MidiOutputRecordValue,
    MidiPortDescriptorValue,
};
use crate::platform::midi::{
    MidiBackend, MidiDataFormat, MidiDataFormatFlags, MidiEventSource, MidiPortDirection,
    MidiProtocolFlags,
};
use std::sync::Arc;

use super::core::{
    MIDI_EVENT_KIND_CODE_BACKEND_DISCONNECTED, MIDI_EVENT_KIND_CODE_PORT_ADDED,
    MIDI_EVENT_KIND_CODE_PORT_CHANGED, MIDI_EVENT_KIND_CODE_PORT_REMOVED,
    MIDI_PORT_DIRECTION_CODE_INPUT, MIDI_PORT_DIRECTION_CODE_OUTPUT, data_format_code,
    decode_data_format_code, decode_framing_code, decode_protocol_code, framing_code,
    protocol_code,
};

/// Decode one required string slice from one shared payload blob.
fn decode_required_string(
    blob: &[u8],
    offset: u32,
    len: u32,
    field: &'static str,
    operation: &'static str,
) -> RuntimeResult<String> {
    let start = offset as usize;
    let end = start.saturating_add(len as usize);
    if end > blob.len() {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi returned one out-of-range {field} string slice"
        )))
        .boxed());
    }

    let value = std::str::from_utf8(&blob[start..end]).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi returned one non-utf8 {field} string"
        )))
        .boxed()
    })?;

    Ok(value.to_string())
}

/// Decode one optional string slice from one shared payload blob.
fn decode_optional_string(
    blob: &[u8],
    offset: u32,
    len: u32,
    field: &'static str,
    operation: &'static str,
) -> RuntimeResult<Option<String>> {
    if len == 0 {
        return Ok(None);
    }

    Ok(Some(decode_required_string(
        blob, offset, len, field, operation,
    )?))
}

/// Decode one Android MIDI descriptor header.
fn decode_port_descriptor(
    backend: MidiBackend,
    header: &AndroidHostMidiPortDescriptorHeader,
    string_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<MidiPortDescriptorValue> {
    Ok(MidiPortDescriptorValue {
        backend,
        id: decode_required_string(
            string_bytes,
            header.id_offset,
            header.id_len,
            "id",
            operation,
        )?,
        group_id: decode_optional_string(
            string_bytes,
            header.group_id_offset,
            header.group_id_len,
            "groupId",
            operation,
        )?,
        backend_id: decode_optional_string(
            string_bytes,
            header.backend_id_offset,
            header.backend_id_len,
            "backendId",
            operation,
        )?,
        name: decode_required_string(
            string_bytes,
            header.name_offset,
            header.name_len,
            "name",
            operation,
        )?,
        group_name: decode_optional_string(
            string_bytes,
            header.group_name_offset,
            header.group_name_len,
            "groupName",
            operation,
        )?,
        manufacturer: decode_optional_string(
            string_bytes,
            header.manufacturer_offset,
            header.manufacturer_len,
            "manufacturer",
            operation,
        )?,
        model: decode_optional_string(
            string_bytes,
            header.model_offset,
            header.model_len,
            "model",
            operation,
        )?,
        version: decode_optional_string(
            string_bytes,
            header.version_offset,
            header.version_len,
            "version",
            operation,
        )?,
        supported_data_formats: MidiDataFormatFlags(header.supported_data_formats),
        default_data_format: decode_data_format_code_optional(
            header.default_data_format,
            operation,
        )?,
        supported_protocols: MidiProtocolFlags(header.supported_protocols),
        default_protocol: decode_protocol_code(header.default_protocol, operation)?,
        is_virtual: header.is_virtual != 0,
        is_connected: header.is_connected != 0,
    })
}

/// Decode one optional Android host callback data-format code.
fn decode_data_format_code_optional(
    code: u32,
    operation: &'static str,
) -> RuntimeResult<Option<MidiDataFormat>> {
    if code == 0 {
        return Ok(None);
    }

    Ok(Some(decode_data_format_code(code, operation)?))
}

/// Decode one vector of Android descriptor rows.
pub(super) fn decode_port_descriptors(
    backend: MidiBackend,
    headers: &[AndroidHostMidiPortDescriptorHeader],
    string_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<MidiPortDescriptorValue>> {
    let mut descriptors = Vec::with_capacity(headers.len());

    // decode each descriptor row
    for header in headers {
        descriptors.push(decode_port_descriptor(
            backend,
            header,
            string_bytes,
            operation,
        )?);
    }

    Ok(descriptors)
}

/// Decode one opened Android MIDI session payload.
pub(super) fn decode_opened_port(
    backend: MidiBackend,
    opened_port: &AndroidHostMidiOpenedPortHeader,
    string_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<(u64, MidiPortDescriptorValue)> {
    let descriptor =
        decode_port_descriptor(backend, &opened_port.descriptor, string_bytes, operation)?;

    Ok((opened_port.session_id, descriptor))
}

/// Decode one vector of Android input records.
pub(super) fn decode_input_records(
    headers: &[AndroidHostMidiInputRecordHeader],
    blob_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<MidiInputRecordValue>> {
    let mut records = Vec::with_capacity(headers.len());

    // decode each input-record row
    for header in headers {
        let data_start = header.data_offset as usize;
        let data_end = data_start.saturating_add(header.data_len as usize);
        if data_end > blob_bytes.len() {
            return Err(RuntimeError::from(PlatformError::invalid_data(format!(
                "{operation}: android host midi returned one out-of-range input payload slice"
            )))
            .boxed());
        }

        let source_id = decode_optional_string(
            blob_bytes,
            header.source_id_offset,
            header.source_id_len,
            "sourceId",
            operation,
        )?
        .map(Arc::<str>::from);

        records.push(MidiInputRecordValue {
            received_at_ns: header.received_at_ns,
            source_id,
            data_format: decode_data_format_code(header.data_format, operation)?,
            protocol: decode_protocol_code(header.protocol, operation)?,
            framing: decode_framing_code(header.framing, operation)?,
            data: blob_bytes[data_start..data_end].to_vec().into(),
        });
    }

    Ok(records)
}

/// Encode one vector of Android output records.
pub(super) fn encode_output_records(
    records: &[MidiOutputRecordValue],
    operation: &'static str,
) -> RuntimeResult<(Vec<AndroidHostMidiOutputRecordHeader>, Vec<u8>)> {
    let mut headers = Vec::with_capacity(records.len());
    let mut blob_bytes = Vec::new();

    // encode each output-record row into one shared blob
    for record in records {
        let data_offset = u32::try_from(blob_bytes.len()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_data(format!(
                "{operation}: android host midi output payload offset exceeds ABI width"
            )))
            .boxed()
        })?;
        let data_len = u32::try_from(record.data.len()).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_data(format!(
                "{operation}: android host midi output payload length exceeds ABI width"
            )))
            .boxed()
        })?;

        blob_bytes.extend_from_slice(&record.data);

        headers.push(AndroidHostMidiOutputRecordHeader {
            send_at_ns: record.send_at_ns.unwrap_or(0),
            has_send_at: u32::from(record.send_at_ns.is_some()),
            data_offset,
            data_len,
            data_format: data_format_code(record.data_format),
            protocol: record.protocol.map(protocol_code).unwrap_or(0),
            framing: framing_code(record.framing),
        });
    }

    Ok((headers, blob_bytes))
}

/// Decode one Android host event direction code.
fn decode_direction_code(code: u32, operation: &'static str) -> RuntimeResult<MidiPortDirection> {
    match code {
        MIDI_PORT_DIRECTION_CODE_INPUT => Ok(MidiPortDirection::Input),
        MIDI_PORT_DIRECTION_CODE_OUTPUT => Ok(MidiPortDirection::Output),
        _ => Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: android host midi returned unsupported event direction code {code}"
        )))
        .boxed()),
    }
}

/// Decode one vector of Android native topology events.
pub(super) fn decode_native_events(
    headers: &[AndroidHostMidiEventHeader],
    string_bytes: &[u8],
    next_sequence: &mut u64,
    operation: &'static str,
) -> RuntimeResult<Vec<MidiEventValue>> {
    let mut events = Vec::with_capacity(headers.len());

    // decode each event row against the shared string payload
    for header in headers {
        let metadata = MidiEventMetadataValue {
            timestamp_ns: header.timestamp_ns,
            sequence: *next_sequence,
            dropped_count: header.dropped_count,
            source: MidiEventSource::Native,
            backend: MidiBackend::AndroidMidi,
        };
        *next_sequence = (*next_sequence).saturating_add(1);

        let event = match header.kind {
            MIDI_EVENT_KIND_CODE_PORT_ADDED => MidiEventValue::PortAdded {
                metadata,
                direction: decode_direction_code(header.direction, operation)?,
                descriptor: decode_port_descriptor(
                    MidiBackend::AndroidMidi,
                    &header.descriptor,
                    string_bytes,
                    operation,
                )?,
            },
            MIDI_EVENT_KIND_CODE_PORT_REMOVED => MidiEventValue::PortRemoved {
                metadata,
                direction: decode_direction_code(header.direction, operation)?,
                id: decode_required_string(
                    string_bytes,
                    header.id_offset,
                    header.id_len,
                    "id",
                    operation,
                )?,
                group_id: decode_optional_string(
                    string_bytes,
                    header.group_id_offset,
                    header.group_id_len,
                    "groupId",
                    operation,
                )?,
            },
            MIDI_EVENT_KIND_CODE_PORT_CHANGED => MidiEventValue::PortChanged {
                metadata,
                direction: decode_direction_code(header.direction, operation)?,
                descriptor: decode_port_descriptor(
                    MidiBackend::AndroidMidi,
                    &header.descriptor,
                    string_bytes,
                    operation,
                )?,
            },
            MIDI_EVENT_KIND_CODE_BACKEND_DISCONNECTED => MidiEventValue::BackendDisconnected {
                metadata,
                flags: header.flags,
            },
            _ => {
                return Err(RuntimeError::from(PlatformError::invalid_data(format!(
                    "{operation}: android host midi returned unsupported event kind code {}",
                    header.kind
                )))
                .boxed());
            }
        };

        events.push(event);
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{
        AndroidHostMidiEventHeader, AndroidHostMidiInputRecordHeader,
        AndroidHostMidiPortDescriptorHeader,
    };
    use crate::platform::midi::core::{MidiInputRecordValue, MidiOutputRecordValue};
    use crate::platform::midi::{
        MIDI_PROTOCOL_FLAG_MIDI1, MidiBackend, MidiDataFormat, MidiProtocol, MidiRecordFraming,
    };
    use crate::tests::platform::error_code_from_runtime_error;

    /// Preserve scheduled-write metadata and blob offsets when encoding Android host output records.
    #[test]
    fn test_encode_output_records_preserves_scheduled_write_metadata() {
        let first_record = MidiOutputRecordValue {
            send_at_ns: Some(123),
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: vec![0x90, 0x40, 0x7f].into(),
        };
        let second_record = MidiOutputRecordValue {
            send_at_ns: None,
            data_format: MidiDataFormat::Ump,
            protocol: Some(MidiProtocol::Midi2),
            framing: MidiRecordFraming::Start,
            data: vec![0x41, 0x10, 0x00, 0x00].into(),
        };

        let (headers, blob_bytes) = encode_output_records(
            &[first_record.clone(), second_record.clone()],
            "test.encode_output_records",
        )
        .expect("android output record encoding should succeed");

        assert_eq!(headers.len(), 2);
        assert_eq!(blob_bytes, vec![0x90, 0x40, 0x7f, 0x41, 0x10, 0x00, 0x00]);

        assert_eq!(headers[0].send_at_ns, 123);
        assert_eq!(headers[0].has_send_at, 1);
        assert_eq!(headers[0].data_offset, 0);
        assert_eq!(headers[0].data_len, 3);
        assert_eq!(
            headers[0].data_format,
            data_format_code(first_record.data_format)
        );
        assert_eq!(
            headers[0].protocol,
            protocol_code(
                first_record
                    .protocol
                    .expect("first record protocol should be present")
            )
        );
        assert_eq!(headers[0].framing, framing_code(first_record.framing));

        assert_eq!(headers[1].send_at_ns, 0);
        assert_eq!(headers[1].has_send_at, 0);
        assert_eq!(headers[1].data_offset, 3);
        assert_eq!(headers[1].data_len, 4);
        assert_eq!(
            headers[1].data_format,
            data_format_code(second_record.data_format)
        );
        assert_eq!(
            headers[1].protocol,
            protocol_code(
                second_record
                    .protocol
                    .expect("second record protocol should be present")
            )
        );
        assert_eq!(headers[1].framing, framing_code(second_record.framing));
    }

    /// Preserve host-provided receive timestamps and source ids when decoding Android input records.
    #[test]
    fn test_decode_input_records_preserves_received_timestamps() {
        let blob_bytes = b"source-id\x90\x40\x7f".to_vec();
        let headers = [AndroidHostMidiInputRecordHeader {
            received_at_ns: 777,
            source_id_offset: 0,
            source_id_len: 9,
            data_offset: 9,
            data_len: 3,
            data_format: data_format_code(MidiDataFormat::Midi1Bytes),
            protocol: protocol_code(MidiProtocol::Midi1),
            framing: framing_code(MidiRecordFraming::Complete),
        }];

        let records = decode_input_records(&headers, &blob_bytes, "test.decode_input_records")
            .expect("android input record decoding should succeed");

        assert_eq!(
            records,
            vec![MidiInputRecordValue {
                received_at_ns: 777,
                source_id: Some("source-id".into()),
                data_format: MidiDataFormat::Midi1Bytes,
                protocol: Some(MidiProtocol::Midi1),
                framing: MidiRecordFraming::Complete,
                data: vec![0x90, 0x40, 0x7f].into(),
            }]
        );
    }

    /// Preserve Android native-event sequence, ids, and timestamps when decoding topology payloads.
    #[test]
    fn test_decode_native_events_preserves_sequence_and_descriptor_identity() {
        let blob_bytes = b"idname".to_vec();
        let headers = [AndroidHostMidiEventHeader {
            timestamp_ns: 99,
            dropped_count: 2,
            kind: MIDI_EVENT_KIND_CODE_PORT_ADDED,
            direction: MIDI_PORT_DIRECTION_CODE_INPUT,
            flags: 0,
            id_offset: 0,
            id_len: 0,
            group_id_offset: 0,
            group_id_len: 0,
            descriptor: AndroidHostMidiPortDescriptorHeader {
                id_offset: 0,
                id_len: 2,
                name_offset: 2,
                name_len: 4,
                supported_data_formats: data_format_code(MidiDataFormat::Midi1Bytes),
                default_data_format: data_format_code(MidiDataFormat::Midi1Bytes),
                supported_protocols: MIDI_PROTOCOL_FLAG_MIDI1.0,
                default_protocol: protocol_code(MidiProtocol::Midi1),
                is_virtual: 1,
                is_connected: 1,
                ..AndroidHostMidiPortDescriptorHeader::default()
            },
        }];
        let mut next_sequence = 41;

        let events = decode_native_events(
            &headers,
            &blob_bytes,
            &mut next_sequence,
            "test.decode_native_events",
        )
        .expect("android native event decoding should succeed");

        assert_eq!(next_sequence, 42);
        assert_eq!(events.len(), 1);

        let MidiEventValue::PortAdded {
            metadata,
            direction,
            descriptor,
        } = &events[0]
        else {
            panic!("android native event decoding should preserve the port-added kind");
        };

        assert_eq!(metadata.timestamp_ns, 99);
        assert_eq!(metadata.sequence, 41);
        assert_eq!(metadata.dropped_count, 2);
        assert_eq!(*direction, MidiPortDirection::Input);
        assert_eq!(descriptor.id, "id");
        assert_eq!(descriptor.name, "name");
        assert!(descriptor.is_virtual);
    }

    /// Reject malformed Android native-event direction codes loudly.
    #[test]
    fn test_decode_native_events_rejects_unknown_direction_codes() {
        let headers = [AndroidHostMidiEventHeader {
            timestamp_ns: 0,
            dropped_count: 0,
            kind: MIDI_EVENT_KIND_CODE_PORT_REMOVED,
            direction: 99,
            flags: 0,
            id_offset: 0,
            id_len: 2,
            group_id_offset: 0,
            group_id_len: 0,
            descriptor: AndroidHostMidiPortDescriptorHeader::default(),
        }];
        let mut next_sequence = 1;
        let error = decode_native_events(
            &headers,
            b"id",
            &mut next_sequence,
            "test.decode_native_events",
        )
        .expect_err("unknown android native event direction codes should be rejected");

        assert_eq!(
            error_code_from_runtime_error(error.as_ref()),
            Some(crate::platform::diagnostic::PlatformErrorCode::IoInvalidData)
        );
    }

    /// Reject malformed Android host descriptor payload slices loudly.
    #[test]
    fn test_decode_port_descriptors_rejects_out_of_range_name_slices() {
        let headers = [AndroidHostMidiPortDescriptorHeader {
            id_offset: 0,
            id_len: 2,
            name_offset: 8,
            name_len: 4,
            supported_data_formats: data_format_code(MidiDataFormat::Midi1Bytes),
            supported_protocols: MIDI_PROTOCOL_FLAG_MIDI1.0,
            default_data_format: data_format_code(MidiDataFormat::Midi1Bytes),
            default_protocol: protocol_code(MidiProtocol::Midi1),
            is_virtual: 0,
            is_connected: 1,
            ..AndroidHostMidiPortDescriptorHeader::default()
        }];
        let error = decode_port_descriptors(
            MidiBackend::AndroidMidi,
            &headers,
            b"id",
            "test.decode_port_descriptors",
        )
        .expect_err("out-of-range host name slices should be rejected");

        assert_eq!(
            error_code_from_runtime_error(error.as_ref()),
            Some(crate::platform::diagnostic::PlatformErrorCode::IoInvalidData)
        );
    }
}
