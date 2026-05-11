#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::{NativeArray, PlatformError};

use crate::runtime::BindingCallContext;

use crate::platform::net::{
    AcceptFlags, KeepAliveConfig, Linger, NetInterface, PacketBackendDescriptor,
    PacketCaptureOptions, PacketCaptureRecord, PacketCaptureStats, PacketFanoutOptions,
    PacketRingOptions, PacketTimestampMode, ResolveQuery, ReverseLookupFlags, ReverseLookupName,
    RouteEntry, SocketAddress, SocketFamily, SocketMessageFlags, SocketOptionLevel,
    SocketOptionName, SocketPair, SocketProtocol, SocketRecvBatchRequest, SocketRecvFrom,
    SocketRecvMessage, SocketSendBatchEntry, SocketSendMessage, SocketSendTo, SocketShutdown,
    SocketTimestampingMode, SocketType, UdpMessageFlags, UdpReceive, UdpSourceMembershipV4,
    UdpSourceMembershipV6, UdsAddress,
};
use crate::platform::{fs, resource};

/// List host packet backends.
pub(crate) unsafe fn destack_net_packet_backend_list(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<PacketBackendDescriptor>,
) -> RuntimeResult<()> {
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetBackendList",
    ))
    .boxed())
}

/// Accept a new connection from a listener.
pub(crate) unsafe fn destack_net_accept(
    _binding: &BindingCallContext,
    out: *mut resource::SocketHandle,
    listener: resource::ListenerHandle,
    flags: AcceptFlags,
) -> RuntimeResult<()> {
    let _ = (out, listener, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.accept")).boxed())
}

/// Read the local socket address as raw bytes.
pub(crate) unsafe fn destack_net_local_address(
    _binding: &BindingCallContext,
    out: *mut SocketAddress,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.address.localAddress",
    ))
    .boxed())
}

/// Read the remote socket address as raw bytes.
pub(crate) unsafe fn destack_net_peer_address(
    _binding: &BindingCallContext,
    out: *mut SocketAddress,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.address.peerAddress",
    ))
    .boxed())
}

/// Bind an existing socket to a raw address.
pub(crate) unsafe fn destack_net_bind(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (handle, address);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.bind")).boxed())
}

/// Close a socket handle.
pub(crate) unsafe fn destack_net_close(
    _binding: &BindingCallContext,
    _handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.close")).boxed())
}

/// Close a listener handle.
pub(crate) unsafe fn destack_net_close_listener(
    _binding: &BindingCallContext,
    _handle: resource::ListenerHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.closeListener")).boxed())
}

/// Connect to a remote socket address.
pub(crate) unsafe fn destack_net_connect(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (handle, address);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.connect")).boxed())
}

/// List network interfaces with addresses and flags.
pub(crate) unsafe fn destack_net_list_interfaces(
    _binding: &BindingCallContext,
    _out: *mut NativeArray<NetInterface>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.interface.listInterfaces",
    ))
    .boxed())
}

/// Resolve an interface name to an index.
pub(crate) unsafe fn destack_net_interface_index(
    _binding: &BindingCallContext,
    out: *mut u32,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (out, name);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.interfaceIndex")).boxed())
}

/// Resolve an interface index to a name.
pub(crate) unsafe fn destack_net_interface_name(
    _binding: &BindingCallContext,
    out: *mut NativeStringRef,
    index: u32,
) -> RuntimeResult<()> {
    let _ = (out, index);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.interfaceName")).boxed())
}

/// Start listening on a raw socket address.
pub(crate) unsafe fn destack_net_listen(
    _binding: &BindingCallContext,
    out: *mut resource::ListenerHandle,
    address: SocketAddress,
    backlog: u32,
) -> RuntimeResult<()> {
    let _ = (out, address, backlog);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.listen")).boxed())
}

/// Read broadcast mode.
pub(crate) unsafe fn destack_net_get_broadcast(
    _binding: &BindingCallContext,
    out: *mut bool,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.getBroadcast",
    ))
    .boxed())
}

/// Read socket linger settings.
pub(crate) unsafe fn destack_net_get_linger(
    _binding: &BindingCallContext,
    out: *mut Linger,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.getLinger",
    ))
    .boxed())
}

