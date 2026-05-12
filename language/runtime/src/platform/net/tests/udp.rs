#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::{
    HarnessValue, assert_platform_error_codes_with_privileged_policy, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{
    SocketFamily, SocketMessageFlags, SocketSendTo, SocketSendToVm, UdpMessageFlags,
};

#[cfg(unix)]
const MESSAGE_FLAG_TRUNC: u32 = libc::MSG_TRUNC as u32;
#[cfg(windows)]
const MESSAGE_FLAG_TRUNC: u32 = windows_sys::Win32::Networking::WinSock::MSG_TRUNC as u32;

/// Send one udp datagram and verify sender metadata on receipt.
#[cfg(any(unix, windows))]
#[test]
fn test_net_udp_roundtrip() {
    with_harness_context(|mut context| {
        // setup server socket
        let server = context.destack_net_udp_socket(SocketFamily::IPv4)?;
        context.destack_net_udp_bind(
            server,
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
        )?;

        // resolve server port
        let server_address = context.destack_net_local_address(server)?;
        let (_host, port, _family) = context.socket_address_from_value(server_address)?;

        // setup client socket
        let client = context.destack_net_udp_socket(SocketFamily::IPv4)?;

        // send a datagram
        let sent = context.destack_net_udp_send_to(
            client,
            context.socket_address_value_for_host_port("127.0.local_id.1", port)?,
            context.bytes_slice_value(b"ping")?,
            UdpMessageFlags(0),
        )?;
        assert_eq!(sent, 4);

        // receive the datagram
        let read_buffer = context.zeroed_bytes_slice_value(32)?;
        let (read_call, read_decode) = context.duplicate_value(read_buffer);
        let receive = context.destack_net_udp_recv_from(server, read_call, UdpMessageFlags(0))?;
        let (host, recv_port, family, bytes, recv_flags) =
            context.udp_receive_from_value(receive)?;
        let buffer = context.bytes_prefix_from_slice_value(read_decode, bytes as usize)?;

        // sender metadata and payload should match the sent datagram
        assert_eq!(family, SocketFamily::IPv4);
        assert_eq!(host, "127.0.local_id.1");
        assert!(recv_port > 0);
        assert_eq!(recv_flags, 0);
        assert_eq!(buffer, b"ping");

        // close sockets
        context.destack_net_close(client)?;
        context.destack_net_close(server)?;

        Ok(())
    });
}

/// Connect udp sockets and exchange bytes through stream-style read and write.
#[cfg(any(unix, windows))]
#[test]
fn test_net_udp_connect_roundtrip() {
    with_harness_context(|mut context| {
        // set up server socket
        let server = context.destack_net_udp_socket(SocketFamily::IPv4)?;
        context.destack_net_udp_bind(
            server,
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
        )?;
        let server_address = context.destack_net_local_address(server)?;
        let (_host, port, _family) = context.socket_address_from_value(server_address)?;

        // set up client socket and connect it
        let client = context.destack_net_udp_socket(SocketFamily::IPv4)?;
        context.destack_net_udp_connect(
            client,
            context.socket_address_value_for_host_port("127.0.local_id.1", port)?,
        )?;

        // write through connected udp socket
        let sent = context.destack_net_write(client, context.bytes_slice_value(b"ping")?)?;
        assert_eq!(sent, 4);

        // read datagram on server
        let read_buffer = context.zeroed_bytes_slice_value(32)?;
        let (read_call, read_decode) = context.duplicate_value(read_buffer);
        let receive = context.destack_net_udp_recv_from(server, read_call, UdpMessageFlags(0))?;
        let (_host, _port, family, bytes, recv_flags) = context.udp_receive_from_value(receive)?;
        let buffer = context.bytes_prefix_from_slice_value(read_decode, bytes as usize)?;
        assert_eq!(family, SocketFamily::IPv4);
        assert_eq!(recv_flags, 0);
        assert_eq!(buffer, b"ping");

        // close resources
        context.destack_net_close(client)?;
        context.destack_net_close(server)?;

        Ok(())
    });
}

