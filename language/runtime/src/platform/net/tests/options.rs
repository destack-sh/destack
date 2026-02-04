use super::{NetHarness, native_string, with_native_harness};
#[cfg(unix)]
use crate::platform::net::destack_net_set_reuse_port;
use crate::platform::net::{
    destack_net_close, destack_net_close_listener, destack_net_connect, destack_net_listen,
    destack_net_read, destack_net_set_nonblocking, destack_net_set_reuse_addr,
};
use crate::platform::resource::{ListenerHandle, ResourceId, SocketHandle};

#[cfg(any(unix, windows))]
#[test]
fn test_net_socket_options() {
    with_native_harness(|harness| {
        let runtime = harness.runtime();

        let listener = harness.with_context(|| {
            let host = native_string("127.0.0.1");
            let mut handle = ListenerHandle(ResourceId(0));
            let status = unsafe { destack_net_listen(&mut handle, host, 0, 64) };
            runtime.assert_status_ok(status, "listen");
            handle
        });

        let port = runtime.listener_port(listener);

        let socket = harness.with_context(|| {
            let host = native_string("127.0.0.1");
            let mut handle = SocketHandle(ResourceId(0));
            let status = unsafe { destack_net_connect(&mut handle, host, port) };
            runtime.assert_status_ok(status, "connect");
            handle
        });

        harness.with_context(|| {
            let status = unsafe { destack_net_set_nonblocking(socket, true) };
            runtime.assert_status_ok(status, "setNonblocking");

            let status = unsafe { destack_net_set_reuse_addr(socket, true) };
            runtime.assert_status_ok(status, "setReuseAddr");

            #[cfg(unix)]
            {
                let status = unsafe { destack_net_set_reuse_port(socket, true) };
                runtime.assert_status_ok(status, "setReusePort");
            }
        });

        harness.with_context(|| {
            let mut buffer = vec![0u8; 16];
            let slice = super::native_slice_mut(&mut buffer);
            let mut out = 0u64;
            let status = unsafe { destack_net_read(&mut out, socket, slice) };
            runtime.assert_status_err(status, "read nonblocking");
        });

        harness.with_context(|| {
            let status = unsafe { destack_net_set_reuse_addr(socket, true) };
            runtime.assert_status_ok(status, "setReuseAddr again");

            #[cfg(unix)]
            {
                let status = unsafe { destack_net_set_reuse_port(socket, true) };
                runtime.assert_status_ok(status, "setReusePort again");
            }
        });

        harness.with_context(|| {
            let status = unsafe { destack_net_close(socket) };
            runtime.assert_status_ok(status, "close");

            let status = unsafe { destack_net_close_listener(listener) };
            runtime.assert_status_ok(status, "close listener");
        });
    });
}
