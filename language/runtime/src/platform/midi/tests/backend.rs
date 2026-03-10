use super::{
    assert_ok_or_expected_error, decode_backend_descriptors_full,
    harness_event_open_options_for_backend, harness_port_list_options_for_backend,
    with_harness_context,
};
use crate::platform::core::BackendSupport;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::midi::{
    MIDI_DATA_FORMAT_FLAG_UMP, MIDI_PORT_DIRECTION_FLAG_INPUT, MIDI_PORT_DIRECTION_FLAG_OUTPUT,
    MIDI_PROTOCOL_FLAG_MIDI2, MidiBackend, MidiEventDeliveryMode, MidiEventSubscriptionFlags,
    MidiPortDirectionFlags, MidiPortListFlags,
};

/// List MIDI backends or report one expected unsupported host error.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_backend_list_returns_rows_when_available_or_expected_error() {
    with_harness_context(|mut context| {
        // backend list
        let result = assert_ok_or_expected_error(
            context.destack_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(result) = result else {
            return Ok(());
        };

        // descriptor invariants
        let descriptors = decode_backend_descriptors_full(&mut context, result)?;
        assert!(
            !descriptors.is_empty(),
            "midi backend list should not be empty when supported",
        );

        for (
            backend,
            name,
            support,
            _priority,
            _capability_flags,
            supported_data_formats,
            supported_protocols,
        ) in descriptors
        {
            assert!(!name.is_empty(), "midi backend name should not be empty");

            if backend == MidiBackend::Null || support != BackendSupport::Available {
                continue;
            }

            assert!(
                supported_data_formats.0 != 0,
                "non-null midi backends should advertise at least one data format",
            );
            assert!(
                supported_protocols.0 != 0,
                "non-null midi backends should advertise at least one protocol",
            );

            // midi 2 semantics require ump transport
            if supported_protocols.0 & MIDI_PROTOCOL_FLAG_MIDI2.0 != 0 {
                assert!(
                    supported_data_formats.0 & MIDI_DATA_FORMAT_FLAG_UMP.0 != 0,
                    "midi backends that advertise MIDI 2 should advertise UMP transport",
                );
            }
        }

        Ok(())
    });
}

/// Keep unavailable backend rows zeroed so selector metadata stays honest.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_backend_rows_zero_capabilities_when_unavailable() {
    with_harness_context(|mut context| {
        // backend rows
        let result = assert_ok_or_expected_error(
            context.destack_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(result) = result else {
            return Ok(());
        };

        // support contract
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
            if support == BackendSupport::Available || backend == MidiBackend::Null {
                continue;
            }

            assert_eq!(
                capability_flags.0, 0,
                "unavailable midi backends should not advertise capabilities",
            );
            assert_eq!(
                supported_data_formats.0, 0,
                "unavailable midi backends should not advertise data formats",
            );
            assert_eq!(
                supported_protocols.0, 0,
                "unavailable midi backends should not advertise protocols",
            );
        }

        Ok(())
    });
}

/// Keep explicit backend selection aligned with the descriptor surface.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_available_backends_support_explicit_list_and_event_selection() {
    with_harness_context(|mut context| {
        // available backends
        let result = assert_ok_or_expected_error(
            context.destack_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(result) = result else {
            return Ok(());
        };

        // explicit selector behavior
        let descriptors = decode_backend_descriptors_full(&mut context, result)?;
        for (backend, _name, support, _priority, capability_flags, _formats, _protocols) in
            descriptors
        {
            if support != BackendSupport::Available || backend == MidiBackend::Null {
                continue;
            }

            let list_options =
                harness_port_list_options_for_backend(&mut context, backend, MidiPortListFlags(0));
            context.destack_midi_input_port_list(list_options)?;

            let list_options =
                harness_port_list_options_for_backend(&mut context, backend, MidiPortListFlags(0));
            context.destack_midi_output_port_list(list_options)?;

            if capability_flags.0 == 0 {
                continue;
            }

            let direction_mask = MidiPortDirectionFlags(
                MIDI_PORT_DIRECTION_FLAG_INPUT.0 | MIDI_PORT_DIRECTION_FLAG_OUTPUT.0,
            );
            let options = harness_event_open_options_for_backend(
                &mut context,
                backend,
                MidiEventSubscriptionFlags(0),
                direction_mask,
                MidiEventDeliveryMode::Auto,
            );
            let handle = context.destack_midi_event_open(options)?;
            context.destack_midi_event_close(handle)?;
        }

        Ok(())
    });
}
