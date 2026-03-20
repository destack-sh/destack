use super::core::{
    assert_platform_error_codes, close_descriptor, default_serial_options, open_serial_handle,
    open_test_pty_pair, read_exact, serial_config_argument, serial_config_value,
    serial_descriptor_value, serial_write_bytes_argument, slave_path, write_all,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::device::tests::{DeviceHarnessContext, with_harness_context};
use crate::platform::device::{SerialDataBits, SerialParity, SerialPortConfig, SerialStopBits};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::SerialPortHandle;
use crate::platform::{NativeAbiCodec, NativeSlice, VmSlice};

/// Open, configure, read, write, and close one serial endpoint through one pty slave path.
#[test]
fn test_device_serial_roundtrips_bytes_through_one_pty_slave() {
    with_harness_context(|mut context| {
        let (controller, worker) = open_test_pty_pair()?;
        let path = slave_path(worker)?;
        close_descriptor(worker);

        // open one pty-backed handle
        let handle = open_serial_handle(&mut context, &path)?;

        // validate descriptor and config snapshots
        let descriptor = context.destack_device_serial_descriptor(handle)?;
        let descriptor = serial_descriptor_value(&mut context, descriptor)?;
        assert_eq!(descriptor.id, path);

        let config = context.destack_device_serial_config(handle)?;
        let config = serial_config_value(&mut context, config)?;
        assert_eq!(config, unsafe {
            NativeAbiCodec::into_value(default_serial_options().config)?
        });

        // write one outbound payload through the public harness
        let outbound = b"destack-serial-outbound";
        let outbound = serial_write_bytes_argument(&mut context, outbound)?;
        let written = context.destack_device_serial_write(handle, outbound, 50_000_000)?;
        assert_eq!(written as usize, b"destack-serial-outbound".len());

        let observed = read_exact(controller, b"destack-serial-outbound".len())?;
        assert_eq!(observed, b"destack-serial-outbound");

        // read one inbound payload through the public harness
        let inbound = b"destack-serial-inbound";
        write_all(controller, inbound)?;

        let observed = read_serial_bytes(&mut context, handle, inbound.len(), 50_000_000)?;
        assert_eq!(observed, inbound);

        // keep the empty queue honest after one full drain
        assert_try_read_would_block(&mut context, handle)?;

        context.destack_device_serial_close(handle)?;
        close_descriptor(controller);

        Ok(())
    });
}

/// Open, describe, and close one serial endpoint through both harness entrypoints.
#[test]
fn test_device_serial_open_reads_descriptor_on_one_pty_slave() {
    with_harness_context(|mut context| {
        let (controller, worker) = open_test_pty_pair()?;
        let path = slave_path(worker)?;
        close_descriptor(worker);

        // open one pty-backed handle
        let handle = open_serial_handle(&mut context, &path)?;

        let result = (|| {
            // validate the stable descriptor identity
            let descriptor = context.destack_device_serial_descriptor(handle)?;
            let descriptor = serial_descriptor_value(&mut context, descriptor)?;
            assert_eq!(descriptor.id, path);

            // validate the opened config snapshot
            let config = context.destack_device_serial_config(handle)?;
            let config = serial_config_value(&mut context, config)?;
            assert_eq!(config.baud_rate, 115_200);
            assert_eq!(config.data_bits, SerialDataBits::Eight);

            Ok(())
        })();

        context.destack_device_serial_close(handle)?;
        close_descriptor(controller);

        result
    });
}

/// Apply one new line configuration and report the live snapshot back to the caller.
#[test]
fn test_device_serial_configure_updates_the_live_serial_snapshot() {
    with_harness_context(|mut context| {
        let (controller, worker) = open_test_pty_pair()?;
        let path = slave_path(worker)?;
        close_descriptor(worker);

        // open one pty-backed handle
        let handle = open_serial_handle(&mut context, &path)?;

        let result = (|| {
            // apply one distinct configuration through the public surface
            let config = SerialPortConfig {
                baud_rate: 9_600,
                data_bits: SerialDataBits::Seven,
                parity: SerialParity::Even,
                stop_bits: SerialStopBits::Two,
                flow_control: default_serial_options().config.flow_control,
            };
            let config = serial_config_argument(&mut context, config)?;
            context.destack_device_serial_configure(handle, config)?;

            // read the live host snapshot back and compare the full configuration
            let config = context.destack_device_serial_config(handle)?;
            let config = serial_config_value(&mut context, config)?;
            assert_eq!(config.baud_rate, 9_600);
            assert_eq!(config.data_bits, SerialDataBits::Seven);
            assert_eq!(config.parity, SerialParity::Even);
            assert_eq!(config.stop_bits, SerialStopBits::Two);

            Ok(())
        })();

        context.destack_device_serial_close(handle)?;
        close_descriptor(controller);

        result
    });
}

/// Discard queued inbound bytes without consuming them through one read call.
#[test]
fn test_device_serial_discard_input_clears_the_pending_receive_queue() {
    with_harness_context(|mut context| {
        let (controller, worker) = open_test_pty_pair()?;
        let path = slave_path(worker)?;
        close_descriptor(worker);

        // open one pty-backed handle
        let handle = open_serial_handle(&mut context, &path)?;

        let result = (|| {
            // queue one inbound payload and drop it at the host boundary
            write_all(controller, b"queued")?;
            context.destack_device_serial_discard_input(handle)?;

            // keep the cleared queue honest after the discard
            assert_try_read_would_block(&mut context, handle)?;

            Ok(())
        })();

        context.destack_device_serial_close(handle)?;
        close_descriptor(controller);

        result
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

/// Assert that one nonblocking read reports one empty queue.
fn assert_try_read_would_block(
    context: &mut DeviceHarnessContext<'_>,
    handle: SerialPortHandle,
) -> RuntimeResult<()> {
    let buffer = context.harness_value_from::<NativeSlice<u8>, VmSlice<u8>>(vec![0u8; 4])?;
    let read = context.destack_device_serial_try_read_into(handle, buffer);
    assert_platform_error_codes(read, &[PlatformErrorCode::IoWouldBlock])?;

    Ok(())
}