/// Report truncation flags for udp recvFrom when the datagram exceeds the caller buffer.
#[cfg(any(unix, windows))]
#[test]
fn test_net_udp_recv_from_reports_truncation_flags() {
    with_harness_context(|mut context| {
        // set up one bound udp socket
        let server = context.destack_net_udp_socket(SocketFamily::IPv4)?;
        context.destack_net_udp_bind(
            server,
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
        )?;
        let server_address = context.destack_net_local_address(server)?;
        let (_host, port, _family) = context.socket_address_from_value(server_address)?;

        // set up one client udp socket
        let client = context.destack_net_udp_socket(SocketFamily::IPv4)?;

        // send one oversized datagram
        let payload = b"truncated-datagram";
        let sent = context.destack_net_udp_send_to(
            client,
            context.socket_address_value_for_host_port("127.0.local_id.1", port)?,
            context.bytes_slice_value(payload)?,
            UdpMessageFlags(0),
        )?;
        assert_eq!(sent as usize, payload.len());

        // receive through a shorter buffer
        let read_buffer = context.zeroed_bytes_slice_value(4)?;
        let (read_call, read_decode) = context.duplicate_value(read_buffer);
        let receive = context.destack_net_udp_recv_from(server, read_call, UdpMessageFlags(0))?;
        let (_host, _port, family, bytes, recv_flags) = context.udp_receive_from_value(receive)?;
        let buffer = context.bytes_prefix_from_slice_value(read_decode, bytes as usize)?;

        // the payload should be truncated and flagged accordingly
        assert_eq!(family, SocketFamily::IPv4);
        assert_eq!(bytes, 4);
        assert_eq!(buffer, &payload[..4]);
        assert_ne!(recv_flags & MESSAGE_FLAG_TRUNC, 0);

        // close sockets
        context.destack_net_close(client)?;
        context.destack_net_close(server)?;

        Ok(())
    });
}

/// Report truncation flags for generic recvFrom when the datagram exceeds the caller buffer.
#[cfg(any(unix, windows))]
#[test]
fn test_net_recv_from_reports_truncation_flags() {
    with_harness_context(|mut context| {
        // set up one bound datagram socket
        let server = context.destack_net_udp_socket(SocketFamily::IPv4)?;
        context.destack_net_bind(
            server,
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
        )?;
        let server_address = context.destack_net_local_address(server)?;
        let (_host, port, _family) = context.socket_address_from_value(server_address)?;

        // set up one client datagram socket
        let client = context.destack_net_udp_socket(SocketFamily::IPv4)?;

        // build one oversized sendTo payload
        let payload = b"truncated-sendto";
        let destination = context.socket_address_value_for_host_port("127.0.local_id.1", port)?;
        let message = match destination {
            HarnessValue::Native(address) => context.harness_value(SocketSendTo {
                address,
                flags: SocketMessageFlags(0),
            }),
            HarnessValue::Vm(address) => context.harness_value_vm(SocketSendToVm {
                address,
                flags: SocketMessageFlags(0),
            }),
        };
        let sent =
            context.destack_net_send_to(client, context.bytes_slice_value(payload)?, message)?;
        assert_eq!(sent as usize, payload.len());

        // receive through a shorter buffer
        let read_buffer = context.zeroed_bytes_slice_value(4)?;
        let (read_call, read_decode) = context.duplicate_value(read_buffer);
        let receive = context.destack_net_recv_from(server, read_call, SocketMessageFlags(0))?;
        let (bytes, recv_flags, source) = match receive {
            HarnessValue::Native(value) => (
                value.bytes,
                value.recv_flags.0,
                context.harness_value(value.address),
            ),
            HarnessValue::Vm(value) => (
                value.bytes,
                value.recv_flags.0,
                context.harness_value_vm(value.address),
            ),
        };
        let payload_prefix = context.bytes_prefix_from_slice_value(read_decode, bytes as usize)?;
        let (_host, _source_port, family) = context.socket_address_from_value(source)?;

        // the payload should be truncated and flagged accordingly
        assert_eq!(family, SocketFamily::IPv4);
        assert_eq!(bytes, 4);
        assert_eq!(payload_prefix, &payload[..4]);
        assert_ne!(recv_flags & MESSAGE_FLAG_TRUNC, 0);

        // close sockets
        context.destack_net_close(client)?;
        context.destack_net_close(server)?;

        Ok(())
    });
}

