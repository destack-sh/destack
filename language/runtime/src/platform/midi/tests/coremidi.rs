use super::{
    assert_platform_error_codes, decode_backend_descriptors_full, decode_event,
    decode_input_record, decode_port_descriptor, decode_port_descriptors,
    harness_event_open_options, harness_event_open_options_for_backend,
    harness_output_open_options_for_transport, harness_output_records,
    harness_port_list_options_for_backend, harness_port_list_options_with_flags, harness_string,
    harness_virtual_input_create_options_for_backend_transport, support_allows_host_execution,
    with_harness_context,
};
use crate::platform::core::monotonic_now_ns;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::midi::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS,
    MIDI_BACKEND_CAP_SCHEDULED_OUTPUT, MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_BACKEND_CAP_UMP,
    MIDI_BACKEND_CAP_VIRTUAL_INPUT, MIDI_BACKEND_CAP_VIRTUAL_OUTPUT,
    MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL, MIDI_PORT_DIRECTION_FLAG_OUTPUT,
    MIDI_PORT_LIST_INCLUDE_VIRTUAL, MidiBackend, MidiDataFormat, MidiEventDeliveryMode,
    MidiEventSource, MidiEventSubscriptionFlags, MidiOutputRecord, MidiPortDirection,
    MidiPortListFlags, MidiProtocol, MidiRecordFraming,
};

/// Return the stable unique-id suffix for one CoreMIDI runtime id.
fn runtime_id_suffix(id: &str) -> &str {
    id.rsplit(':')
        .next()
        .expect("CoreMIDI runtime ids should always contain one numeric suffix")
}

/// Match the CoreMIDI backend row to its expected capability contract.
#[test]
fn test_midi_coremidi_backend_row_matches_expected_capability_contract() {
    with_harness_context(|mut context| {
        // backend row
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let descriptor = descriptors
            .into_iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::CoreMIDI)
            .expect("midi backend list should include a CoreMIDI selector row");

        let (
            _,
            name,
            support,
            priority,
            capability_flags,
            supported_data_formats,
            supported_protocols,
        ) = descriptor;

        // identity and support
        assert_eq!(name, "coremidi");
        assert!(
            priority != 0,
            "CoreMIDI should participate in auto selection"
        );

        // unavailable rows must stay zeroed
        if !support_allows_host_execution(support) {
            assert_eq!(
                capability_flags.0, 0,
                "host-unavailable CoreMIDI rows should not advertise capabilities",
            );
            assert_eq!(
                supported_data_formats.0, 0,
                "host-unavailable CoreMIDI rows should not advertise transport data formats",
            );
            assert_eq!(
                supported_protocols.0, 0,
                "host-unavailable CoreMIDI rows should not advertise protocols",
            );

            return Ok(());
        }

        // advertised capabilities
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0 != 0,
            "CoreMIDI should advertise topology events",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_INPUT.0 != 0,
            "CoreMIDI should advertise virtual input support",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_OUTPUT.0 != 0,
            "CoreMIDI should advertise virtual output support",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_SCHEDULED_OUTPUT.0 != 0,
            "CoreMIDI should advertise scheduled output support",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0 != 0,
            "CoreMIDI should advertise receive timestamps",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_UMP.0 != 0,
            "CoreMIDI should advertise UMP transport support",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0 != 0,
            "CoreMIDI should advertise a native event feed",
        );
        // advertised transport shape
        assert!(
            supported_data_formats.0 != 0,
            "CoreMIDI should advertise at least one transport data format",
        );
        assert!(
            supported_protocols.0 != 0,
            "CoreMIDI should advertise at least one protocol",
        );

        Ok(())
    });
}

/// Reject explicit CoreMIDI operations with not-supported when the host cannot initialize it.
#[test]
fn test_midi_coremidi_unavailable_host_rejects_explicit_operations_loudly() {
    with_harness_context(|mut context| {
        // backend support
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let support = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::CoreMIDI)
            .map(|(_, _, support, _, _, _, _)| *support)
            .expect("midi backend list should include a CoreMIDI selector row");
        if support_allows_host_execution(support) {
            return Ok(());
        }

        // input enumeration
        let list_options = harness_port_list_options_for_backend(
            &mut context,
            MidiBackend::CoreMIDI,
            MidiPortListFlags(0),
        );
        assert_platform_error_codes(
            context.destack_midi_input_port_list(list_options),
            &[PlatformErrorCode::NotSupported],
        )?;

        // output enumeration
        let list_options = harness_port_list_options_for_backend(
            &mut context,
            MidiBackend::CoreMIDI,
            MidiPortListFlags(0),
        );
        assert_platform_error_codes(
            context.destack_midi_output_port_list(list_options),
            &[PlatformErrorCode::NotSupported],
        )?;

        // event subscription
        let options = harness_event_open_options_for_backend(
            &mut context,
            MidiBackend::CoreMIDI,
            MidiEventSubscriptionFlags(0),
            MIDI_PORT_DIRECTION_FLAG_OUTPUT,
            MidiEventDeliveryMode::Auto,
        );
        assert_platform_error_codes(
            context.destack_midi_event_open(options),
            &[PlatformErrorCode::NotSupported],
        )?;

        Ok(())
    });
}

