use super::{
    assert_ok_or_expected_error, assert_platform_error_codes, decode_backend_descriptors_full,
    decode_event, decode_events, harness_event_open_options,
    harness_event_open_options_for_backend,
    harness_virtual_input_create_options_for_backend_transport, preferred_transport_pair,
    with_harness_context,
};
use crate::platform::core::BackendSupport;
use crate::platform::device::{
    MIDI_BACKEND_CAP_TOPOLOGY_EVENTS, MIDI_BACKEND_CAP_VIRTUAL_INPUT,
    MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL, MIDI_PORT_DIRECTION_FLAG_INPUT,
    MIDI_PORT_DIRECTION_FLAG_OUTPUT, MidiBackend, MidiEventDeliveryMode, MidiEventSource,
    MidiEventSubscriptionFlags, MidiPortDirection, MidiPortDirectionFlags,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource;

/// Open MIDI event subscriptions through general delivery modes or report one expected error.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_event_subscriptions_open_or_report_expected_errors() {
    with_harness_context(|mut context| {
        let direction_mask = MidiPortDirectionFlags(
            MIDI_PORT_DIRECTION_FLAG_INPUT.0 | MIDI_PORT_DIRECTION_FLAG_OUTPUT.0,
        );

        // auto delivery
        let options = harness_event_open_options(
            &mut context,
            MidiEventSubscriptionFlags(0),
            direction_mask,
            MidiEventDeliveryMode::Auto,
        );
        let handle = assert_ok_or_expected_error(
            context.destack_device_midi_event_open(options),
            &[PlatformErrorCode::NotSupported],
        )?;

        if let Some(handle) = handle {
            context.destack_device_midi_event_close(handle)?;
        }

        // native delivery
        let options = harness_event_open_options(
            &mut context,
            MidiEventSubscriptionFlags(0),
            direction_mask,
            MidiEventDeliveryMode::NativeOnly,
        );
        let handle = assert_ok_or_expected_error(
            context.destack_device_midi_event_open(options),
            &[PlatformErrorCode::NotSupported],
        )?;

        if let Some(handle) = handle {
            context.destack_device_midi_event_close(handle)?;
        }

        Ok(())
    });
}

/// Reject invalid MIDI event handles with one expected error set.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_missing_event_handles_fail_with_expected_errors() {
    with_harness_context(|mut context| {
        let missing_event_handle = resource::MidiEventHandle(resource::ResourceId::local(0));

        // handle failures
        assert_platform_error_codes(
            context.destack_device_midi_event_close(missing_event_handle),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::InvalidArgument,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_device_midi_event_read(missing_event_handle, 0),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::InvalidArgument,
                PlatformErrorCode::IoWouldBlock,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_device_midi_event_read_batch(missing_event_handle, 1, 0),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::InvalidArgument,
                PlatformErrorCode::IoWouldBlock,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_device_midi_event_try_read(missing_event_handle),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::InvalidArgument,
                PlatformErrorCode::IoWouldBlock,
            ],
        )?;
        assert_platform_error_codes(
            context.destack_device_midi_event_try_read_batch(missing_event_handle, 1),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::InvalidArgument,
                PlatformErrorCode::IoWouldBlock,
            ],
        )?;

        Ok(())
    });
}

/// Deliver one generic topology event when a backend advertises native topology support.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_topology_event_feed_reports_virtual_port_additions_when_supported() {
    with_harness_context(|mut context| {
        // backend rows
        let result = assert_ok_or_expected_error(
            context.destack_device_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(result) = result else {
            return Ok(());
        };

        // topology event success
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

            let options = harness_event_open_options_for_backend(
                &mut context,
                backend,
                MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL,
                MidiPortDirectionFlags(MIDI_PORT_DIRECTION_FLAG_OUTPUT.0),
                MidiEventDeliveryMode::Auto,
            );
            let event_handle = context.destack_device_midi_event_open(options)?;

            let name = format!("Destack MIDI Generic Event {backend:?}");
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

            context.destack_device_midi_input_port_close(input_handle)?;
            context.destack_device_midi_event_close(event_handle)?;
        }

        Ok(())
    });
}

/// Read one non-empty event batch when a backend advertises virtual topology notifications.
#[cfg(any(unix, windows))]
#[test]
fn test_midi_topology_event_read_batch_reports_virtual_port_additions_when_supported() {
    with_harness_context(|mut context| {
        // backend rows
        let result = assert_ok_or_expected_error(
            context.destack_device_midi_backend_list(),
            &[PlatformErrorCode::NotSupported],
        )?;

        let Some(result) = result else {
            return Ok(());
        };

        // topology batch success
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

            let options = harness_event_open_options_for_backend(
                &mut context,
                backend,
                MIDI_EVENT_SUBSCRIPTION_INCLUDE_VIRTUAL,
                MidiPortDirectionFlags(MIDI_PORT_DIRECTION_FLAG_OUTPUT.0),
                MidiEventDeliveryMode::Auto,
            );
            let event_handle = context.destack_device_midi_event_open(options)?;

            let name = format!("Destack MIDI Generic Event Batch {backend:?}");
            let create_options = harness_virtual_input_create_options_for_backend_transport(
                &mut context,
                backend,
                &name,
                data_format,
                protocol,
            )?;
            let input_handle = context.destack_device_midi_input_virtual_create(create_options)?;

            // successful batch read
            let events =
                context.destack_device_midi_event_read_batch(event_handle, 8, 1_000_000_000)?;
            let events = decode_events(&mut context, events)?;
            assert!(
                !events.is_empty(),
                "midi event batch reads should return queued topology events",
            );

            context.destack_device_midi_input_port_close(input_handle)?;
            context.destack_device_midi_event_close(event_handle)?;
        }

        Ok(())
    });
}
