#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{
    NetHarnessHandle, assert_platform_error_codes_with_privileged_policy, tcp_protocol,
    tcp_stream_socket_type, with_harness_context, with_harnesses,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{AcceptFlags, SocketFamily};

/// Accept-flag bit for nonblocking sockets.
const ACCEPT_FLAG_NONBLOCK: u32 = 1 << 0;
/// Accept-flag bit for close-on-exec or non-inheritable sockets.
const ACCEPT_FLAG_CLOEXEC: u32 = 1 << 1;
/// One unsupported accept-flag bit for validation tests.
const ACCEPT_FLAG_UNKNOWN: u32 = 1 << 2;

/// Exchange bytes over a tcp connection and verify address metadata.
#[cfg(any(unix, windows))]
#[test]
fn test_net_roundtrip() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
            128,
        )?;
        let port = context.listener_port(listener);

        // connect a client socket
        let client = context.destack_net_socket(
            SocketFamily::IPv4,
            tcp_stream_socket_type(),
            tcp_protocol(),
        )?;
        context.destack_net_connect(
            client,
            context.socket_address_value_for_host_port("127.0.local_id.1", port)?,
        )?;

        // accept on the server side
        let server = context.destack_net_accept(listener, AcceptFlags(0))?;

        // client writes to server
        let sent = context.destack_net_write(client, context.bytes_slice_value(b"ping")?)?;
        assert_eq!(sent, 4);

        // server reads from client
        let read_buffer = context.zeroed_bytes_slice_value(8)?;
        let (read_call, read_decode) = context.duplicate_value(read_buffer);
        let received = context.destack_net_read(server, read_call)?;
        let buffer = context.bytes_prefix_from_slice_value(read_decode, received as usize)?;
        assert_eq!(buffer, b"ping");

        // server writes to client
        let sent_back = context.destack_net_write(server, context.bytes_slice_value(b"pong")?)?;
        assert_eq!(sent_back, 4);

        // client reads from server
        let read_buffer = context.zeroed_bytes_slice_value(8)?;
        let (read_call, read_decode) = context.duplicate_value(read_buffer);
        let received_back = context.destack_net_read(client, read_call)?;
        let buffer = context.bytes_prefix_from_slice_value(read_decode, received_back as usize)?;
        assert_eq!(buffer, b"pong");

        // local and peer addresses should match the active connection endpoints
        let local_address = context.destack_net_local_address(client)?;
        let peer_address = context.destack_net_peer_address(client)?;
        let (local_host, local_port, local_family) =
            context.socket_address_from_value(local_address)?;
        let (peer_host, peer_port, peer_family) =
            context.socket_address_from_value(peer_address)?;

        assert_eq!(local_family, SocketFamily::IPv4);
        assert_eq!(peer_family, SocketFamily::IPv4);
        assert_eq!(peer_port, port);
        assert_eq!(peer_host, "127.0.local_id.1");
        assert_eq!(local_host, "127.0.local_id.1");
        assert!(local_port > 0);

        // close sockets and listener
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}

/// Exchange bytes over a localhost tcp connection.
#[cfg(any(unix, windows))]
#[test]
fn test_net_roundtrip_localhost() {
    with_harness_context(|mut context| {
        // start listening on localhost and an ephemeral port
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("localhost", 0)?,
            128,
        )?;
        let port = context.listener_port(listener);

        // connect a client socket through hostname resolution
        let client = context.destack_net_socket(
            SocketFamily::IPv4,
            tcp_stream_socket_type(),
            tcp_protocol(),
        )?;
        context.destack_net_connect(
            client,
            context.socket_address_value_for_host_port("localhost", port)?,
        )?;

        // accept on the server side
        let server = context.destack_net_accept(listener, AcceptFlags(0))?;

        // client writes to server
        let sent = context.destack_net_write(client, context.bytes_slice_value(b"ping")?)?;
        assert_eq!(sent, 4);

        // server reads from client
        let read_buffer = context.zeroed_bytes_slice_value(8)?;
        let (read_call, read_decode) = context.duplicate_value(read_buffer);
        let received = context.destack_net_read(server, read_call)?;
        let buffer = context.bytes_prefix_from_slice_value(read_decode, received as usize)?;
        assert_eq!(buffer, b"ping");

        // local and peer addresses should resolve through localhost
        let local_address = context.destack_net_local_address(client)?;
        let peer_address = context.destack_net_peer_address(client)?;
        let (local_host, local_port, _) = context.socket_address_from_value(local_address)?;
        let (peer_host, peer_port, _) = context.socket_address_from_value(peer_address)?;
        assert!(!local_host.is_empty());
        assert!(!peer_host.is_empty());
        assert!(local_port > 0);
        assert_eq!(peer_port, port);

        // close sockets and listener
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}

/// Exchange bytes over tcp using vectored read and write operations.
#[cfg(any(unix, windows))]
#[test]
fn test_net_readv_writev_roundtrip() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
            128,
        )?;
        let port = context.listener_port(listener);

        // connect and accept
        let client = context.destack_net_socket(
            SocketFamily::IPv4,
            tcp_stream_socket_type(),
            tcp_protocol(),
        )?;
        context.destack_net_connect(
            client,
            context.socket_address_value_for_host_port("127.0.local_id.1", port)?,
        )?;
        let server = context.destack_net_accept(listener, AcceptFlags(0))?;

        // write two iovecs from the client
        let first = b"ping";
        let second = b"pong";
        let sent = context.destack_net_writev(
            client,
            context.bytes_slices_value(&[first.as_slice(), second.as_slice()])?,
        )?;
        assert_eq!(sent, 8);

        // read into two iovecs on the server
        let mut read_buffers = vec![vec![0u8; 4], vec![0u8; 4]];
        let read_values = context.mutable_bytes_slices_value(&mut read_buffers)?;
        let (read_call, read_decode) = context.duplicate_value(read_values);
        let received = context.destack_net_readv(server, read_call)?;
        let read_buffers = context.bytes_slices_from_value(read_decode)?;
        assert_eq!(received, 8);
        assert_eq!(read_buffers[0], b"ping");
        assert_eq!(read_buffers[1], b"pong");

        // close resources
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}

