use super::{
    assert_ok_or_expected_error, assert_platform_error_codes, decode_backend_descriptors_full,
    decode_port_descriptors, harness_input_open_options_for_transport,
    harness_output_open_options_for_transport, harness_output_records, harness_port_list_options,
    harness_string, harness_virtual_output_create_options_for_backend_transport,
    preferred_transport_pair, supports_data_format, supports_protocol, with_harness_context,
};
use crate::platform::core::BackendSupport;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::midi::{
    MIDI_BACKEND_CAP_UMP, MIDI_BACKEND_CAP_VIRTUAL_OUTPUT, MidiBackend, MidiDataFormat,
    MidiOutputRecord, MidiProtocol, MidiRecordFraming,
};

/// Open listed MIDI ports through their advertised exact transport pairs when available.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_listed_ports_open_with_their_advertised_exact_transport_pairs() {
    with_harness_context(|mut context| {
        let list_options = harness_port_list_options(&mut context);

        // input rows
        let input_result = assert_ok_or_expected_error(
            context.destack_midi_input_port_list(list_options),
            &[PlatformErrorCode::NotSupported],
        )?;

        if let Some(input_result) = input_result {
            let input_descriptors = decode_port_descriptors(&mut context, input_result)?;

            for (
                id,
                _name,
                _is_connected,
                supported_data_formats,
                default_data_format,
                supported_protocols,
                default_protocol,
            ) in input_descriptors
            {
                let Some((data_format, protocol)) = preferred_transport_pair(
                    supported_data_formats,
                    default_data_format,
                    supported_protocols,
                    default_protocol,
                ) else {
                    continue;
                };

                let id = harness_string(&mut context, &id)?;
                let options = harness_input_open_options_for_transport(
                    &mut context,
                    Some(data_format),
                    Some(protocol),
                );
                let handle = context.destack_midi_input_port_open(id, options)?;

                context.destack_midi_input_port_close(handle)?;
            }
        }

        let list_options = harness_port_list_options(&mut context);

        // output rows
        let output_result = assert_ok_or_expected_error(
            context.destack_midi_output_port_list(list_options),
            &[PlatformErrorCode::NotSupported],
        )?;

        if let Some(output_result) = output_result {
            let output_descriptors = decode_port_descriptors(&mut context, output_result)?;

            for (
                id,
                _name,
                _is_connected,
                supported_data_formats,
                default_data_format,
                supported_protocols,
                default_protocol,
            ) in output_descriptors
            {
                let Some((data_format, protocol)) = preferred_transport_pair(
                    supported_data_formats,
                    default_data_format,
                    supported_protocols,
                    default_protocol,
                ) else {
                    continue;
                };

                let id = harness_string(&mut context, &id)?;
                let options = harness_output_open_options_for_transport(
                    &mut context,
                    Some(data_format),
                    Some(protocol),
                );
                let handle = context.destack_midi_output_port_open(id, options)?;

                context.destack_midi_output_port_close(handle)?;
            }
        }

        Ok(())
    });
}

/// Reject incompatible MIDI 2 over byte-stream transport when opening listed ports.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_port_open_rejects_midi2_protocol_on_byte_stream_transport() {
    with_harness_context(|mut context| {
        let list_options = harness_port_list_options(&mut context);

        // input rows
        let input_result = assert_ok_or_expected_error(
            context.destack_midi_input_port_list(list_options),
            &[PlatformErrorCode::NotSupported],
        )?;

        if let Some(input_result) = input_result {
            let input_descriptors = decode_port_descriptors(&mut context, input_result)?;

            if let Some((
                id,
                _name,
                _is_connected,
                _formats,
                _default_format,
                _protocols,
                _default_protocol,
            )) = input_descriptors.first()
            {
                let id = harness_string(&mut context, id)?;
                let options = harness_input_open_options_for_transport(
                    &mut context,
                    Some(MidiDataFormat::Midi1Bytes),
                    Some(MidiProtocol::Midi2),
                );

                assert_platform_error_codes(
                    context.destack_midi_input_port_open(id, options),
                    &[
                        PlatformErrorCode::NotSupported,
                        PlatformErrorCode::InvalidArgument,
                        PlatformErrorCode::InvalidArgumentValue,
                    ],
                )?;
            }
        }

        let list_options = harness_port_list_options(&mut context);

        // output rows
        let output_result = assert_ok_or_expected_error(
            context.destack_midi_output_port_list(list_options),
            &[PlatformErrorCode::NotSupported],
        )?;

        if let Some(output_result) = output_result {
            let output_descriptors = decode_port_descriptors(&mut context, output_result)?;

            if let Some((
                id,
                _name,
                _is_connected,
                _formats,
                _default_format,
                _protocols,
                _default_protocol,
            )) = output_descriptors.first()
            {
                let id = harness_string(&mut context, id)?;
                let options = harness_output_open_options_for_transport(
                    &mut context,
                    Some(MidiDataFormat::Midi1Bytes),
                    Some(MidiProtocol::Midi2),
                );

                assert_platform_error_codes(
                    context.destack_midi_output_port_open(id, options),
                    &[
                        PlatformErrorCode::NotSupported,
                        PlatformErrorCode::InvalidArgument,
                        PlatformErrorCode::InvalidArgumentValue,
                    ],
                )?;
            }
        }

        Ok(())
    });
}

