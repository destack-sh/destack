#![cfg_attr(windows, allow(dead_code, unused_imports))]
#[cfg(windows)]
use super::assert_platform_error_code;
use super::{NetHarnessKind, assert_platform_error_codes, native_slice_mut, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{KeepAliveConfig, destack_net_read};

/// Configure basic socket options and verify nonblocking read behavior.
#[cfg(unix)]
#[test]
fn test_net_socket_options() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.listen("127.0.0.1", 0, 64)?;
        let port = context.listener_port(listener);

        // connect a client socket
        let socket = context.connect("127.0.0.1", port)?;

        // set basic options
        context.set_nonblocking(socket, true)?;
        context.set_reuse_addr(socket, true)?;
        #[cfg(unix)]
        {
            context.set_reuse_port(socket, true)?;
        }
        #[cfg(windows)]
        {
            assert_platform_error_code(
                context.set_reuse_port(socket, true),
                PlatformErrorCode::NotSupported,
            )?;
        }

        // nonblocking read should fail predictably without inbound data
        match context.kind() {
            NetHarnessKind::Native => {
                let mut buffer = vec![0u8; 16];
                let slice = native_slice_mut(&mut buffer);
                let mut out = 0u64;
                let status = unsafe { destack_net_read(&mut out, socket, slice) };
                assert_platform_error_codes(
                    context.status_result(status, "read nonblocking"),
                    &[
                        PlatformErrorCode::IoWouldBlock,
                        PlatformErrorCode::Io,
                        PlatformErrorCode::Net,
                    ],
                )?;
            }
            NetHarnessKind::Vm => {
                let mut buffer = vec![0u8; 16];
                assert_platform_error_codes(
                    context.read(socket, &mut buffer),
                    &[
                        PlatformErrorCode::IoWouldBlock,
                        PlatformErrorCode::Io,
                        PlatformErrorCode::Net,
                    ],
                )?;
            }
        }

        // repeated option writes should remain idempotent
        match context.kind() {
            NetHarnessKind::Native => {
                context.set_reuse_addr(socket, true)?;
                #[cfg(unix)]
                {
                    context.set_reuse_port(socket, true)?;
                }
                #[cfg(windows)]
                {
                    assert_platform_error_code(
                        context.set_reuse_port(socket, true),
                        PlatformErrorCode::NotSupported,
                    )?;
                }
            }
            NetHarnessKind::Vm => {
                context.set_reuse_addr(socket, true)?;
                #[cfg(unix)]
                {
                    context.set_reuse_port(socket, true)?;
                }
                #[cfg(windows)]
                {
                    assert_platform_error_code(
                        context.set_reuse_port(socket, true),
                        PlatformErrorCode::NotSupported,
                    )?;
                }
            }
        }

        // close sockets and listener
        context.close(socket)?;
        context.close_listener(listener)?;

        Ok(())
    });
}

/// Configure extended stream and datagram socket options.
#[cfg(unix)]
#[test]
fn test_net_socket_options_extended() {
    with_harness_context(|mut context| {
        // start a TCP pair for stream-level options
        let listener = context.listen("127.0.0.1", 0, 64)?;
        let port = context.listener_port(listener);
        let client = context.connect("127.0.0.1", port)?;
        let server = context.accept(listener)?;

        // start a UDP socket for datagram options
        let udp = context.udp_socket(crate::platform::net::SocketFamily::IPv4)?;
        context.udp_bind(udp, "127.0.0.1", 0)?;

        // stream options
        context.set_keep_alive(client, true, 30)?;
        context.set_linger(client, false, 0)?;
        context.set_recv_buffer(client, 64 * 1024)?;
        context.set_send_buffer(client, 64 * 1024)?;
        context.set_read_timeout(client, 50)?;
        context.set_write_timeout(client, 50)?;
        context.set_ttl(client, 64)?;
        context.set_no_delay(client, true)?;

        // extended keepalive fields
        let keepalive_result = context.set_keep_alive_config(
            client,
            KeepAliveConfig {
                enabled: true,
                idle_seconds: 30,
                interval_seconds: 2,
                probe_count: 3,
            },
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
        context.set_broadcast(udp, false)?;
        context.set_multicast_loop(udp, false)?;
        context.set_multicast_ttl(udp, 8)?;
        context.set_ttl(udp, 32)?;
        context.set_tos(udp, 0)?;

        // multicast membership may not be supported by host setup
        let join_result = context.join_multicast(udp, "224.0.0.251", "127.0.0.1");
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
            context.leave_multicast(udp, "224.0.0.251", "127.0.0.1")?;
        }

        // close resources
        context.close(udp)?;
        context.close(server)?;
        context.close(client)?;
        context.close_listener(listener)?;

        Ok(())
    });
}