/// Read IPv6-only mode.
pub(crate) unsafe fn destack_net_get_only_v6(
    _binding: &BindingCallContext,
    out: *mut bool,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.getOnlyV6",
    ))
    .boxed())
}

/// Read socket packet mark.
pub(crate) unsafe fn destack_net_get_packet_mark(
    _binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.getPacketMark",
    ))
    .boxed())
}

/// Read the read timeout in milliseconds.
pub(crate) unsafe fn destack_net_get_read_timeout(
    _binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.getReadTimeout",
    ))
    .boxed())
}

/// Read the receive buffer size.
pub(crate) unsafe fn destack_net_get_recv_buffer(
    _binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.getRecvBuffer",
    ))
    .boxed())
}

/// Read the send buffer size.
pub(crate) unsafe fn destack_net_get_send_buffer(
    _binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.getSendBuffer",
    ))
    .boxed())
}

/// Read one raw socket option payload.
pub(crate) unsafe fn destack_net_get_sock_opt_raw(
    _binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: resource::SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    maxbytes: u32,
) -> RuntimeResult<()> {
    let _ = (out, handle, level, name, maxbytes);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.getSockOptRaw",
    ))
    .boxed())
}

/// Read packet timestamping mode.
pub(crate) unsafe fn destack_net_get_timestamping(
    _binding: &BindingCallContext,
    out: *mut SocketTimestampingMode,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.getTimestamping",
    ))
    .boxed())
}

/// Read the IP type-of-service field.
pub(crate) unsafe fn destack_net_get_tos(
    _binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.options.getTos")).boxed())
}

/// Read the IP time-to-live.
pub(crate) unsafe fn destack_net_get_ttl(
    _binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.options.getTtl")).boxed())
}

/// Read the write timeout in milliseconds.
pub(crate) unsafe fn destack_net_get_write_timeout(
    _binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.getWriteTimeout",
    ))
    .boxed())
}

/// Enable or disable broadcast.
pub(crate) unsafe fn destack_net_set_broadcast(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.setBroadcast",
    ))
    .boxed())
}

/// Set socket linger settings.
pub(crate) unsafe fn destack_net_set_linger(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    linger: Linger,
) -> RuntimeResult<()> {
    let _ = (handle, linger);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.setLinger",
    ))
    .boxed())
}

/// Restrict an IPv6 socket to IPv6 traffic only.
pub(crate) unsafe fn destack_net_set_only_v6(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.setOnlyV6",
    ))
    .boxed())
}

/// Set socket packet mark.
pub(crate) unsafe fn destack_net_set_packet_mark(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    mark: u32,
) -> RuntimeResult<()> {
    let _ = (handle, mark);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.setPacketMark",
    ))
    .boxed())
}

/// Set the read timeout in milliseconds.
pub(crate) unsafe fn destack_net_set_read_timeout(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    timeoutms: u32,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutms);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.setReadTimeout",
    ))
    .boxed())
}

/// Set the receive buffer size.
pub(crate) unsafe fn destack_net_set_recv_buffer(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let _ = (handle, size);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.setRecvBuffer",
    ))
    .boxed())
}

/// Set the send buffer size.
pub(crate) unsafe fn destack_net_set_send_buffer(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let _ = (handle, size);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.setSendBuffer",
    ))
    .boxed())
}

/// Set one raw socket option payload.
pub(crate) unsafe fn destack_net_set_sock_opt_raw(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    argument_value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, level, name, argument_value);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.setSockOptRaw",
    ))
    .boxed())
}

/// Set packet timestamping mode.
pub(crate) unsafe fn destack_net_set_timestamping(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    mode: SocketTimestampingMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.setTimestamping",
    ))
    .boxed())
}

/// Set the IP type-of-service field.
pub(crate) unsafe fn destack_net_set_tos(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    let _ = (handle, tos);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.options.setTos")).boxed())
}

/// Set the IP time-to-live.
pub(crate) unsafe fn destack_net_set_ttl(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    let _ = (handle, ttl);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.options.setTtl")).boxed())
}

/// Set the write timeout in milliseconds.
pub(crate) unsafe fn destack_net_set_write_timeout(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    timeoutms: u32,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutms);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.options.setWriteTimeout",
    ))
    .boxed())
}

