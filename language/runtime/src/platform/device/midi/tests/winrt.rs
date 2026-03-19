use super::{
    assert_platform_error_codes, decode_backend_descriptors_full, decode_port_descriptors,
    harness_event_open_options_for_backend, harness_output_open_options_for_backend_transport,
    harness_output_records, harness_port_list_options_for_backend, harness_string,
    harness_virtual_input_create_options_for_backend_transport,
    harness_virtual_output_create_options_for_backend_transport, support_allows_host_execution,
    with_harness_context,
};
use crate::platform::device::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS,
    MIDI_BACKEND_CAP_SCHEDULED_OUTPUT, MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_BACKEND_CAP_UMP,
    MIDI_BACKEND_CAP_VIRTUAL_INPUT, MIDI_BACKEND_CAP_VIRTUAL_OUTPUT,
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PORT_DIRECTION_FLAG_INPUT,
    MIDI_PORT_DIRECTION_FLAG_OUTPUT, MIDI_PROTOCOL_FLAG_MIDI1, MidiBackend, MidiDataFormat,
    MidiEventDeliveryMode, MidiEventSubscriptionFlags, MidiOutputRecord, MidiPortDirectionFlags,
    MidiPortListFlags, MidiProtocol, MidiRecordFraming,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// Match the WinRT backend row to its expected capability contract.
#[test]
fn test_midi_winrt_backend_row_matches_expected_capability_contract() {
    with_harness_context(|mut context| {
        // backend rows
        let descriptors = context.destack_device_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let winrt_row = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::WinRT)
            .expect("midi backend list should include a WinRT selector row");
        let (
            _backend,
            name,
            support,
            priority,
            capability_flags,
            supported_data_formats,
            supported_protocols,
        ) = winrt_row;

        // winrt row
        assert_eq!(name, "winrt");
        assert!(*priority != 0, "WinRT should participate in auto selection");

        // unavailable rows must stay zeroed
        if !support_allows_host_execution(*support) {
            assert_eq!(capability_flags.0, 0);
            assert_eq!(supported_data_formats.0, 0);
            assert_eq!(supported_protocols.0, 0);

            return Ok(());
        }

        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0 != 0,
            "WinRT should advertise topology events",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0 != 0,
            "WinRT should advertise a native event feed",
        );
        assert!(
            capability_flags.0 & MIDI_BACKEND_CAP_RECEIVE_TIMESTAMPS.0 != 0,
            "WinRT should advertise receive timestamps",
        );
        assert_eq!(
            capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_INPUT.0,
            0,
            "WinRT should not advertise virtual input support",
        );
        assert_eq!(
            capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_OUTPUT.0,
            0,
            "WinRT should not advertise virtual output support",
        );
        assert_eq!(
            capability_flags.0 & MIDI_BACKEND_CAP_SCHEDULED_OUTPUT.0,
            0,
            "WinRT should not advertise scheduled output support",
        );
        assert_eq!(
            capability_flags.0 & MIDI_BACKEND_CAP_UMP.0,
            0,
            "WinRT should not advertise UMP transport support",
        );
        assert_eq!(
            supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0,
            "WinRT should advertise only MIDI 1 byte-stream transport",
        );
        assert_eq!(
            supported_protocols.0, MIDI_PROTOCOL_FLAG_MIDI1.0,
            "WinRT should advertise only MIDI 1 protocol semantics",
        );

        Ok(())
    });
}

/// Open one native WinRT event subscription without queued topology noise.
#[test]
fn test_midi_winrt_native_event_subscription_opens_without_pending_events() {
    with_harness_context(|mut context| {
        // backend support
        let descriptors = context.destack_device_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let support = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::WinRT)
            .map(|(_, _, support, _, _, _, _)| *support)
            .expect("midi backend list should include a WinRT selector row");
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        // native event subscription
        let options = harness_event_open_options_for_backend(
            &mut context,
            MidiBackend::WinRT,
            MidiEventSubscriptionFlags(0),
            MidiPortDirectionFlags(
                MIDI_PORT_DIRECTION_FLAG_INPUT.0 | MIDI_PORT_DIRECTION_FLAG_OUTPUT.0,
            ),
            MidiEventDeliveryMode::NativeOnly,
        );
        let handle = context.destack_device_midi_event_open(options)?;

        // initial queue state
        assert_platform_error_codes(
            context.destack_device_midi_event_try_read(handle),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        // teardown
        context.destack_device_midi_event_close(handle)?;

        Ok(())
    });
}

