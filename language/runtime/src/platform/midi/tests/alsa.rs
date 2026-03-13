use super::{
    assert_platform_error_codes, decode_backend_descriptors_full, decode_event,
    decode_input_record, decode_port_descriptor, decode_port_descriptors,
    harness_event_open_options, harness_output_open_options_for_transport, harness_output_records,
    harness_port_list_options_with_flags, harness_string,
    harness_virtual_input_create_options_for_backend_transport,
    harness_virtual_output_create_options_for_backend_transport, support_allows_host_execution,
    with_harness_context,
};
use crate::platform::core::monotonic_now_ns;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::midi::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS,
    MIDI_BACKEND_CAP_SCHEDULED_OUTPUT, MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_BACKEND_CAP_UMP,
    MIDI_BACKEND_CAP_VIRTUAL_INPUT, MIDI_BACKEND_CAP_VIRTUAL_OUTPUT,
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL,
    MIDI_PORT_DIRECTION_FLAG_INPUT, MIDI_PORT_LIST_INCLUDE_VIRTUAL, MIDI_PROTOCOL_FLAG_MIDI1,
    MidiBackend, MidiDataFormat, MidiEventDeliveryMode, MidiEventSource, MidiOutputRecord,
    MidiPortDirection, MidiProtocol, MidiRecordFraming,
};

/// Return one stable ALSA runtime-id suffix for one endpoint id.
fn runtime_id_suffix(id: &str) -> &str {
    id.rsplit(':')
        .next()
        .expect("ALSA runtime ids should contain one numeric port suffix")
}

/// Match the ALSA backend row to its expected capability contract.
#[test]
fn test_midi_alsa_backend_row_matches_expected_capability_contract() {
    with_harness_context(|mut context| {
        // backend rows
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let alsa_row = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::Alsa)
            .expect("midi backend list should include an ALSA selector row");

        let (
            _backend,
            name,
            support,
            priority,
            capability_flags,
            supported_data_formats,
            supported_protocols,
        ) = alsa_row;

        // row shape
        assert_eq!(name, "alsa");
        assert!(*priority != 0, "ALSA should participate in auto selection");

        // unavailable rows must stay zeroed
        if !support_allows_host_execution(*support) {
            assert_eq!(
                capability_flags.0, 0,
                "unavailable ALSA rows should not advertise capabilities",
            );
            assert_eq!(
                supported_data_formats.0, 0,
                "unavailable ALSA rows should not advertise data formats",
            );
            assert_eq!(
                supported_protocols.0, 0,
                "unavailable ALSA rows should not advertise protocols",
            );

            return Ok(());
        }

        // advertised capabilities
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0 != 0,
            "ALSA should advertise topology events",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0 != 0,
            "ALSA should advertise a native event feed",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_INPUT.0 != 0,
            "ALSA should advertise virtual input support",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_OUTPUT.0 != 0,
            "ALSA should advertise virtual output support",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_SCHEDULED_OUTPUT.0 != 0,
            "ALSA should advertise scheduled output support",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0 != 0,
            "ALSA should advertise receive timestamps",
        );
        assert_eq!(
            capability_flags.0 & MIDI_BACKEND_CAP_UMP.0,
            0,
            "ALSA should not advertise UMP transport support",
        );

        // advertised transport
        assert_eq!(
            supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0,
            "ALSA should advertise only MIDI 1 byte transport",
        );
        assert_eq!(
            supported_protocols.0, MIDI_PROTOCOL_FLAG_MIDI1.0,
            "ALSA should advertise only MIDI 1 protocol semantics",
        );

        Ok(())
    });
}