/// Deliver native CoreMIDI topology events for virtual endpoint lifetime changes.
#[test]
fn test_midi_coremidi_native_event_feed_reports_virtual_endpoint_additions_and_removals() {
    with_harness_context(|mut context| {
        // native subscription
        let options = harness_event_open_options(
            &mut context,
            MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL,
            MIDI_PORT_DIRECTION_FLAG_OUTPUT,
            MidiEventDeliveryMode::NativeOnly,
        );
        let event_handle = context.destack_midi_event_open(options)?;

        // virtual destination creation
        let create_options = harness_virtual_input_create_options_for_backend_transport(
            &mut context,
            MidiBackend::CoreMIDI,
            "Destack CoreMIDI Native Event Destination",
            MidiDataFormat::Ump,
            MidiProtocol::Midi1,
        )?;
        let input_handle = context.destack_midi_input_virtual_create(create_options)?;
        let descriptor = context.destack_midi_input_port_descriptor(input_handle)?;
        let (input_id, _backend_id, _name, _is_virtual, _is_connected, _format, _protocol) =
            decode_port_descriptor(&mut context, descriptor)?;
        let input_suffix = runtime_id_suffix(&input_id).to_string();

        // output-side identity
        let list_options =
            harness_port_list_options_with_flags(&mut context, MIDI_PORT_LIST_INCLUDE_VIRTUAL);
        let output_ports = context.destack_midi_output_port_list(list_options)?;
        let output_ports = decode_port_descriptors(&mut context, output_ports)?;
        let output_port_id = output_ports
            .into_iter()
            .find(|(id, _, _, _, _, _, _)| runtime_id_suffix(id) == input_suffix)
            .map(|(id, _, _, _, _, _, _)| id)
            .expect("virtual CoreMIDI destinations should appear in output port enumeration");

        // event read
        let event = context.destack_midi_event_read(event_handle, 1_000_000_000)?;
        let event = decode_event(&mut context, event)?;
        assert_eq!(event.kind, "portAdded");
        assert_eq!(event.source, MidiEventSource::Native);
        assert_eq!(event.direction, Some(MidiPortDirection::Output));
        assert_eq!(event.id.as_deref(), Some(output_port_id.as_str()));
        assert_eq!(event.is_virtual, Some(true));

        // endpoint removal
        context.destack_midi_input_port_close(input_handle)?;

        // removal read
        let event = context.destack_midi_event_read(event_handle, 1_000_000_000)?;
        let event = decode_event(&mut context, event)?;
        assert_eq!(event.kind, "portRemoved");
        assert_eq!(event.source, MidiEventSource::Native);
        assert_eq!(event.direction, Some(MidiPortDirection::Output));
        assert_eq!(event.id.as_deref(), Some(output_port_id.as_str()));
        assert_eq!(event.is_virtual, None);

        // resource teardown
        context.destack_midi_event_close(event_handle)?;

        Ok(())
    });
}

