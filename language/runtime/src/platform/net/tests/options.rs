#![cfg_attr(windows, allow(dead_code, unused_imports))]
#[cfg(any(windows, target_os = "linux"))]
use super::assert_platform_error_code_with_privileged_policy;
use super::{
    HarnessValue, assert_platform_error_codes_with_privileged_policy, tcp_protocol,
    tcp_stream_socket_type, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{
    AcceptFlags, KeepAliveConfig, Linger, SocketFamily, SocketOptionLevel, SocketOptionName,
    SocketTimestampingMode,
};
#[cfg(any(windows, target_os = "linux"))]
use crate::platform::net::{UdpSourceMembershipV4, UdpSourceMembershipV4Vm};

const ERR_UNSUPPORTED_PROTOCOL: [PlatformErrorCode; 2] = [
    PlatformErrorCode::NotSupported,
    PlatformErrorCode::NetUnsupportedProtocol,
];
#[cfg(target_os = "linux")]
const ERR_PACKET_MARK_LINUX: [PlatformErrorCode; 5] = [
    PlatformErrorCode::IoPermissionDenied,
    PlatformErrorCode::NotSupported,
    PlatformErrorCode::NetUnsupportedProtocol,
    PlatformErrorCode::Net,
    PlatformErrorCode::Io,
];
const ERR_MULTICAST_MEMBERSHIP: [PlatformErrorCode; 7] = [
    PlatformErrorCode::NotSupported,
    PlatformErrorCode::NetAddressNotAvailable,
    PlatformErrorCode::NetNetworkUnreachable,
    PlatformErrorCode::NetUnsupportedProtocol,
    PlatformErrorCode::NetNoBufferSpace,
    PlatformErrorCode::Io,
    PlatformErrorCode::Net,
];
const ERR_RAW_SOCKOPT: [PlatformErrorCode; 6] = [
    PlatformErrorCode::InvalidArgumentValue,
    PlatformErrorCode::NotSupported,
    PlatformErrorCode::IoInvalidData,
    PlatformErrorCode::Io,
    PlatformErrorCode::Net,
    PlatformErrorCode::NetUnsupportedProtocol,
];
const ERR_MULTICAST_INTERFACE: [PlatformErrorCode; 6] = [
    PlatformErrorCode::NotSupported,
    PlatformErrorCode::NetAddressNotAvailable,
    PlatformErrorCode::InvalidArgumentValue,
    PlatformErrorCode::Io,
    PlatformErrorCode::Net,
    PlatformErrorCode::NetUnsupportedProtocol,
];

/// Configure basic socket options and verify nonblocking read behavior.
#[cfg(unix)]
#[test]
fn test_net_socket_options() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.destack_net_listen(
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
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
            context.socket_address_value_for_host_port("127.0.local_id.1", port)?,
        )?;

        // set basic options
        context.destack_net_set_nonblocking(socket, true)?;
        context.destack_net_set_reuse_addr(socket, true)?;
        let reuse_addr = context.destack_net_get_reuse_addr(socket)?;
        assert!(reuse_addr);
        context.destack_net_set_reuse_port(socket, true)?;

        // nonblocking read should fail predictably without inbound data
        let buffer = context.zeroed_bytes_slice_value(16)?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_net_read(socket, buffer),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        // repeated option writes should remain idempotent
        context.destack_net_set_reuse_addr(socket, true)?;
        context.destack_net_set_reuse_port(socket, true)?;

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
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
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
            context.socket_address_value_for_host_port("127.0.local_id.1", port)?,
        )?;
        let server = context.destack_net_accept(listener, AcceptFlags(0))?;

        // start a UDP socket for datagram options
        let udp = context.destack_net_udp_socket(SocketFamily::IPv4)?;
        context.destack_net_udp_bind(
            udp,
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
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
        let no_delay = context.destack_net_get_no_delay(client)?;
        assert!(no_delay);

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
            assert_platform_error_codes_with_privileged_policy::<()>(
                Err(error),
                &ERR_UNSUPPORTED_PROTOCOL,
            )?;
        }
        let keep_alive = context.destack_net_get_keep_alive(client)?;
        match keep_alive {
            HarnessValue::Native(config) => {
                assert!(config.enabled);
            }
            HarnessValue::Vm(config) => {
                assert!(config.enabled);
            }
        }
        let linger = context.destack_net_get_linger(client)?;
        match linger {
            HarnessValue::Native(linger) => {
                assert!(!linger.enabled);
            }
            HarnessValue::Vm(linger) => {
                assert!(!linger.enabled);
            }
        }
        let receive_buffer = context.destack_net_get_recv_buffer(client)?;
        assert!(receive_buffer > 0);
        let send_buffer = context.destack_net_get_send_buffer(client)?;
        assert!(send_buffer > 0);
        let read_timeout = context.destack_net_get_read_timeout(client)?;
        assert!(read_timeout > 0);
        let write_timeout = context.destack_net_get_write_timeout(client)?;
        assert!(write_timeout > 0);
        let stream_ttl = context.destack_net_get_ttl(client)?;
        assert!(stream_ttl > 0);

        // udp options
        context.destack_net_set_broadcast(udp, false)?;
        context.destack_net_set_multicast_loop(udp, false)?;
        context.destack_net_set_multicast_ttl(udp, 8)?;
        context.destack_net_set_ttl(udp, 32)?;
        context.destack_net_set_tos(udp, 0)?;
        let broadcast = context.destack_net_get_broadcast(udp)?;
        assert!(!broadcast);
        let multicast_loop = context.destack_net_get_multicast_loop(udp)?;
        assert!(!multicast_loop);
        let multicast_ttl = context.destack_net_get_multicast_ttl(udp)?;
        assert!(multicast_ttl <= 255);
        let udp_ttl = context.destack_net_get_ttl(udp)?;
        assert!(udp_ttl > 0);
        let tos = context.destack_net_get_tos(udp)?;
        assert!(tos <= 255);

        // multicast membership may not be supported by host setup
        let join_result = context.destack_net_join_multicast_v4(
            udp,
            context.string_value("224.0.local_id.251"),
            context.string_value("127.0.local_id.1"),
        );
        if let Err(error) = join_result {
            assert_platform_error_codes_with_privileged_policy::<()>(
                Err(error),
                &ERR_MULTICAST_MEMBERSHIP,
            )?;
        } else {
            context.destack_net_leave_multicast_v4(
                udp,
                context.string_value("224.0.local_id.251"),
                context.string_value("127.0.local_id.1"),
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

/// Reject unsupported hardware timestamp mode on windows.
#[cfg(windows)]
#[test]
fn test_net_set_timestamping_hardware_rejects_unsupported_mode() {
    with_harness_context(|mut context| {
        // open one udp socket
        let socket = context.destack_net_udp_socket(SocketFamily::IPv4)?;

        // hardware timestamp mode is not available on windows socket backends
        assert_platform_error_codes_with_privileged_policy(
            context.destack_net_set_timestamping(socket, SocketTimestampingMode::Hardware),
            &[PlatformErrorCode::NotSupported],
        )?;

        // close the socket
        context.destack_net_close(socket)?;

        Ok(())
    });
}

/// Roundtrip software and off timestamp modes on unix.
#[cfg(unix)]
#[test]
fn test_net_timestamping_software_roundtrip() {
    with_harness_context(|mut context| {
        // open one udp socket
        let socket = context.destack_net_udp_socket(SocketFamily::IPv4)?;

        // enable software timestamping and verify mode
        context.destack_net_set_timestamping(socket, SocketTimestampingMode::Software)?;
        let mode = context.destack_net_get_timestamping(socket)?;
        assert_eq!(mode, SocketTimestampingMode::Software);

        // disable timestamping and verify mode
        context.destack_net_set_timestamping(socket, SocketTimestampingMode::Off)?;
        let mode = context.destack_net_get_timestamping(socket)?;
        assert_eq!(mode, SocketTimestampingMode::Off);

        // close the socket
        context.destack_net_close(socket)?;

        Ok(())
    });
}

/// Match packet-mark behavior with host socket support.
#[cfg(any(unix, windows))]
#[test]
fn test_net_packet_mark_support_matches_platform() {
    with_harness_context(|mut context| {
        // open one udp socket for option probes
        let socket = context.destack_net_udp_socket(SocketFamily::IPv4)?;

        // linux may allow mark updates with privileges and deny without them
        #[cfg(target_os = "linux")]
        {
            let set_result = context.destack_net_set_packet_mark(socket, 0x1234);
            if let Err(error) = set_result {
                assert_platform_error_codes_with_privileged_policy::<()>(
                    Err(error),
                    &ERR_PACKET_MARK_LINUX,
                )?;
            }

            let get_result = context.destack_net_get_packet_mark(socket);
            if let Err(error) = get_result {
                assert_platform_error_codes_with_privileged_policy::<u32>(
                    Err(error),
                    &ERR_PACKET_MARK_LINUX,
                )?;
            }
        }

        // non-linux hosts return explicit notSupported for packet marks
        #[cfg(not(target_os = "linux"))]
        {
            assert_platform_error_codes_with_privileged_policy(
                context.destack_net_set_packet_mark(socket, 0x1234),
                &ERR_UNSUPPORTED_PROTOCOL,
            )?;
            assert_platform_error_codes_with_privileged_policy(
                context.destack_net_get_packet_mark(socket),
                &ERR_UNSUPPORTED_PROTOCOL,
            )?;
        }

        // close the socket
        context.destack_net_close(socket)?;

        Ok(())
    });
}

/// Validate IPv4 source-membership parsing on windows and linux.
#[cfg(any(windows, target_os = "linux"))]
#[test]
fn test_net_join_multicast_source_v4_rejects_invalid_group() {
    with_harness_context(|mut context| {
        // open one udp socket
        let socket = context.destack_net_udp_socket(SocketFamily::IPv4)?;

        // invalid group text must fail as invalid argument
        let group = context.string_value("not-an-ip");
        let source = context.string_value("10.0.local_id.1");
        let interface_address = context.string_value("");
        let membership = match (group, source, interface_address) {
            (
                HarnessValue::Native(group),
                HarnessValue::Native(source),
                HarnessValue::Native(interface_address),
            ) => context.harness_value(UdpSourceMembershipV4 {
                group,
                source,
                interface_address,
            }),
            (
                HarnessValue::Vm(group),
                HarnessValue::Vm(source),
                HarnessValue::Vm(interface_address),
            ) => context.harness_value_vm(UdpSourceMembershipV4Vm {
                group,
                source,
                interface_address,
            }),
            _ => unreachable!("mixed harness values are not possible"),
        };
        assert_platform_error_code_with_privileged_policy(
            context.destack_net_join_multicast_source_v4(socket, membership),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        // close the socket
        context.destack_net_close(socket)?;

        Ok(())
    });
}

/// Validate IPv6 source-membership parsing on windows and linux.
#[cfg(any(windows, target_os = "linux"))]
#[test]
fn test_net_join_multicast_source_v6_rejects_invalid_source() {
    use crate::platform::net::{UdpSourceMembershipV6, UdpSourceMembershipV6Vm};

    with_harness_context(|mut context| {
        // open one udp socket
        let socket = context.destack_net_udp_socket(SocketFamily::IPv6)?;

        // invalid source text must fail as invalid argument
        let group = context.string_value("ff02::1");
        let source = context.string_value("not-an-ipv6");
        let membership = match (group, source) {
            (HarnessValue::Native(group), HarnessValue::Native(source)) => {
                context.harness_value(UdpSourceMembershipV6 {
                    group,
                    source,
                    interface_index: 0,
                })
            }
            (HarnessValue::Vm(group), HarnessValue::Vm(source)) => {
                context.harness_value_vm(UdpSourceMembershipV6Vm {
                    group,
                    source,
                    interface_index: 0,
                })
            }
            _ => unreachable!("mixed harness values are not possible"),
        };
        assert_platform_error_code_with_privileged_policy(
            context.destack_net_join_multicast_source_v6(socket, membership),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        // close the socket
        context.destack_net_close(socket)?;

        Ok(())
    });
}

/// Roundtrip reuse-port control on windows when the host supports SO_REUSE_UNICASTPORT.
#[cfg(windows)]
#[test]
fn test_net_reuse_port_windows_roundtrip_or_expected_error() {
    with_harness_context(|mut context| {
        // open one udp socket
        let socket = context.destack_net_udp_socket(SocketFamily::IPv4)?;

        // set reuse-port and accept unsupported windows socket configurations
        let set_enabled = context.destack_net_set_reuse_port(socket, true);
        if let Err(error) = set_enabled {
            assert_platform_error_codes_with_privileged_policy::<()>(
                Err(error),
                &[
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::NetUnsupportedProtocol,
                    PlatformErrorCode::IoInvalidData,
                    PlatformErrorCode::Io,
                    PlatformErrorCode::Net,
                ],
            )?;
            context.destack_net_close(socket)?;
            return Ok(());
        }

        // read back the option state when the lane is available
        let enabled = context.destack_net_get_reuse_port(socket);
        let enabled = match enabled {
            Ok(enabled) => enabled,
            Err(error) => {
                assert_platform_error_codes_with_privileged_policy::<bool>(
                    Err(error),
                    &[
                        PlatformErrorCode::NotSupported,
                        PlatformErrorCode::NetUnsupportedProtocol,
                        PlatformErrorCode::IoInvalidData,
                        PlatformErrorCode::Io,
                        PlatformErrorCode::Net,
                    ],
                )?;
                context.destack_net_close(socket)?;
                return Ok(());
            }
        };
        assert!(enabled);

        // clear the option and verify the state when the lane is available
        let clear_result = context.destack_net_set_reuse_port(socket, false);
        if let Err(error) = clear_result {
            assert_platform_error_codes_with_privileged_policy::<()>(
                Err(error),
                &[
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::NetUnsupportedProtocol,
                    PlatformErrorCode::IoInvalidData,
                    PlatformErrorCode::Io,
                    PlatformErrorCode::Net,
                ],
            )?;
            context.destack_net_close(socket)?;
            return Ok(());
        }

        let enabled = context.destack_net_get_reuse_port(socket);
        let enabled = match enabled {
            Ok(enabled) => enabled,
            Err(error) => {
                assert_platform_error_codes_with_privileged_policy::<bool>(
                    Err(error),
                    &[
                        PlatformErrorCode::NotSupported,
                        PlatformErrorCode::NetUnsupportedProtocol,
                        PlatformErrorCode::IoInvalidData,
                        PlatformErrorCode::Io,
                        PlatformErrorCode::Net,
                    ],
                )?;
                context.destack_net_close(socket)?;
                return Ok(());
            }
        };
        assert!(!enabled);

        // close the socket
        context.destack_net_close(socket)?;

        Ok(())
    });
}

/// Roundtrip IPv6-only mode and probe raw socket-option lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_net_only_v6_and_raw_socket_option_lanes() {
    with_harness_context(|mut context| {
        // open one IPv6 datagram socket
        let socket = context.destack_net_udp_socket(SocketFamily::IPv6)?;

        // set and read IPv6-only mode
        context.destack_net_set_only_v6(socket, true)?;
        let is_only_v6 = context.destack_net_get_only_v6(socket)?;
        assert!(is_only_v6);

        // probe raw socket-option lanes with one intentionally generic option tuple
        let argument = context.bytes_slice_value(&[0, 0, 0, 0])?;
        let set_result = context.destack_net_set_sock_opt_raw(
            socket,
            SocketOptionLevel(0),
            SocketOptionName(0),
            argument,
        );
        if let Err(error) = set_result {
            assert_platform_error_codes_with_privileged_policy::<()>(Err(error), &ERR_RAW_SOCKOPT)?;
        }

        // read raw socket-option bytes with the same generic tuple
        let get_result = context.destack_net_get_sock_opt_raw(
            socket,
            SocketOptionLevel(0),
            SocketOptionName(0),
            8,
        );
        if let Err(error) = get_result {
            assert_platform_error_codes_with_privileged_policy::<()>(Err(error), &ERR_RAW_SOCKOPT)?;
        }

        // close the socket
        context.destack_net_close(socket)?;

        Ok(())
    });
}

/// Probe multicast-interface lanes and accept explicit host-level unsupported outcomes.
#[cfg(any(unix, windows))]
#[test]
fn test_net_multicast_interface_lanes() {
    with_harness_context(|mut context| {
        // open one IPv4 datagram socket for v4 interface controls
        let socket_v4 = context.destack_net_udp_socket(SocketFamily::IPv4)?;

        // set one IPv4 multicast interface candidate
        let set_v4_result = context.destack_net_set_multicast_interface_v4(
            socket_v4,
            context.string_value("127.0.local_id.1"),
        );
        if let Err(error) = set_v4_result {
            assert_platform_error_codes_with_privileged_policy::<()>(
                Err(error),
                &ERR_MULTICAST_INTERFACE,
            )?;
        } else {
            let interface = context.destack_net_get_multicast_interface_v4(socket_v4)?;
            let interface = context.string_from_value(interface)?;
            assert!(!interface.is_empty());
        }

        // close the IPv4 socket
        context.destack_net_close(socket_v4)?;

        // open one IPv6 datagram socket for v6 interface controls
        let socket_v6 = context.destack_net_udp_socket(SocketFamily::IPv6)?;

        // set one IPv6 multicast interface index
        let set_v6_result = context.destack_net_set_multicast_interface_v6(socket_v6, 1);
        if let Err(error) = set_v6_result {
            assert_platform_error_codes_with_privileged_policy::<()>(
                Err(error),
                &ERR_MULTICAST_INTERFACE,
            )?;
        } else {
            let _interface_index = context.destack_net_get_multicast_interface_v6(socket_v6)?;
        }

        // close the IPv6 socket
        context.destack_net_close(socket_v6)?;

        Ok(())
    });
}

/// Reject malformed IPv6 multicast group values across join and leave lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_net_join_leave_multicast_v6_rejects_invalid_group() {
    with_harness_context(|mut context| {
        // open one IPv6 datagram socket
        let socket = context.destack_net_udp_socket(SocketFamily::IPv6)?;

        // reject invalid group text on join
        let join_result =
            context.destack_net_join_multicast_v6(socket, context.string_value("not-an-ipv6"), 0);
        assert_platform_error_codes_with_privileged_policy(
            join_result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // reject invalid group text on leave
        let leave_result =
            context.destack_net_leave_multicast_v6(socket, context.string_value("not-an-ipv6"), 0);
        assert_platform_error_codes_with_privileged_policy(
            leave_result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;

        // close the socket
        context.destack_net_close(socket)?;

        Ok(())
    });
}

/// Reject malformed source-membership payloads across leave-source lanes.
#[cfg(any(windows, target_os = "linux"))]
#[test]
fn test_net_leave_multicast_source_lanes_reject_invalid_membership() {
    use crate::platform::net::{UdpSourceMembershipV6, UdpSourceMembershipV6Vm};

    with_harness_context(|mut context| {
        // open one IPv4 socket for source-membership leave probes
        let socket_v4 = context.destack_net_udp_socket(SocketFamily::IPv4)?;

        // build one malformed IPv4 source-membership payload
        let group = context.string_value("not-an-ip");
        let source = context.string_value("10.0.local_id.1");
        let interface_address = context.string_value("");
        let membership_v4 = match (group, source, interface_address) {
            (
                HarnessValue::Native(group),
                HarnessValue::Native(source),
                HarnessValue::Native(interface_address),
            ) => context.harness_value(UdpSourceMembershipV4 {
                group,
                source,
                interface_address,
            }),
            (
                HarnessValue::Vm(group),
                HarnessValue::Vm(source),
                HarnessValue::Vm(interface_address),
            ) => context.harness_value_vm(UdpSourceMembershipV4Vm {
                group,
                source,
                interface_address,
            }),
            _ => unreachable!("mixed harness values are not possible"),
        };
        let leave_v4_result =
            context.destack_net_leave_multicast_source_v4(socket_v4, membership_v4);
        assert_platform_error_codes_with_privileged_policy(
            leave_v4_result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;
        context.destack_net_close(socket_v4)?;

        // open one IPv6 socket for source-membership leave probes
        let socket_v6 = context.destack_net_udp_socket(SocketFamily::IPv6)?;

        // build one malformed IPv6 source-membership payload
        let group = context.string_value("ff02::1");
        let source = context.string_value("not-an-ipv6");
        let membership_v6 = match (group, source) {
            (HarnessValue::Native(group), HarnessValue::Native(source)) => {
                context.harness_value(UdpSourceMembershipV6 {
                    group,
                    source,
                    interface_index: 0,
                })
            }
            (HarnessValue::Vm(group), HarnessValue::Vm(source)) => {
                context.harness_value_vm(UdpSourceMembershipV6Vm {
                    group,
                    source,
                    interface_index: 0,
                })
            }
            _ => unreachable!("mixed harness values are not possible"),
        };
        let leave_v6_result =
            context.destack_net_leave_multicast_source_v6(socket_v6, membership_v6);
        assert_platform_error_codes_with_privileged_policy(
            leave_v6_result,
            &[PlatformErrorCode::InvalidArgumentValue],
        )?;
        context.destack_net_close(socket_v6)?;

        Ok(())
    });
}
