use super::core::{assert_platform_error_codes, default_serial_config};
#[cfg(unix)]
use super::core::{
    assert_supported_or_not_supported, close_descriptor, open_serial_handle,
    open_serial_handle_with_options, open_test_pty_pair, serial_output_signals_argument,
    slave_path,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::VmSlice;
use crate::platform::abi::NativeSlice;
use crate::platform::device::SerialOutputSignals;
#[cfg(unix)]
use crate::platform::device::SerialPortOpenOptions;
use crate::platform::device::tests::{DeviceHarnessContext, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{ResourceId, SerialPortHandle, SerialWatchHandle};

/// Reject one unknown serial handle across the full session surface.
#[test]
fn test_device_serial_rejects_one_unknown_handle_across_session_operations() {
    with_harness_context(|mut context| {
        let unknown = SerialPortHandle(ResourceId::local(0));

        // reject every public session lane with one invalid handle
        assert_invalid_serial_port_handle(&mut context, unknown)?;

        Ok(())
    });
}

/// Reject one unknown serial watch handle across the full watch surface.
#[test]
fn test_device_serial_rejects_one_unknown_watch_handle() {
    with_harness_context(|mut context| {
        let unknown = SerialWatchHandle(ResourceId::local(0));

        // reject every public watch lane with one invalid handle
        let close_result = context.destack_device_serial_watch_close(unknown);
        assert_platform_error_codes(close_result, &[PlatformErrorCode::InvalidArgumentValue])?;

        let read_result = context.destack_device_serial_watch_read(unknown, 0);
        assert_platform_error_codes(read_result, &[PlatformErrorCode::InvalidArgumentValue])?;

        let try_read_result = context.destack_device_serial_watch_try_read(unknown);
        assert_platform_error_codes(try_read_result, &[PlatformErrorCode::InvalidArgumentValue])?;

        Ok(())
    });
}

/// Reject unsupported unix serial queue-size hints at the public entrypoint.
#[cfg(unix)]
#[test]
fn test_device_serial_open_rejects_unsupported_queue_size_hints() {
    with_harness_context(|mut context| {
        let (controller, worker) = open_test_pty_pair()?;
        let path = slave_path(worker)?;
        close_descriptor(worker);

        // request one unsupported host queue hint
        let options = SerialPortOpenOptions {
            config: default_serial_config(),
            exclusive: None,
            read_buffer_size: Some(4096),
            write_buffer_size: None,
        };
        let open_result = open_serial_handle_with_options(&mut context, &path, options);

        close_descriptor(controller);

        assert_platform_error_codes(open_result, &[PlatformErrorCode::NotSupported])?;

        Ok(())
    });
}

/// Report unsupported modem-signal operations honestly on one host that cannot provide them.
#[cfg(unix)]
#[test]
fn test_device_serial_modem_signal_queries_are_explicit_when_unsupported() {
    with_harness_context(|mut context| {
        let (controller, worker) = open_test_pty_pair()?;
        let path = slave_path(worker)?;
        close_descriptor(worker);

        // open one pty-backed serial handle
        let handle = open_serial_handle(&mut context, &path)?;

        // query input signals and allow one explicit unsupported result
        let get_signals = context.destack_device_serial_get_signals(handle);
        let _ = assert_supported_or_not_supported(get_signals)?;

        // update output signals and allow one explicit unsupported result
        let signals = SerialOutputSignals {
            data_terminal_ready: Some(true),
            request_to_send: Some(true),
            break_condition_active: Some(false),
        };
        let signals = serial_output_signals_argument(&mut context, signals)?;
        let set_signals = context.destack_device_serial_set_signals(handle, signals);
        let _ = assert_supported_or_not_supported(set_signals)?;

        context.destack_device_serial_close(handle)?;
        close_descriptor(controller);

        Ok(())
    });
}

/// Reject one invalid serial handle across all public session operations.
fn assert_invalid_serial_port_handle(
    context: &mut DeviceHarnessContext<'_>,
    unknown: SerialPortHandle,
) -> RuntimeResult<()> {
    let expected = [PlatformErrorCode::InvalidArgumentValue];

    // scalar session calls
    let close_result = context.destack_device_serial_close(unknown);
    assert_platform_error_codes(close_result, &expected)?;

    let descriptor_result = context.destack_device_serial_descriptor(unknown);
    assert_platform_error_codes(descriptor_result, &expected)?;

    let config_result = context.destack_device_serial_config(unknown);
    assert_platform_error_codes(config_result, &expected)?;

    let drain_result = context.destack_device_serial_drain(unknown);
    assert_platform_error_codes(drain_result, &expected)?;

    let discard_input_result = context.destack_device_serial_discard_input(unknown);
    assert_platform_error_codes(discard_input_result, &expected)?;

    let discard_output_result = context.destack_device_serial_discard_output(unknown);
    assert_platform_error_codes(discard_output_result, &expected)?;

    let get_signals_result = context.destack_device_serial_get_signals(unknown);
    assert_platform_error_codes(get_signals_result, &expected)?;

    let read_event_result = context.destack_device_serial_read_event(unknown, 0);
    assert_platform_error_codes(read_event_result, &expected)?;

    let try_read_event_result = context.destack_device_serial_try_read_event(unknown);
    assert_platform_error_codes(try_read_event_result, &expected)?;

    // mutation calls
    let config = context.harness_value_from(default_serial_config())?;
    let configure_result = context.destack_device_serial_configure(unknown, config);
    assert_platform_error_codes(configure_result, &expected)?;

    let signals = SerialOutputSignals {
        data_terminal_ready: Some(true),
        request_to_send: Some(true),
        break_condition_active: Some(false),
    };
    let signals = context.harness_value_from(signals)?;
    let set_signals_result = context.destack_device_serial_set_signals(unknown, signals);
    assert_platform_error_codes(set_signals_result, &expected)?;

    // buffer lanes
    let read_buffer = context.harness_value_from::<NativeSlice<u8>, VmSlice<u8>>(vec![0u8; 8])?;
    let read_into_result = context.destack_device_serial_read_into(unknown, read_buffer, 0);
    assert_platform_error_codes(read_into_result, &expected)?;

    let try_read_buffer =
        context.harness_value_from::<NativeSlice<u8>, VmSlice<u8>>(vec![0u8; 8])?;
    let try_read_into_result =
        context.destack_device_serial_try_read_into(unknown, try_read_buffer);
    assert_platform_error_codes(try_read_into_result, &expected)?;

    let write_buffer = context.harness_value_from::<NativeSlice<u8>, VmSlice<u8>>(vec![1, 2, 3])?;
    let write_result = context.destack_device_serial_write(unknown, write_buffer, 0);
    assert_platform_error_codes(write_result, &expected)?;

    Ok(())
}
