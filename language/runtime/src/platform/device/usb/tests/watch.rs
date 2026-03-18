use crate::diagnostic::RuntimeResult;
use crate::platform::device::tests::{
    DeviceHarnessContext, assert_ok_or_expected_error, assert_supported_or_not_supported,
    with_harness_context,
};
use crate::platform::device::{UsbDeviceDescriptorValue, UsbHotplugEventValue};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::UsbWatchHandle;

use super::core::{
    assert_usb_descriptor_shape, usb_descriptor_list_value, usb_hotplug_event_value,
};

/// Prevent one broken USB backend from running an infinite drain loop in tests.
const MAX_WATCH_EVENTS: usize = 256;

/// Open and drain one USB watch stream through both harnesses.
#[test]
fn test_device_usb_watch_try_read_returns_valid_seeded_topology_events() {
    with_harness_context(|mut context| {
        let listed = assert_supported_or_not_supported(context.destack_device_usb_list())?;
        let listed = listed
            .map(|listed| usb_descriptor_list_value(&mut context, listed))
            .transpose()?
            .unwrap_or_default();

        // open one usb watch and always close it during cleanup
        let handle = assert_supported_or_not_supported(context.destack_device_usb_watch_open())?;
        let Some(handle) = handle else {
            return Ok(());
        };

        // drain the queued watch seed and validate the public event shape
        let result = (|| {
            let events = drain_watch_events(&mut context, handle)?;
            assert_usb_watch_events(&events);
            assert_seeded_watch_delivery(&listed, &events);

            Ok(())
        })();

        context.destack_device_usb_watch_close(handle)?;

        result
    });
}

// watch draining

/// Drain all currently queued watch events without blocking.
fn drain_watch_events(
    context: &mut DeviceHarnessContext<'_>,
    handle: UsbWatchHandle,
) -> RuntimeResult<Vec<UsbHotplugEventValue>> {
    let mut events = Vec::new();

    // consume already queued events until the watch is empty
    for _ in 0..MAX_WATCH_EVENTS {
        let event = context.destack_device_usb_watch_try_read(handle);
        let event = match assert_ok_or_expected_error(event, &[PlatformErrorCode::IoWouldBlock])? {
            Some(event) => usb_hotplug_event_value(context, event)?,
            None => return Ok(events),
        };

        events.push(event);
    }

    panic!("usb watch produced more than {MAX_WATCH_EVENTS} immediate events")
}

// public watch semantics

/// Assert that one drained watch sequence is ordered and self-consistent.
fn assert_usb_watch_events(events: &[UsbHotplugEventValue]) {
    let mut last_sequence = None;

    // validate every event in delivery order
    for event in events {
        match event {
            UsbHotplugEventValue::UsbHotplugInstanceEvent(value) => {
                assert_eq!(value.kind, "attached");
                assert_watch_sequence(value.metadata.sequence, &mut last_sequence);
                assert_usb_descriptor_shape(&value.metadata.device);
            }
            UsbHotplugEventValue::UsbHotplugDetachedEvent(value) => {
                assert_eq!(value.kind, "detached");
                assert_watch_sequence(value.metadata.sequence, &mut last_sequence);
                assert_usb_descriptor_shape(&value.metadata.device);
            }
            UsbHotplugEventValue::UsbHotplugOverflowEvent(value) => {
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

/// Assert that one non-empty initial topology is reflected in the seeded watch stream.
fn assert_seeded_watch_delivery(
    listed: &[UsbDeviceDescriptorValue],
    events: &[UsbHotplugEventValue],
) {
    if listed.is_empty() {
        return;
    }

    let attached_count = events
        .iter()
        .filter(|event| matches!(event, UsbHotplugEventValue::UsbHotplugInstanceEvent(_)))
        .count();

    assert!(
        attached_count > 0,
        "usb watch must seed attached events for the current topology",
    );
}
