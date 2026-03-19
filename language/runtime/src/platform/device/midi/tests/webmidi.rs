use super::{
    assert_ok_or_expected_error, decode_backend_descriptors_full, decode_event,
    decode_port_descriptor_identities, harness_event_open_options_for_backend,
    harness_input_open_options_for_backend_transport,
    harness_output_open_options_for_backend_transport, harness_port_list_options_for_backend,
    harness_string, harness_virtual_input_create_options_for_backend_transport,
    preferred_transport_pair, preferred_web_midi_transport_pair, support_allows_host_execution,
    supports_web_midi_transport, with_harness_context,
};
use crate::platform::device::{
    MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_BACKEND_CAP_VIRTUAL_INPUT,
    MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL, MIDI_PORT_DIRECTION_FLAG_OUTPUT, MidiBackend,
    MidiEventDeliveryMode, MidiEventSource, MidiPortDirection, MidiPortDirectionFlags,
    MidiPortListFlags,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// Expose enough descriptor identity and state to back one Web MIDI port object.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_port_descriptors_expose_identity_and_state_needed_for_web_midi() {
    with_harness_context(|mut context| {
        let backends = assert_ok_or_expected_error(
            context.destack_device_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(backends) = backends else {
            return Ok(());
        };

        let backends = decode_backend_descriptors_full(&mut context, backends)?;
        for (backend, _name, support, _priority, _caps, _formats, _protocols) in backends {
            if !support_allows_host_execution(support) {
                continue;
            }

            if matches!(backend, MidiBackend::Auto | MidiBackend::Null) {
                continue;
            }

            let input_options =
                harness_port_list_options_for_backend(&mut context, backend, MidiPortListFlags(0));
            let input_rows = context.destack_device_midi_input_port_list(input_options)?;
            let input_rows = decode_port_descriptor_identities(&mut context, input_rows)?;
            for (id, _manufacturer, name, _version, _backend_id, _is_virtual, _is_connected) in
                input_rows
            {
                assert!(!id.is_empty(), "web midi port ids must not be empty");
                assert!(!name.is_empty(), "web midi port names must not be empty");
            }

            let output_options =
                harness_port_list_options_for_backend(&mut context, backend, MidiPortListFlags(0));
            let output_rows = context.destack_device_midi_output_port_list(output_options)?;
            let output_rows = decode_port_descriptor_identities(&mut context, output_rows)?;
            for (id, _manufacturer, name, _version, _backend_id, _is_virtual, _is_connected) in
                output_rows
            {
                assert!(!id.is_empty(), "web midi port ids must not be empty");
                assert!(!name.is_empty(), "web midi port names must not be empty");
            }
        }

        Ok(())
    });
}

/// Open listed ports with one Web MIDI compatible transport shape when the port advertises one.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_listed_ports_open_with_web_midi_compatible_transport_when_available() {
    with_harness_context(|mut context| {
        let backends = assert_ok_or_expected_error(
            context.destack_device_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(backends) = backends else {
            return Ok(());
        };

        let backends = decode_backend_descriptors_full(&mut context, backends)?;
        for (backend, _name, support, _priority, _caps, _formats, _protocols) in backends {
            if !support_allows_host_execution(support) {
                continue;
            }

            if matches!(backend, MidiBackend::Auto | MidiBackend::Null) {
                continue;
            }

            let input_options =
                harness_port_list_options_for_backend(&mut context, backend, MidiPortListFlags(0));
            let input_rows = context.destack_device_midi_input_port_list(input_options)?;
            let input_rows = super::decode_port_descriptors(&mut context, input_rows)?;
            for (
                id,
                _name,
                _is_connected,
                supported_data_formats,
                _default_data_format,
                supported_protocols,
                _default_protocol,
            ) in input_rows
            {
                if !supports_web_midi_transport(supported_data_formats, supported_protocols) {
                    continue;
                }

                let Some((data_format, protocol)) =
                    preferred_web_midi_transport_pair(supported_data_formats, supported_protocols)
                else {
                    continue;
                };

                let id = harness_string(&mut context, &id)?;
                let options = harness_input_open_options_for_backend_transport(
                    &mut context,
                    backend,
                    Some(data_format),
                    Some(protocol),
                );
                let handle = context.destack_device_midi_input_port_open(id, options)?;

                context.destack_device_midi_input_port_close(handle)?;
            }

            let output_options =
                harness_port_list_options_for_backend(&mut context, backend, MidiPortListFlags(0));
            let output_rows = context.destack_device_midi_output_port_list(output_options)?;
            let output_rows = super::decode_port_descriptors(&mut context, output_rows)?;
            for (
                id,
                _name,
                _is_connected,
                supported_data_formats,
                _default_data_format,
                supported_protocols,
                _default_protocol,
            ) in output_rows
            {
                if !supports_web_midi_transport(supported_data_formats, supported_protocols) {
                    continue;
                }

                let Some((data_format, protocol)) =
                    preferred_web_midi_transport_pair(supported_data_formats, supported_protocols)
                else {
                    continue;
                };

                let id = harness_string(&mut context, &id)?;
                let options = harness_output_open_options_for_backend_transport(
                    &mut context,
                    backend,
                    Some(data_format),
                    Some(protocol),
                );
                let handle = context.destack_device_midi_output_port_open(id, options)?;

                context.destack_device_midi_output_port_close(handle)?;
            }
        }

        Ok(())
    });
}

/// Expose stable identity through topology events so one Web MIDI wrapper can synthesize statechange.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_topology_events_expose_statechange_identity_needed_for_web_midi() {
    with_harness_context(|mut context| {
        let backends = assert_ok_or_expected_error(
            context.destack_device_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(backends) = backends else {
            return Ok(());
        };

        let backends = decode_backend_descriptors_full(&mut context, backends)?;
        for (
            backend,
            _name,
            support,
            _priority,
            capability_flags,
            supported_data_formats,
            supported_protocols,
        ) in backends
        {
            if !support_allows_host_execution(support) {
                continue;
            }

            if matches!(backend, MidiBackend::Auto | MidiBackend::Null) {
                continue;
            }

            if capability_flags.0 & MIDI_BACKEND_CAP_TOPOLOGY_EVENTS.0 == 0 {
                continue;
            }

            if capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_INPUT.0 == 0 {
                continue;
            }

            let Some((data_format, protocol)) =
                preferred_transport_pair(supported_data_formats, None, supported_protocols, None)
            else {
                continue;
            };

            let event_options = harness_event_open_options_for_backend(
                &mut context,
                backend,
                MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL,
                MidiPortDirectionFlags(MIDI_PORT_DIRECTION_FLAG_OUTPUT.0),
                MidiEventDeliveryMode::Auto,
            );
            let event_handle = context.destack_device_midi_event_open(event_options)?;

            let name = format!("Destack WebMIDI StateChange {backend:?}");
            let create_options = harness_virtual_input_create_options_for_backend_transport(
                &mut context,
                backend,
                &name,
                data_format,
                protocol,
            )?;
            let input_handle = context.destack_device_midi_input_virtual_create(create_options)?;

            let event = context.destack_device_midi_event_read(event_handle, 1_000_000_000)?;
            let event = decode_event(&mut context, event)?;

            assert_eq!(event.kind, "portAdded");
            assert_eq!(event.source, MidiEventSource::Native);
            assert_eq!(event.direction, Some(MidiPortDirection::Output));
            assert_eq!(event.is_virtual, Some(true));
            assert!(
                event.id.as_deref().is_some_and(|id| !id.is_empty()),
                "web midi statechange needs one stable non-empty port id",
            );

            context.destack_device_midi_input_port_close(input_handle)?;
            context.destack_device_midi_event_close(event_handle)?;
        }

        Ok(())
    });
}
