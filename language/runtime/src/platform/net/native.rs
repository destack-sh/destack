use crate::diagnostic::RuntimeResult;
use crate::platform::net::{
    KeepAliveConfig, Linger, ResolveFlags, SocketAddress, SocketMessageFlags, SocketPair,
    SocketProtocol, SocketRecvFrom, SocketSendTo, SocketType, UdpMessageFlags, UdpReceive,
    core as core_net,
};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef};
use crate::runtime::RuntimeCallContext;

pub(crate) use super::native_missing::{
    destack_net_get_packet_mark, destack_net_get_timestamping, destack_net_interface_index,
    destack_net_interface_name, destack_net_list_interfaces, destack_net_packet_open,
    destack_net_packet_receive, destack_net_packet_send, destack_net_packet_set_timestamp_mode,
    destack_net_raw_set_header_included, destack_net_raw_socket, destack_net_route_add,
    destack_net_route_delete, destack_net_route_list, destack_net_route_set_namespace,
    destack_net_set_packet_mark, destack_net_set_timestamping,
};

pub(crate) use core_net::{
    destack_net_accept, destack_net_close, destack_net_close_listener,
    destack_net_join_multicast_v4, destack_net_join_multicast_v6, destack_net_leave_multicast_v4,
    destack_net_leave_multicast_v6, destack_net_read, destack_net_readv, destack_net_recv_mmsg,
    destack_net_recv_msg, destack_net_send_mmsg, destack_net_send_msg, destack_net_set_broadcast,
    destack_net_set_linger, destack_net_set_multicast_loop, destack_net_set_multicast_ttl,
    destack_net_set_no_delay, destack_net_set_nonblocking, destack_net_set_read_timeout,
    destack_net_set_recv_buffer, destack_net_set_reuse_addr, destack_net_set_reuse_port,
    destack_net_set_send_buffer, destack_net_set_tos, destack_net_set_ttl,
    destack_net_set_write_timeout, destack_net_shutdown, destack_net_udp_socket,
    destack_net_uds_accept, destack_net_uds_close_listener, destack_net_uds_connect,
    destack_net_uds_listen, destack_net_write, destack_net_writev,
};

/// Bind an existing socket to a raw address.
pub(crate) unsafe fn destack_net_bind(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_bind(context, handle, address) }
}

/// Connect an existing socket to a raw address.
pub(crate) unsafe fn destack_net_connect(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_connect_raw(context, handle, address) }
}

/// Read TCP keepalive settings.
pub(crate) unsafe fn destack_net_get_keep_alive(
    context: &RuntimeCallContext,
    out: *mut KeepAliveConfig,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_keep_alive(context, out, handle) }
}

/// Read TCP_NODELAY.
pub(crate) unsafe fn destack_net_get_no_delay(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_no_delay(context, out, handle) }
}

/// Read SO_REUSEADDR.
pub(crate) unsafe fn destack_net_get_reuse_addr(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_reuse_addr(context, out, handle) }
}

/// Read SO_REUSEPORT.
pub(crate) unsafe fn destack_net_get_reuse_port(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_reuse_port(context, out, handle) }
}

/// Read socket linger settings.
pub(crate) unsafe fn destack_net_get_linger(
    context: &RuntimeCallContext,
    out: *mut Linger,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_linger(context, out, handle) }
}

/// Read receive buffer size.
pub(crate) unsafe fn destack_net_get_recv_buffer(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_recv_buffer(context, out, handle) }
}

/// Read send buffer size.
pub(crate) unsafe fn destack_net_get_send_buffer(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_send_buffer(context, out, handle) }
}

/// Read broadcast mode.
pub(crate) unsafe fn destack_net_get_broadcast(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_broadcast(context, out, handle) }
}

/// Read IP time to live.
pub(crate) unsafe fn destack_net_get_ttl(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_ttl(context, out, handle) }
}

/// Read IP type of service.
pub(crate) unsafe fn destack_net_get_tos(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_tos(context, out, handle) }
}

/// Read read timeout in milliseconds.
pub(crate) unsafe fn destack_net_get_read_timeout(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_read_timeout(context, out, handle) }
}