/// Clear packet fanout from a packet endpoint.
pub(crate) unsafe fn destack_net_packet_clear_fanout(
    _binding: &BindingCallContext,
    _handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetClearFanout",
    ))
    .boxed())
}

/// Clear the active packet filter program.
pub(crate) unsafe fn destack_net_packet_clear_filter(
    _binding: &BindingCallContext,
    _handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetClearFilter",
    ))
    .boxed())
}

/// Clear packet rx and tx ring configuration.
pub(crate) unsafe fn destack_net_packet_clear_ring(
    _binding: &BindingCallContext,
    _handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetClearRing",
    ))
    .boxed())
}

/// Open a packet capture or inject endpoint.
pub(crate) unsafe fn destack_net_packet_open(
    _binding: &BindingCallContext,
    out: *mut resource::SocketHandle,
    options: PacketCaptureOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.raw.packetOpen")).boxed())
}

/// Receive one packet from a packet endpoint.
pub(crate) unsafe fn destack_net_packet_receive(
    _binding: &BindingCallContext,
    out: *mut PacketCaptureRecord,
    handle: resource::SocketHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetReceive",
    ))
    .boxed())
}

/// Send one packet through a packet endpoint.
pub(crate) unsafe fn destack_net_packet_send(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.raw.packetSend")).boxed())
}

/// Set packet fanout on a packet endpoint.
pub(crate) unsafe fn destack_net_packet_set_fanout(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    options: PacketFanoutOptions,
) -> RuntimeResult<()> {
    let _ = (handle, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetSetFanout",
    ))
    .boxed())
}

/// Attach one packet filter program to a raw endpoint.
pub(crate) unsafe fn destack_net_packet_set_filter(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    filterprogram: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, filterprogram);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetSetFilter",
    ))
    .boxed())
}

/// Configure one packet rx ring for zero-copy capture.
pub(crate) unsafe fn destack_net_packet_set_rx_ring(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    options: PacketRingOptions,
) -> RuntimeResult<()> {
    let _ = (handle, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetSetRxRing",
    ))
    .boxed())
}

/// Configure packet timestamp mode for a socket or packet endpoint.
pub(crate) unsafe fn destack_net_packet_set_timestamp_mode(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    mode: PacketTimestampMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetSetTimestampMode",
    ))
    .boxed())
}

/// Configure one packet tx ring for zero-copy transmit.
pub(crate) unsafe fn destack_net_packet_set_tx_ring(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    options: PacketRingOptions,
) -> RuntimeResult<()> {
    let _ = (handle, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetSetTxRing",
    ))
    .boxed())
}

/// Read packet capture statistics from one endpoint.
pub(crate) unsafe fn destack_net_packet_stats(
    _binding: &BindingCallContext,
    out: *mut PacketCaptureStats,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.raw.packetStats")).boxed())
}

/// Enable or disable IP header inclusion on a raw socket.
pub(crate) unsafe fn destack_net_raw_set_header_included(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.setHeaderIncluded",
    ))
    .boxed())
}

/// Open a raw IP socket.
pub(crate) unsafe fn destack_net_raw_socket(
    _binding: &BindingCallContext,
    out: *mut resource::SocketHandle,
    family: SocketFamily,
    protocol: i32,
) -> RuntimeResult<()> {
    let _ = (out, family, protocol);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.raw.socket")).boxed())
}

/// Read from a socket into the provided slice.
pub(crate) unsafe fn destack_net_read(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.read")).boxed())
}

/// Read into multiple buffers.
pub(crate) unsafe fn destack_net_readv(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.readv")).boxed())
}

/// Receive a packet from a remote socket address.
pub(crate) unsafe fn destack_net_recv_from(
    _binding: &BindingCallContext,
    out: *mut SocketRecvFrom,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
    recvflags: SocketMessageFlags,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer, recvflags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.recvFrom")).boxed())
}

/// Receive multiple datagrams.
pub(crate) unsafe fn destack_net_recv_mmsg(
    _binding: &BindingCallContext,
    out: *mut NativeArray<SocketRecvMessage>,
    handle: resource::SocketHandle,
    requests: NativeSlice<SocketRecvBatchRequest>,
    maxfds: u32,
    wantcredentials: bool,
    maxcontrolbytes: u32,
) -> RuntimeResult<()> {
    let _ = (
        out,
        handle,
        requests,
        maxfds,
        wantcredentials,
        maxcontrolbytes,
    );

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.recvMmsg")).boxed())
}

