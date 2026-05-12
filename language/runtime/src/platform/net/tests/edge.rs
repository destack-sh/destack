#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{assert_platform_error_codes_with_privileged_policy, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::AcceptFlags;
use crate::platform::resource::{ListenerHandle, ResourceId, SocketHandle};

/// Reject operations that use invalid socket and listener handles.
#[cfg(any(unix, windows))]
#[test]
fn test_net_invalid_handles() {
    with_harness_context(|mut context| {
        assert_platform_error_codes_with_privileged_policy(
            context.destack_net_close(SocketHandle(ResourceId::local(9999))),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_net_close_listener(ListenerHandle(ResourceId::local(9999))),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_net_write(
                SocketHandle(ResourceId::local(9999)),
                context.bytes_slice_value(b"data")?,
            ),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        let buffer = context.zeroed_bytes_slice_value(8)?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_net_read(SocketHandle(ResourceId::local(9999)), buffer),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        Ok(())
    });
}

/// Reject accept calls on a listener that has already been closed.
#[cfg(any(unix, windows))]
#[test]
fn test_net_accept_after_close() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
            8,
        )?;

        // close the listener before accepting
        context.destack_net_close_listener(listener)?;

        assert_platform_error_codes_with_privileged_policy(
            context.destack_net_accept(listener, AcceptFlags(0)),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        Ok(())
    });
}
