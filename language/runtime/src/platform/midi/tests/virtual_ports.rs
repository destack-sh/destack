use super::{
    assert_ok_or_expected_error, assert_platform_error_codes, decode_backend_descriptors_full,
    decode_port_descriptor, harness_virtual_input_create_options_for_backend_transport,
    harness_virtual_output_create_options_for_backend_transport, preferred_transport_pair,
    with_harness_context,
};
use crate::platform::core::BackendSupport;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::midi::{
    MIDI_BACKEND_CAP_VIRTUAL_INPUT, MIDI_BACKEND_CAP_VIRTUAL_OUTPUT, MidiBackend,
};

/// Create virtual MIDI input endpoints only on backends that advertise that capability.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_virtual_input_creation_matches_backend_capabilities() {
    with_harness_context(|mut context| {
        // backend rows
        let result = assert_ok_or_expected_error(
            context.destack_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(result) = result else {
            return Ok(());
        };

        // backend behavior
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
            if support != BackendSupport::Available || backend == MidiBackend::Null {
                continue;
            }

            let Some((data_format, protocol)) =
                preferred_transport_pair(supported_data_formats, None, supported_protocols, None)
            else {
                continue;
            };

            let name = format!("Destack MIDI Virtual Input {backend:?}");
            let options = harness_virtual_input_create_options_for_backend_transport(
                &mut context,
                backend,
                &name,
                data_format,
                protocol,
            )?;

            // advertised support
            if capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_INPUT.0 == 0 {
                assert_platform_error_codes(
                    context.destack_midi_input_virtual_create(options),
                    &[PlatformErrorCode::NotSupported],
                )?;

                continue;
            }

            // created descriptor
            let handle = context.destack_midi_input_virtual_create(options)?;
            let descriptor = context.destack_midi_input_port_descriptor(handle)?;
            let (
                _id,
                _backend_id,
                opened_name,
                is_virtual,
                _is_connected,
                opened_format,
                opened_protocol,
            ) = decode_port_descriptor(&mut context, descriptor)?;
            assert_eq!(opened_name, name);
            assert!(
                is_virtual,
                "virtual MIDI input descriptors should report isVirtual"
            );
            assert_eq!(opened_format, Some(data_format));
            assert_eq!(opened_protocol, Some(protocol));

            // resource teardown
            context.destack_midi_input_port_close(handle)?;
        }

        Ok(())
    });
}

/// Create virtual MIDI output endpoints only on backends that advertise that capability.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_virtual_output_creation_matches_backend_capabilities() {
    with_harness_context(|mut context| {
        // backend rows
        let result = assert_ok_or_expected_error(
            context.destack_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(result) = result else {
            return Ok(());
        };

        // backend behavior
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
            if support != BackendSupport::Available || backend == MidiBackend::Null {
                continue;
            }

            let Some((data_format, protocol)) =
                preferred_transport_pair(supported_data_formats, None, supported_protocols, None)
            else {
                continue;
            };

            let name = format!("Destack MIDI Virtual Output {backend:?}");
            let options = harness_virtual_output_create_options_for_backend_transport(
                &mut context,
                backend,
                &name,
                data_format,
                protocol,
            )?;

            // advertised support
            if capability_flags.0 & MIDI_BACKEND_CAP_VIRTUAL_OUTPUT.0 == 0 {
                assert_platform_error_codes(
                    context.destack_midi_output_virtual_create(options),
                    &[PlatformErrorCode::NotSupported],
                )?;

                continue;
            }

            // created descriptor
            let handle = context.destack_midi_output_virtual_create(options)?;
            let descriptor = context.destack_midi_output_port_descriptor(handle)?;
            let (
                _id,
                _backend_id,
                opened_name,
                is_virtual,
                _is_connected,
                opened_format,
                opened_protocol,
            ) = decode_port_descriptor(&mut context, descriptor)?;
            assert_eq!(opened_name, name);
            assert!(
                is_virtual,
                "virtual MIDI output descriptors should report isVirtual",
            );
            assert_eq!(opened_format, Some(data_format));
            assert_eq!(opened_protocol, Some(protocol));

            // resource teardown
            context.destack_midi_output_port_close(handle)?;
        }

        Ok(())
    });
}
