use super::host;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{OsPath, OsPathBytes, PathBytesAbi};
use crate::platform::net::{
    KeepAliveConfig, Linger, NetInterface, PacketBackendDescriptor, PacketCaptureOptions,
    PacketCaptureRecord, PacketCaptureStats, PacketFanoutOptions, PacketRingOptions,
    PacketTimestampMode, ResolveQuery, ReverseLookupFlags, ReverseLookupName, RouteEntry,
    SocketAddress, SocketFamily, SocketMessageFlags, SocketOptionLevel, SocketOptionName,
    SocketPair, SocketProtocol, SocketRecvBatchRequest, SocketRecvFrom, SocketRecvMessage,
    SocketSendBatchEntry, SocketSendTo, SocketTimestampingMode, SocketType, UdpMessageFlags,
    UdpReceive, UdpSourceMembershipV4, UdpSourceMembershipV6, UdsAbstractAddress, UdsAddress,
    UdsPathAddress, UdsUnnamedAddress, host as host_net,
};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeArray, PlatformError};

use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::runtime::BindingCallContext;

pub(crate) use host_net::{
    destack_net_accept, destack_net_close, destack_net_close_listener,
    destack_net_join_multicast_v4, destack_net_join_multicast_v6, destack_net_leave_multicast_v4,
    destack_net_leave_multicast_v6, destack_net_read, destack_net_readv, destack_net_send_msg,
    destack_net_set_broadcast, destack_net_set_linger, destack_net_set_multicast_loop,
    destack_net_set_multicast_ttl, destack_net_set_no_delay, destack_net_set_nonblocking,
    destack_net_set_read_timeout, destack_net_set_recv_buffer, destack_net_set_reuse_addr,
    destack_net_set_reuse_port, destack_net_set_send_buffer, destack_net_set_tos,
    destack_net_set_ttl, destack_net_set_write_timeout, destack_net_shutdown,
    destack_net_udp_socket, destack_net_uds_accept, destack_net_uds_close_listener,
    destack_net_write, destack_net_writev,
};

/// Bind an existing socket to a raw address.
pub(crate) unsafe fn destack_net_bind(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_bind(binding, handle, address) }
}

/// Connect to a remote socket address.
pub(crate) unsafe fn destack_net_connect(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_connect_raw(binding, handle, address) }
}

/// Read full TCP keepalive parameters.
pub(crate) unsafe fn destack_net_get_keep_alive(
    binding: &BindingCallContext,
    out: *mut KeepAliveConfig,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_keep_alive(binding, out, handle) }
}

/// Read TCP_NODELAY.
pub(crate) unsafe fn destack_net_get_no_delay(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_no_delay(binding, out, handle) }
}

/// Read SO_REUSEADDR.
pub(crate) unsafe fn destack_net_get_reuse_addr(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_reuse_addr(binding, out, handle) }
}

/// Read SO_REUSEPORT.
pub(crate) unsafe fn destack_net_get_reuse_port(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_reuse_port(binding, out, handle) }
}

/// Read socket linger settings.
pub(crate) unsafe fn destack_net_get_linger(
    binding: &BindingCallContext,
    out: *mut Linger,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_linger(binding, out, handle) }
}

/// Read the receive buffer size.
pub(crate) unsafe fn destack_net_get_recv_buffer(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_recv_buffer(binding, out, handle) }
}

/// Read the send buffer size.
pub(crate) unsafe fn destack_net_get_send_buffer(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_send_buffer(binding, out, handle) }
}

/// Read broadcast mode.
pub(crate) unsafe fn destack_net_get_broadcast(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_broadcast(binding, out, handle) }
}

/// Read the IP time-to-live.
pub(crate) unsafe fn destack_net_get_ttl(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_ttl(binding, out, handle) }
}

/// Read the IP type-of-service field.
pub(crate) unsafe fn destack_net_get_tos(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_tos(binding, out, handle) }
}

/// Read the read timeout in milliseconds.
pub(crate) unsafe fn destack_net_get_read_timeout(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_read_timeout(binding, out, handle) }
}

