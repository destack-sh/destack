use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource;

use super::core::assert_platform_error_code;
use super::with_harness_context;

/// Verify message-queue lanes report explicit notSupported.
#[cfg(any(
    windows,
    all(unix, not(any(target_os = "linux", target_os = "android")))
))]
#[test]
fn test_message_queue_lanes_report_not_supported() {
    with_harness_context(|mut context| {
        let name = context.string_value("/ipc_message_queue_not_supported")?;
        let open_error = context
            .destack_ipc_message_queue_open(name, 0, 0, 1, 64)
            .err()
            .expect("expected messageQueueOpen to report notSupported");
        assert_platform_error_code(&open_error, PlatformErrorCode::NotSupported);

        let close_error = context
            .destack_ipc_message_queue_close(resource::MessageQueueHandle(resource::ResourceId(0)))
            .err()
            .expect("expected messageQueueClose to report notSupported");
        assert_platform_error_code(&close_error, PlatformErrorCode::NotSupported);

        Ok(())
    });
}

/// Verify futex lanes report explicit notSupported.
#[cfg(any(
    windows,
    all(unix, not(any(target_os = "linux", target_os = "android")))
))]
#[test]
fn test_futex_lanes_report_not_supported() {
    with_harness_context(|mut context| {
        let handle = resource::SharedMemoryHandle(resource::ResourceId(0));

        let wait_error = context
            .destack_ipc_futex_wait(handle, 0, 0, 0)
            .err()
            .expect("expected futexWait to report notSupported");
        assert_platform_error_code(&wait_error, PlatformErrorCode::NotSupported);

        let wake_error = context
            .destack_ipc_futex_wake(handle, 0, 1)
            .err()
            .expect("expected futexWake to report notSupported");
        assert_platform_error_code(&wake_error, PlatformErrorCode::NotSupported);

        Ok(())
    });
}

/// Verify unix ancillary lanes report explicit notSupported.
#[cfg(windows)]
#[test]
fn test_unix_ancillary_lanes_report_not_supported() {
    with_harness_context(|mut context| {
        let socket = resource::SocketHandle(resource::ResourceId(0));

        let receive_error = context
            .destack_ipc_unix_receive(socket, 0)
            .err()
            .expect("expected unixReceive to report notSupported");
        assert_platform_error_code(&receive_error, PlatformErrorCode::NotSupported);

        let payload = context.bytes_value(b"payload")?;
        let handles = context.transferred_handles_value(&[])?;
        let send_error = context
            .destack_ipc_unix_send(socket, payload, handles)
            .err()
            .expect("expected unixSend to report notSupported");
        assert_platform_error_code(&send_error, PlatformErrorCode::NotSupported);

        Ok(())
    });
}
