use super::{
    assert_platform_error_codes, decode_backend_descriptors_full, decode_port_descriptors,
    harness_event_open_options_for_backend, harness_output_open_options_for_backend_transport,
    harness_output_records, harness_port_list_options_for_backend, harness_string,
    support_allows_host_execution, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::midi::{
    MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS, MIDI_BACKEND_CAP_TOPOLOGY_EVENTS,
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PORT_DIRECTION_FLAG_INPUT,
    MIDI_PORT_DIRECTION_FLAG_OUTPUT, MIDI_PROTOCOL_FLAG_MIDI1, MidiBackend, MidiDataFormat,
    MidiEventDeliveryMode, MidiEventSubscriptionFlags, MidiOutputRecord, MidiPortDirectionFlags,
    MidiPortListFlags, MidiProtocol, MidiRecordFraming,
};

/// Match the WinMM backend row to its expected capability contract.
#[test]
fn test_midi_winmm_backend_row_matches_expected_capability_contract() {
    with_harness_context(|mut context| {
        // backend rows
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let winmm_row = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::WinMM)
            .expect("midi backend list should include a WinMM selector row");

        let (
            _backend,
            name,
            support,
            priority,
            capability_flags,
            supported_data_formats,
            supported_protocols,
        ) = winmm_row;

        // winmm row
        assert_eq!(name, "winmm");
        assert!(*priority != 0, "WinMM should participate in auto selection");

        // unavailable rows must stay zeroed
        if !support_allows_host_execution(*support) {
            assert_eq!(capability_flags.0, 0);
            assert_eq!(supported_data_formats.0, 0);
            assert_eq!(supported_protocols.0, 0);

            return Ok(());
        }

        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0 != 0,
            "WinMM should advertise topology events",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0 != 0,
            "WinMM should advertise receive timestamps",
        );
        assert_eq!(
            supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0,
            "WinMM should advertise only MIDI 1 byte-stream transport",
        );
        assert_eq!(
            supported_protocols.0, MIDI_PROTOCOL_FLAG_MIDI1.0,
            "WinMM should advertise only MIDI 1 protocol semantics",
        );

        Ok(())
    })
}

/// Reject native event delivery on WinMM.
#[test]
fn test_midi_winmm_native_event_delivery_reports_not_supported() {
    with_harness_context(|mut context| {
        // native event subscription
        let options = harness_event_open_options_for_backend(
            &mut context,
            MidiBackend::WinMM,
            MidiEventSubscriptionFlags(0),
            MidiPortDirectionFlags(
                MIDI_PORT_DIRECTION_FLAG_INPUT.0 | MIDI_PORT_DIRECTION_FLAG_OUTPUT.0,
            ),
            MidiEventDeliveryMode::NativeOnly,
        );

        assert_platform_error_codes(
            context.destack_midi_event_open(options),
            &[PlatformErrorCode::NotSupported],
        )?;

        Ok(())
    })
}

/// Advertise stable WinMM endpoint ids and MIDI 1 transport when ports are present.
#[test]
fn test_midi_winmm_port_rows_advertise_midi1_transport_only_when_present() {
    with_harness_context(|mut context| {
        // backend support
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let support = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::WinMM)
            .map(|(_, _, support, _, _, _, _)| *support)
            .expect("midi backend list should include a WinMM selector row");
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        // input rows
        let input_options = harness_port_list_options_for_backend(
            &mut context,
            MidiBackend::WinMM,
            MidiPortListFlags(0),
        );
        let input_rows = context.destack_midi_input_port_list(input_options)?;
        let input_rows = decode_port_descriptors(&mut context, input_rows)?;

        for (
            id,
            name,
            _is_connected,
            supported_data_formats,
            _default_data_format,
            supported_protocols,
            _default_protocol,
        ) in input_rows
        {
            assert!(
                id.starts_with("winmm:input:"),
                "WinMM input ids should use the winmm:input prefix"
            );
            assert!(!name.is_empty(), "WinMM input names should not be empty");
            assert_eq!(
                supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0,
                "WinMM input rows should advertise only MIDI 1 byte-stream transport",
            );
            assert_eq!(
                supported_protocols.0, MIDI_PROTOCOL_FLAG_MIDI1.0,
                "WinMM input rows should advertise only MIDI 1 protocol semantics",
            );
        }

        // output rows
        let output_options = harness_port_list_options_for_backend(
            &mut context,
            MidiBackend::WinMM,
            MidiPortListFlags(0),
        );
        let output_rows = context.destack_midi_output_port_list(output_options)?;
        let output_rows = decode_port_descriptors(&mut context, output_rows)?;

        for (
            id,
            name,
            _is_connected,
            supported_data_formats,
            _default_data_format,
            supported_protocols,
            _default_protocol,
        ) in output_rows
        {
            assert!(
                id.starts_with("winmm:output:"),
                "WinMM output ids should use the winmm:output prefix"
            );
            assert!(!name.is_empty(), "WinMM output names should not be empty");
            assert_eq!(
                supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0,
                "WinMM output rows should advertise only MIDI 1 byte-stream transport",
            );
            assert_eq!(
                supported_protocols.0, MIDI_PROTOCOL_FLAG_MIDI1.0,
                "WinMM output rows should advertise only MIDI 1 protocol semantics",
            );
        }

        Ok(())
    })
}

/// Reject scheduled output timestamps on opened WinMM output ports.
#[test]
fn test_midi_winmm_output_write_rejects_scheduled_timestamps_when_ports_are_present() {
    with_harness_context(|mut context| {
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let support = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::WinMM)
            .map(|(_, _, support, _, _, _, _)| *support)
            .expect("midi backend list should include a WinMM selector row");
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        let output_options = harness_port_list_options_for_backend(
            &mut context,
            MidiBackend::WinMM,
            MidiPortListFlags(0),
        );
        let output_rows = context.destack_midi_output_port_list(output_options)?;
        let output_rows = decode_port_descriptors(&mut context, output_rows)?;

        let Some((
            id,
            _name,
            _is_connected,
            _formats,
            _default_format,
            _protocols,
            _default_protocol,
        )) = output_rows.first()
        else {
            return Ok(());
        };

        let id = harness_string(&mut context, id)?;
        let open_options = harness_output_open_options_for_backend_transport(
            &mut context,
            MidiBackend::WinMM,
            Some(MidiDataFormat::Midi1Bytes),
            Some(MidiProtocol::Midi1),
        );
        let handle = context.destack_midi_output_port_open(id, open_options)?;

        let record = MidiOutputRecord {
            send_at_ns: Some(1),
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: context.call_context.store_slice(vec![0x90, 0x3C, 0x40]),
        };
        let records = harness_output_records(&mut context, &[record])?;
        assert_platform_error_codes(
            context.destack_midi_output_write(handle, records),
            &[PlatformErrorCode::InvalidArgument],
        )?;

        context.destack_midi_output_port_close(handle)?;

        Ok(())
    })
}
