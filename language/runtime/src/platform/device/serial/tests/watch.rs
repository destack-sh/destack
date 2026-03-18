use crate::diagnostic::RuntimeResult;
use crate::platform::device::SerialWatchEventValue;
use crate::platform::device::tests::{
    DeviceHarnessContext, assert_ok_or_expected_error, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::SerialWatchHandle;

use super::core::{assert_serial_descriptor_shape, serial_watch_event_value};

/// Prevent one broken watch backend from running an infinite drain loop in tests.
const MAX_WATCH_EVENTS: usize = 256;

/// Open and drain one serial watch stream through both harnesses.
#[test]
fn test_device_serial_watch_try_read_returns_valid_seeded_topology_events() {
    with_harness_context(|mut context| {
        // open one serial watch and always close it during cleanup
        let handle = context.destack_device_serial_watch_open()?;

        // drain the queued watch seed and validate the public event shape
        let result = (|| {
            let events = drain_watch_events(&mut context, handle)?;
            assert_serial_watch_events(&events);

            Ok(())
        })();

        context.destack_device_serial_watch_close(handle)?;

        result
    });
}

// watch draining

/// Drain all currently queued watch events without blocking.
fn drain_watch_events(
    context: &mut DeviceHarnessContext<'_>,
    handle: SerialWatchHandle,
) -> RuntimeResult<Vec<SerialWatchEventValue>> {
    let mut events = Vec::new();

    // consume already queued events until the watch is empty
    for _ in 0..MAX_WATCH_EVENTS {
        let event = context.destack_device_serial_watch_try_read(handle);
        let event = match assert_ok_or_expected_error(event, &[PlatformErrorCode::IoWouldBlock])? {
            Some(event) => serial_watch_event_value(context, event)?,
            None => return Ok(events),
        };

        events.push(event);
    }

    panic!("serial watch produced more than {MAX_WATCH_EVENTS} immediate events")
}

// public watch semantics

/// Assert that one drained watch sequence is ordered and self-consistent.
fn assert_serial_watch_events(events: &[SerialWatchEventValue]) {
    let mut last_sequence = None;

    // validate every event in delivery order
    for event in events {
        match event {
            SerialWatchEventValue::SerialAttachedEvent(value) => {
                assert_eq!(value.kind, "attached");
                assert_watch_sequence(value.metadata.sequence, &mut last_sequence);
                assert_serial_descriptor_shape(&value.metadata.port);
            }
            SerialWatchEventValue::SerialDetachedEvent(value) => {
                assert_eq!(value.kind, "detached");
                assert_watch_sequence(value.metadata.sequence, &mut last_sequence);
                assert_serial_descriptor_shape(&value.metadata.port);
            }
            SerialWatchEventValue::SerialWatchOverflowEvent(value) => {
                assert_eq!(value.kind, "overflow");
                assert_watch_sequence(value.metadata.sequence, &mut last_sequence);
                assert!(value.metadata.dropped_count > 0);
            }
        }
    }
}

/// Assert that one event sequence number strictly increases.
fn assert_watch_sequence(sequence: u64, last_sequence: &mut Option<u64>) {
    // first event
    let Some(previous_sequence) = last_sequence.replace(sequence) else {
        return;
    };

    // monotonic order
    assert!(sequence > previous_sequence);
}
