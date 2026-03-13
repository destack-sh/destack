use super::{
    assert_platform_error_codes, decode_backend_descriptors_full, decode_port_descriptors,
    harness_event_open_options_for_backend, harness_input_open_options_for_backend_transport,
    harness_output_open_options_for_backend_transport, harness_port_list_options_for_backend,
    harness_string, preferred_transport_pair, support_allows_host_execution, with_harness_context,
};
use crate::platform::core::BackendSupport;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::midi::{
    MIDI_BACKEND_CAP_NATIVE_EVENT_FEED, MIDI_PORT_DIRECTION_FLAG_INPUT,
    MIDI_PORT_DIRECTION_FLAG_OUTPUT, MidiBackend, MidiEventDeliveryMode,
    MidiEventSubscriptionFlags, MidiPortDirectionFlags, MidiPortListFlags,
};

/// Keep the Android MIDI selector row identity stable.
#[test]
fn test_midi_android_backend_row_has_stable_identity_and_priority() {
    with_harness_context(|mut context| {
        // backend rows
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let descriptor = descriptors
            .into_iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::AndroidMidi)
            .expect("midi backend list should include an AndroidMidi selector row");

        let (
            _backend,
            name,
            support,
            priority,
            capability_flags,
            supported_data_formats,
            supported_protocols,
        ) = descriptor;

        // row identity
        assert_eq!(name, "android-midi");
        assert_ne!(
            priority, 0,
            "AndroidMidi should participate in auto selection"
        );

        let _ = support;
        let _ = capability_flags;
        let _ = supported_data_formats;
        let _ = supported_protocols;

        Ok(())
    });
}

/// Match Android MIDI transport metadata to the advertised availability state.
#[test]
fn test_midi_android_backend_row_zeroes_metadata_when_unavailable_and_advertises_transport_when_available()
 {
    with_harness_context(|mut context| {
        // backend rows
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let descriptor = descriptors
            .into_iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::AndroidMidi)
            .expect("midi backend list should include an AndroidMidi selector row");

        let (
            _backend,
            _name,
            support,
            _priority,
            capability_flags,
            supported_data_formats,
            supported_protocols,
        ) = descriptor;

        // unavailable rows must stay zeroed
        if support != BackendSupport::Available {
            assert_eq!(
                capability_flags.0, 0,
                "unavailable AndroidMidi rows should not advertise capabilities",
            );
            assert_eq!(
                supported_data_formats.0, 0,
                "unavailable AndroidMidi rows should not advertise data formats",
            );
            assert_eq!(
                supported_protocols.0, 0,
                "unavailable AndroidMidi rows should not advertise protocols",
            );

            return Ok(());
        }

        // available hosts must expose at least one transport contract
        assert_ne!(
            supported_data_formats.0, 0,
            "available AndroidMidi rows should advertise at least one data format",
        );
        assert_ne!(
            supported_protocols.0, 0,
            "available AndroidMidi rows should advertise at least one protocol",
        );

        Ok(())
    });
}

/// Respect the advertised Android native-event capability contract.
#[test]
fn test_midi_android_native_event_delivery_matches_the_advertised_capability() {
    with_harness_context(|mut context| {
        // backend rows
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let (_backend, _name, support, _priority, capability_flags, _formats, _protocols) =
            descriptors
                .into_iter()
                .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::AndroidMidi)
                .expect("midi backend list should include an AndroidMidi selector row");

        if !support_allows_host_execution(support) {
            return Ok(());
        }

        let options = harness_event_open_options_for_backend(
            &mut context,
            MidiBackend::AndroidMidi,
            MidiEventSubscriptionFlags(0),
            MidiPortDirectionFlags(
                MIDI_PORT_DIRECTION_FLAG_INPUT.0 | MIDI_PORT_DIRECTION_FLAG_OUTPUT.0,
            ),
            MidiEventDeliveryMode::NativeOnly,
        );

        // native feed advertised: native-only subscription must work
        if capability_flags.0 & MIDI_BACKEND_CAP_NATIVE_EVENT_FEED.0 != 0 {
            let handle = context.destack_midi_event_open(options)?;

            assert_platform_error_codes(
                context.destack_midi_event_try_read(handle),
                &[PlatformErrorCode::IoWouldBlock],
            )?;

            context.destack_midi_event_close(handle)?;

            return Ok(());
        }

        // native feed not advertised: native-only subscription must fail loudly
        assert_platform_error_codes(
            context.destack_midi_event_open(options),
            &[PlatformErrorCode::NotSupported],
        )?;

        Ok(())
    });
}

/// Open Android MIDI listed ports through the transport pairs they advertise.
#[test]
fn test_midi_android_listed_ports_open_with_their_advertised_transport_pairs() {
    with_harness_context(|mut context| {
        let descriptors = context.destack_midi_backend_list()?;
        let descriptors = decode_backend_descriptors_full(&mut context, descriptors)?;
        let support = descriptors
            .iter()
            .find(|(backend, _, _, _, _, _, _)| *backend == MidiBackend::AndroidMidi)
            .map(|(_, _, support, _, _, _, _)| *support)
            .expect("midi backend list should include an AndroidMidi selector row");
        if !support_allows_host_execution(support) {
            return Ok(());
        }

        let input_options = harness_port_list_options_for_backend(
            &mut context,
            MidiBackend::AndroidMidi,
            MidiPortListFlags(0),
        );
        let input_rows = context.destack_midi_input_port_list(input_options)?;
        let input_rows = decode_port_descriptors(&mut context, input_rows)?;

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
                MidiBackend::AndroidMidi,
                Some(data_format),
                Some(protocol),
            );
            let handle = context.destack_midi_input_port_open(id, options)?;

            context.destack_midi_input_port_close(handle)?;
        }

        let output_options = harness_port_list_options_for_backend(
            &mut context,
            MidiBackend::AndroidMidi,
            MidiPortListFlags(0),
        );
        let output_rows = context.destack_midi_output_port_list(output_options)?;
        let output_rows = decode_port_descriptors(&mut context, output_rows)?;

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
                MidiBackend::AndroidMidi,
                Some(data_format),
                Some(protocol),
            );
            let handle = context.destack_midi_output_port_open(id, options)?;

            context.destack_midi_output_port_close(handle)?;
        }

        Ok(())
    });
}