/// Deliver native ALSA topology events for virtual source lifetime changes.
#[test]
fn test_midi_alsa_native_event_feed_reports_virtual_source_additions_and_removals() {
    with_harness_context(|mut context| {
        // backend support
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let support = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::Alsa)
            .map(|(_, _, support, _, _, _, _)| *support)
            .expect("midi backend list should include an ALSA selector row");
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        // native subscription
        let options = harness_event_open_options(
            &mut context,
            MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL,
            MIDI_PORT_DIRECTION_FLAG_INPUT,
            MidiEventDeliveryMode::NativeOnly,
        );
        let event_handle = context.destack_midi_event_open(options)?;

        // virtual source creation
        let create_options = harness_virtual_output_create_options_for_backend_transport(
            &mut context,
            MidiBackend::Alsa,
            "Destack ALSA Virtual Source",
            MidiDataFormat::Midi1Bytes,
            MidiProtocol::Midi1,
        )?;
        let output_handle = context.destack_midi_output_virtual_create(create_options)?;
        let descriptor = context.destack_midi_output_port_descriptor(output_handle)?;
        let (_output_id, _backend_id, output_name, _is_virtual, _is_connected, _format, _protocol) =
            decode_port_descriptor(&mut context, descriptor)?;

        // enumerated input identity
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
            .expect("virtual ALSA sources should appear in input port enumeration");

        // addition event
        let event = context.destack_midi_event_read(event_handle, 1_000_000_000)?;
        let event = decode_event(&mut context, event)?;
        assert_eq!(event.kind, "portAdded");
        assert_eq!(event.source, MidiEventSource::Native);
        assert_eq!(event.direction, Some(MidiPortDirection::Input));
        assert_eq!(event.id.as_deref(), Some(input_port_id.as_str()));
        assert_eq!(event.is_virtual, Some(true));

        // endpoint removal
        context.destack_midi_output_port_close(output_handle)?;

        // removal event
        let event = context.destack_midi_event_read(event_handle, 1_000_000_000)?;
        let event = decode_event(&mut context, event)?;
        assert_eq!(event.kind, "portRemoved");
        assert_eq!(event.source, MidiEventSource::Native);
        assert_eq!(event.direction, Some(MidiPortDirection::Input));
        assert_eq!(event.id.as_deref(), Some(input_port_id.as_str()));
        assert_eq!(event.is_virtual, None);

        // resource teardown
        context.destack_midi_event_close(event_handle)?;

        Ok(())
    });
}

/// Roundtrip one virtual ALSA destination through one opened output session.
#[test]
fn test_midi_alsa_virtual_destination_roundtrips_midi1_records() {
    with_harness_context(|mut context| {
        // backend support
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let support = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::Alsa)
            .map(|(_, _, support, _, _, _, _)| *support)
            .expect("midi backend list should include an ALSA selector row");
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        // virtual destination creation
        let create_options = harness_virtual_input_create_options_for_backend_transport(
            &mut context,
            MidiBackend::Alsa,
            "Destack ALSA Virtual Destination",
            MidiDataFormat::Midi1Bytes,
            MidiProtocol::Midi1,
        )?;
        let input_handle = context.destack_midi_input_virtual_create(create_options)?;
        let descriptor = context.destack_midi_input_port_descriptor(input_handle)?;
        let (
            input_id,
            _backend_id,
            _name,
            is_virtual,
            is_connected,
            default_data_format,
            default_protocol,
        ) = decode_port_descriptor(&mut context, descriptor)?;

        // descriptor shape
        assert!(
            is_virtual,
            "virtual ALSA destinations should report isVirtual"
        );
        assert!(
            is_connected,
            "virtual ALSA destinations should be routable immediately"
        );
        assert_eq!(default_data_format, Some(MidiDataFormat::Midi1Bytes));
        assert_eq!(default_protocol, Some(MidiProtocol::Midi1));

        // matching output row
        let input_suffix = runtime_id_suffix(&input_id).to_string();
        let list_options =
            harness_port_list_options_with_flags(&mut context, MIDI_PORT_LIST_INCLUDE_VIRTUAL);
        let output_ports = context.destack_midi_output_port_list(list_options)?;
        let output_ports = decode_port_descriptors(&mut context, output_ports)?;
        let output_port_id = output_ports
            .into_iter()
            .find(
                |(
                    id,
                    _name,
                    _is_connected,
                    _formats,
                    _default_format,
                    _protocols,
                    _default_protocol,
                )| { runtime_id_suffix(id) == input_suffix },
            )
            .map(|(id, _, _, _, _, _, _)| id)
            .expect("virtual ALSA destinations should appear in output port enumeration");

        // output open
        let id = harness_string(&mut context, &output_port_id)?;
        let options = harness_output_open_options_for_transport(
            &mut context,
            Some(MidiDataFormat::Midi1Bytes),
            Some(MidiProtocol::Midi1),
        );
        let output_handle = context.destack_midi_output_port_open(id, options)?;

        // transport write
        let record = MidiOutputRecord {
            send_at_ns: None,
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: context.call_context.store_slice(vec![0x90, 0x3C, 0x40]),
        };
        let records = harness_output_records(&mut context, &[record])?;
        let written = context.destack_midi_output_write(output_handle, records)?;
        assert_eq!(written, 1, "virtual ALSA writes should report one record");

        // transport read
        let received = context.destack_midi_input_read(input_handle, 1_000_000_000)?;
        let (received_at_ns, source_id, data_format, protocol, data) =
            decode_input_record(&mut context, received)?;
        assert_ne!(received_at_ns, 0, "ALSA should populate receive timestamps");
        assert_eq!(source_id, None);
        assert_eq!(data_format, MidiDataFormat::Midi1Bytes);
        assert_eq!(protocol, Some(MidiProtocol::Midi1));
        assert_eq!(data, vec![0x90, 0x3C, 0x40]);

        // resource teardown
        context.destack_midi_output_port_close(output_handle)?;
        context.destack_midi_input_port_close(input_handle)?;

        Ok(())
    });
}