/// Receive a message with ancillary data.
pub(crate) unsafe fn destack_net_recv_msg(
    _binding: &BindingCallContext,
    out: *mut SocketRecvMessage,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
    recvflags: SocketMessageFlags,
    maxfds: u32,
    wantcredentials: bool,
    maxcontrolbytes: u32,
) -> RuntimeResult<()> {
    let _ = (
        out,
        handle,
        buffer,
        recvflags,
        maxfds,
        wantcredentials,
        maxcontrolbytes,
    );

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.recvMsg")).boxed())
}

/// Resolve a host and service query into raw socket addresses.
pub(crate) unsafe fn destack_net_resolve(
    _binding: &BindingCallContext,
    out: *mut NativeArray<SocketAddress>,
    query: ResolveQuery,
) -> RuntimeResult<()> {
    let _ = (out, query);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.resolve.lookup")).boxed())
}

/// Reverse lookup a raw socket address into host and service names.
pub(crate) unsafe fn destack_net_reverse_lookup(
    _binding: &BindingCallContext,
    out: *mut NativeArray<ReverseLookupName>,
    address: SocketAddress,
    flags: ReverseLookupFlags,
) -> RuntimeResult<()> {
    let _ = (out, address, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.resolve.reverseLookup",
    ))
    .boxed())
}

/// Read SO_REUSEADDR.
pub(crate) unsafe fn destack_net_get_reuse_addr(
    _binding: &BindingCallContext,
    out: *mut bool,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.reuse.getReuseAddr",
    ))
    .boxed())
}

/// Read SO_REUSEPORT.
pub(crate) unsafe fn destack_net_get_reuse_port(
    _binding: &BindingCallContext,
    out: *mut bool,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.reuse.getReusePort",
    ))
    .boxed())
}

/// Enable or disable SO_REUSEADDR.
pub(crate) unsafe fn destack_net_set_reuse_addr(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.reuse.setReuseAddr",
    ))
    .boxed())
}

/// Enable or disable SO_REUSEPORT.
pub(crate) unsafe fn destack_net_set_reuse_port(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.reuse.setReusePort",
    ))
    .boxed())
}

/// Add a route table entry.
pub(crate) unsafe fn destack_net_route_add(
    _binding: &BindingCallContext,
    _route: RouteEntry,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.routeAdd")).boxed())
}

/// Remove a route table entry.
pub(crate) unsafe fn destack_net_route_delete(
    _binding: &BindingCallContext,
    _route: RouteEntry,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.routeDelete")).boxed())
}

/// List route table entries.
pub(crate) unsafe fn destack_net_route_list(
    _binding: &BindingCallContext,
    out: *mut NativeArray<RouteEntry>,
    family: SocketFamily,
) -> RuntimeResult<()> {
    let _ = (out, family);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.routeList")).boxed())
}

/// Send multiple datagrams.
pub(crate) unsafe fn destack_net_send_mmsg(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    messages: NativeSlice<SocketSendBatchEntry>,
) -> RuntimeResult<()> {
    let _ = (out, handle, messages);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.sendMmsg")).boxed())
}

/// Send a message with ancillary data.
pub(crate) unsafe fn destack_net_send_msg(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
    message: SocketSendMessage,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer, message);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.sendMsg")).boxed())
}

/// Send a packet to a remote socket address.
pub(crate) unsafe fn destack_net_send_to(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
    message: SocketSendTo,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer, message);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.sendTo")).boxed())
}

/// Enable or disable nonblocking mode on a socket.
pub(crate) unsafe fn destack_net_set_nonblocking(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setNonblocking")).boxed())
}

/// Shut down a socket for reads, writes, or both.
pub(crate) unsafe fn destack_net_shutdown(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    let _ = (handle, how);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.shutdown")).boxed())
}

/// Create a socket from a native family, type, and protocol.
pub(crate) unsafe fn destack_net_socket(
    _binding: &BindingCallContext,
    out: *mut resource::SocketHandle,
    family: SocketFamily,
    sockettype: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    let _ = (out, family, sockettype, protocol);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.socket")).boxed())
}

