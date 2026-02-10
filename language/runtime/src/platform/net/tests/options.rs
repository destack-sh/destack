use super::{NetHarnessKind, native_slice_mut, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(unix)]
use crate::platform::net::destack_net_set_reuse_port;
use crate::platform::net::{KeepAliveConfig, destack_net_read, destack_net_set_reuse_addr};

#[cfg(any(unix, windows))]
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
            let result = context.set_reuse_port(socket, true);
            assert!(result.is_err(), "set reuse port should fail on windows");
        }

        match context.kind() {
            NetHarnessKind::Native => {
                let mut buffer = vec![0u8; 16];
                let slice = native_slice_mut(&mut buffer);
                let mut out = 0u64;
                let status = unsafe { destack_net_read(&mut out, socket, slice) };
                context.status_err(status, "read nonblocking")?;
            }
            NetHarnessKind::Vm => {
                let mut buffer = vec![0u8; 16];
                let result = context.read(socket, &mut buffer);
                assert!(result.is_err(), "read nonblocking should error");
            }
        }

        match context.kind() {
            NetHarnessKind::Native => {
                let status = unsafe { destack_net_set_reuse_addr(socket, true) };
                context.status_ok(status, "set reuse addr again")?;
                #[cfg(unix)]
                {
                    let status = unsafe { destack_net_set_reuse_port(socket, true) };
                    context.status_ok(status, "set reuse port again")?;
                }
                #[cfg(windows)]
                {
                    let status = unsafe { destack_net_set_reuse_port(socket, true) };
                    context.status_err(status, "set reuse port unsupported")?;
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
                    let result = context.set_reuse_port(socket, true);
                    assert!(result.is_err(), "set reuse port should fail on windows");
                }
            }
        }

        // close sockets and listener
        context.close(socket)?;
        context.close_listener(listener)?;

        Ok(())
    });
}

#[cfg(any(unix, windows))]
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
            let code = error.platform_error().map(|error| error.code);
            assert!(
                matches!(
                    code,
                    Some(PlatformErrorCode::NotSupported) | Some(PlatformErrorCode::Io)
                ),
                "unexpected setKeepAlive error code: {code:?}",
            );
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
            let code = error.platform_error().map(|error| error.code);
            assert!(
                matches!(
                    code,
                    Some(PlatformErrorCode::NotSupported)
                        | Some(PlatformErrorCode::NetAddressNotAvailable)
                        | Some(PlatformErrorCode::Net)
                        | Some(PlatformErrorCode::Io)
                ),
                "unexpected joinMulticast error code: {code:?}",
            );
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