/// Defer scheduled ALSA delivery until the requested deadline.
#[test]
fn test_midi_alsa_scheduled_virtual_destination_delivery_waits_for_the_deadline() {
    with_harness_context(|mut context| {
        // backend support
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let support = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::Alsa)
            .map(|(_, _, support, _, _, _, _)| *support)
            .expect("midi backend list should include an ALSA selector row");
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        // virtual destination creation
        let create_options = harness_virtual_input_create_options_for_backend_transport(
            &mut context,
            MidiBackend::Alsa,
            "Destack ALSA Scheduled Destination",
            MidiDataFormat::Midi1Bytes,
            MidiProtocol::Midi1,
        )?;
        let input_handle = context.destack_midi_input_virtual_create(create_options)?;
        let descriptor = context.destack_midi_input_port_descriptor(input_handle)?;
        let (input_id, _backend_id, _name, _is_virtual, _is_connected, _format, _protocol) =
            decode_port_descriptor(&mut context, descriptor)?;

        // matching output row
        let input_suffix = runtime_id_suffix(&input_id).to_string();
        let list_options =
            harness_port_list_options_with_flags(&mut context, MIDI_PORT_LIST_INCLUDE_VIRTUAL);
        let output_ports = context.destack_midi_output_port_list(list_options)?;
        let output_ports = decode_port_descriptors(&mut context, output_ports)?;
        let output_port_id = output_ports
            .into_iter()
            .find(
                |(
                    id,
                    _name,
                    _is_connected,
                    _formats,
                    _default_format,
                    _protocols,
                    _default_protocol,
                )| { runtime_id_suffix(id) == input_suffix },
            )
            .map(|(id, _, _, _, _, _, _)| id)
            .expect("virtual ALSA destinations should appear in output port enumeration");

        // output open
        let id = harness_string(&mut context, &output_port_id)?;
        let options = harness_output_open_options_for_transport(
            &mut context,
            Some(MidiDataFormat::Midi1Bytes),
            Some(MidiProtocol::Midi1),
        );
        let output_handle = context.destack_midi_output_port_open(id, options)?;

        // scheduled write
        let send_at_ns = monotonic_now_ns().saturating_add(100_000_000);
        let record = MidiOutputRecord {
            send_at_ns: Some(send_at_ns),
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: context.call_context.store_slice(vec![0x90, 0x3C, 0x40]),
        };
        let records = harness_output_records(&mut context, &[record])?;
        let written = context.destack_midi_output_write(output_handle, records)?;
        assert_eq!(written, 1, "ALSA should queue one scheduled record");

        // immediate queue state
        assert_platform_error_codes(
            context.destack_midi_input_try_read(input_handle),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        // eventual delivery
        let received = context.destack_midi_input_read(input_handle, 1_000_000_000)?;
        let (received_at_ns, source_id, data_format, protocol, data) =
            decode_input_record(&mut context, received)?;
        assert!(
            monotonic_now_ns() >= send_at_ns,
            "ALSA scheduled delivery should not arrive before the requested deadline",
        );
        assert_ne!(received_at_ns, 0, "ALSA should populate receive timestamps");
        assert_eq!(source_id, None);
        assert_eq!(data_format, MidiDataFormat::Midi1Bytes);
        assert_eq!(protocol, Some(MidiProtocol::Midi1));
        assert_eq!(data, vec![0x90, 0x3C, 0x40]);

        // resource teardown
        context.destack_midi_output_port_close(output_handle)?;
        context.destack_midi_input_port_close(input_handle)?;

        Ok(())
    });
}
