use super::{
    decode_backend_descriptors_full, decode_event, decode_input_record, decode_port_descriptor,
    decode_port_descriptors, harness_event_open_options, harness_output_open_options_for_transport,
    harness_output_records, harness_port_list_options_with_flags,
    harness_virtual_input_create_options_for_backend_transport,
    harness_virtual_output_create_options_for_backend_transport, support_allows_host_execution,
    with_harness_context,
};
use crate::platform::midi::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS,
    MIDI_BACKEND_CAP_SCHEDULED_OUTPUT, MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_BACKEND_CAP_UMP,
    MIDI_BACKEND_CAP_VIRTUAL_INPUT, MIDI_BACKEND_CAP_VIRTUAL_OUTPUT,
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL,
    MIDI_PORT_DIRECTION_FLAG_INPUT, MIDI_PORT_LIST_INCLUDE_VIRTUAL, MIDI_PROTOCOL_FLAG_MIDI1,
    MidiBackend, MidiDataFormat, MidiEventDeliveryMode, MidiEventSource, MidiOutputRecord,
    MidiPortDirection, MidiProtocol, MidiRecordFraming,
};

/// Match the JACK backend row to its expected capability contract.
#[test]
fn test_midi_jack_backend_row_matches_expected_capability_contract() {
    with_harness_context(|mut context| {
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let jack_row = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::JackMidi)
            .expect("midi backend list should include one JACK selector row");

        let (
            _backend,
            name,
            support,
            priority,
            capability_flags,
            supported_data_formats,
            supported_protocols,
        ) = jack_row;

        assert_eq!(name, "jack-midi");
        assert!(*priority != 0, "JACK should participate in auto selection");

        if !support_allows_host_execution(*support) {
            assert_eq!(
                capability_flags.0, 0,
                "unavailable JACK rows should not advertise capabilities",
            );
            assert_eq!(
                supported_data_formats.0, 0,
                "unavailable JACK rows should not advertise data formats",
            );
            assert_eq!(
                supported_protocols.0, 0,
                "unavailable JACK rows should not advertise protocols",
            );

            return Ok(());
        }

        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0 != 0,
            "JACK should advertise topology events",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0 != 0,
            "JACK should advertise a native event feed",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_INPUT.0 != 0,
            "JACK should advertise virtual input support",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_OUTPUT.0 != 0,
            "JACK should advertise virtual output support",
        );
        assert_eq!(
            capability_flags.0 & MIDI_BACKEND_CAP_SCHEDULED_OUTPUT.0,
            0,
            "JACK should not advertise scheduled output support",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0 != 0,
            "JACK should advertise receive timestamps",
        );
        assert_eq!(
            capability_flags.0 & MIDI_BACKEND_CAP_UMP.0,
            0,
            "JACK should not advertise UMP support",
        );

        assert_eq!(
            supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0,
            "JACK should advertise only MIDI 1 byte transport",
        );
        assert_eq!(
            supported_protocols.0, MIDI_PROTOCOL_FLAG_MIDI1.0,
            "JACK should advertise only MIDI 1 protocol semantics",
        );

        Ok(())
    });
}

/// Deliver native JACK topology events for virtual source lifetime changes.
#[test]
fn test_midi_jack_native_event_feed_reports_virtual_source_additions_and_removals() {
    with_harness_context(|mut context| {
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let support = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::JackMidi)
            .map(|(_, _, support, _, _, _, _)| *support)
            .expect("midi backend list should include one JACK selector row");
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        let options = harness_event_open_options(
            &mut context,
            MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL,
            MIDI_PORT_DIRECTION_FLAG_INPUT,
            MidiEventDeliveryMode::NativeOnly,
        );
        let event_handle = context.destack_midi_event_open(options)?;

        let create_options = harness_virtual_output_create_options_for_backend_transport(
            &mut context,
            MidiBackend::JackMidi,
            "Destack JACK Virtual Source",
            MidiDataFormat::Midi1Bytes,
            MidiProtocol::Midi1,
        )?;
        let output_handle = context.destack_midi_output_virtual_create(create_options)?;
        let descriptor = context.destack_midi_output_port_descriptor(output_handle)?;
        let (_output_id, _backend_id, output_name, _is_virtual, _is_connected, _format, _protocol) =
            decode_port_descriptor(&mut context, descriptor)?;

        let list_options =
            harness_port_list_options_with_flags(&mut context, MIDI_PORT_LIST_INCLUDE_VIRTUAL);
        let input_ports = context.destack_midi_input_port_list(list_options)?;
        let input_ports = decode_port_descriptors(&mut context, input_ports)?;
        let input_port_id = input_ports
            .into_iter()
            .find(
                |(
                    _id,
                    name,
                    _is_connected,
                    _formats,
                    _default_format,
                    _protocols,
                    _default_protocol,
                )| { *name == output_name },
            )
            .map(|(id, _, _, _, _, _, _)| id)
            .expect("virtual JACK sources should appear in input port enumeration");

        let event = context.destack_midi_event_read(event_handle, 1_000_000_000)?;
        let event = decode_event(&mut context, event)?;
        assert_eq!(event.kind, "portAdded");
        assert_eq!(event.source, MidiEventSource::Native);
        assert_eq!(event.direction, Some(MidiPortDirection::Input));
        assert_eq!(event.id.as_deref(), Some(input_port_id.as_str()));
        assert_eq!(event.is_virtual, Some(true));

        context.destack_midi_output_port_close(output_handle)?;

        let event = context.destack_midi_event_read(event_handle, 1_000_000_000)?;
        let event = decode_event(&mut context, event)?;
        assert_eq!(event.kind, "portRemoved");
        assert_eq!(event.source, MidiEventSource::Native);
        assert_eq!(event.direction, Some(MidiPortDirection::Input));
        assert_eq!(event.id.as_deref(), Some(input_port_id.as_str()));
        assert_eq!(event.is_virtual, None);

        context.destack_midi_event_close(event_handle)?;

        Ok(())
    });
}

