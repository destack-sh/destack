use super::{
    assert_ok_or_expected_error, assert_platform_error_codes, decode_backend_descriptors_full,
    harness_event_open_options_for_backend, harness_event_open_options_for_backend_with_policy,
    harness_input_open_options_for_backend_transport,
    harness_output_open_options_for_backend_transport, harness_port_list_options_for_backend,
    harness_port_list_options_for_backend_with_policy, harness_string, with_harness_context,
};
use crate::platform::VmValueCodec;
use crate::platform::core::BackendSupport;
use crate::platform::device::{
    MIDI_DATA_FORMAT_FLAG_UMP, MIDI_PORT_DIRECTION_FLAG_INPUT, MIDI_PORT_DIRECTION_FLAG_OUTPUT,
    MIDI_PROTOCOL_FLAG_MIDI2, MidiBackend, MidiBackendSelectionPolicy, MidiEventDeliveryMode,
    MidiEventSubscriptionFlags, MidiPortDirectionFlags, MidiPortListFlags,
};
use crate::platform::diagnostic::PlatformErrorCode;
use destack_vm as vm;

/// List MIDI backends or report one expected unsupported host error.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_backend_list_returns_rows_when_available_or_expected_error() {
    with_harness_context(|mut context| {
        // backend list
        let result = assert_ok_or_expected_error(
            context.destack_device_midi_backend_list(),
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

            if support != BackendSupport::Available {
                continue;
            }

            // selector-only rows are not concrete transport backends
            if matches!(backend, MidiBackend::Auto | MidiBackend::Null) {
                assert_eq!(
                    supported_data_formats.0, 0,
                    "selector-only midi backend rows should not advertise transport data formats",
                );
                assert_eq!(
                    supported_protocols.0, 0,
                    "selector-only midi backend rows should not advertise transport protocols",
                );

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

/// Keep the backend selector row set and order stable across native and VM bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_backend_list_reports_the_full_selector_inventory_in_order() {
    with_harness_context(|mut context| {
        // backend list
        let result = assert_ok_or_expected_error(
            context.destack_device_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(result) = result else {
            return Ok(());
        };

        // exact selector inventory
        let descriptors = decode_backend_descriptors_full(&mut context, result)?;
        let actual_backends = descriptors
            .iter()
            .map(
                |(backend, _name, _support, _priority, _capability_flags, _formats, _protocols)| {
                    *backend
                },
            )
            .collect::<Vec<_>>();
        let expected_backends = vec![
            MidiBackend::Auto,
            MidiBackend::Alsa,
            MidiBackend::JackMidi,
            MidiBackend::CoreMIDI,
            MidiBackend::WindowsMidi,
            MidiBackend::WinMM,
            MidiBackend::WinRT,
            MidiBackend::AndroidMidi,
            MidiBackend::Null,
        ];

        assert_eq!(
            actual_backends, expected_backends,
            "midi backend list should report the full selector inventory in selector order",
        );

        Ok(())
    });
}

/// Keep the raw MIDI backend wire values stable across native and VM bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_backend_vm_codec_preserves_exact_backend_wire_values() {
    // exact wire inventory
    let backend_wire_values = [
        (MidiBackend::Auto, 0u8),
        (MidiBackend::Alsa, 1u8),
        (MidiBackend::JackMidi, 2u8),
        (MidiBackend::CoreMIDI, 3u8),
        (MidiBackend::WindowsMidi, 4u8),
        (MidiBackend::WinMM, 5u8),
        (MidiBackend::WinRT, 6u8),
        (MidiBackend::AndroidMidi, 7u8),
        (MidiBackend::Null, 255u8),
    ];

    for (backend, raw_wire_value) in backend_wire_values {
        // repr stability
        assert_eq!(
            backend as u8, raw_wire_value,
            "midi backend discriminants should stay pinned to their public wire values",
        );

        // vm encoding
        let encoded = <MidiBackend as VmValueCodec>::encode(backend);
        assert_eq!(
            encoded.as_int(),
            raw_wire_value as i64,
            "midi backend VM encoding should use the generated 32 bit enum wire value",
        );

        // vm decoding
        let decoded =
            <MidiBackend as VmValueCodec>::decode(vm::Word::int(raw_wire_value as i64, 32))
                .expect("midi backend VM decoding should accept declared wire values");
        assert_eq!(
            decoded, backend,
            "midi backend VM decoding should roundtrip exact wire values",
        );
    }
}

/// Reject unknown MIDI backend wire values loudly.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_backend_vm_codec_rejects_unknown_backend_wire_values() {
    // invalid wire values
    let invalid_wire_values = [8u8, 9u8, 254u8];

    for raw_wire_value in invalid_wire_values {
        let error = <MidiBackend as VmValueCodec>::decode(vm::Word::int(raw_wire_value as i64, 32))
            .expect_err("midi backend VM decoding should reject unknown wire values");

        // exact failure class
        assert_eq!(
            error.platform_error().map(|error| error.code),
            Some(PlatformErrorCode::InvalidArgumentValue),
            "unknown midi backend wire values should fail with invalidArgumentValue",
        );
        assert_eq!(
            error.message(),
            "unknown MidiBackend value",
            "unknown midi backend wire values should keep the generated codec error message stable",
        );
    }
}

/// Keep unavailable backend rows zeroed so selector metadata matches availability.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_backend_rows_zero_capabilities_when_unavailable() {
    with_harness_context(|mut context| {
        // backend rows
        let result = assert_ok_or_expected_error(
            context.destack_device_midi_backend_list(),
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

/// Reject explicit backend operations when selector support says the host lane is unavailable.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_unavailable_backends_reject_explicit_list_and_event_selection() {
    with_harness_context(|mut context| {
        // backend rows
        let result = assert_ok_or_expected_error(
            context.destack_device_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(result) = result else {
            return Ok(());
        };

        // explicit unavailable selectors
        let descriptors = decode_backend_descriptors_full(&mut context, result)?;
        for (backend, _name, support, _priority, _capability_flags, _formats, _protocols) in
            descriptors
        {
            if support == BackendSupport::Available || backend == MidiBackend::Null {
                continue;
            }

            let list_options = harness_port_list_options_for_backend_with_policy(
                &mut context,
                backend,
                MidiBackendSelectionPolicy::Strict,
                MidiPortListFlags(0),
            );
            assert_platform_error_codes(
                context.destack_device_midi_input_port_list(list_options),
                &[PlatformErrorCode::NotSupported],
            )?;

            let list_options = harness_port_list_options_for_backend_with_policy(
                &mut context,
                backend,
                MidiBackendSelectionPolicy::Strict,
                MidiPortListFlags(0),
            );
            assert_platform_error_codes(
                context.destack_device_midi_output_port_list(list_options),
                &[PlatformErrorCode::NotSupported],
            )?;

            let direction_mask = MidiPortDirectionFlags(
                MIDI_PORT_DIRECTION_FLAG_INPUT.0 | MIDI_PORT_DIRECTION_FLAG_OUTPUT.0,
            );
            let options = harness_event_open_options_for_backend_with_policy(
                &mut context,
                backend,
                MidiBackendSelectionPolicy::Strict,
                MidiEventSubscriptionFlags(0),
                direction_mask,
                MidiEventDeliveryMode::Auto,
            );
            assert_platform_error_codes(
                context.destack_device_midi_event_open(options),
                &[PlatformErrorCode::NotSupported],
            )?;
        }

        Ok(())
    });
}

/// Reject explicit unavailable backend port opens before endpoint fallback can happen.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_unavailable_backends_reject_explicit_port_open_selection() {
    with_harness_context(|mut context| {
        let result = assert_ok_or_expected_error(
            context.destack_device_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(result) = result else {
            return Ok(());
        };

        let descriptors = decode_backend_descriptors_full(&mut context, result)?;
        for (backend, _name, support, _priority, _capability_flags, _formats, _protocols) in
            descriptors
        {
            if support == BackendSupport::Available || backend == MidiBackend::Null {
                continue;
            }

            let missing_port_id = harness_string(&mut context, "missing:strict-backend-port")?;
            let options =
                harness_input_open_options_for_backend_transport(&mut context, backend, None, None);
            assert_platform_error_codes(
                context.destack_device_midi_input_port_open(missing_port_id, options),
                &[PlatformErrorCode::NotSupported],
            )?;

            let missing_port_id = harness_string(&mut context, "missing:strict-backend-port")?;
            let options = harness_output_open_options_for_backend_transport(
                &mut context,
                backend,
                None,
                None,
            );
            assert_platform_error_codes(
                context.destack_device_midi_output_port_open(missing_port_id, options),
                &[PlatformErrorCode::NotSupported],
            )?;
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
            context.destack_device_midi_backend_list(),
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
            context.destack_device_midi_input_port_list(list_options)?;

            let list_options =
                harness_port_list_options_for_backend(&mut context, backend, MidiPortListFlags(0));
            context.destack_device_midi_output_port_list(list_options)?;

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
            let handle = context.destack_device_midi_event_open(options)?;
            context.destack_device_midi_event_close(handle)?;
        }

        Ok(())
    });
}