/// Read the write timeout in milliseconds.
pub(crate) unsafe fn destack_net_get_write_timeout(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_write_timeout(binding, out, handle) }
}

/// Read IPv6-only mode.
pub(crate) unsafe fn destack_net_get_only_v6(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_only_v6(binding, out, handle) }
}

/// Start listening on a raw socket address.
pub(crate) unsafe fn destack_net_listen(
    binding: &BindingCallContext,
    out: *mut ListenerHandle,
    address: SocketAddress,
    backlog: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_listen_raw(binding, out, address, backlog) }
}

/// Read the local socket address as raw bytes.
pub(crate) unsafe fn destack_net_local_address(
    binding: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_local_address_raw(binding, out, handle) }
}

/// Read the remote socket address as raw bytes.
pub(crate) unsafe fn destack_net_peer_address(
    binding: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_peer_address_raw(binding, out, handle) }
}

/// Receive a packet from a remote socket address.
pub(crate) unsafe fn destack_net_recv_from(
    binding: &BindingCallContext,
    out: *mut SocketRecvFrom,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    recvflags: SocketMessageFlags,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_recv_from(binding, out, handle, buffer, recvflags) }
}

/// Resolve a host and service query into raw socket addresses.
pub(crate) unsafe fn destack_net_resolve(
    binding: &BindingCallContext,
    out: *mut NativeArray<SocketAddress>,
    query: ResolveQuery,
) -> RuntimeResult<()> {
    // validate query components
    let (host, service) = resolve_query_parts(query)?;

    // delegate to the raw resolver
    unsafe {
        host_net::destack_net_resolve_raw(binding, out, host, service, query.family, query.flags)
    }
}

/// Reverse lookup a raw socket address into host and service names.
pub(crate) unsafe fn destack_net_reverse_lookup(
    binding: &BindingCallContext,
    out: *mut NativeArray<ReverseLookupName>,
    address: SocketAddress,
    flags: ReverseLookupFlags,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_reverse_lookup_names_raw(binding, out, address, flags) }
}

/// Send a packet to a remote socket address.
pub(crate) unsafe fn destack_net_send_to(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    message: SocketSendTo,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_send_to(binding, out, handle, buffer, message) }
}

/// Write full TCP keepalive parameters.
pub(crate) unsafe fn destack_net_set_keep_alive(
    binding: &BindingCallContext,
    handle: SocketHandle,
    config: KeepAliveConfig,
) -> RuntimeResult<()> {
    unsafe {
        host_net::destack_net_set_keep_alive(
            binding,
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_only_v6(binding, handle, enabled) }
}

/// Create a socket from a native family, type, and protocol.
pub(crate) unsafe fn destack_net_socket(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
    sockettype: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_socket(binding, out, family, sockettype, protocol) }
}

/// Create a connected socket pair.
pub(crate) unsafe fn destack_net_socket_pair(
    binding: &BindingCallContext,
    out: *mut SocketPair,
    family: SocketFamily,
    sockettype: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_socket_pair(binding, out, family, sockettype, protocol) }
}

/// Bind a UDP socket to a raw local address.
pub(crate) unsafe fn destack_net_udp_bind(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_udp_bind_raw(binding, handle, address) }
}

/// Connect a UDP socket to a raw remote address.
pub(crate) unsafe fn destack_net_udp_connect(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_udp_connect_raw(binding, handle, address) }
}

/// Receive a datagram from a remote address with raw address output.
pub(crate) unsafe fn destack_net_udp_recv_from(
    binding: &BindingCallContext,
    out: *mut UdpReceive,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    recvflags: UdpMessageFlags,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_udp_recv_from_raw(binding, out, handle, buffer, recvflags) }
}

/// Send a datagram to a raw remote address.
pub(crate) unsafe fn destack_net_udp_send_to(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    address: SocketAddress,
    buffer: NativeSlice<u8>,
    sendflags: UdpMessageFlags,
) -> RuntimeResult<()> {
    unsafe {
        host_net::destack_net_udp_send_to_raw(binding, out, handle, address, buffer, sendflags)
    }
}

