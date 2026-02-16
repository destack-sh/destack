#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{NetHarnessKind, assert_platform_error_codes, native_slice, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{SocketShutdown, destack_net_write};

#[cfg(unix)]
#[test]
fn test_net_shutdown() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.listen("127.0.0.1", 0, 16)?;
        let port = context.listener_port(listener);

        // connect a client socket
        let client = context.connect("127.0.0.1", port)?;

        // accept on the server side
        let server = context.accept(listener)?;

        // shut down the client
        context.shutdown(client, SocketShutdown::ReadWrite)?;

        match context.kind() {
            NetHarnessKind::Native => {
                let buffer = b"after-shutdown".to_vec();
                let slice = native_slice(&buffer);
                let mut out = 0u64;
                let status = unsafe { destack_net_write(&mut out, client, slice) };
                context.status_err(status, "write after shutdown")?;
            }
            NetHarnessKind::Vm => {
                assert_platform_error_codes(
                    context.write(client, b"after-shutdown"),
                    &[
                        PlatformErrorCode::NetShutdown,
                        PlatformErrorCode::NetBrokenPipe,
                        PlatformErrorCode::NetNotConnected,
                        PlatformErrorCode::NetConnectionReset,
                        PlatformErrorCode::Net,
                    ],
                )?;
            }
        }

        // close sockets and listener
        context.close(server)?;
        context.close(client)?;
        context.close_listener(listener)?;

        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_net_shutdown_write_keeps_read_path() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.listen("127.0.0.1", 0, 16)?;
        let port = context.listener_port(listener);

        // connect a client socket
        let client = context.connect("127.0.0.1", port)?;

        // accept on the server side
        let server = context.accept(listener)?;

        // shut down client writes and ensure reads still work
        context.shutdown(client, SocketShutdown::Write)?;
        let payload = b"after-write-shutdown";
        context.write(server, payload)?;
        let mut read_buffer = vec![0u8; payload.len()];
        let read = context.read(client, &mut read_buffer)?;
        read_buffer.truncate(read as usize);
        assert_eq!(read_buffer, payload);

        // write should now fail on the shutdown side
        match context.kind() {
            NetHarnessKind::Native => {
                let buffer = b"write-should-fail".to_vec();
                let slice = native_slice(&buffer);
                let mut out = 0u64;
                let status = unsafe { destack_net_write(&mut out, client, slice) };
                context.status_err(status, "write after write-shutdown")?;
            }
            NetHarnessKind::Vm => {
                assert_platform_error_codes(
                    context.write(client, b"write-should-fail"),
                    &[
                        PlatformErrorCode::NetShutdown,
                        PlatformErrorCode::NetBrokenPipe,
                        PlatformErrorCode::NetNotConnected,
                        PlatformErrorCode::NetConnectionReset,
                        PlatformErrorCode::Net,
                    ],
                )?;
            }
        }

        // close sockets and listener
        context.close(server)?;
        context.close(client)?;
        context.close_listener(listener)?;

        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_net_shutdown_read_keeps_write_path() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.listen("127.0.0.1", 0, 16)?;
        let port = context.listener_port(listener);

        // connect a client socket
        let client = context.connect("127.0.0.1", port)?;

        // accept on the server side
        let server = context.accept(listener)?;

        // shut down client reads and ensure writes still work
        context.shutdown(client, SocketShutdown::Read)?;
        let payload = b"after-read-shutdown";
        let written = context.write(client, payload)?;
        assert_eq!(written as usize, payload.len());
        let mut read_buffer = vec![0u8; payload.len()];
        let read = context.read(server, &mut read_buffer)?;
        read_buffer.truncate(read as usize);
        assert_eq!(read_buffer, payload);

        // close sockets and listener
        context.close(server)?;
        context.close(client)?;
        context.close_listener(listener)?;

        Ok(())
    });
}
