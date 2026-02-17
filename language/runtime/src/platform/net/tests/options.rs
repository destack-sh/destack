#![cfg_attr(windows, allow(dead_code, unused_imports))]
#[cfg(windows)]
use super::assert_platform_error_code;
use super::{
    assert_platform_error_codes, tcp_protocol, tcp_stream_socket_type, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{AcceptFlags, KeepAliveConfig, Linger, SocketFamily};

/// Configure basic socket options and verify nonblocking read behavior.
#[cfg(unix)]
#[test]
fn test_net_socket_options() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.0.1", 0)?,
            64,
        )?;
        let port = context.listener_port(listener);

        // connect a client socket
        let socket = context.destack_net_socket(
            SocketFamily::IPv4,
            tcp_stream_socket_type(),
            tcp_protocol(),
        )?;
        context.destack_net_connect(
            socket,
            context.socket_address_value_for_host_port("127.0.0.1", port)?,
        )?;

        // set basic options
        context.destack_net_set_nonblocking(socket, true)?;
        context.destack_net_set_reuse_addr(socket, true)?;
        #[cfg(unix)]
        {
            context.destack_net_set_reuse_port(socket, true)?;
        }
        #[cfg(windows)]
        {
            assert_platform_error_code(
                context.destack_net_set_reuse_port(socket, true),
                PlatformErrorCode::NotSupported,
            )?;
        }

        // nonblocking read should fail predictably without inbound data
        let buffer = context.zeroed_bytes_slice_value(16)?;
        assert_platform_error_codes(
            context.destack_net_read(socket, buffer),
            &[
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::Io,
                PlatformErrorCode::Net,
            ],
        )?;

        // repeated option writes should remain idempotent
        context.destack_net_set_reuse_addr(socket, true)?;
        #[cfg(unix)]
        {
            context.destack_net_set_reuse_port(socket, true)?;
        }
        #[cfg(windows)]
        {
            assert_platform_error_code(
                context.destack_net_set_reuse_port(socket, true),
                PlatformErrorCode::NotSupported,
            )?;
        }

        // close sockets and listener
        context.destack_net_close(socket)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}

/// Configure extended stream and datagram socket options.
#[cfg(unix)]
#[test]
fn test_net_socket_options_extended() {
    with_harness_context(|mut context| {
        // start a TCP pair for stream-level options
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.0.1", 0)?,
            64,
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

        // start a UDP socket for datagram options
        let udp = context.destack_net_udp_socket(SocketFamily::IPv4)?;
        context.destack_net_udp_bind(
            udp,
            context.socket_address_value_for_host_port("127.0.0.1", 0)?,
        )?;

        // stream options
        context.destack_net_set_keep_alive(
            client,
            context.keep_alive_config_value(KeepAliveConfig {
                enabled: true,
                idle_seconds: 30,
                interval_seconds: 0,
                probe_count: 0,
            }),
        )?;
        context.destack_net_set_linger(
            client,
            context.linger_value(Linger {
                enabled: false,
                seconds: 0,
            }),
        )?;
        context.destack_net_set_recv_buffer(client, 64 * 1024)?;
        context.destack_net_set_send_buffer(client, 64 * 1024)?;
        context.destack_net_set_read_timeout(client, 50)?;
        context.destack_net_set_write_timeout(client, 50)?;
        context.destack_net_set_ttl(client, 64)?;
        context.destack_net_set_no_delay(client, true)?;

        // extended keepalive fields
        let keepalive_result = context.destack_net_set_keep_alive(
            client,
            context.keep_alive_config_value(KeepAliveConfig {
                enabled: true,
                idle_seconds: 30,
                interval_seconds: 2,
                probe_count: 3,
            }),
        );
        if let Err(error) = keepalive_result {
            assert_platform_error_codes::<()>(
                Err(error),
                &[
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::NetUnsupportedProtocol,
                ],
            )?;
        }

        // udp options
        context.destack_net_set_broadcast(udp, false)?;
        context.destack_net_set_multicast_loop(udp, false)?;
        context.destack_net_set_multicast_ttl(udp, 8)?;
        context.destack_net_set_ttl(udp, 32)?;
        context.destack_net_set_tos(udp, 0)?;

        // multicast membership may not be supported by host setup
        let join_result = context.destack_net_join_multicast_v4(
            udp,
            context.string_value("224.0.0.251"),
            context.string_value("127.0.0.1"),
        );
        if let Err(error) = join_result {
            assert_platform_error_codes::<()>(
                Err(error),
                &[
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::NetAddressNotAvailable,
                    PlatformErrorCode::NetNetworkUnreachable,
                    PlatformErrorCode::NetUnsupportedProtocol,
                    PlatformErrorCode::NetNoBufferSpace,
                    PlatformErrorCode::Io,
                    PlatformErrorCode::Net,
                ],
            )?;
        } else {
            context.destack_net_leave_multicast_v4(
                udp,
                context.string_value("224.0.0.251"),
                context.string_value("127.0.0.1"),
            )?;
        }

        // close resources
        context.destack_net_close(udp)?;
        context.destack_net_close(server)?;
        context.destack_net_close(client)?;
        context.destack_net_close_listener(listener)?;

        Ok(())
    });
}