/// Advertise MIDI 1 byte-stream transport for WinRT endpoint rows when ports are present.
#[test]
fn test_midi_winrt_port_rows_advertise_midi1_transport_only_when_present() {
    with_harness_context(|mut context| {
        // backend support
        let descriptors = context.destack_device_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let support = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::WinRT)
            .map(|(_, _, support, _, _, _, _)| *support)
            .expect("midi backend list should include a WinRT selector row");
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        // input rows
        let input_options = harness_port_list_options_for_backend(
            &mut context,
            MidiBackend::WinRT,
            MidiPortListFlags(0),
        );
        let input_rows = context.destack_device_midi_input_port_list(input_options)?;
        let input_rows = decode_port_descriptors(&mut context, input_rows)?;

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
                id.starts_with("winrt:input:"),
                "WinRT input ids should use the winrt:input prefix"
            );
            assert!(!name.is_empty(), "WinRT input names should not be empty");
            assert_eq!(
                supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0,
                "WinRT input rows should advertise only MIDI 1 byte-stream transport",
            );
            assert_eq!(
                supported_protocols.0, MIDI_PROTOCOL_FLAG_MIDI1.0,
                "WinRT input rows should advertise only MIDI 1 protocol semantics",
            );
            assert_eq!(default_data_format, Some(MidiDataFormat::Midi1Bytes));
            assert_eq!(default_protocol, Some(MidiProtocol::Midi1));
        }

        // output rows
        let output_options = harness_port_list_options_for_backend(
            &mut context,
            MidiBackend::WinRT,
            MidiPortListFlags(0),
        );
        let output_rows = context.destack_device_midi_output_port_list(output_options)?;
        let output_rows = decode_port_descriptors(&mut context, output_rows)?;

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
                id.starts_with("winrt:output:"),
                "WinRT output ids should use the winrt:output prefix"
            );
            assert!(!name.is_empty(), "WinRT output names should not be empty");
            assert_eq!(
                supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0,
                "WinRT output rows should advertise only MIDI 1 byte-stream transport",
            );
            assert_eq!(
                supported_protocols.0, MIDI_PROTOCOL_FLAG_MIDI1.0,
                "WinRT output rows should advertise only MIDI 1 protocol semantics",
            );
            assert_eq!(default_data_format, Some(MidiDataFormat::Midi1Bytes));
            assert_eq!(default_protocol, Some(MidiProtocol::Midi1));
        }

        Ok(())
    });
}

/// Reject virtual endpoint creation on WinRT.
#[test]
fn test_midi_winrt_virtual_ports_report_not_supported() {
    with_harness_context(|mut context| {
        // virtual input
        let input_options = harness_virtual_input_create_options_for_backend_transport(
            &mut context,
            MidiBackend::WinRT,
            "Destack WinRT Virtual Input",
            MidiDataFormat::Midi1Bytes,
            MidiProtocol::Midi1,
        )?;
        assert_platform_error_codes(
            context.destack_device_midi_input_virtual_create(input_options),
            &[PlatformErrorCode::NotSupported],
        )?;

        // virtual output
        let output_options = harness_virtual_output_create_options_for_backend_transport(
            &mut context,
            MidiBackend::WinRT,
            "Destack WinRT Virtual Output",
            MidiDataFormat::Midi1Bytes,
            MidiProtocol::Midi1,
        )?;
        assert_platform_error_codes(
            context.destack_device_midi_output_virtual_create(output_options),
            &[PlatformErrorCode::NotSupported],
        )?;

        Ok(())
    });
}

/// Reject scheduled output timestamps on opened WinRT output ports.
#[test]
fn test_midi_winrt_output_write_rejects_scheduled_timestamps_when_ports_are_present() {
    with_harness_context(|mut context| {
        let descriptors = context.destack_device_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let support = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::WinRT)
            .map(|(_, _, support, _, _, _, _)| *support)
            .expect("midi backend list should include a WinRT selector row");
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        let output_options = harness_port_list_options_for_backend(
            &mut context,
            MidiBackend::WinRT,
            MidiPortListFlags(0),
        );
        let output_rows = context.destack_device_midi_output_port_list(output_options)?;
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
            MidiBackend::WinRT,
            Some(MidiDataFormat::Midi1Bytes),
            Some(MidiProtocol::Midi1),
        );
        let handle = context.destack_device_midi_output_port_open(id, open_options)?;

        let record = MidiOutputRecord {
            send_at_ns: Some(1),
            data_format: MidiDataFormat::Midi1Bytes,
            protocol: Some(MidiProtocol::Midi1),
            framing: MidiRecordFraming::Complete,
            data: context.call_context.store_slice(vec![0x90, 0x3C, 0x40]),
        };
        let records = harness_output_records(&mut context, &[record])?;
        assert_platform_error_codes(
            context.destack_device_midi_output_write(handle, records),
            &[PlatformErrorCode::InvalidArgument],
        )?;

        context.destack_device_midi_output_port_close(handle)?;

        Ok(())
    });
}
