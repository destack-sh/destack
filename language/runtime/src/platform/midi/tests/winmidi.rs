use super::{
    assert_platform_error_codes, backend_descriptor_row, harness_event_open_options_for_backend,
    harness_input_open_options_for_backend_transport,
    harness_output_open_options_for_backend_transport, harness_output_records, harness_string,
    harness_virtual_input_create_options_for_backend_transport,
    harness_virtual_output_create_options_for_backend_transport, listed_backend_input_rows,
    listed_backend_output_rows, output_record_for_transport,
    output_record_for_transport_with_timestamp, preferred_transport_pair,
    support_allows_host_execution, with_harness_context,
};
use crate::platform::core::monotonic_now_ns;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::midi::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS,
    MIDI_BACKEND_CAP_SCHEDULED_OUTPUT, MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_BACKEND_CAP_UMP,
    MIDI_BACKEND_CAP_VIRTUAL_INPUT, MIDI_BACKEND_CAP_VIRTUAL_OUTPUT, MIDI_DATA_FORMAT_FLAG_UMP,
    MIDI_PORT_DIRECTION_FLAG_INPUT, MIDI_PORT_DIRECTION_FLAG_OUTPUT, MIDI_PROTOCOL_FLAG_MIDI1,
    MIDI_PROTOCOL_FLAG_MIDI2, MidiBackend, MidiDataFormat, MidiEventDeliveryMode,
    MidiEventSubscriptionFlags, MidiPortDirectionFlags, MidiPortListFlags, MidiProtocol,
};

/// Match the Windows MIDI backend row to its expected capability contract.
#[test]
fn test_midi_windows_midi_backend_row_matches_expected_capability_contract() {
    with_harness_context(|mut context| {
        let (
            _backend,
            name,
            support,
            priority,
            capability_flags,
            supported_data_formats,
            supported_protocols,
        ) = backend_descriptor_row(&mut context, MidiBackend::WindowsMidi)?;

        assert_eq!(name, "windows-midi");
        assert!(
            priority != 0,
            "WindowsMidi should participate in auto selection"
        );

        if !support_allows_host_execution(support) {
            assert_eq!(
                capability_flags.0, 0,
                "unavailable WindowsMidi rows should not advertise capabilities",
            );
            assert_eq!(
                supported_data_formats.0, 0,
                "unavailable WindowsMidi rows should not advertise data formats",
            );
            assert_eq!(
                supported_protocols.0, 0,
                "unavailable WindowsMidi rows should not advertise protocols",
            );

            return Ok(());
        }

        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0 != 0,
            "WindowsMidi should advertise topology events",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0 != 0,
            "WindowsMidi should advertise a native event feed",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0 != 0,
            "WindowsMidi should advertise receive timestamps",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_SCHEDULED_OUTPUT.0 != 0,
            "WindowsMidi should advertise scheduled output",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_UMP.0 != 0,
            "WindowsMidi should advertise UMP support",
        );
        assert_eq!(
            capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_INPUT.0,
            0,
            "WindowsMidi should not advertise virtual input support",
        );
        assert_eq!(
            capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_OUTPUT.0,
            0,
            "WindowsMidi should not advertise virtual output support",
        );
        assert_eq!(
            supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_UMP.0,
            "WindowsMidi should advertise only UMP transport",
        );
        assert_eq!(
            supported_protocols.0,
            MIDI_PROTOCOL_FLAG_MIDI1.0 | MIDI_PROTOCOL_FLAG_MIDI2.0,
            "WindowsMidi should advertise MIDI 1 and MIDI 2 protocol semantics",
        );

        Ok(())
    });
}

