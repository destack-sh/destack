use super::{assert_platform_error_code, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;

/// Read raw local and peer socket address payloads for connected sockets.
#[cfg(unix)]
#[test]
fn test_net_raw_address_roundtrip() {
    with_harness_context(|mut context| {
        // set up a connected pair
        let listener = context.listen("127.0.0.1", 0, 128)?;
        let port = context.listener_port(listener);
        let client = context.connect("127.0.0.1", port)?;
        let server = context.accept(listener)?;

        // query raw local and peer addresses
        let (client_local_family, client_local_bytes) = context.local_address_raw(client)?;
        let (client_peer_family, client_peer_bytes) = context.peer_address_raw(client)?;
        let (server_local_family, server_local_bytes) = context.local_address_raw(server)?;
        let (server_peer_family, server_peer_bytes) = context.peer_address_raw(server)?;

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
        context.close(server)?;
        context.close(client)?;
        context.close_listener(listener)?;

        Ok(())
    });
}

/// Send and receive multiple datagrams through sendmmsg and recvmmsg.
#[cfg(any(unix, windows))]
#[test]
fn test_net_mmsg_roundtrip() {
    with_harness_context(|mut context| {
        // set up a connected pair
        let listener = context.listen("127.0.0.1", 0, 128)?;
        let port = context.listener_port(listener);
        let client = context.connect("127.0.0.1", port)?;
        let server = context.accept(listener)?;

        // send two fixed-size chunks
        let first = b"ping".as_slice();
        let second = b"pong".as_slice();
        let sent = match context.send_mmsg(client, &[first, second], 0) {
            Ok(sent) => sent,
            Err(error) => {
                // sendmmsg may be unavailable on some harnesses
                assert_platform_error_code::<u64>(Err(error), PlatformErrorCode::NotSupported)?;
                context.close(server)?;
                context.close(client)?;
                context.close_listener(listener)?;
                return Ok(());
            }
        };
        assert_eq!(sent, 2);

        // receive both chunks into fixed-size buffers
        let mut receive_buffers = vec![vec![0u8; 4], vec![0u8; 4]];
        let counts = context.recv_mmsg(server, &mut receive_buffers, 0)?;
        assert_eq!(counts, vec![4, 4]);
        assert_eq!(&receive_buffers[0], b"ping");
        assert_eq!(&receive_buffers[1], b"pong");

        // close resources
        context.close(server)?;
        context.close(client)?;
        context.close_listener(listener)?;

        Ok(())
    });
}

/// Send and receive one message through sendmsg and recvmsg.
#[cfg(any(unix, windows))]
#[test]
fn test_net_sendmsg_recvmsg_roundtrip() {
    with_harness_context(|mut context| {
        // set up a connected pair
        let listener = context.listen("127.0.0.1", 0, 128)?;
        let port = context.listener_port(listener);
        let client = context.connect("127.0.0.1", port)?;
        let server = context.accept(listener)?;

        // send payload bytes with sendmsg
        let sent = context.send_msg(client, b"hello")?;
        assert_eq!(sent, 5);

        // receive payload bytes with recvmsg and explicit flags
        let mut buffer = vec![0u8; 16];
        let recv_result = context.recv_msg_with_flags(server, &mut buffer, 0, 0, false);
        let (bytes, fds, has_credentials, recv_flags, payload_truncated, control_truncated) =
            match recv_result {
                Ok(receive) => receive,
                Err(error) => {
                    // recvmsg may be unavailable on some harnesses
                    assert_platform_error_code::<(u64, u32, bool, u32, bool, bool)>(
                        Err(error),
                        PlatformErrorCode::NotSupported,
                    )?;
                    context.close(server)?;
                    context.close(client)?;
                    context.close_listener(listener)?;
                    return Ok(());
                }
            };
        buffer.truncate(bytes as usize);
        assert_eq!(buffer, b"hello");
        assert_eq!(fds, 0);
        assert!(!has_credentials);
        assert_eq!(recv_flags, 0);
        assert!(!payload_truncated);
        assert!(!control_truncated);

        // close resources
        context.close(server)?;
        context.close(client)?;
        context.close_listener(listener)?;

        Ok(())
    });
}

/// Reject ancillary descriptor requests for recvmsg on windows.
#[cfg(windows)]
#[test]
fn test_net_recvmsg_rejects_ancillary_requests() {
    with_harness_context(|mut context| {
        // set up a connected pair
        let listener = context.listen("127.0.0.1", 0, 128)?;
        let port = context.listener_port(listener);
        let client = context.connect("127.0.0.1", port)?;
        let server = context.accept(listener)?;

        // send payload bytes first so recvmsg does not block
        let sent = context.send_msg(client, b"hello")?;
        assert_eq!(sent, 5);

        // reject descriptor capture on windows
        let mut buffer = vec![0u8; 16];
        assert_platform_error_code(
            context.recv_msg(server, &mut buffer, 1, false),
            PlatformErrorCode::NotSupported,
        )?;

        // close resources
        context.close(server)?;
        context.close(client)?;
        context.close_listener(listener)?;

        Ok(())
    });
}

/// Reject explicit credential requests where sendmsg credentials are unsupported.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "android"))))]
#[test]
fn test_net_sendmsg_rejects_credential_requests() {
    with_harness_context(|mut context| {
        // set up a connected pair
        let listener = context.listen("127.0.0.1", 0, 128)?;
        let port = context.listener_port(listener);
        let client = context.connect("127.0.0.1", port)?;
        let server = context.accept(listener)?;

        // reject explicit credentials on unsupported unix targets
        assert_platform_error_code(
            context.send_msg_with_options(client, b"hello", 0, true),
            PlatformErrorCode::NotSupported,
        )?;

        // close resources
        context.close(server)?;
        context.close(client)?;
        context.close_listener(listener)?;

        Ok(())
    });
}
