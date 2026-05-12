#[cfg(any(
    windows,
    all(unix, not(any(target_os = "linux", target_os = "android")))
))]
use super::assert_platform_error_code_with_privileged_policy;
use super::{tcp_protocol, tcp_stream_socket_type, with_harness_context};
#[cfg(any(
    windows,
    all(unix, not(any(target_os = "linux", target_os = "android")))
))]
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{AcceptFlags, SocketFamily, SocketMessageFlags};

/// Read raw local and peer socket address payloads for connected sockets.
#[cfg(any(unix, windows))]
#[test]
fn test_net_raw_address_roundtrip() {
    with_harness_context(|mut context| {
        // set up a connected pair
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
            128,
        )?;
        let port = context.listener_port(listener);
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

        // query raw local and peer addresses
        let client_local = context.destack_net_local_address(client)?;
        let client_peer = context.destack_net_peer_address(client)?;
        let server_local = context.destack_net_local_address(server)?;
        let server_peer = context.destack_net_peer_address(server)?;
        let (client_local_family, client_local_bytes) =
            context.socket_address_raw_from_value(client_local)?;
        let (client_peer_family, client_peer_bytes) =
            context.socket_address_raw_from_value(client_peer)?;
        let (server_local_family, server_local_bytes) =
            context.socket_address_raw_from_value(server_local)?;
        let (server_peer_family, server_peer_bytes) =
            context.socket_address_raw_from_value(server_peer)?;

        // validate peer and local payloads mirror the connected pair exactly
        assert_eq!(client_local_family, client_peer_family);
        assert_eq!(client_local_family, server_local_family);
        assert_eq!(server_local_family, server_peer_family);
        assert!(!client_local_bytes.is_empty());
        assert!(!client_peer_bytes.is_empty());
        assert!(!server_local_bytes.is_empty());
        assert!(!server_peer_bytes.is_empty());
        assert_eq!(client_local_bytes, server_peer_bytes);
        assert_eq!(client_peer_bytes, server_local_bytes);

        // close resources
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}

/// Send and receive multiple datagrams through sendmmsg and recvmmsg.
#[cfg(any(unix, windows))]
#[test]
fn test_net_mmsg_roundtrip() {
    with_harness_context(|mut context| {
        // set up one bound udp socket
        let server = context.destack_net_udp_socket(SocketFamily::IPv4)?;
        context.destack_net_udp_bind(
            server,
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
        )?;
        let server_address = context.destack_net_local_address(server)?;
        let (_host, port, _family) = context.socket_address_from_value(server_address)?;

        // connect one udp client to the server endpoint
        let client = context.destack_net_udp_socket(SocketFamily::IPv4)?;
        context.destack_net_udp_connect(
            client,
            context.socket_address_value_for_host_port("127.0.local_id.1", port)?,
        )?;

        // send two datagrams through sendmmsg
        let first = b"ping".as_slice();
        let second = b"pong".as_slice();
        let messages = match context.send_mmsg_messages_value(&[first, second], 0) {
            Ok(messages) => messages,
            Err(error) => {
                context.destack_net_close(client)?;
                context.destack_net_close(server)?;
                return Err(error);
            }
        };
        let sent = context.destack_net_send_mmsg(client, messages)?;
        assert_eq!(sent, 2);

        // receive both datagrams into fixed-size buffers
        let mut receive_buffers = vec![vec![0u8; 4], vec![0u8; 4]];
        let requests = context.recv_mmsg_requests_value(&mut receive_buffers, 0)?;
        let (requests_call, requests_decode) = context.duplicate_value(requests);
        let messages = context.destack_net_recv_mmsg(server, requests_call, 0, false, 0)?;
        let counts = context.recv_mmsg_counts_from_value(messages)?;
        let payloads = context.recv_mmsg_payloads_from_requests(requests_decode, &counts)?;
        assert_eq!(counts, vec![4, 4]);
        assert_eq!(&payloads[0], b"ping");
        assert_eq!(&payloads[1], b"pong");

        // close sockets
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;

        Ok(())
    });
}

/// Send and receive one message through sendmsg and recvmsg.
#[cfg(any(unix, windows))]
#[test]
fn test_net_sendmsg_recvmsg_roundtrip() {
    with_harness_context(|mut context| {
        // set up a connected pair
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
            128,
        )?;
        let port = context.listener_port(listener);
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

        // send payload bytes with sendmsg
        let sent = context.destack_net_send_msg(
            client,
            context.bytes_slice_value(b"hello")?,
            context.empty_send_message_value(0, false)?,
        )?;
        assert_eq!(sent, 5);

        // receive payload bytes with recvmsg and explicit flags
        let recv_buffer = context.zeroed_bytes_slice_value(16)?;
        let (recv_call, recv_decode) = context.duplicate_value(recv_buffer);
        let receive =
            context.destack_net_recv_msg(server, recv_call, SocketMessageFlags(0), 0, false, 0)?;
        let (bytes, fds, has_credentials, recv_flags, payload_truncated, control_truncated) =
            context.recv_message_fields(receive);
        let buffer = context.bytes_prefix_from_slice_value(recv_decode, bytes as usize)?;
        assert_eq!(buffer, b"hello");
        assert_eq!(fds, 0);
        assert!(!has_credentials);
        assert_eq!(recv_flags, 0);
        assert!(!payload_truncated);
        assert!(!control_truncated);

        // connected stream sockets may omit or include source-address metadata by backend
        let recv_buffer = context.zeroed_bytes_slice_value(16)?;
        let sent = context.destack_net_send_msg(
            client,
            context.bytes_slice_value(b"hello")?,
            context.empty_send_message_value(0, false)?,
        )?;
        assert_eq!(sent, 5);
        let receive = context.destack_net_recv_msg(
            server,
            recv_buffer,
            SocketMessageFlags(0),
            0,
            false,
            64,
        )?;
        let (_has_address, control_len) = context.recv_message_meta(receive);
        assert!(control_len <= 64);

        // close resources
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}

/// Send one datagram with sendmsg destination metadata and decode source address with recvmsg.
#[cfg(any(unix, windows))]
#[test]
fn test_net_sendmsg_recvmsg_datagram_address_roundtrip() {
    with_harness_context(|mut context| {
        // set up one bound server socket
        let server = context.destack_net_udp_socket(SocketFamily::IPv4)?;
        context.destack_net_udp_bind(
            server,
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
        )?;
        let server_address = context.destack_net_local_address(server)?;
        let (_host, port, _family) = context.socket_address_from_value(server_address)?;

        // set up one unconnected client socket
        let client = context.destack_net_udp_socket(SocketFamily::IPv4)?;

        // send one datagram through sendmsg with explicit destination address
        let destination = context.socket_address_value_for_host_port("127.0.local_id.1", port)?;
        let message = context.send_message_value(destination, &[], 0, false)?;
        let sent =
            context.destack_net_send_msg(client, context.bytes_slice_value(b"hello")?, message)?;
        assert_eq!(sent, 5);

        // receive through recvmsg and decode source metadata
        let recv_buffer = context.zeroed_bytes_slice_value(32)?;
        let (recv_call, recv_decode) = context.duplicate_value(recv_buffer);
        let receive = context.destack_net_recv_msg(
            server,
            recv_call,
            SocketMessageFlags(0),
            0,
            false,
            256,
        )?;
        let (receive_fields, receive_meta) = context.duplicate_value(receive);
        let (receive_meta, receive_address) = context.duplicate_value(receive_meta);
        let (bytes, fds, has_credentials, recv_flags, payload_truncated, control_truncated) =
            context.recv_message_fields(receive_fields);
        assert_eq!(bytes, 5);
        assert_eq!(fds, 0);
        assert!(!has_credentials);
        assert_eq!(recv_flags, 0);
        assert!(!payload_truncated);
        assert!(!control_truncated);

        // verify source-address and control metadata from recvmsg
        let (has_address, control_len) = context.recv_message_meta(receive_meta);
        assert!(has_address);
        assert!(control_len <= 256);
        let source = context
            .recv_message_address(receive_address)
            .expect("recvmsg should include source address for datagrams");
        let (host, source_port, family) = context.socket_address_from_value(source)?;
        assert_eq!(family, SocketFamily::IPv4);
        assert_eq!(host, "127.0.local_id.1");
        assert!(source_port > 0);

        let payload = context.bytes_prefix_from_slice_value(recv_decode, bytes as usize)?;
        assert_eq!(payload, b"hello");

        // close sockets
        context.destack_net_close(client)?;
        context.destack_net_close(server)?;

        Ok(())
    });
}

/// Reject ancillary descriptor requests for recvmsg on windows.
#[cfg(windows)]
#[test]
fn test_net_recvmsg_rejects_ancillary_requests() {
    with_harness_context(|mut context| {
        // set up a connected pair
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
            128,
        )?;
        let port = context.listener_port(listener);
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

        // send payload bytes first so recvmsg does not block
        let sent = context.destack_net_send_msg(
            client,
            context.bytes_slice_value(b"hello")?,
            context.empty_send_message_value(0, false)?,
        )?;
        assert_eq!(sent, 5);

        // reject descriptor capture on windows
        let buffer = context.zeroed_bytes_slice_value(16)?;
        assert_platform_error_code_with_privileged_policy(
            context.destack_net_recv_msg(server, buffer, SocketMessageFlags(0), 1, false, 0),
            PlatformErrorCode::NotSupported,
        )?;

        // close resources
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}

/// Reject explicit credential requests where sendmsg credentials are unsupported.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
#[test]
fn test_net_sendmsg_rejects_credential_requests() {
    with_harness_context(|mut context| {
        // set up a connected pair
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
            128,
        )?;
        let port = context.listener_port(listener);
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

        // reject explicit credentials on unsupported unix targets
        let message = context.empty_send_message_value(0, true)?;
        assert_platform_error_code_with_privileged_policy(
            context.destack_net_send_msg(client, context.bytes_slice_value(b"hello")?, message),
            PlatformErrorCode::NotSupported,
        )?;

        // close resources
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}
