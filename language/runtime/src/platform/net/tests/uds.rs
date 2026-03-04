#![cfg_attr(windows, allow(dead_code, unused_imports))]
use std::path::PathBuf;

use super::{
    assert_platform_error_code_with_privileged_policy,
    assert_platform_error_codes_with_privileged_policy, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::SocketMessageFlags;

/// Exchange bytes over a unix domain socket listener and client pair.
#[cfg(unix)]
#[test]
fn test_net_uds_roundtrip() {
    with_harness_context(|mut context| {
        // build a unique socket path
        let socket_path = uds_path("net_uds_roundtrip");

        // start listening on the unix socket
        let listener =
            match context.destack_net_uds_listen(context.uds_address_value(&socket_path), 16) {
                Ok(listener) => listener,
                Err(error) => {
                    // uds may be unavailable on some harnesses
                    assert_platform_error_code_with_privileged_policy::<()>(
                        Err(error),
                        PlatformErrorCode::NotSupported,
                    )?;
                    let _ = std::fs::remove_file(&socket_path);
                    return Ok(());
                }
            };

        // connect a client socket
        let client = context.destack_net_uds_connect(context.uds_address_value(&socket_path))?;

        // accept on the server side
        let server = context.destack_net_uds_accept(listener)?;

        // client writes to server
        let sent = context.destack_net_write(client, context.bytes_slice_value(b"ping")?)?;
        assert_eq!(sent, 4);

        // server reads from client
        let read_buffer = context.zeroed_bytes_slice_value(8)?;
        let (read_call, read_decode) = context.duplicate_value(read_buffer);
        let received = context.destack_net_read(server, read_call)?;
        let buffer = context.bytes_prefix_from_slice_value(read_decode, received as usize)?;
        assert_eq!(buffer, b"ping");

        // close sockets and listener
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_uds_close_listener(listener)?;

        // cleanup socket path if it exists
        let _ = std::fs::remove_file(&socket_path);

        Ok(())
    });
}

/// Exchange bytes over unix domain sockets using sendmsg and recvmsg.
#[cfg(unix)]
#[test]
fn test_net_uds_sendmsg_recvmsg() {
    with_harness_context(|mut context| {
        // build a unique socket path
        let socket_path = uds_path("net_uds_sendmsg");

        // start listening on the unix socket
        let listener =
            match context.destack_net_uds_listen(context.uds_address_value(&socket_path), 16) {
                Ok(listener) => listener,
                Err(error) => {
                    // uds may be unavailable on some harnesses
                    assert_platform_error_code_with_privileged_policy::<()>(
                        Err(error),
                        PlatformErrorCode::NotSupported,
                    )?;
                    let _ = std::fs::remove_file(&socket_path);
                    return Ok(());
                }
            };

        // connect a client socket
        let client = context.destack_net_uds_connect(context.uds_address_value(&socket_path))?;

        // accept on the server side
        let server = context.destack_net_uds_accept(listener)?;

        // client sends using sendmsg
        let sent = context.destack_net_send_msg(
            client,
            context.bytes_slice_value(b"hello")?,
            context.empty_send_message_value(0, false)?,
        )?;
        assert_eq!(sent, 5);

        // server receives using recvmsg
        let recv_buffer = context.zeroed_bytes_slice_value(16)?;
        let (recv_call, recv_decode) = context.duplicate_value(recv_buffer);
        let recv_result =
            context.destack_net_recv_msg(server, recv_call, SocketMessageFlags(0), 0, false, 0);
        let (bytes, fds, has_credentials, recv_flags, payload_truncated, control_truncated) =
            match recv_result {
                Ok(receive) => context.recv_message_fields(receive),
                Err(error) => {
                    // recvmsg may be unavailable on some harnesses
                    assert_platform_error_code_with_privileged_policy::<(
                        u64,
                        u32,
                        bool,
                        u32,
                        bool,
                        bool,
                    )>(Err(error), PlatformErrorCode::NotSupported)?;
                    context.destack_net_close(server)?;
                    context.destack_net_close(client)?;
                    context.destack_net_uds_close_listener(listener)?;
                    let _ = std::fs::remove_file(&socket_path);
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

        // close sockets and listener
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_uds_close_listener(listener)?;

        // cleanup socket path if it exists
        let _ = std::fs::remove_file(&socket_path);

        Ok(())
    });
}

/// Receive peer credentials through unix domain recvmsg.
#[cfg(unix)]
#[test]
fn test_net_uds_recvmsg_credentials() {
    with_harness_context(|mut context| {
        // build a unique socket path
        let socket_path = uds_path("net_uds_recvmsg_credentials");

        // start listening on the unix socket
        let listener =
            match context.destack_net_uds_listen(context.uds_address_value(&socket_path), 16) {
                Ok(listener) => listener,
                Err(error) => {
                    // uds may be unavailable on some harnesses
                    assert_platform_error_code_with_privileged_policy::<()>(
                        Err(error),
                        PlatformErrorCode::NotSupported,
                    )?;
                    let _ = std::fs::remove_file(&socket_path);
                    return Ok(());
                }
            };

        // connect a client socket
        let client = context.destack_net_uds_connect(context.uds_address_value(&socket_path))?;

        // accept on the server side
        let server = context.destack_net_uds_accept(listener)?;

        // client sends payload bytes
        let sent = context.destack_net_send_msg(
            client,
            context.bytes_slice_value(b"hello")?,
            context.empty_send_message_value(0, false)?,
        )?;
        assert_eq!(sent, 5);

        // server receives and requests credentials
        let recv_buffer = context.zeroed_bytes_slice_value(16)?;
        let (recv_call, recv_decode) = context.duplicate_value(recv_buffer);
        let recv_result =
            context.destack_net_recv_msg(server, recv_call, SocketMessageFlags(0), 0, true, 0);
        let (bytes, fds, has_credentials, recv_flags, payload_truncated, control_truncated) =
            match recv_result {
                Ok(receive) => context.recv_message_fields(receive),
                Err(error) => {
                    // credentials are unavailable on some hosts
                    assert_platform_error_code_with_privileged_policy::<(
                        u64,
                        u32,
                        bool,
                        u32,
                        bool,
                        bool,
                    )>(Err(error), PlatformErrorCode::NotSupported)?;
                    context.destack_net_close(server)?;
                    context.destack_net_close(client)?;
                    context.destack_net_uds_close_listener(listener)?;
                    let _ = std::fs::remove_file(&socket_path);
                    return Ok(());
                }
            };
        let buffer = context.bytes_prefix_from_slice_value(recv_decode, bytes as usize)?;
        assert_eq!(buffer, b"hello");
        assert_eq!(fds, 0);
        assert!(has_credentials);
        assert_eq!(recv_flags, 0);
        assert!(!payload_truncated);
        assert!(!control_truncated);

        // close sockets and listener
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_uds_close_listener(listener)?;

        // cleanup socket path if it exists
        let _ = std::fs::remove_file(&socket_path);

        Ok(())
    });
}

/// Reject out-of-range sendmsg flags on unix domain sockets.
#[cfg(unix)]
#[test]
fn test_net_uds_sendmsg_invalid_flags() {
    with_harness_context(|mut context| {
        // build a unique socket path
        let socket_path = uds_path("net_uds_sendmsg_invalid_flags");

        // start listening on the unix socket
        let listener =
            match context.destack_net_uds_listen(context.uds_address_value(&socket_path), 16) {
                Ok(listener) => listener,
                Err(error) => {
                    // uds may be unavailable on some harnesses
                    assert_platform_error_code_with_privileged_policy::<()>(
                        Err(error),
                        PlatformErrorCode::NotSupported,
                    )?;
                    let _ = std::fs::remove_file(&socket_path);
                    return Ok(());
                }
            };

        // connect a client socket
        let client = context.destack_net_uds_connect(context.uds_address_value(&socket_path))?;

        // accept on the server side
        let server = context.destack_net_uds_accept(listener)?;

        // reject out of range sendmsg flags
        let message = context.empty_send_message_value(u32::MAX, false)?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_net_send_msg(client, context.bytes_slice_value(b"hello")?, message),
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // close sockets and listener
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_uds_close_listener(listener)?;

        // cleanup socket path if it exists
        let _ = std::fs::remove_file(&socket_path);

        Ok(())
    });
}

/// Exchange bytes over unix domain sockets using utf16 path bindings.
#[cfg(unix)]
#[test]
fn test_net_uds_roundtrip_utf16_path() {
    with_harness_context(|mut context| {
        // build a unique socket path
        let socket_path = uds_path("net_uds_roundtrip_utf16");

        // start listening on the unix socket with utf16 path encoding
        let listener = match context
            .destack_net_uds_listen(context.uds_address_utf16_value(&socket_path), 16)
        {
            Ok(listener) => listener,
            Err(error) => {
                // utf16 uds bindings may be unavailable on some harnesses
                assert_platform_error_code_with_privileged_policy::<()>(
                    Err(error),
                    PlatformErrorCode::NotSupported,
                )?;
                let _ = std::fs::remove_file(&socket_path);
                return Ok(());
            }
        };

        // connect a client socket with utf16 path encoding
        let client =
            context.destack_net_uds_connect(context.uds_address_utf16_value(&socket_path))?;

        // accept on the server side
        let server = context.destack_net_uds_accept(listener)?;

        // client writes to server
        let sent = context.destack_net_write(client, context.bytes_slice_value(b"ping")?)?;
        assert_eq!(sent, 4);

        // server reads from client
        let read_buffer = context.zeroed_bytes_slice_value(8)?;
        let (read_call, read_decode) = context.duplicate_value(read_buffer);
        let received = context.destack_net_read(server, read_call)?;
        let buffer = context.bytes_prefix_from_slice_value(read_decode, received as usize)?;
        assert_eq!(buffer, b"ping");

        // close sockets and listener
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_uds_close_listener(listener)?;

        // cleanup socket path if it exists
        let _ = std::fs::remove_file(&socket_path);

        Ok(())
    });
}

fn uds_path(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    let file_name = format!("ds_{label}_{nonce}.sock");

    uds_base_dir().join(file_name)
}

#[cfg(unix)]
fn uds_base_dir() -> PathBuf {
    PathBuf::from("/tmp")
}

#[cfg(windows)]
fn uds_base_dir() -> PathBuf {
    std::env::temp_dir()
}