/// Receive a message with ancillary data.
pub(crate) unsafe fn destack_net_recv_msg(
    binding: &BindingCallContext,
    out: *mut SocketRecvMessage,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    recv_flags: SocketMessageFlags,
    max_fds: u32,
    want_credentials: bool,
    max_control_bytes: u32,
) -> RuntimeResult<()> {
    unsafe {
        host::destack_net_recv_msg(
            binding,
            out,
            handle,
            buffer,
            recv_flags,
            max_fds,
            want_credentials,
            max_control_bytes,
        )
    }
}

/// Receive multiple datagrams.
pub(crate) unsafe fn destack_net_recv_mmsg(
    binding: &BindingCallContext,
    out: *mut NativeArray<SocketRecvMessage>,
    handle: SocketHandle,
    requests: NativeSlice<SocketRecvBatchRequest>,
    max_fds: u32,
    want_credentials: bool,
    max_control_bytes: u32,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the request list
    let requests = unsafe { requests.as_slice()? };
    let mut messages = Vec::with_capacity(requests.len());

    // process requests one at a time
    for request in requests {
        let mut message = std::mem::MaybeUninit::<SocketRecvMessage>::uninit();
        unsafe {
            host::destack_net_recv_msg(
                binding,
                message.as_mut_ptr(),
                handle,
                request.payload,
                request.recv_flags,
                max_fds,
                want_credentials,
                max_control_bytes,
            )
        }?;
        let message = unsafe { message.assume_init() };
        messages.push(message);
    }

    unsafe {
        *out = binding.store_array(messages);
    }

    Ok(())
}

/// Send multiple datagrams.
pub(crate) unsafe fn destack_net_send_mmsg(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    messages: NativeSlice<SocketSendBatchEntry>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the message list
    let messages = unsafe { messages.as_slice()? };
    let mut sent_count = 0u64;

    // send each message entry
    for entry in messages {
        let mut _bytes = 0u64;
        unsafe {
            host_net::destack_net_send_msg(
                binding,
                &mut _bytes,
                handle,
                entry.payload,
                entry.message,
            )
        }?;
        sent_count = sent_count.saturating_add(1);
    }

    unsafe {
        *out = sent_count;
    }

    Ok(())
}

/// Connect to a UDS endpoint.
pub(crate) unsafe fn destack_net_uds_connect(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    address: UdsAddress,
) -> RuntimeResult<()> {
    // resolve the low level uds address into a path encoding
    let path = uds_path(binding, address)?;

    // delegate to the os implementation
    unsafe { host::destack_net_uds_connect(binding, out, path) }
}

/// Listen on a UDS address.
pub(crate) unsafe fn destack_net_uds_listen(
    binding: &BindingCallContext,
    out: *mut ListenerHandle,
    address: UdsAddress,
    backlog: u32,
) -> RuntimeResult<()> {
    // resolve the low level uds address into a path encoding
    let path = uds_path(binding, address)?;

    // delegate to the os implementation
    unsafe { host::destack_net_uds_listen(binding, out, path, backlog) }
}

/// Create a connected UDS socket pair.
pub(crate) unsafe fn destack_net_uds_socket_pair(
    binding: &BindingCallContext,
    out: *mut SocketPair,
    sockettype: SocketType,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_uds_socket_pair(binding, out, sockettype) }
}

/// Validate one resolve query and return its raw host and service parts.
fn resolve_query_parts(
    query: ResolveQuery,
) -> RuntimeResult<(Option<NativeStringRef>, Option<NativeStringRef>)> {
    // require at least one query component
    if query.host.is_none() && query.service.is_none() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "query",
            "host or service is required",
        ))
        .boxed());
    }

    Ok((query.host, query.service))
}

