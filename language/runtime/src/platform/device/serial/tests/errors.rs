use super::core::{
    assert_platform_error_codes, assert_supported_or_not_supported, close_descriptor,
    default_serial_config, open_serial_handle, open_serial_handle_with_options, open_test_pty_pair,
    serial_config_argument, serial_output_signals_argument, slave_path,
};
use crate::platform::device::tests::with_harness_context;
use crate::platform::device::{SerialOutputSignals, SerialPortOpenOptions};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{ResourceId, SerialPortHandle};

/// Reject one unknown serial handle for close and configure operations.
#[test]
fn test_device_serial_rejects_one_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = SerialPortHandle(ResourceId(0));

        // reject one invalid close target
        let close_result = context.destack_device_serial_close(unknown);
        assert_platform_error_codes(close_result, &[PlatformErrorCode::InvalidArgumentValue])?;

        // reject one invalid configure target
        let config = default_serial_config();
        let config = serial_config_argument(&mut context, config)?;
        let configure_result = context.destack_device_serial_configure(unknown, config);
        assert_platform_error_codes(configure_result, &[PlatformErrorCode::InvalidArgumentValue])?;

        Ok(())
    });
}

/// Reject unsupported unix serial queue-size hints at the public entrypoint.
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
