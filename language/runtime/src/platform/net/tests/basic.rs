#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{tcp_protocol, tcp_stream_socket_type, with_harness_context};
use crate::platform::net::{AcceptFlags, SocketFamily};

/// Exchange bytes over a tcp connection and verify address metadata.
#[cfg(unix)]
#[test]
fn test_net_roundtrip() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.0.1", 0)?,
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
            context.socket_address_value_for_host_port("127.0.0.1", port)?,
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
        assert_eq!(peer_host, "127.0.0.1");
        assert_eq!(local_host, "127.0.0.1");
        assert!(local_port > 0);

        // close sockets and listener
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}

/// Exchange bytes over a localhost tcp connection.
#[cfg(unix)]
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
#[cfg(unix)]
#[test]
fn test_net_readv_writev_roundtrip() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.0.1", 0)?,
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
            context.socket_address_value_for_host_port("127.0.0.1", port)?,
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