/// Connect through the VM text helper even when host resolution returns multiple addresses.
#[cfg(any(unix, windows))]
#[test]
fn test_net_vm_connect_text_roundtrip() {
    with_harnesses(|harness| {
        let NetHarnessHandle::Vm(_) = harness else {
            return;
        };

        let result: crate::diagnostic::RuntimeResult<()> = harness.with_context(|mut context| {
            // start one IPv4 listener on loopback
            let listener = context.destack_net_listen(
                context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
                128,
            )?;
            let port = context.listener_port(listener);

            // connect through the vm text helper using hostname resolution
            let client = context.vm_connect_text("localhost", port)?;
            let server = context.destack_net_accept(listener, AcceptFlags(0))?;

            // exchange one short payload
            let sent = context.destack_net_write(client, context.bytes_slice_value(b"ping")?)?;
            assert_eq!(sent, 4);
            let read_buffer = context.zeroed_bytes_slice_value(8)?;
            let (read_call, read_decode) = context.duplicate_value(read_buffer);
            let received = context.destack_net_read(server, read_call)?;
            let payload = context.bytes_prefix_from_slice_value(read_decode, received as usize)?;
            assert_eq!(payload, b"ping");

            // close resources
            context.destack_net_close(server)?;
            context.destack_net_close(client)?;
            context.destack_net_close_listener(listener)?;

            Ok(())
        });

        result.expect("network harness call should succeed");
    });
}

/// Listen through the VM text helper and accept one raw IPv4 client.
#[cfg(any(unix, windows))]
#[test]
fn test_net_vm_listen_text_roundtrip() {
    with_harnesses(|harness| {
        let NetHarnessHandle::Vm(_) = harness else {
            return;
        };

        let result: crate::diagnostic::RuntimeResult<()> = harness.with_context(|mut context| {
            // start one listener through the vm text helper
            let listener = context.vm_listen_text("127.0.local_id.1", 0, 128)?;
            let port = context.listener_port(listener);

            // connect one raw ipv4 client
            let client = context.destack_net_socket(
                SocketFamily::IPv4,
                tcp_stream_socket_type(),
                tcp_protocol(),
            )?;
            context.destack_net_connect(
                client,
                context.socket_address_value_for_host_port("127.0.local_id.1", port)?,
            )?;
            let server = context.destack_net_accept(listener, AcceptFlags(0))?;

            // exchange one short payload
            let sent = context.destack_net_write(client, context.bytes_slice_value(b"ping")?)?;
            assert_eq!(sent, 4);
            let read_buffer = context.zeroed_bytes_slice_value(8)?;
            let (read_call, read_decode) = context.duplicate_value(read_buffer);
            let received = context.destack_net_read(server, read_call)?;
            let payload = context.bytes_prefix_from_slice_value(read_decode, received as usize)?;
            assert_eq!(payload, b"ping");

            // close resources
            context.destack_net_close(server)?;
            context.destack_net_close(client)?;
            context.destack_net_close_listener(listener)?;

            Ok(())
        });

        result.expect("network harness call should succeed");
    });
}

/// Apply requested accept flags to the accepted socket instead of silently ignoring them.
#[cfg(any(unix, windows))]
#[test]
fn test_net_accept_applies_requested_flags() {
    with_harness_context(|mut context| {
        // start listening on one ephemeral loopback port
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
            128,
        )?;
        let port = context.listener_port(listener);

        // connect one client before accepting
        let client = context.destack_net_socket(
            SocketFamily::IPv4,
            tcp_stream_socket_type(),
            tcp_protocol(),
        )?;
        context.destack_net_connect(
            client,
            context.socket_address_value_for_host_port("127.0.local_id.1", port)?,
        )?;

        // accept with explicit normalized flags
        let server = context.destack_net_accept(
            listener,
            AcceptFlags(ACCEPT_FLAG_NONBLOCK | ACCEPT_FLAG_CLOEXEC),
        )?;

        // unix should expose both file-status and descriptor flag changes directly
        #[cfg(unix)]
        {
            assert!(context.socket_is_nonblocking(server));
            assert!(context.socket_is_close_on_exec(server));
        }

        // windows should report nonblocking behavior on empty reads and clear handle inheritance
        #[cfg(windows)]
        {
            assert!(context.socket_is_close_on_exec(server));

            let buffer = context.zeroed_bytes_slice_value(8)?;
            assert_platform_error_codes_with_privileged_policy(
                context.destack_net_read(server, buffer),
                &[PlatformErrorCode::IoWouldBlock],
            )?;
        }

        // close resources
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}

/// Reject unknown accept-flag bits instead of ignoring them.
#[cfg(any(unix, windows))]
#[test]
fn test_net_accept_rejects_unknown_flags() {
    with_harness_context(|mut context| {
        // start listening on one valid loopback endpoint
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
            128,
        )?;

        // reject unsupported flag bits up front
        assert_platform_error_codes_with_privileged_policy(
            context.destack_net_accept(listener, AcceptFlags(ACCEPT_FLAG_UNKNOWN)),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // close the listener
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}
