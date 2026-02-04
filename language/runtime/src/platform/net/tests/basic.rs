use super::{NetHarness, native_slice, native_slice_mut, native_string, with_native_harness};
use crate::platform::net::{
    SocketAddress, SocketFamily, destack_net_accept, destack_net_close, destack_net_close_listener,
    destack_net_connect, destack_net_listen, destack_net_local_address, destack_net_peer_address,
    destack_net_read, destack_net_write,
};
use crate::platform::resource::{ListenerHandle, ResourceId, SocketHandle};

#[cfg(any(unix, windows))]
#[test]
fn test_net_roundtrip() {
    with_native_harness(|harness| {
        // setup runtime
        let runtime = harness.runtime();

        // start listening on an ephemeral port
        let listener = harness.with_context(|| {
            let host = native_string("127.0.0.1");
            let mut handle = ListenerHandle(ResourceId(0));
            let status = unsafe { destack_net_listen(&mut handle, host, 0, 128) };
            runtime.assert_status_ok(status, "listen");
            handle
        });
        let port = runtime.listener_port(listener);

        // connect a client socket
        let client = harness.with_context(|| {
            let host = native_string("127.0.0.1");
            let mut handle = SocketHandle(ResourceId(0));
            let status = unsafe { destack_net_connect(&mut handle, host, port) };
            runtime.assert_status_ok(status, "connect");
            handle
        });

        // accept on the server side
        let server = harness.with_context(|| {
            let mut handle = SocketHandle(ResourceId(0));
            let status = unsafe { destack_net_accept(&mut handle, listener) };
            runtime.assert_status_ok(status, "accept");
            handle
        });

        // client writes to server
        let sent = harness.with_context(|| {
            let buffer = b"ping".to_vec();
            let slice = native_slice(&buffer);
            let mut out = 0u64;
            let status = unsafe { destack_net_write(&mut out, client, slice) };
            runtime.assert_status_ok(status, "write");
            out
        });
        assert_eq!(sent, 4);

        // server reads from client
        let received = harness.with_context(|| {
            let mut buffer = vec![0u8; 8];
            let slice = native_slice_mut(&mut buffer);
            let mut out = 0u64;
            let status = unsafe { destack_net_read(&mut out, server, slice) };
            runtime.assert_status_ok(status, "read");
            buffer.truncate(out as usize);
            buffer
        });
        assert_eq!(received, b"ping");

        // server writes to client
        let sent_back = harness.with_context(|| {
            let buffer = b"pong".to_vec();
            let slice = native_slice(&buffer);
            let mut out = 0u64;
            let status = unsafe { destack_net_write(&mut out, server, slice) };
            runtime.assert_status_ok(status, "write");
            out
        });
        assert_eq!(sent_back, 4);

        // client reads from server
        let received_back = harness.with_context(|| {
            let mut buffer = vec![0u8; 8];
            let slice = native_slice_mut(&mut buffer);
            let mut out = 0u64;
            let status = unsafe { destack_net_read(&mut out, client, slice) };
            runtime.assert_status_ok(status, "read");
            buffer.truncate(out as usize);
            buffer
        });
        assert_eq!(received_back, b"pong");

        // validate local/peer addresses
        let (local_host, local_port, local_family) = harness.with_context(|| {
            let mut out = std::mem::MaybeUninit::<SocketAddress>::uninit();
            let status = unsafe { destack_net_local_address(out.as_mut_ptr(), client) };
            runtime.assert_status_ok(status, "localAddress");
            let address = unsafe { out.assume_init() };
            let host = unsafe { address.host.as_str() }
                .expect("local address host should be utf8")
                .to_string();
            (host, address.port, address.family)
        });

        let (peer_host, peer_port, peer_family) = harness.with_context(|| {
            let mut out = std::mem::MaybeUninit::<SocketAddress>::uninit();
            let status = unsafe { destack_net_peer_address(out.as_mut_ptr(), client) };
            runtime.assert_status_ok(status, "peerAddress");
            let address = unsafe { out.assume_init() };
            let host = unsafe { address.host.as_str() }
                .expect("peer address host should be utf8")
                .to_string();
            (host, address.port, address.family)
        });

        assert_eq!(local_family, SocketFamily::IPv4);
        assert_eq!(peer_family, SocketFamily::IPv4);
        assert_eq!(peer_port, port);
        assert_eq!(peer_host, "127.0.0.1");
        assert_eq!(local_host, "127.0.0.1");
        assert!(local_port > 0);

        // close sockets and listener
        harness.with_context(|| {
            let status = unsafe { destack_net_close(server) };
            runtime.assert_status_ok(status, "close server");

            let status = unsafe { destack_net_close(client) };
            runtime.assert_status_ok(status, "close client");

            let status = unsafe { destack_net_close_listener(listener) };
            runtime.assert_status_ok(status, "close listener");
        });
    });
}