/// Reject malformed UMP record payload lengths when one UMP-capable virtual output exists.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_output_write_rejects_misaligned_ump_payloads() {
    with_harness_context(|mut context| {
        // backend rows
        let result = assert_ok_or_expected_error(
            context.destack_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(result) = result else {
            return Ok(());
        };

        // one UMP-capable backend
        let descriptors = decode_backend_descriptors_full(&mut context, result)?;
        for (
            backend,
            _name,
            support,
            _priority,
            capability_flags,
            supported_data_formats,
            supported_protocols,
        ) in descriptors
        {
            if support != BackendSupport::Available || backend == MidiBackend::Null {
                continue;
            }

            if capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_OUTPUT.0 == 0 {
                continue;
            }

            if capability_flags.0 & MIDI_BACKEND_CAP_UMP.0 == 0 {
                continue;
            }

            if !supports_data_format(supported_data_formats, MidiDataFormat::Ump) {
                continue;
            }

            let protocol = if supports_protocol(supported_protocols, MidiProtocol::Midi2) {
                MidiProtocol::Midi2
            } else if supports_protocol(supported_protocols, MidiProtocol::Midi1) {
                MidiProtocol::Midi1
            } else {
                continue;
            };

            let name = format!("Destack MIDI UMP Output {backend:?}");
            let options = harness_virtual_output_create_options_for_backend_transport(
                &mut context,
                backend,
                &name,
                MidiDataFormat::Ump,
                protocol,
            )?;
            let handle = context.destack_midi_output_virtual_create(options)?;

            // malformed ump record
            let record = MidiOutputRecord {
                send_at_ns: None,
                data_format: MidiDataFormat::Ump,
                protocol: Some(protocol),
                framing: MidiRecordFraming::Complete,
                data: context.call_context.store_slice(vec![0x40, 0x90, 0x3C]),
            };
            let records = harness_output_records(&mut context, &[record])?;
            assert_platform_error_codes(
                context.destack_midi_output_write(handle, records),
                &[
                    PlatformErrorCode::InvalidArgument,
                    PlatformErrorCode::InvalidArgumentValue,
                ],
            )?;

            // resource teardown
            context.destack_midi_output_port_close(handle)?;

            break;
        }

        Ok(())
    });
}

/// Reject malformed MIDI 1 byte-stream records when one byte-stream virtual output exists.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_output_write_rejects_invalid_midi1_byte_stream_records() {
    with_harness_context(|mut context| {
        // backend rows
        let result = assert_ok_or_expected_error(
            context.destack_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(result) = result else {
            return Ok(());
        };

        // one midi1 byte-stream virtual output
        let descriptors = decode_backend_descriptors_full(&mut context, result)?;
        for (
            backend,
            _name,
            support,
            _priority,
            capability_flags,
            supported_data_formats,
            supported_protocols,
        ) in descriptors
        {
            if support != BackendSupport::Available || backend == MidiBackend::Null {
                continue;
            }

            if capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_OUTPUT.0 == 0 {
                continue;
            }

            if !supports_data_format(supported_data_formats, MidiDataFormat::Midi1Bytes) {
                continue;
            }

            let protocol = if supports_protocol(supported_protocols, MidiProtocol::Midi1) {
                MidiProtocol::Midi1
            } else {
                continue;
            };

            let name = format!("Destack MIDI Invalid MIDI1 Output {backend:?}");
            let options = harness_virtual_output_create_options_for_backend_transport(
                &mut context,
                backend,
                &name,
                MidiDataFormat::Midi1Bytes,
                protocol,
            )?;
            let handle = context.destack_midi_output_virtual_create(options)?;

            // malformed midi1 record
            let record = MidiOutputRecord {
                send_at_ns: None,
                data_format: MidiDataFormat::Midi1Bytes,
                protocol: Some(protocol),
                framing: MidiRecordFraming::Complete,
                data: context.call_context.store_slice(vec![0x3C, 0x40]),
            };
            let records = harness_output_records(&mut context, &[record])?;
            assert_platform_error_codes(
                context.destack_midi_output_write(handle, records),
                &[
                    PlatformErrorCode::InvalidArgument,
                    PlatformErrorCode::InvalidArgumentValue,
                ],
            )?;

            // resource teardown
            context.destack_midi_output_port_close(handle)?;

            break;
        }

        Ok(())
    });
}