/// Map a UDS address into the path shape expected by OS backends.
fn uds_path(binding: &BindingCallContext, address: UdsAddress) -> RuntimeResult<OsPath> {
    match address {
        UdsAddress::UdsPathAddress(UdsPathAddress { path, .. }) => Ok(path),
        UdsAddress::UdsAbstractAddress(UdsAbstractAddress { abstract_name, .. }) => {
            let name = unsafe { abstract_name.as_slice()? };
            let mut bytes = Vec::with_capacity(name.len().saturating_add(1));
            bytes.push(0);
            bytes.extend_from_slice(name);
            let bytes = PathBytesAbi(binding.store_array(bytes));
            Ok(OsPath::OsPathBytes(OsPathBytes {
                kind: binding.store_string("bytes"),
                bytes,
            }))
        }
        UdsAddress::UdsUnnamedAddress(UdsUnnamedAddress { .. }) => {
            Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "address",
                "unnamed unix domain socket addresses cannot be used here",
            ))
            .boxed())
        }
    }
}

/// Read socket packet mark.
pub(crate) unsafe fn destack_net_get_packet_mark(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_packet_mark(binding, out, handle) }
}

/// Read one raw socket option payload.
pub(crate) unsafe fn destack_net_get_sock_opt_raw(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    maxbytes: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_sock_opt_raw(binding, out, handle, level, name, maxbytes) }
}

/// Read packet timestamping mode.
pub(crate) unsafe fn destack_net_get_timestamping(
    binding: &BindingCallContext,
    out: *mut SocketTimestampingMode,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_timestamping(binding, out, handle) }
}

/// Resolve an interface name to an index.
pub(crate) unsafe fn destack_net_interface_index(
    binding: &BindingCallContext,
    out: *mut u32,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_interface_index(binding, out, name) }
}

/// Resolve an interface index to a name.
pub(crate) unsafe fn destack_net_interface_name(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    index: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_interface_name(binding, out, index) }
}

/// List network interfaces with addresses and flags.
pub(crate) unsafe fn destack_net_list_interfaces(
    binding: &BindingCallContext,
    out: *mut NativeArray<NetInterface>,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_list_interfaces(binding, out) }
}

/// List host packet backends.
pub(crate) unsafe fn destack_net_packet_backend_list(
    binding: &BindingCallContext,
    out: *mut NativeSlice<PacketBackendDescriptor>,
) -> RuntimeResult<()> {
    unsafe { super::core::destack_net_packet_backend_list(binding, out) }
}

/// Open a packet capture or inject endpoint.
pub(crate) unsafe fn destack_net_packet_open(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    options: PacketCaptureOptions,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_open(binding, out, options) }
}

/// Receive one packet from a packet endpoint.
pub(crate) unsafe fn destack_net_packet_receive(
    binding: &BindingCallContext,
    out: *mut PacketCaptureRecord,
    handle: SocketHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_receive(binding, out, handle, payload) }
}

/// Send one packet through a packet endpoint.
pub(crate) unsafe fn destack_net_packet_send(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_send(binding, out, handle, payload) }
}

/// Configure packet timestamp mode for a socket or packet endpoint.
pub(crate) unsafe fn destack_net_packet_set_timestamp_mode(
    binding: &BindingCallContext,
    handle: SocketHandle,
    mode: PacketTimestampMode,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_timestamp_mode(binding, handle, mode) }
}

/// Clear packet fanout from a packet endpoint.
pub(crate) unsafe fn destack_net_packet_clear_fanout(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_clear_fanout(binding, handle) }
}

/// Clear the active packet filter program.
pub(crate) unsafe fn destack_net_packet_clear_filter(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_clear_filter(binding, handle) }
}

/// Clear packet rx and tx ring configuration.
pub(crate) unsafe fn destack_net_packet_clear_ring(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_clear_ring(binding, handle) }
}

/// Set packet fanout on a packet endpoint.
pub(crate) unsafe fn destack_net_packet_set_fanout(
    binding: &BindingCallContext,
    handle: SocketHandle,
    options: PacketFanoutOptions,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_fanout(binding, handle, options) }
}

/// Attach one packet filter program to a raw endpoint.
pub(crate) unsafe fn destack_net_packet_set_filter(
    binding: &BindingCallContext,
    handle: SocketHandle,
    filterprogram: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_filter(binding, handle, filterprogram) }
}

/// Configure one packet rx ring for zero-copy capture.
pub(crate) unsafe fn destack_net_packet_set_rx_ring(
    binding: &BindingCallContext,
    handle: SocketHandle,
    options: PacketRingOptions,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_rx_ring(binding, handle, options) }
}