/// Bind one datagram socket and exchange one payload through sendTo and recvFrom lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_net_send_to_recv_from_roundtrip() {
    with_harness_context(|mut context| {
        // set up one server datagram socket with explicit bind
        let server = context.destack_net_udp_socket(SocketFamily::IPv4)?;
        context.destack_net_bind(
            server,
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
        )?;
        let server_address = context.destack_net_local_address(server)?;
        let (_host, port, _family) = context.socket_address_from_value(server_address)?;

        // set up one client datagram socket
        let client = context.destack_net_udp_socket(SocketFamily::IPv4)?;

        // build one sendTo message payload with destination metadata
        let destination = context.socket_address_value_for_host_port("127.0.local_id.1", port)?;
        let message = match destination {
            HarnessValue::Native(address) => context.harness_value(SocketSendTo {
                address,
                flags: SocketMessageFlags(0),
            }),
            HarnessValue::Vm(address) => context.harness_value_vm(SocketSendToVm {
                address,
                flags: SocketMessageFlags(0),
            }),
        };

        // send one payload to the server
        let sent =
            context.destack_net_send_to(client, context.bytes_slice_value(b"ping")?, message)?;
        assert_eq!(sent, 4);

        // receive one payload with source metadata through recvFrom
        let read_buffer = context.zeroed_bytes_slice_value(32)?;
        let (read_call, read_decode) = context.duplicate_value(read_buffer);
        let receive = context.destack_net_recv_from(server, read_call, SocketMessageFlags(0))?;
        let (bytes, recv_flags, source) = match receive {
            HarnessValue::Native(value) => (
                value.bytes,
                value.recv_flags.0,
                context.harness_value(value.address),
            ),
            HarnessValue::Vm(value) => (
                value.bytes,
                value.recv_flags.0,
                context.harness_value_vm(value.address),
            ),
        };
        let payload = context.bytes_prefix_from_slice_value(read_decode, bytes as usize)?;
        let (host, source_port, family) = context.socket_address_from_value(source)?;

        // payload and source metadata should match sendTo traffic
        assert_eq!(payload, b"ping");
        assert_eq!(recv_flags, 0);
        assert_eq!(family, SocketFamily::IPv4);
        assert_eq!(host, "127.0.local_id.1");
        assert!(source_port > 0);

        // close sockets
        context.destack_net_close(client)?;
        context.destack_net_close(server)?;

        Ok(())
    });
}

/// Return ioWouldBlock for nonblocking UDP receive without queued datagrams.
#[cfg(any(unix, windows))]
#[test]
fn test_net_udp_recv_from_nonblocking_reports_would_block() {
    with_harness_context(|mut context| {
        // set up one bound datagram socket with no pending traffic
        let socket = context.destack_net_udp_socket(SocketFamily::IPv4)?;
        context.destack_net_udp_bind(
            socket,
            context.socket_address_value_for_host_port("127.0.local_id.1", 0)?,
        )?;
        context.destack_net_set_nonblocking(socket, true)?;

        // receive without any queued datagrams
        let buffer = context.zeroed_bytes_slice_value(32)?;
        assert_platform_error_codes_with_privileged_policy(
            context.destack_net_udp_recv_from(socket, buffer, UdpMessageFlags(0)),
            &[PlatformErrorCode::IoWouldBlock],
        )?;

        // close the socket
        context.destack_net_close(socket)?;

        Ok(())
    });
}
