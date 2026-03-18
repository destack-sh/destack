use crate::diagnostic::RuntimeResult;
use crate::platform::device::SerialEventValue;
use crate::platform::device::tests::{DeviceHarnessContext, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::SerialPortHandle;
use crate::platform::{NativeSlice, VmSlice};

use super::core::{
    assert_platform_error_codes, close_descriptor, open_serial_handle, open_test_pty_pair,
    serial_event_value, slave_path, write_all,
};

/// Emit one read-ready event when bytes arrive on one serial endpoint.
#[test]
fn test_device_serial_read_event_reports_one_read_ready_transition() {
    with_harness_context(|mut context| {
        let (controller, worker) = open_test_pty_pair()?;
        let path = slave_path(worker)?;
        close_descriptor(worker);

        // open one pty-backed handle
        let handle = open_serial_handle(&mut context, &path)?;

        // publish one inbound payload and wait for readiness
        write_all(controller, b"evt")?;

        let event = context.destack_device_serial_read_event(handle, 50_000_000)?;
        let event = serial_event_value(&mut context, event)?;
        match event {
            SerialEventValue::SerialReadReadyEvent(event) => {
                assert_eq!(event.kind, "readReady");
                assert!(event.available_bytes.is_some());
            }
            other => panic!("expected readReady event, got {other:?}"),
        }

        context.destack_device_serial_close(handle)?;
        close_descriptor(controller);

        Ok(())
    });
}

/// Deliver one read-ready event once until the caller consumes input.
#[test]
fn test_device_serial_read_event_does_not_repeat_without_one_read() {
    with_harness_context(|mut context| {
        let (controller, worker) = open_test_pty_pair()?;
        let path = slave_path(worker)?;
        close_descriptor(worker);

        // open one pty-backed handle
        let handle = open_serial_handle(&mut context, &path)?;

        // publish one first readiness transition
        write_all(controller, b"evt")?;

        let first_event = context.destack_device_serial_read_event(handle, 50_000_000)?;
        let first_event = serial_event_value(&mut context, first_event)?;
        match first_event {
            SerialEventValue::SerialReadReadyEvent(_) => {}
            other => panic!("expected readReady event, got {other:?}"),
        }

        // keep the unread bytes pending and confirm the queue stays quiet
        let try_event = context.destack_device_serial_try_read_event(handle);
        assert_platform_error_codes(try_event, &[PlatformErrorCode::IoWouldBlock])?;

        // consume the pending bytes and confirm readiness can arm again
        let _ = read_serial_bytes(&mut context, handle, 3, 50_000_000)?;

        write_all(controller, b"next")?;

        let second_event = context.destack_device_serial_read_event(handle, 50_000_000)?;
        let second_event = serial_event_value(&mut context, second_event)?;
        match second_event {
            SerialEventValue::SerialReadReadyEvent(_) => {}
            other => panic!("expected second readReady event, got {other:?}"),
        }

        context.destack_device_serial_close(handle)?;
        close_descriptor(controller);

        Ok(())
    });
}

// serial buffers

/// Read one exact byte count through the active harness.
fn read_serial_bytes(
    context: &mut DeviceHarnessContext<'_>,
    handle: SerialPortHandle,
    length: usize,
    timeout_ns: u64,
) -> RuntimeResult<Vec<u8>> {
    let buffer = context.harness_value_from::<NativeSlice<u8>, VmSlice<u8>>(vec![0u8; length])?;
    let read = context.destack_device_serial_read_into(handle, buffer.clone(), timeout_ns)?;
    assert_eq!(read as usize, length);

    context.harness_value_into(buffer)
}
