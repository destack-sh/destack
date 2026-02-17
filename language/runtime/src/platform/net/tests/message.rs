use super::{
    assert_platform_error_code, tcp_protocol, tcp_stream_socket_type, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{AcceptFlags, SocketFamily, SocketMessageFlags};

/// Read raw local and peer socket address payloads for connected sockets.
#[cfg(unix)]
#[test]
fn test_net_raw_address_roundtrip() {
    with_harness_context(|mut context| {
        // set up a connected pair
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.0.1", 0)?,
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
            context.socket_address_value_for_host_port("127.0.0.1", port)?,
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

        // validate raw payloads are present
        assert!(client_local_family != 0);
        assert!(client_peer_family != 0);
        assert!(server_local_family != 0);
        assert!(server_peer_family != 0);
        assert!(!client_local_bytes.is_empty());
        assert!(!client_peer_bytes.is_empty());
        assert!(!server_local_bytes.is_empty());
        assert!(!server_peer_bytes.is_empty());

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
        // set up a connected pair
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.0.1", 0)?,
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
            context.socket_address_value_for_host_port("127.0.0.1", port)?,
        )?;
        let server = context.destack_net_accept(listener, AcceptFlags(0))?;

        // send two fixed-size chunks
        let first = b"ping".as_slice();
        let second = b"pong".as_slice();
        let messages = match context.send_mmsg_messages_value(&[first, second], 0) {
            Ok(messages) => messages,
            Err(error) => {
                // sendmmsg may be unavailable on some harnesses
                assert_platform_error_code::<u64>(Err(error), PlatformErrorCode::NotSupported)?;
                context.destack_net_close(server)?;
                context.destack_net_close(client)?;
                context.destack_net_close_listener(listener)?;
                return Ok(());
            }
        };
        let sent = match context.destack_net_send_mmsg(client, messages) {
            Ok(sent) => sent,
            Err(error) => {
                // sendmmsg may be unavailable on some harnesses
                assert_platform_error_code::<u64>(Err(error), PlatformErrorCode::NotSupported)?;
                context.destack_net_close(server)?;
                context.destack_net_close(client)?;
                context.destack_net_close_listener(listener)?;
                return Ok(());
            }
        };
        assert_eq!(sent, 2);

        // receive both chunks into fixed-size buffers
        let mut receive_buffers = vec![vec![0u8; 4], vec![0u8; 4]];
        let requests = context.recv_mmsg_requests_value(&mut receive_buffers, 0)?;
        let counts = context.destack_net_recv_mmsg(server, requests, 0, false, 0)?;
        let counts = context.recv_mmsg_counts_from_value(counts)?;
        assert_eq!(counts, vec![4, 4]);
        assert_eq!(&receive_buffers[0], b"ping");
        assert_eq!(&receive_buffers[1], b"pong");

        // close resources
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

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
            context.socket_address_value_for_host_port("127.0.0.1", 0)?,
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
            context.socket_address_value_for_host_port("127.0.0.1", port)?,
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
        let recv_result =
            context.destack_net_recv_msg(server, recv_call, SocketMessageFlags(0), 0, false, 0);
        let (bytes, fds, has_credentials, recv_flags, payload_truncated, control_truncated) =
            match recv_result {
                Ok(receive) => context.recv_message_fields(receive),
                Err(error) => {
                    // recvmsg may be unavailable on some harnesses
                    assert_platform_error_code::<(u64, u32, bool, u32, bool, bool)>(
                        Err(error),
                        PlatformErrorCode::NotSupported,
                    )?;
                    context.destack_net_close(server)?;
                    context.destack_net_close(client)?;
                    context.destack_net_close_listener(listener)?;
                    return Ok(());
                }
            };
        let buffer = context.bytes_prefix_from_slice_value(recv_decode, bytes as usize)?;
        assert_eq!(buffer, b"hello");
        assert_eq!(fds, 0);
        assert!(!has_credentials);
        assert_eq!(recv_flags, 0);
        assert!(!payload_truncated);
        assert!(!control_truncated);

        // close resources
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

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
            context.socket_address_value_for_host_port("127.0.0.1", 0)?,
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
            context.socket_address_value_for_host_port("127.0.0.1", port)?,
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
        assert_platform_error_code(
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
            context.socket_address_value_for_host_port("127.0.0.1", 0)?,
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
            context.socket_address_value_for_host_port("127.0.0.1", port)?,
        )?;
        let server = context.destack_net_accept(listener, AcceptFlags(0))?;

        // reject explicit credentials on unsupported unix targets
        let message = context.empty_send_message_value(0, true)?;
        assert_platform_error_code(
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
