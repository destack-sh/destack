use super::{NetHarness, native_slice, native_string, with_native_harness};
use crate::platform::net::{
    destack_net_accept, destack_net_close, destack_net_close_listener, destack_net_connect,
    destack_net_listen, destack_net_read, destack_net_write,
};
use crate::platform::resource::{ListenerHandle, ResourceId, SocketHandle};

#[cfg(any(unix, windows))]
#[test]
fn test_net_invalid_handles() {
    with_native_harness(|harness| {
        let runtime = harness.runtime();

        harness.with_context(|| {
            let status = unsafe { destack_net_close(SocketHandle(ResourceId(9999))) };
            runtime.assert_status_err(status, "close invalid socket");

            let status = unsafe { destack_net_close_listener(ListenerHandle(ResourceId(9999))) };
            runtime.assert_status_err(status, "close invalid listener");
        });

        harness.with_context(|| {
            let buffer = b"data".to_vec();
            let slice = native_slice(&buffer);
            let mut out = 0u64;
            let status =
                unsafe { destack_net_write(&mut out, SocketHandle(ResourceId(9999)), slice) };
            runtime.assert_status_err(status, "write invalid socket");

            let mut buffer = vec![0u8; 8];
            let slice = super::native_slice_mut(&mut buffer);
            let status =
                unsafe { destack_net_read(&mut out, SocketHandle(ResourceId(9999)), slice) };
            runtime.assert_status_err(status, "read invalid socket");
        });
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_net_accept_after_close() {
    with_native_harness(|harness| {
        let runtime = harness.runtime();

        let listener = harness.with_context(|| {
            let host = native_string("127.0.0.1");
            let mut handle = ListenerHandle(ResourceId(0));
            let status = unsafe { destack_net_listen(&mut handle, host, 0, 8) };
            runtime.assert_status_ok(status, "listen");
            handle
        });

        harness.with_context(|| {
            let status = unsafe { destack_net_close_listener(listener) };
            runtime.assert_status_ok(status, "close listener");
        });

        harness.with_context(|| {
            let mut handle = SocketHandle(ResourceId(0));
            let status = unsafe { destack_net_accept(&mut handle, listener) };
            runtime.assert_status_err(status, "accept after close");
        });
    });
}