/// Open one native Windows MIDI event subscription without queued topology noise.
#[test]
fn test_midi_windows_midi_native_event_subscription_opens_without_pending_events() {
    with_harness_context(|mut context| {
        let (_backend, _name, support, _priority, _caps, _formats, _protocols) =
            backend_descriptor_row(&mut context, MidiBackend::WindowsMidi)?;
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        let options = harness_event_open_options_for_backend(
            &mut context,
            MidiBackend::WindowsMidi,
            MidiEventSubscriptionFlags(0),
            MidiPortDirectionFlags(
                MIDI_PORT_DIRECTION_FLAG_INPUT.0 | MIDI_PORT_DIRECTION_FLAG_OUTPUT.0,
            ),
            MidiEventDeliveryMode::NativeOnly,
        );
        let handle = context.destack_midi_event_open(options)?;

        assert_platform_error_codes(
            context.destack_midi_event_try_read(handle),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        context.destack_midi_event_close(handle)?;

        Ok(())
    });
}

/// Advertise stable Windows MIDI endpoint ids and UMP transport when ports are present.
#[test]
fn test_midi_windows_midi_port_rows_advertise_ump_transport() {
    with_harness_context(|mut context| {
        let (_backend, _name, support, _priority, _caps, _formats, _protocols) =
            backend_descriptor_row(&mut context, MidiBackend::WindowsMidi)?;
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        let input_rows = listed_backend_input_rows(
            &mut context,
            MidiBackend::WindowsMidi,
            MidiPortListFlags(0),
        )?;

        for (
            id,
            name,
            _is_connected,
            supported_data_formats,
            default_data_format,
            supported_protocols,
            default_protocol,
        ) in input_rows
        {
            assert!(
                id.starts_with("windows-midi:input:"),
                "WindowsMidi input ids should use the windows-midi:input prefix",
            );
            assert!(
                !name.is_empty(),
                "WindowsMidi input names should not be empty"
            );
            assert_eq!(
                supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_UMP.0,
                "WindowsMidi input rows should advertise only UMP transport",
            );
            assert!(
                supported_protocols.0 & (MIDI_PROTOCOL_FLAG_MIDI1.0 | MIDI_PROTOCOL_FLAG_MIDI2.0)
                    != 0,
                "WindowsMidi input rows should advertise at least one MIDI protocol",
            );
            assert_eq!(default_data_format, Some(MidiDataFormat::Ump));
            assert!(
                default_protocol == Some(MidiProtocol::Midi1)
                    || default_protocol == Some(MidiProtocol::Midi2)
                    || default_protocol.is_none(),
                "WindowsMidi input rows should default to MIDI 1, MIDI 2, or no protocol",
            );
        }

        let output_rows = listed_backend_output_rows(
            &mut context,
            MidiBackend::WindowsMidi,
            MidiPortListFlags(0),
        )?;

        for (
            id,
            name,
            _is_connected,
            supported_data_formats,
            default_data_format,
            supported_protocols,
            default_protocol,
        ) in output_rows
        {
            assert!(
                id.starts_with("windows-midi:output:"),
                "WindowsMidi output ids should use the windows-midi:output prefix",
            );
            assert!(
                !name.is_empty(),
                "WindowsMidi output names should not be empty"
            );
            assert_eq!(
                supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_UMP.0,
                "WindowsMidi output rows should advertise only UMP transport",
            );
            assert!(
                supported_protocols.0 & (MIDI_PROTOCOL_FLAG_MIDI1.0 | MIDI_PROTOCOL_FLAG_MIDI2.0)
                    != 0,
                "WindowsMidi output rows should advertise at least one MIDI protocol",
            );
            assert_eq!(default_data_format, Some(MidiDataFormat::Ump));
            assert!(
                default_protocol == Some(MidiProtocol::Midi1)
                    || default_protocol == Some(MidiProtocol::Midi2)
                    || default_protocol.is_none(),
                "WindowsMidi output rows should default to MIDI 1, MIDI 2, or no protocol",
            );
        }

        Ok(())
    });
}

/// Open Windows MIDI listed ports through the exact transport pairs they advertise.
#[test]
fn test_midi_windows_midi_listed_ports_open_with_their_advertised_transport_pairs() {
    with_harness_context(|mut context| {
        let (_backend, _name, support, _priority, _caps, _formats, _protocols) =
            backend_descriptor_row(&mut context, MidiBackend::WindowsMidi)?;
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        let input_rows = listed_backend_input_rows(
            &mut context,
            MidiBackend::WindowsMidi,
            MidiPortListFlags(0),
        )?;

        for (
            id,
            _name,
            _is_connected,
            supported_data_formats,
            default_data_format,
            supported_protocols,
            default_protocol,
        ) in input_rows
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
            let options = harness_input_open_options_for_backend_transport(
                &mut context,
                MidiBackend::WindowsMidi,
                Some(data_format),
                Some(protocol),
            );
            let handle = context.destack_midi_input_port_open(id, options)?;

            context.destack_midi_input_port_close(handle)?;
        }

        let output_rows = listed_backend_output_rows(
            &mut context,
            MidiBackend::WindowsMidi,
            MidiPortListFlags(0),
        )?;

        for (
            id,
            _name,
            _is_connected,
            supported_data_formats,
            default_data_format,
            supported_protocols,
            default_protocol,
        ) in output_rows
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
            let options = harness_output_open_options_for_backend_transport(
                &mut context,
                MidiBackend::WindowsMidi,
                Some(data_format),
                Some(protocol),
            );
            let handle = context.destack_midi_output_port_open(id, options)?;

            context.destack_midi_output_port_close(handle)?;
        }

        Ok(())
    });
}

