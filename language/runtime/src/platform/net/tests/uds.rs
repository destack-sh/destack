#![cfg_attr(windows, allow(dead_code, unused_imports))]
use std::path::PathBuf;

use super::{assert_platform_error_code, assert_platform_error_codes, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;

#[cfg(unix)]
#[test]
fn test_net_uds_roundtrip() {
    with_harness_context(|mut context| {
        let result = (|| {
            // build a unique socket path
            let socket_path = uds_path("net_uds_roundtrip");

            // start listening on the unix socket
            let listener = context.uds_listen(&socket_path, 16)?;

            // connect a client socket
            let client = context.uds_connect(&socket_path)?;

            // accept on the server side
            let server = context.uds_accept(listener)?;

            // client writes to server
            let sent = context.write(client, b"ping")?;
            assert_eq!(sent, 4);

            // server reads from client
            let mut buffer = vec![0u8; 8];
            let received = context.read(server, &mut buffer)?;
            buffer.truncate(received as usize);
            assert_eq!(buffer, b"ping");

            // close sockets and listener
            context.close(server)?;
            context.close(client)?;
            context.uds_close_listener(listener)?;

            // cleanup socket path if it exists
            let _ = std::fs::remove_file(&socket_path);

            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                assert_platform_error_code::<()>(Err(error), PlatformErrorCode::NotSupported)
            }
        }
    });
}

#[cfg(unix)]
#[test]
fn test_net_uds_sendmsg_recvmsg() {
    with_harness_context(|mut context| {
        let result = (|| {
            // build a unique socket path
            let socket_path = uds_path("net_uds_sendmsg");

            // start listening on the unix socket
            let listener = context.uds_listen(&socket_path, 16)?;

            // connect a client socket
            let client = context.uds_connect(&socket_path)?;

            // accept on the server side
            let server = context.uds_accept(listener)?;

            // client sends using sendmsg
            let sent = context.send_msg(client, b"hello")?;
            assert_eq!(sent, 5);

            // server receives using recvmsg
            let mut buffer = vec![0u8; 16];
            let (bytes, fds, has_credentials, recv_flags, payload_truncated, control_truncated) =
                context.recv_msg(server, &mut buffer, 0, false)?;
            buffer.truncate(bytes as usize);
            assert_eq!(buffer, b"hello");
            assert_eq!(fds, 0);
            assert!(!has_credentials);
            assert_eq!(recv_flags, 0);
            assert!(!payload_truncated);
            assert!(!control_truncated);

            // close sockets and listener
            context.close(server)?;
            context.close(client)?;
            context.uds_close_listener(listener)?;

            // cleanup socket path if it exists
            let _ = std::fs::remove_file(&socket_path);

            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                assert_platform_error_code::<()>(Err(error), PlatformErrorCode::NotSupported)
            }
        }
    });
}

#[cfg(unix)]
#[test]
fn test_net_uds_recvmsg_credentials() {
    with_harness_context(|mut context| {
        let result = (|| {
            // build a unique socket path
            let socket_path = uds_path("net_uds_recvmsg_credentials");

            // start listening on the unix socket
            let listener = context.uds_listen(&socket_path, 16)?;

            // connect a client socket
            let client = context.uds_connect(&socket_path)?;

            // accept on the server side
            let server = context.uds_accept(listener)?;

            // client sends payload bytes
            let sent = context.send_msg(client, b"hello")?;
            assert_eq!(sent, 5);

            // server receives and requests credentials
            let mut buffer = vec![0u8; 16];
            let (bytes, fds, has_credentials, recv_flags, payload_truncated, control_truncated) =
                context.recv_msg(server, &mut buffer, 0, true)?;
            buffer.truncate(bytes as usize);
            assert_eq!(buffer, b"hello");
            assert_eq!(fds, 0);
            assert!(has_credentials);
            assert_eq!(recv_flags, 0);
            assert!(!payload_truncated);
            assert!(!control_truncated);

            // close sockets and listener
            context.close(server)?;
            context.close(client)?;
            context.uds_close_listener(listener)?;

            // cleanup socket path if it exists
            let _ = std::fs::remove_file(&socket_path);

            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                assert_platform_error_code::<()>(Err(error), PlatformErrorCode::NotSupported)
            }
        }
    });
}

#[cfg(unix)]
#[test]
fn test_net_uds_sendmsg_invalid_flags() {
    with_harness_context(|mut context| {
        let result = (|| {
            // build a unique socket path
            let socket_path = uds_path("net_uds_sendmsg_invalid_flags");

            // start listening on the unix socket
            let listener = context.uds_listen(&socket_path, 16)?;

            // connect a client socket
            let client = context.uds_connect(&socket_path)?;

            // accept on the server side
            let server = context.uds_accept(listener)?;

            // reject out of range sendmsg flags
            assert_platform_error_codes(
                context.send_msg_with_flags(client, b"hello", u32::MAX),
                &[
                    PlatformErrorCode::InvalidArgumentValue,
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::Io,
                    PlatformErrorCode::Net,
                ],
            )?;

            // close sockets and listener
            context.close(server)?;
            context.close(client)?;
            context.uds_close_listener(listener)?;

            // cleanup socket path if it exists
            let _ = std::fs::remove_file(&socket_path);

            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                assert_platform_error_code::<()>(Err(error), PlatformErrorCode::NotSupported)
            }
        }
    });
}

#[cfg(unix)]
#[test]
fn test_net_uds_roundtrip_utf16_path() {
    with_harness_context(|mut context| {
        let result = (|| {
            // build a unique socket path
            let socket_path = uds_path("net_uds_roundtrip_utf16");

            // start listening on the unix socket with utf16 path encoding
            let listener = context.uds_listen_utf16(&socket_path, 16)?;

            // connect a client socket with utf16 path encoding
            let client = context.uds_connect_utf16(&socket_path)?;

            // accept on the server side
            let server = context.uds_accept(listener)?;

            // client writes to server
            let sent = context.write(client, b"ping")?;
            assert_eq!(sent, 4);

            // server reads from client
            let mut buffer = vec![0u8; 8];
            let received = context.read(server, &mut buffer)?;
            buffer.truncate(received as usize);
            assert_eq!(buffer, b"ping");

            // close sockets and listener
            context.close(server)?;
            context.close(client)?;
            context.uds_close_listener(listener)?;

            // cleanup socket path if it exists
            let _ = std::fs::remove_file(&socket_path);

            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                assert_platform_error_code::<()>(Err(error), PlatformErrorCode::NotSupported)
            }
        }
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
