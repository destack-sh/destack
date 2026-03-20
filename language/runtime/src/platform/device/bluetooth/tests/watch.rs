use crate::diagnostic::RuntimeResult;
use crate::platform::device::tests::{
    DeviceHarnessContext, assert_ok_or_expected_error, with_harness_context,
};
use crate::platform::device::{BluetoothAdapterDescriptorValue, BluetoothAdapterEventValue};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::BluetoothAdapterWatchHandle;

use super::core::{
    assert_bluetooth_adapter_descriptor_shape,
    assert_bluetooth_supported_not_supported_or_permission,
    bluetooth_adapter_descriptor_list_value, bluetooth_adapter_event_value,
};

/// Prevent one broken bluetooth backend from running an infinite drain loop in tests.
const MAX_WATCH_EVENTS: usize = 256;

/// Open and drain one bluetooth adapter watch stream through both harnesses.
#[cfg(any(unix, windows))]
#[test]
fn test_device_bluetooth_adapter_watch_try_read_returns_valid_seeded_topology_events() {
    with_harness_context(|mut context| {
        let listed = assert_bluetooth_supported_not_supported_or_permission(
            context.destack_device_bluetooth_adapter_list(),
        )?;
        let listed = listed
            .map(|listed| bluetooth_adapter_descriptor_list_value(&mut context, listed))
            .transpose()?
            .unwrap_or_default();

        // open one adapter watch and always close it during cleanup
        let handle = assert_bluetooth_supported_not_supported_or_permission(
            context.destack_device_bluetooth_adapter_watch_open(),
        )?;
        let Some(handle) = handle else {
            return Ok(());
        };

        // drain the queued watch seed and validate the public event shape
        let result = (|| {
            let events = drain_watch_events(&mut context, handle)?;
            assert_bluetooth_adapter_watch_events(&events);
            assert_seeded_watch_delivery(&listed, &events);

            Ok(())
        })();

        context.destack_device_bluetooth_adapter_watch_close(handle)?;

        result
    });
}

// watch draining

/// Drain all currently queued adapter watch events without blocking.
fn drain_watch_events(
    context: &mut DeviceHarnessContext<'_>,
    handle: BluetoothAdapterWatchHandle,
) -> RuntimeResult<Vec<BluetoothAdapterEventValue>> {
    let mut events = Vec::new();

    // consume already queued events until the watch is empty
    for _ in 0..MAX_WATCH_EVENTS {
        let event = context.destack_device_bluetooth_adapter_watch_try_read(handle);
        let event = match assert_ok_or_expected_error(event, &[PlatformErrorCode::IoWouldBlock])? {
            Some(event) => bluetooth_adapter_event_value(context, event)?,
            None => return Ok(events),
        };

        events.push(event);
    }

    panic!("bluetooth adapter watch produced more than {MAX_WATCH_EVENTS} immediate events")
}

// public watch semantics

/// Assert that one drained bluetooth adapter watch sequence is ordered and self consistent.
fn assert_bluetooth_adapter_watch_events(events: &[BluetoothAdapterEventValue]) {
    let mut last_sequence = None;

    // validate every event in delivery order
    for event in events {
        match event {
            BluetoothAdapterEventValue::BluetoothAdapterAttachedEvent(value) => {
                assert_eq!(value.kind, "attached");
                assert_watch_sequence(value.metadata.sequence, &mut last_sequence);
                assert_bluetooth_adapter_descriptor_shape(&value.metadata.adapter);
            }
            BluetoothAdapterEventValue::BluetoothAdapterChangedEvent(value) => {
                assert_eq!(value.kind, "changed");
                assert_watch_sequence(value.metadata.sequence, &mut last_sequence);
                assert_bluetooth_adapter_descriptor_shape(&value.metadata.adapter);
            }
            BluetoothAdapterEventValue::BluetoothAdapterDetachedEvent(value) => {
                assert_eq!(value.kind, "detached");
                assert_watch_sequence(value.metadata.sequence, &mut last_sequence);
                assert_bluetooth_adapter_descriptor_shape(&value.metadata.adapter);
            }
        }
    }
}

/// Assert that one event sequence number strictly increases.
fn assert_watch_sequence(sequence: u64, last_sequence: &mut Option<u64>) {
    let Some(previous_sequence) = last_sequence.replace(sequence) else {
        return;
    };

    assert!(sequence > previous_sequence);
}

/// Assert that one non empty initial topology is reflected in the seeded watch stream.
fn assert_seeded_watch_delivery(
    listed: &[BluetoothAdapterDescriptorValue],
    events: &[BluetoothAdapterEventValue],
) {
    if listed.is_empty() {
        return;
    }

    let attached_count = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                BluetoothAdapterEventValue::BluetoothAdapterAttachedEvent(_)
            )
        })
        .count();

    assert!(
        attached_count > 0,
        "bluetooth adapter watch must seed attached events for the current topology",
    );
}