/// Roundtrip one legacy virtual CoreMIDI destination through one opened output session.
#[test]
fn test_midi_coremidi_virtual_destination_roundtrips_midi1_transport_records() {
    with_harness_context(|mut context| {
        // virtual destination creation
        let create_options = harness_virtual_input_create_options_for_backend_transport(
            &mut context,
            MidiBackend::CoreMIDI,
            "Destack CoreMIDI Virtual MIDI1 Destination",
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
            "virtual CoreMIDI destinations should report isVirtual"
        );
        assert!(
            is_connected,
            "virtual CoreMIDI destinations should be routable immediately"
        );
        assert_eq!(default_data_format, Some(MidiDataFormat::Midi1Bytes));
        assert_eq!(default_protocol, Some(MidiProtocol::Midi1));

        // list visibility
        let input_suffix = runtime_id_suffix(&input_id).to_string();
        let list_options =
            harness_port_list_options_with_flags(&mut context, MIDI_PORT_LIST_INCLUDE_VIRTUAL);
        let output_ports = context.destack_midi_output_port_list(list_options)?;
        let output_ports = decode_port_descriptors(&mut context, output_ports)?;
        let output_row = output_ports
            .into_iter()
            .find(|(id, _, _, _, _, _, _)| runtime_id_suffix(id) == input_suffix)
            .expect("virtual CoreMIDI destinations should appear in output port enumeration");
        let output_port_id = output_row.0.clone();
        assert!(
            output_row.3.0 != 0,
            "enumerated CoreMIDI output rows should advertise one transport data format",
        );

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
        assert_eq!(
            written, 1,
            "virtual CoreMIDI writes should report one submitted record"
        );

        // transport read
        let received = context.destack_midi_input_read(input_handle, 1_000_000_000)?;
        let (received_at_ns, source_id, data_format, protocol, data) =
            decode_input_record(&mut context, received)?;
        assert!(
            received_at_ns != 0,
            "CoreMIDI input timestamps should use the monotonic domain"
        );
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

/// Hold one scheduled CoreMIDI write until the requested monotonic deadline.
#[test]
fn test_midi_coremidi_scheduled_virtual_output_defers_delivery_until_deadline() {
    with_harness_context(|mut context| {
        // virtual destination creation
        let create_options = harness_virtual_input_create_options_for_backend_transport(
            &mut context,
            MidiBackend::CoreMIDI,
            "Destack CoreMIDI Scheduled Destination",
            MidiDataFormat::Midi1Bytes,
            MidiProtocol::Midi1,
        )?;
        let input_handle = context.destack_midi_input_virtual_create(create_options)?;
        let descriptor = context.destack_midi_input_port_descriptor(input_handle)?;
        let (input_id, _backend_id, _name, _is_virtual, _is_connected, _format, _protocol) =
            decode_port_descriptor(&mut context, descriptor)?;

        // output-side identity
        let input_suffix = runtime_id_suffix(&input_id).to_string();
        let list_options =
            harness_port_list_options_with_flags(&mut context, MIDI_PORT_LIST_INCLUDE_VIRTUAL);
        let output_ports = context.destack_midi_output_port_list(list_options)?;
        let output_ports = decode_port_descriptors(&mut context, output_ports)?;
        let output_port_id = output_ports
            .into_iter()
            .find(|(id, _, _, _, _, _, _)| runtime_id_suffix(id) == input_suffix)
            .map(|(id, _, _, _, _, _, _)| id)
            .expect("scheduled CoreMIDI destinations should appear in output port enumeration");

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
        assert_eq!(
            written, 1,
            "scheduled CoreMIDI writes should accept one record"
        );

        // early read rejection
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
            "scheduled CoreMIDI writes should not arrive before the requested deadline",
        );
        assert!(
            received_at_ns != 0,
            "scheduled CoreMIDI writes should preserve receive timestamps",
        );
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

/// Roundtrip one virtual CoreMIDI destination through one opened output session.
#[test]
fn test_midi_coremidi_virtual_destination_roundtrips_ump_transport_records() {
    with_harness_context(|mut context| {
        // virtual destination creation
        let create_options = harness_virtual_input_create_options_for_backend_transport(
            &mut context,
            MidiBackend::CoreMIDI,
            "Destack CoreMIDI Virtual Destination",
            MidiDataFormat::Ump,
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
            "virtual CoreMIDI destinations should report isVirtual"
        );
        assert!(
            is_connected,
            "virtual CoreMIDI destinations should be routable immediately"
        );
        assert_eq!(default_data_format, Some(MidiDataFormat::Ump));
        assert_eq!(default_protocol, Some(MidiProtocol::Midi1));

        // list visibility
        let input_suffix = runtime_id_suffix(&input_id).to_string();
        let list_options =
            harness_port_list_options_with_flags(&mut context, MIDI_PORT_LIST_INCLUDE_VIRTUAL);
        let output_ports = context.destack_midi_output_port_list(list_options)?;
        let output_ports = decode_port_descriptors(&mut context, output_ports)?;
        let output_row = output_ports
            .into_iter()
            .find(|(id, _, _, _, _, _, _)| runtime_id_suffix(id) == input_suffix)
            .expect("virtual CoreMIDI destinations should appear in output port enumeration");
        let output_port_id = output_row.0.clone();
        assert!(
            output_row.3.0 != 0,
            "enumerated CoreMIDI output rows should advertise one transport data format",
        );

        // output open
        let id = harness_string(&mut context, &output_port_id)?;
        let options = harness_output_open_options_for_transport(
            &mut context,
            Some(MidiDataFormat::Ump),
            Some(MidiProtocol::Midi1),
        );
        let output_handle = context.destack_midi_output_port_open(id, options)?;

        // transport write
        let record = MidiOutputRecord {
            send_at_ns: None,
            data_format: MidiDataFormat::Ump,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: context
                .call_context
                .store_slice(vec![0x20, 0x90, 0x3C, 0x40]),
        };
        let records = harness_output_records(&mut context, &[record])?;
        let written = context.destack_midi_output_write(output_handle, records)?;
        assert_eq!(
            written, 1,
            "virtual CoreMIDI writes should report one submitted record"
        );

        // transport read
        let received = context.destack_midi_input_read(input_handle, 1_000_000_000)?;
        let (received_at_ns, source_id, data_format, protocol, data) =
            decode_input_record(&mut context, received)?;
        assert!(
            received_at_ns != 0,
            "CoreMIDI input timestamps should use the monotonic domain"
        );
        assert_eq!(source_id, None);
        assert_eq!(data_format, MidiDataFormat::Ump);
        assert_eq!(protocol, Some(MidiProtocol::Midi1));
        assert_eq!(data, vec![0x20, 0x90, 0x3C, 0x40]);

        // resource teardown
        context.destack_midi_output_port_close(output_handle)?;
        context.destack_midi_input_port_close(input_handle)?;

        Ok(())
    });
}