/// Roundtrip one virtual JACK destination through one opened output session.
#[test]
fn test_midi_jack_virtual_destination_roundtrips_midi1_records() {
    with_harness_context(|mut context| {
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let support = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::JackMidi)
            .map(|(_, _, support, _, _, _, _)| *support)
            .expect("midi backend list should include one JACK selector row");
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        let create_options = harness_virtual_input_create_options_for_backend_transport(
            &mut context,
            MidiBackend::JackMidi,
            "Destack JACK Virtual Destination",
            MidiDataFormat::Midi1Bytes,
            MidiProtocol::Midi1,
        )?;
        let input_handle = context.destack_midi_input_virtual_create(create_options)?;
        let descriptor = context.destack_midi_input_port_descriptor(input_handle)?;
        let (
            _input_id,
            _backend_id,
            input_name,
            is_virtual,
            is_connected,
            default_data_format,
            default_protocol,
        ) = decode_port_descriptor(&mut context, descriptor)?;

        assert!(
            is_virtual,
            "virtual JACK destinations should report isVirtual"
        );
        assert!(
            is_connected,
            "virtual JACK destinations should be routable immediately",
        );
        assert_eq!(default_data_format, Some(MidiDataFormat::Midi1Bytes));
        assert_eq!(default_protocol, Some(MidiProtocol::Midi1));

        let list_options =
            harness_port_list_options_with_flags(&mut context, MIDI_PORT_LIST_INCLUDE_VIRTUAL);
        let output_ports = context.destack_midi_output_port_list(list_options)?;
        let output_ports = decode_port_descriptors(&mut context, output_ports)?;
        let output_port_id = output_ports
            .into_iter()
            .find(
                |(
                    _id,
                    name,
                    _is_connected,
                    _formats,
                    _default_format,
                    _protocols,
                    _default_protocol,
                )| { *name == input_name },
            )
            .map(|(id, _, _, _, _, _, _)| id)
            .expect("virtual JACK destinations should appear in output port enumeration");

        let open_options = harness_output_open_options_for_transport(
            &mut context,
            Some(MidiDataFormat::Midi1Bytes),
            Some(MidiProtocol::Midi1),
        );
        let id = super::harness_string(&mut context, &output_port_id)?;
        let output_handle = context.destack_midi_output_port_open(id, open_options)?;

        let record = MidiOutputRecord {
            send_at_ns: None,
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: context.call_context.store_slice(vec![0x90, 0x3c, 0x40]),
        };
        let records = harness_output_records(&mut context, &[record])?;
        let written_count = context.destack_midi_output_write(output_handle, records)?;
        assert_eq!(written_count, 1);

        let record = context.destack_midi_input_read(input_handle, 1_000_000_000)?;
        let (received_at_ns, source_id, data_format, protocol, data) =
            decode_input_record(&mut context, record)?;
        assert!(
            received_at_ns != 0,
            "JACK input timestamps should use the monotonic domain",
        );
        assert_eq!(source_id, None);
        assert_eq!(data_format, MidiDataFormat::Midi1Bytes);
        assert_eq!(protocol, Some(MidiProtocol::Midi1));
        assert_eq!(data, vec![0x90, 0x3c, 0x40]);

        context.destack_midi_output_port_close(output_handle)?;
        context.destack_midi_input_port_close(input_handle)?;

        Ok(())
    });
}