/// Create a connected socket pair.
pub(crate) unsafe fn destack_net_socket_pair(
    _binding: &BindingCallContext,
    out: *mut SocketPair,
    family: SocketFamily,
    sockettype: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    let _ = (out, family, sockettype, protocol);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.socketPair")).boxed())
}

/// Read full TCP keepalive parameters.
pub(crate) unsafe fn destack_net_get_keep_alive(
    _binding: &BindingCallContext,
    out: *mut KeepAliveConfig,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.tcp.getKeepAlive")).boxed())
}

/// Read TCP_NODELAY.
pub(crate) unsafe fn destack_net_get_no_delay(
    _binding: &BindingCallContext,
    out: *mut bool,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.tcp.getNoDelay")).boxed())
}

/// Write full TCP keepalive parameters.
pub(crate) unsafe fn destack_net_set_keep_alive(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    config: KeepAliveConfig,
) -> RuntimeResult<()> {
    let _ = (handle, config);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.tcp.setKeepAlive")).boxed())
}

/// Enable or disable TCP_NODELAY.
pub(crate) unsafe fn destack_net_set_no_delay(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.tcp.setNoDelay")).boxed())
}

/// Bind a UDP socket to a raw local address.
pub(crate) unsafe fn destack_net_udp_bind(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (handle, address);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udp.bind")).boxed())
}

/// Connect a UDP socket to a raw remote address.
pub(crate) unsafe fn destack_net_udp_connect(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (handle, address);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udp.connect")).boxed())
}

/// Read the default IPv4 multicast interface for one socket.
pub(crate) unsafe fn destack_net_get_multicast_interface_v4(
    _binding: &BindingCallContext,
    out: *mut NativeStringRef,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.getMulticastInterfaceV4",
    ))
    .boxed())
}

/// Read the default IPv6 multicast interface for one socket.
pub(crate) unsafe fn destack_net_get_multicast_interface_v6(
    _binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.getMulticastInterfaceV6",
    ))
    .boxed())
}

/// Read multicast loopback mode.
pub(crate) unsafe fn destack_net_get_multicast_loop(
    _binding: &BindingCallContext,
    out: *mut bool,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.getMulticastLoop",
    ))
    .boxed())
}

/// Read multicast TTL or hop-limit.
pub(crate) unsafe fn destack_net_get_multicast_ttl(
    _binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.getMulticastTtl",
    ))
    .boxed())
}

/// Join one IPv4 source-specific multicast membership.
pub(crate) unsafe fn destack_net_join_multicast_source_v4(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    membership: UdpSourceMembershipV4,
) -> RuntimeResult<()> {
    let _ = (handle, membership);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.joinMulticastSourceV4",
    ))
    .boxed())
}

/// Join one IPv6 source-specific multicast membership.
pub(crate) unsafe fn destack_net_join_multicast_source_v6(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    membership: UdpSourceMembershipV6,
) -> RuntimeResult<()> {
    let _ = (handle, membership);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.joinMulticastSourceV6",
    ))
    .boxed())
}

/// Join an IPv4 multicast group.
pub(crate) unsafe fn destack_net_join_multicast_v4(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    group: NativeStringRef,
    interfaceaddress: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (handle, group, interfaceaddress);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.joinMulticastV4",
    ))
    .boxed())
}

/// Join an IPv6 multicast group.
pub(crate) unsafe fn destack_net_join_multicast_v6(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    group: NativeStringRef,
    interfaceindex: u32,
) -> RuntimeResult<()> {
    let _ = (handle, group, interfaceindex);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.joinMulticastV6",
    ))
    .boxed())
}

/// Leave one IPv4 source-specific multicast membership.
pub(crate) unsafe fn destack_net_leave_multicast_source_v4(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    membership: UdpSourceMembershipV4,
) -> RuntimeResult<()> {
    let _ = (handle, membership);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.leaveMulticastSourceV4",
    ))
    .boxed())
}

/// Leave one IPv6 source-specific multicast membership.
pub(crate) unsafe fn destack_net_leave_multicast_source_v6(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    membership: UdpSourceMembershipV6,
) -> RuntimeResult<()> {
    let _ = (handle, membership);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.leaveMulticastSourceV6",
    ))
    .boxed())
}