/// Accept valid immediate and scheduled UMP writes on opened Windows MIDI outputs.
#[test]
fn test_midi_windows_midi_opened_output_ports_accept_valid_ump_writes() {
    with_harness_context(|mut context| {
        let (_backend, _name, support, _priority, _caps, _formats, _protocols) =
            backend_descriptor_row(&mut context, MidiBackend::WindowsMidi)?;
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        let output_rows = listed_backend_output_rows(
            &mut context,
            MidiBackend::WindowsMidi,
            MidiPortListFlags(0),
        )?;

        for (
            id,
            _name,
            _is_connected,
            supported_data_formats,
            default_data_format,
            supported_protocols,
            default_protocol,
        ) in output_rows
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
            let options = harness_output_open_options_for_backend_transport(
                &mut context,
                MidiBackend::WindowsMidi,
                Some(data_format),
                Some(protocol),
            );
            let handle = context.destack_midi_output_port_open(id, options)?;

            let record = output_record_for_transport(&mut context, data_format, protocol);
            let records = harness_output_records(&mut context, &[record])?;
            let written = context.destack_midi_output_write(handle, records)?;
            assert_eq!(
                written, 1,
                "WindowsMidi should accept one immediate UMP record"
            );

            let send_at_ns = monotonic_now_ns().saturating_add(10_000_000);
            let record = output_record_for_transport_with_timestamp(
                &mut context,
                data_format,
                protocol,
                Some(send_at_ns),
            );
            let records = harness_output_records(&mut context, &[record])?;
            let written = context.destack_midi_output_write(handle, records)?;
            assert_eq!(
                written, 1,
                "WindowsMidi should accept one scheduled UMP record"
            );

            context.destack_midi_output_port_close(handle)?;
        }

        Ok(())
    });
}

/// Reject virtual endpoint creation on Windows MIDI.
#[test]
fn test_midi_windows_midi_virtual_ports_report_not_supported() {
    with_harness_context(|mut context| {
        let input_options = harness_virtual_input_create_options_for_backend_transport(
            &mut context,
            MidiBackend::WindowsMidi,
            "Destack Windows MIDI Virtual Input",
            MidiDataFormat::Ump,
            MidiProtocol::Midi2,
        )?;
        assert_platform_error_codes(
            context.destack_midi_input_virtual_create(input_options),
            &[PlatformErrorCode::NotSupported],
        )?;

        let output_options = harness_virtual_output_create_options_for_backend_transport(
            &mut context,
            MidiBackend::WindowsMidi,
            "Destack Windows MIDI Virtual Output",
            MidiDataFormat::Ump,
            MidiProtocol::Midi2,
        )?;
        assert_platform_error_codes(
            context.destack_midi_output_virtual_create(output_options),
            &[PlatformErrorCode::NotSupported],
        )?;

        Ok(())
    });
}
