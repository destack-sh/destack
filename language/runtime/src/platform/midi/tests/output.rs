use super::{
    assert_ok_or_expected_error, assert_platform_error_codes, decode_port_descriptor,
    decode_port_descriptors, harness_output_open_options, harness_output_records,
    harness_port_list_options, harness_string, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::midi::{
    MIDI_DATA_FORMAT_FLAG_UMP, MIDI_PROTOCOL_FLAG_MIDI2, MidiDataFormat, MidiOutputRecord,
    MidiProtocol, MidiRecordFraming,
};
use crate::platform::resource;

/// List MIDI output ports or report one expected unsupported host error.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_output_port_list_returns_stable_ids_when_available_or_expected_error() {
    with_harness_context(|mut context| {
        let options = harness_port_list_options(&mut context);

        // output list
        let output_result = assert_ok_or_expected_error(
            context.destack_midi_output_port_list(options),
            &[PlatformErrorCode::NotSupported],
        )?;

        if let Some(output_result) = output_result {
            let descriptors = decode_port_descriptors(&mut context, output_result)?;

            for (
                id,
                name,
                _is_connected,
                supported_data_formats,
                default_data_format,
                supported_protocols,
                default_protocol,
            ) in descriptors
            {
                assert!(!id.is_empty(), "midi output port id should not be empty");
                assert!(
                    !name.is_empty(),
                    "midi output port name should not be empty"
                );
                assert!(
                    supported_data_formats.0 != 0,
                    "midi output port should advertise at least one data format",
                );
                assert!(
                    supported_protocols.0 != 0,
                    "midi output port should advertise at least one protocol",
                );

                // midi 2 semantics require ump transport
                if supported_protocols.0 & MIDI_PROTOCOL_FLAG_MIDI2.0 != 0 {
                    assert!(
                        supported_data_formats.0 & MIDI_DATA_FORMAT_FLAG_UMP.0 != 0,
                        "midi output ports that advertise MIDI 2 should advertise UMP transport",
                    );
                }

                // default transport must be inside the advertised transport mask
                if let Some(default_data_format) = default_data_format {
                    let default_data_format_flag = 1u32 << (default_data_format as u32 - 1);
                    assert!(
                        supported_data_formats.0 & default_data_format_flag != 0,
                        "midi output default data format should be included in supported data formats",
                    );
                }

                // default protocol must be inside the advertised protocol mask
                if let Some(default_protocol) = default_protocol {
                    let default_protocol_flag = 1u32 << (default_protocol as u32 - 1);
                    assert!(
                        supported_protocols.0 & default_protocol_flag != 0,
                        "midi output default protocol should be included in supported protocols",
                    );

                    if default_protocol == MidiProtocol::Midi2 {
                        assert!(
                            supported_data_formats.0 & MIDI_DATA_FORMAT_FLAG_UMP.0 != 0,
                            "midi output ports with default MIDI 2 protocol should advertise UMP transport",
                        );
                    }
                }
            }
        }

        Ok(())
    });
}

/// Open listed MIDI output ports and roundtrip their descriptors when endpoints are present.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_listed_output_ports_open_and_describe_when_available() {
    with_harness_context(|mut context| {
        let list_options = harness_port_list_options(&mut context);

        // output open and descriptor
        let output_result = assert_ok_or_expected_error(
            context.destack_midi_output_port_list(list_options),
            &[PlatformErrorCode::NotSupported],
        )?;

        if let Some(output_result) = output_result {
            let descriptors = decode_port_descriptors(&mut context, output_result)?;

            if let Some((
                id,
                _name,
                _is_connected,
                _formats,
                _default_format,
                _protocols,
                _default_protocol,
            )) = descriptors.first()
            {
                let id_value = id.clone();
                let id = harness_string(&mut context, &id_value)?;
                let options = harness_output_open_options(&mut context);
                let handle = context.destack_midi_output_port_open(id, options)?;

                let descriptor = context.destack_midi_output_port_descriptor(handle)?;
                let (opened_id, _backend_id, _name, _is_virtual, _is_connected, _format, _protocol) =
                    decode_port_descriptor(&mut context, descriptor)?;
                assert_eq!(opened_id, id_value);

                context.destack_midi_output_port_close(handle)?;
            }
        }

        Ok(())
    });
}

/// Flush opened MIDI output ports successfully when endpoints are present.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_listed_output_ports_flush_when_available() {
    with_harness_context(|mut context| {
        let list_options = harness_port_list_options(&mut context);

        // output flush
        let output_result = assert_ok_or_expected_error(
            context.destack_midi_output_port_list(list_options),
            &[PlatformErrorCode::NotSupported],
        )?;

        if let Some(output_result) = output_result {
            let descriptors = decode_port_descriptors(&mut context, output_result)?;

            if let Some((
                id,
                _name,
                _is_connected,
                _formats,
                _default_format,
                _protocols,
                _default_protocol,
            )) = descriptors.first()
            {
                let id = harness_string(&mut context, id)?;
                let options = harness_output_open_options(&mut context);
                let handle = context.destack_midi_output_port_open(id, options)?;

                context.destack_midi_output_flush(handle)?;
                context.destack_midi_output_port_close(handle)?;
            }
        }

        Ok(())
    });
}

/// Reject missing MIDI output endpoints and invalid handles with one expected error set.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_missing_output_endpoints_and_handles_fail_with_expected_errors() {
    with_harness_context(|mut context| {
        let missing_output_id = harness_string(&mut context, "__destack_missing_midi_output__")?;
        let output_options = harness_output_open_options(&mut context);

        // open failures
        assert_platform_error_codes(
            context.destack_midi_output_port_open(missing_output_id, output_options),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::InvalidArgument,
                PlatformErrorCode::InvalidArgumentValue,
            ],
        )?;

        let missing_output_handle = resource::MidiOutputPortHandle(resource::ResourceId(0));
        let one_record = MidiOutputRecord {
            send_at_ns: None,
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: None,
            framing: MidiRecordFraming::Complete,
            data: context.call_context.store_slice(vec![0x90, 0x3C, 0x40]),
        };
        let records = harness_output_records(&mut context, &[one_record])?;

        // handle failures
        assert_platform_error_codes(
            context.destack_midi_output_port_close(missing_output_handle),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::InvalidArgument,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_midi_output_port_descriptor(missing_output_handle),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::InvalidArgument,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_midi_output_write(missing_output_handle, records),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::InvalidArgument,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_midi_output_flush(missing_output_handle),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::InvalidArgument,
            ],
        )?;

        Ok(())
    });
}
