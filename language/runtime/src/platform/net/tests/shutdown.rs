#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{
    assert_platform_error_codes_with_privileged_policy, tcp_protocol, tcp_stream_socket_type,
    with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{AcceptFlags, SocketFamily, SocketShutdown};

/// Shut down both directions and ensure writes fail afterward.
#[cfg(any(unix, windows))]
#[test]
fn test_net_shutdown() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
            16,
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

        // shut down the client
        context.destack_net_shutdown(client, SocketShutdown::ReadWrite)?;

        // writes should fail after full shutdown
        assert_platform_error_codes_with_privileged_policy(
            context.destack_net_write(client, context.bytes_slice_value(b"after-shutdown")?),
            &[
                PlatformErrorCode::NetShutdown,
                PlatformErrorCode::NetBrokenPipe,
                PlatformErrorCode::NetNotConnected,
                PlatformErrorCode::NetConnectionReset,
                PlatformErrorCode::IoBrokenPipe,
                PlatformErrorCode::Io,
            ],
        )?;

        // close sockets and listener
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}

/// Shut down writes only and keep the read side functional.
#[cfg(any(unix, windows))]
#[test]
fn test_net_shutdown_write_keeps_read_path() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
            16,
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

        // shut down client writes and ensure reads still work
        context.destack_net_shutdown(client, SocketShutdown::Write)?;
        let payload = b"after-write-shutdown";
        context.destack_net_write(server, context.bytes_slice_value(payload)?)?;
        let read_buffer = context.zeroed_bytes_slice_value(payload.len())?;
        let (read_call, read_decode) = context.duplicate_value(read_buffer);
        let read = context.destack_net_read(client, read_call)?;
        let read_buffer = context.bytes_prefix_from_slice_value(read_decode, read as usize)?;
        assert_eq!(read_buffer, payload);

        // write should now fail on the shutdown side
        // writes should fail after write-side shutdown
        assert_platform_error_codes_with_privileged_policy(
            context.destack_net_write(client, context.bytes_slice_value(b"write-should-fail")?),
            &[
                PlatformErrorCode::NetShutdown,
                PlatformErrorCode::NetBrokenPipe,
                PlatformErrorCode::NetNotConnected,
                PlatformErrorCode::NetConnectionReset,
                PlatformErrorCode::IoBrokenPipe,
                PlatformErrorCode::Io,
            ],
        )?;

        // close sockets and listener
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}

/// Shut down reads only and keep the write side functional.
#[cfg(any(unix, windows))]
#[test]
fn test_net_shutdown_read_keeps_write_path() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
            16,
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

        // shut down client reads and ensure writes still work
        context.destack_net_shutdown(client, SocketShutdown::Read)?;
        let payload = b"after-read-shutdown";
        let written = context.destack_net_write(client, context.bytes_slice_value(payload)?)?;
        assert_eq!(written as usize, payload.len());
        let read_buffer = context.zeroed_bytes_slice_value(payload.len())?;
        let (read_call, read_decode) = context.duplicate_value(read_buffer);
        let read = context.destack_net_read(server, read_call)?;
        let read_buffer = context.bytes_prefix_from_slice_value(read_decode, read as usize)?;
        assert_eq!(read_buffer, payload);

        // close sockets and listener
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}
