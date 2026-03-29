use std::thread;
use std::time::{Duration, Instant};

use crate::diagnostic::RuntimeResult;
use crate::platform::VmSlice;
use crate::platform::abi::NativeSlice;
use crate::platform::device::tests::{DeviceHarnessContext, with_harness_context};
use crate::platform::device::{SerialEventValue, SerialInputSignals, SerialOutputSignals};
use crate::platform::resource::SerialPortHandle;

use super::core::{
    open_virtual_serial_handle, serial_event_value, serial_input_signals_value,
    serial_output_signals_argument, serial_write_bytes_argument,
};

/// Timeout used by deterministic virtual serial tests.
const VIRTUAL_SERIAL_TIMEOUT_NS: u64 = 50_000_000;

/// Delay used to prove that `serialDrain` waits for transmit consumption.
const VIRTUAL_SERIAL_DRAIN_RELEASE_DELAY: Duration = Duration::from_millis(20);

/// Wait for virtual output consumption before reporting one drained session.
#[test]
fn test_device_serial_drain_waits_for_virtual_output_consumption() {
    with_harness_context(|mut context| {
        let payload = b"virtual-drain-payload";
        let (controller, handle) = open_virtual_serial_handle(&mut context, "drain")?;

        // write one outbound payload through the public surface
        let data = serial_write_bytes_argument(&mut context, payload)?;
        let written =
            context.destack_device_serial_write(handle, data, VIRTUAL_SERIAL_TIMEOUT_NS)?;
        assert_eq!(written as usize, payload.len());

        // drain after the peer actually consumes the queued output
        let expected = payload.to_vec();
        let drain_controller = controller.clone();
        let consumer = thread::spawn(move || {
            drain_controller.wait_for_output(expected.len());
            thread::sleep(VIRTUAL_SERIAL_DRAIN_RELEASE_DELAY);

            drain_controller.take_output_exact(expected.len())
        });

        let start = Instant::now();
        context.destack_device_serial_drain(handle)?;
        let elapsed = start.elapsed();

        let observed = consumer
            .join()
            .expect("virtual serial consumer should join");
        assert_eq!(observed, payload);
        assert!(elapsed >= VIRTUAL_SERIAL_DRAIN_RELEASE_DELAY);

        context.destack_device_serial_close(handle)?;

        Ok(())
    });
}

/// Publish one deterministic read-ready transition from the virtual serial substrate.
#[test]
fn test_device_serial_virtual_read_ready_reports_pending_input() {
    with_harness_context(|mut context| {
        let payload = b"virtual-input";
        let (controller, handle) = open_virtual_serial_handle(&mut context, "read-ready")?;

        // enqueue one inbound payload and surface one read-ready event
        controller.enqueue_input(payload);

        let event = context.destack_device_serial_read_event(handle, VIRTUAL_SERIAL_TIMEOUT_NS)?;
        let event = serial_event_value(&mut context, event)?;
        match event {
            SerialEventValue::SerialReadReadyEvent(event) => {
                assert_eq!(event.kind, "readReady");
                assert_eq!(event.available_bytes, Some(payload.len() as u64));
            }
            other => panic!("expected readReady event, got {other:?}"),
        }

        // consume the pending payload through the public read path
        let observed = read_serial_bytes(
            &mut context,
            handle,
            payload.len(),
            VIRTUAL_SERIAL_TIMEOUT_NS,
        )?;
        assert_eq!(observed, payload);

        context.destack_device_serial_close(handle)?;

        Ok(())
    });
}

/// Reflect virtual modem signals and output-signal writes through the public surface.
#[test]
fn test_device_serial_virtual_signals_roundtrip_through_the_public_surface() {
    with_harness_context(|mut context| {
        let (controller, handle) = open_virtual_serial_handle(&mut context, "signals")?;

        // start from the default virtual signal snapshot
        let signals = context.destack_device_serial_get_signals(handle)?;
        let signals = serial_input_signals_value(&mut context, signals)?;
        assert_eq!(
            signals,
            SerialInputSignals {
                clear_to_send: false,
                data_set_ready: false,
                data_carrier_detect: false,
                ring_indicator: false,
            }
        );

        // publish one input-signal transition and assert both event and snapshot
        let published_signals = SerialInputSignals {
            clear_to_send: true,
            data_set_ready: true,
            data_carrier_detect: false,
            ring_indicator: true,
        };
        controller.set_input_signals(published_signals);

        let event = context.destack_device_serial_read_event(handle, VIRTUAL_SERIAL_TIMEOUT_NS)?;
        let event = serial_event_value(&mut context, event)?;
        match event {
            SerialEventValue::SerialModemStatusChangedEvent(event) => {
                assert_eq!(event.kind, "modemStatusChanged");
                assert_eq!(event.signals, published_signals);
            }
            other => panic!("expected modemStatusChanged event, got {other:?}"),
        }

        let signals = context.destack_device_serial_get_signals(handle)?;
        let signals = serial_input_signals_value(&mut context, signals)?;
        assert_eq!(signals, published_signals);

        // update the peer-visible output lines through the public API
        let output_signals = SerialOutputSignals {
            data_terminal_ready: Some(true),
            request_to_send: Some(false),
            break_condition_active: Some(true),
        };
        let output_signals = serial_output_signals_argument(&mut context, output_signals)?;
        context.destack_device_serial_set_signals(handle, output_signals)?;

        assert_eq!(
            controller.output_signals(),
            SerialOutputSignals {
                data_terminal_ready: Some(true),
                request_to_send: Some(false),
                break_condition_active: Some(true),
            }
        );

        context.destack_device_serial_close(handle)?;

        Ok(())
    });
}

/// Publish one deterministic disconnect event from the virtual serial substrate.
#[test]
fn test_device_serial_virtual_disconnect_reports_one_disconnected_event() {
    with_harness_context(|mut context| {
        let (controller, handle) = open_virtual_serial_handle(&mut context, "disconnect")?;

        // drop the peer and surface one disconnect event
        controller.disconnect();

        let event = context.destack_device_serial_read_event(handle, VIRTUAL_SERIAL_TIMEOUT_NS)?;
        let event = serial_event_value(&mut context, event)?;
        match event {
            SerialEventValue::SerialDisconnectedEvent(event) => {
                assert_eq!(event.kind, "disconnected");
            }
            other => panic!("expected disconnected event, got {other:?}"),
        }

        context.destack_device_serial_close(handle)?;

        Ok(())
    });
}

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
