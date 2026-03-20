use super::core::{
    assert_camera_device_descriptor_shape, assert_camera_supported_not_supported_or_permission,
    camera_device_descriptor_list_value, camera_watch_event_value, with_camera_harness_context,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::device::tests::assert_ok_or_expected_error;
use crate::platform::device::{CameraDeviceDescriptorValue, CameraWatchEventValue};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::CameraWatchHandle;

/// Prevent one broken camera backend from running an infinite drain loop in tests.
const MAX_WATCH_EVENTS: usize = 256;

/// Open and drain one camera watch stream through both harnesses.
#[cfg(any(unix, windows))]
#[test]
fn test_device_camera_watch_try_read_returns_valid_seeded_topology_events() {
    with_camera_harness_context(|mut context| {
        let listed = assert_camera_supported_not_supported_or_permission(
            context.destack_device_camera_device_list(),
        )?;
        let listed = listed
            .map(|listed| camera_device_descriptor_list_value(&mut context, listed))
            .transpose()?
            .unwrap_or_default();

        // open one camera watch and always close it during cleanup
        let handle = assert_camera_supported_not_supported_or_permission(
            context.destack_device_camera_device_watch_open(),
        )?;
        let Some(handle) = handle else {
            return Ok(());
        };

        // drain the seeded watch queue and validate its public shape
        let result = (|| {
            let events = drain_watch_events(&mut context, handle)?;
            assert_camera_watch_events(&events);
            assert_seeded_watch_delivery(&listed, &events);

            Ok(())
        })();

        context.destack_device_camera_device_watch_close(handle)?;

        result
    });
}

// watch draining

/// Drain all currently queued camera watch events without blocking.
fn drain_watch_events(
    context: &mut crate::platform::device::tests::DeviceHarnessContext<'_>,
    handle: CameraWatchHandle,
) -> RuntimeResult<Vec<CameraWatchEventValue>> {
    let mut events = Vec::new();

    // consume already queued watch events until the queue is empty
    for _ in 0..MAX_WATCH_EVENTS {
        let event = context.destack_device_camera_device_watch_try_read(handle);
        let event = match assert_ok_or_expected_error(event, &[PlatformErrorCode::IoWouldBlock])? {
            Some(event) => camera_watch_event_value(context, event)?,
            None => return Ok(events),
        };

        events.push(event);
    }

    panic!("camera watch produced more than {MAX_WATCH_EVENTS} immediate events")
}

// public watch semantics

/// Assert that one drained camera watch sequence is ordered and self consistent.
fn assert_camera_watch_events(events: &[CameraWatchEventValue]) {
    let mut last_sequence = None;

    // validate every event in delivery order
    for event in events {
        match event {
            CameraWatchEventValue::CameraAttachedEvent(value) => {
                assert_eq!(value.kind, "attached");
                assert_watch_sequence(value.metadata.sequence, &mut last_sequence);
                assert_camera_device_descriptor_shape(&value.metadata.device);
            }
            CameraWatchEventValue::CameraDetachedEvent(value) => {
                assert_eq!(value.kind, "detached");
                assert_watch_sequence(value.metadata.sequence, &mut last_sequence);
                assert_camera_device_descriptor_shape(&value.metadata.device);
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

/// Assert that one non-empty initial topology is reflected in the seeded watch stream.
fn assert_seeded_watch_delivery(
    listed: &[CameraDeviceDescriptorValue],
    events: &[CameraWatchEventValue],
) {
    if listed.is_empty() {
        return;
    }

    let attached_count = events
        .iter()
        .filter(|event| matches!(event, CameraWatchEventValue::CameraAttachedEvent(_)))
        .count();

    assert!(
        attached_count > 0,
        "camera watch must seed attached events for the current topology",
    );
}