/// Configure one packet tx ring for zero-copy transmit.
pub(crate) unsafe fn destack_net_packet_set_tx_ring(
    binding: &BindingCallContext,
    handle: SocketHandle,
    options: PacketRingOptions,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_tx_ring(binding, handle, options) }
}

/// Read packet capture statistics from one endpoint.
pub(crate) unsafe fn destack_net_packet_stats(
    binding: &BindingCallContext,
    out: *mut PacketCaptureStats,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_stats(binding, out, handle) }
}

/// Read the default IPv4 multicast interface for one socket.
pub(crate) unsafe fn destack_net_get_multicast_interface_v4(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_multicast_interface_v4(binding, out, handle) }
}

/// Read the default IPv6 multicast interface for one socket.
pub(crate) unsafe fn destack_net_get_multicast_interface_v6(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_multicast_interface_v6(binding, out, handle) }
}

/// Select the default IPv4 multicast interface for one socket.
pub(crate) unsafe fn destack_net_set_multicast_interface_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    interfaceaddress: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_multicast_interface_v4(binding, handle, interfaceaddress) }
}

/// Select the default IPv6 multicast interface for one socket.
pub(crate) unsafe fn destack_net_set_multicast_interface_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    interfaceindex: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_multicast_interface_v6(binding, handle, interfaceindex) }
}

/// Read multicast loopback mode.
pub(crate) unsafe fn destack_net_get_multicast_loop(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_multicast_loop(binding, out, handle) }
}

/// Read multicast TTL or hop-limit.
pub(crate) unsafe fn destack_net_get_multicast_ttl(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_multicast_ttl(binding, out, handle) }
}

/// Join one IPv4 source-specific multicast membership.
pub(crate) unsafe fn destack_net_join_multicast_source_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV4,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_join_multicast_source_v4(binding, handle, membership) }
}

/// Join one IPv6 source-specific multicast membership.
pub(crate) unsafe fn destack_net_join_multicast_source_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV6,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_join_multicast_source_v6(binding, handle, membership) }
}

/// Leave one IPv4 source-specific multicast membership.
pub(crate) unsafe fn destack_net_leave_multicast_source_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV4,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_leave_multicast_source_v4(binding, handle, membership) }
}

/// Leave one IPv6 source-specific multicast membership.
pub(crate) unsafe fn destack_net_leave_multicast_source_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV6,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_leave_multicast_source_v6(binding, handle, membership) }
}

/// Enable or disable IP header inclusion on a raw socket.
pub(crate) unsafe fn destack_net_raw_set_header_included(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_raw_set_header_included(binding, handle, enabled) }
}

/// Open a raw IP socket.
pub(crate) unsafe fn destack_net_raw_socket(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
    protocol: i32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_raw_socket(binding, out, family, protocol) }
}

/// Add a route table entry.
pub(crate) unsafe fn destack_net_route_add(
    binding: &BindingCallContext,
    route: RouteEntry,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_route_add(binding, route) }
}

/// Remove a route table entry.
pub(crate) unsafe fn destack_net_route_delete(
    binding: &BindingCallContext,
    route: RouteEntry,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_route_delete(binding, route) }
}

/// List route table entries.
pub(crate) unsafe fn destack_net_route_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<RouteEntry>,
    family: SocketFamily,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_route_list(binding, out, family) }
}

/// Set socket packet mark.
pub(crate) unsafe fn destack_net_set_packet_mark(
    binding: &BindingCallContext,
    handle: SocketHandle,
    mark: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_packet_mark(binding, handle, mark) }
}

/// Set one raw socket option payload.
pub(crate) unsafe fn destack_net_set_sock_opt_raw(
    binding: &BindingCallContext,
    handle: SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_sock_opt_raw(binding, handle, level, name, value) }
}

/// Set packet timestamping mode.
pub(crate) unsafe fn destack_net_set_timestamping(
    binding: &BindingCallContext,
    handle: SocketHandle,
    mode: SocketTimestampingMode,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_timestamping(binding, handle, mode) }
}