/// Leave an IPv4 multicast group.
pub(crate) unsafe fn destack_net_leave_multicast_v4(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    group: NativeStringRef,
    interfaceaddress: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (handle, group, interfaceaddress);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.leaveMulticastV4",
    ))
    .boxed())
}

/// Leave an IPv6 multicast group.
pub(crate) unsafe fn destack_net_leave_multicast_v6(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    group: NativeStringRef,
    interfaceindex: u32,
) -> RuntimeResult<()> {
    let _ = (handle, group, interfaceindex);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.leaveMulticastV6",
    ))
    .boxed())
}

/// Receive a datagram from a remote address with raw address output.
pub(crate) unsafe fn destack_net_udp_recv_from(
    _binding: &BindingCallContext,
    out: *mut UdpReceive,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
    recvflags: UdpMessageFlags,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer, recvflags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udp.recvFrom")).boxed())
}

/// Send a datagram to a raw remote address.
pub(crate) unsafe fn destack_net_udp_send_to(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    address: SocketAddress,
    buffer: NativeSlice<u8>,
    sendflags: UdpMessageFlags,
) -> RuntimeResult<()> {
    let _ = (out, handle, address, buffer, sendflags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udp.sendTo")).boxed())
}

/// Select the default IPv4 multicast interface for one socket.
pub(crate) unsafe fn destack_net_set_multicast_interface_v4(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    interfaceaddress: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (handle, interfaceaddress);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.setMulticastInterfaceV4",
    ))
    .boxed())
}

/// Select the default IPv6 multicast interface for one socket.
pub(crate) unsafe fn destack_net_set_multicast_interface_v6(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    interfaceindex: u32,
) -> RuntimeResult<()> {
    let _ = (handle, interfaceindex);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.setMulticastInterfaceV6",
    ))
    .boxed())
}

/// Enable or disable multicast loopback.
pub(crate) unsafe fn destack_net_set_multicast_loop(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.setMulticastLoop",
    ))
    .boxed())
}

/// Set multicast TTL.
pub(crate) unsafe fn destack_net_set_multicast_ttl(
    _binding: &BindingCallContext,
    handle: resource::SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    let _ = (handle, ttl);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udp.setMulticastTtl",
    ))
    .boxed())
}

/// Create a UDP socket.
pub(crate) unsafe fn destack_net_udp_socket(
    _binding: &BindingCallContext,
    out: *mut resource::SocketHandle,
    family: SocketFamily,
) -> RuntimeResult<()> {
    let _ = (out, family);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udp.socket")).boxed())
}

/// Accept a connection from a UDS listener.
pub(crate) unsafe fn destack_net_uds_accept(
    _binding: &BindingCallContext,
    out: *mut resource::SocketHandle,
    listener: resource::ListenerHandle,
) -> RuntimeResult<()> {
    let _ = (out, listener);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsAccept")).boxed())
}

/// Close a UDS listener handle.
pub(crate) unsafe fn destack_net_uds_close_listener(
    _binding: &BindingCallContext,
    _handle: resource::ListenerHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsCloseListener")).boxed())
}

/// Connect to a UDS endpoint.
pub(crate) unsafe fn destack_net_uds_connect(
    _binding: &BindingCallContext,
    out: *mut resource::SocketHandle,
    address: UdsAddress,
) -> RuntimeResult<()> {
    let _ = (out, address);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsConnect")).boxed())
}

/// Listen on a UDS address.
pub(crate) unsafe fn destack_net_uds_listen(
    _binding: &BindingCallContext,
    out: *mut resource::ListenerHandle,
    address: UdsAddress,
    backlog: u32,
) -> RuntimeResult<()> {
    let _ = (out, address, backlog);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsListen")).boxed())
}

/// Create a connected UDS socket pair.
pub(crate) unsafe fn destack_net_uds_socket_pair(
    _binding: &BindingCallContext,
    out: *mut SocketPair,
    sockettype: SocketType,
) -> RuntimeResult<()> {
    let _ = (out, sockettype);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsSocketPair")).boxed())
}

/// Write to a socket from the provided slice.
pub(crate) unsafe fn destack_net_write(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.write")).boxed())
}

/// Write from multiple buffers.
pub(crate) unsafe fn destack_net_writev(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.writev")).boxed())
}