/// Read write timeout in milliseconds.
pub(crate) unsafe fn destack_net_get_write_timeout(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_write_timeout(context, out, handle) }
}

/// Read IPv6 only mode.
pub(crate) unsafe fn destack_net_get_only_v6(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_get_only_v6(context, out, handle) }
}

/// Start listening on a raw address.
pub(crate) unsafe fn destack_net_listen(
    context: &RuntimeCallContext,
    out: *mut ListenerHandle,
    address: SocketAddress,
    backlog: u32,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_listen_raw(context, out, address, backlog) }
}

/// Read the local socket address as raw bytes.
pub(crate) unsafe fn destack_net_local_address(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_local_address_raw(context, out, handle) }
}

/// Read the remote socket address as raw bytes.
pub(crate) unsafe fn destack_net_peer_address(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_peer_address_raw(context, out, handle) }
}

/// Receive a packet with raw source address metadata.
pub(crate) unsafe fn destack_net_recv_from(
    context: &RuntimeCallContext,
    out: *mut SocketRecvFrom,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    recvflags: SocketMessageFlags,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_recv_from(context, out, handle, buffer, recvflags) }
}

/// Resolve a hostname and port into raw socket addresses.
pub(crate) unsafe fn destack_net_resolve(
    context: &RuntimeCallContext,
    out: *mut NativeArray<SocketAddress>,
    host: NativeStringRef,
    port: u16,
    family: crate::platform::net::SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_resolve_raw(context, out, host, port, family, flags) }
}

/// Reverse lookup a raw socket address.
pub(crate) unsafe fn destack_net_reverse_lookup(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_reverse_lookup_raw(context, out, address) }
}

/// Send a packet to a raw destination address.
pub(crate) unsafe fn destack_net_send_to(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    message: SocketSendTo,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_send_to(context, out, handle, buffer, message) }
}

/// Write full TCP keepalive settings.
pub(crate) unsafe fn destack_net_set_keep_alive(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    config: KeepAliveConfig,
) -> RuntimeResult<()> {
    unsafe {
        core_net::destack_net_set_keep_alive(
            context,
            handle,
            config.enabled,
            config.idle_seconds,
            config.interval_seconds,
            config.probe_count,
        )
    }
}

/// Restrict an IPv6 socket to IPv6 traffic only.
pub(crate) unsafe fn destack_net_set_only_v6(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_only_v6(context, handle, enabled) }
}

/// Create a socket from family, type, and protocol.
pub(crate) unsafe fn destack_net_socket(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    family: crate::platform::net::SocketFamily,
    sockettype: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_socket(context, out, family, sockettype, protocol) }
}

/// Create a connected socket pair.
pub(crate) unsafe fn destack_net_socket_pair(
    context: &RuntimeCallContext,
    out: *mut SocketPair,
    family: crate::platform::net::SocketFamily,
    sockettype: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_socket_pair(context, out, family, sockettype, protocol) }
}

/// Bind a UDP socket to a raw address.
pub(crate) unsafe fn destack_net_udp_bind(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_udp_bind_raw(context, handle, address) }
}

/// Connect a UDP socket to a raw address.
pub(crate) unsafe fn destack_net_udp_connect(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_udp_connect_raw(context, handle, address) }
}

/// Receive a UDP packet with raw sender metadata.
pub(crate) unsafe fn destack_net_udp_recv_from(
    context: &RuntimeCallContext,
    out: *mut UdpReceive,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    recvflags: UdpMessageFlags,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_udp_recv_from_raw(context, out, handle, buffer, recvflags) }
}

/// Send a UDP packet to a raw destination address.
pub(crate) unsafe fn destack_net_udp_send_to(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    address: SocketAddress,
    buffer: NativeSlice<u8>,
    sendflags: UdpMessageFlags,
) -> RuntimeResult<()> {
    unsafe {
        core_net::destack_net_udp_send_to_raw(context, out, handle, address, buffer, sendflags)
    }
}

/// Create a connected unix domain socket pair.
pub(crate) unsafe fn destack_net_uds_socket_pair(
    context: &RuntimeCallContext,
    out: *mut SocketPair,
    sockettype: SocketType,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_uds_socket_pair(context, out, sockettype) }
}
