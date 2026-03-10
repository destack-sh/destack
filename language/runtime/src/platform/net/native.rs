use super::host;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{OsPath, OsPathBytes, PathBytesAbi};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::platform::net::PACKET_BACKEND_CAP_TIMESTAMP;
use crate::platform::net::{
    KeepAliveConfig, Linger, NetInterface, PacketBackend, PacketBackendCapabilityFlags,
    PacketBackendDescriptor, PacketCaptureOptions, PacketCaptureRecord, PacketCaptureStats,
    PacketFanoutOptions, PacketRingOptions, PacketTimestampMode, ResolveQuery, ReverseLookupFlags,
    ReverseLookupName, RouteEntry, SocketAddress, SocketFamily, SocketMessageFlags,
    SocketOptionLevel, SocketOptionName, SocketPair, SocketProtocol, SocketRecvBatchRequest,
    SocketRecvFrom, SocketRecvMessage, SocketSendBatchEntry, SocketSendTo, SocketTimestampingMode,
    SocketType, UdpMessageFlags, UdpReceive, UdpSourceMembershipV4, UdpSourceMembershipV6,
    UdsAbstractAddress, UdsAddress, UdsPathAddress, UdsUnnamedAddress, host as host_net,
};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use crate::platform::net::{
    PACKET_BACKEND_CAP_CAPTURE, PACKET_BACKEND_CAP_FILTER, PACKET_BACKEND_CAP_SEND,
};
#[cfg(target_os = "linux")]
use crate::platform::net::{PACKET_BACKEND_CAP_FANOUT, PACKET_BACKEND_CAP_RING};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeArray, PlatformError};

use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef};

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
///
/// Bind an existing socket descriptor to the specified local address.
/// Address validation and reuse checks are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses bind(2) on Unix and bind on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.listen`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_bind(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_bind(binding, handle, address) }
}

/// Connect to a remote socket address.
///
/// Connect the socket to a specific remote endpoint.
/// Handshake progress and connection failure conditions follow host kernel connect semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses connect(2) on Unix and connect/WSAConnect on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.connect`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_connect(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_connect_raw(binding, handle, address) }
}

/// Read full TCP keepalive parameters.
///
/// Read host keepalive configuration fields for this TCP socket.
/// Field units and defaults follow platform TCP stack semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_KEEPALIVE and TCP_KEEP*) on Unix and WSAIoctl on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_keep_alive(
    binding: &BindingCallContext,
    out: *mut KeepAliveConfig,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_keep_alive(binding, out, handle) }
}

/// Read TCP_NODELAY.
///
/// Read the current TCP_NODELAY setting from the host TCP option layer.
/// The returned boolean reflects whether Nagle aggregation is disabled for this socket.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(TCP_NODELAY) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_no_delay(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_no_delay(binding, out, handle) }
}

/// Read SO_REUSEADDR.
///
/// Read the current SO_REUSEADDR value from the host socket option layer.
/// The returned boolean reflects host option state at the instant of the call.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_REUSEADDR) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_reuse_addr(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_reuse_addr(binding, out, handle) }
}

/// Read SO_REUSEPORT.
///
/// Read the current SO_REUSEPORT value from the host socket option layer.
/// The returned boolean reflects host option state at the instant of the call.
///
/// # Platform
/// Unix only. This operation returns `notSupported` on Windows.
/// Uses getsockopt(SO_REUSEPORT) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_reuse_port(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_reuse_port(binding, out, handle) }
}

/// Read socket linger settings.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_LINGER) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_linger(
    binding: &BindingCallContext,
    out: *mut Linger,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_linger(binding, out, handle) }
}

/// Read the receive buffer size.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_RCVBUF) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_recv_buffer(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_recv_buffer(binding, out, handle) }
}

/// Read the send buffer size.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_SNDBUF) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_send_buffer(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_send_buffer(binding, out, handle) }
}

/// Read broadcast mode.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_BROADCAST) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_broadcast(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_broadcast(binding, out, handle) }
}

/// Read the IP time-to-live.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IP_TTL/IPV6_UNICAST_HOPS) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_ttl(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_ttl(binding, out, handle) }
}

/// Read the IP type-of-service field.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IP_TOS/IPV6_TCLASS) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_tos(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_tos(binding, out, handle) }
}

/// Read the read timeout in milliseconds.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_RCVTIMEO) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_read_timeout(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_read_timeout(binding, out, handle) }
}

/// Read the write timeout in milliseconds.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_SNDTIMEO) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_write_timeout(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_write_timeout(binding, out, handle) }
}

/// Read IPv6-only mode.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IPV6_V6ONLY) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_only_v6(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_only_v6(binding, out, handle) }
}

/// Start listening on a raw socket address.
///
/// Create, bind, and place a listener socket into passive accept mode.
/// Backlog limits and bind conflicts are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses bind(2)+listen(2) on Unix and bind+listen on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.listen`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_listen(
    binding: &BindingCallContext,
    out: *mut ListenerHandle,
    address: SocketAddress,
    backlog: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_listen_raw(binding, out, address, backlog) }
}

/// Read the local socket address as raw bytes.
///
/// Query the local socket endpoint currently bound to this handle.
/// The returned `length` and `bytes` preserve the host sockaddr layout for the active family.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockname(2) on Unix and getsockname on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_local_address(
    binding: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_local_address_raw(binding, out, handle) }
}

/// Read the remote socket address as raw bytes.
///
/// Query the remote peer endpoint currently associated with this handle.
/// The returned `length` and `bytes` preserve the host sockaddr layout for the active family.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getpeername(2) on Unix and getpeername on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_peer_address(
    binding: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_peer_address_raw(binding, out, handle) }
}

/// Receive a packet from a remote socket address.
///
/// Receive one datagram and source address from a datagram socket.
/// Source address decoding and flag reporting follow host kernel recvfrom semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses recvfrom(2) on Unix and recvfrom on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.udp`.
///
/// # Replay
/// External, recordable.
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
///
/// Resolve the requested host and service to one or more socket addresses.
/// Name-service order, search domains, and canonicalization follow host resolver policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getaddrinfo(3) on Unix and GetAddrInfoW on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.dns`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_resolve(
    binding: &BindingCallContext,
    out: *mut NativeArray<SocketAddress>,
    query: ResolveQuery,
) -> RuntimeResult<()> {
    // validate query components
    let host = resolve_host(binding, query)?;
    let port = resolve_port(query)?;

    // delegate to the raw resolver
    unsafe {
        host_net::destack_net_resolve_raw(binding, out, host, port, query.family, query.flags)
    }
}

/// Reverse lookup a raw socket address into host and service names.
///
/// Resolve a socket address back to host and service names.
/// Reverse lookup policy and name formatting follow host resolver behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getnameinfo(3) on Unix and GetNameInfoW on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.dns`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_reverse_lookup(
    binding: &BindingCallContext,
    out: *mut NativeArray<ReverseLookupName>,
    address: SocketAddress,
    flags: ReverseLookupFlags,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_reverse_lookup_names_raw(binding, out, address, flags) }
}

/// Send a packet to a remote socket address.
///
/// Send a packet to a remote socket address via host kernel APIs.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses sendto(2) on Unix and sendto on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.udp`.
///
/// # Replay
/// External, recordable.
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
///
/// Set host keepalive configuration fields for this TCP socket.
/// Unsupported subfields are returned as `notSupported` rather than silently ignored.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_KEEPALIVE and TCP_KEEP*) on Unix and WSAIoctl on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
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
///
/// Set IPV6_V6ONLY on the target socket.
/// Dual-stack behavior follows host kernel policy after this option is applied.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IPV6_V6ONLY) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_only_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_only_v6(binding, handle, enabled) }
}

/// Create a socket from a native family, type, and protocol.
///
/// Allocate a new socket endpoint with the requested family, type, and protocol number.
/// Protocol defaults and socket limits are determined by the host kernel.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses socket(2) on Unix and WSASocketW on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
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
///
/// Allocate two already-connected peer sockets for local full-duplex communication.
/// Pair creation semantics and descriptor inheritance follow host kernel behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses socketpair(2) on Unix and loopback-pair emulation on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.connect`.
///
/// # Replay
/// External, recordable.
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
///
/// Bind the socket to a local address and port.
/// Address ownership and port conflict checks are enforced by the host kernel.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses bind(2) on Unix and bind on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.udp`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_udp_bind(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_udp_bind_raw(binding, handle, address) }
}

/// Connect a UDP socket to a raw remote address.
///
/// Set a default peer for datagrams sent on this socket.
/// Peer filtering and async error delivery follow host connected-UDP semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses connect(2) on Unix and connect on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.udp`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_udp_connect(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_udp_connect_raw(binding, handle, address) }
}

/// Receive a datagram from a remote address with raw address output.
///
/// Receive one UDP datagram and return source address plus recv flags.
/// Datagrams larger than the destination buffer follow host truncation semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses recvfrom(2) on Unix and recvfrom on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.udp`.
///
/// # Replay
/// External, recordable.
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
///
/// Send one UDP datagram to the supplied destination address.
/// Packet truncation, MTU checks, and route lookup follow host kernel UDP semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses sendto(2) on Unix and sendto on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.udp`.
///
/// # Replay
/// External, recordable.
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
///
/// Receive a message with ancillary data via host kernel APIs.
/// Caller controls descriptor and control payload extraction limits through `maxFds` and `maxControlBytes`.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses recvmsg(2) on Unix and WSARecvMsg on Windows where available.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
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
///
/// Receive multiple datagrams via host kernel APIs with per-message metadata and ancillary extraction.
/// Caller controls descriptor and control payload extraction limits through `maxFds` and `maxControlBytes`.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses recvmmsg(2) on linux and runtime loop fallback on other targets.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.udp`.
///
/// # Replay
/// External, recordable.
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
///
/// Send multiple datagrams via host kernel APIs with per-message metadata and address control.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses sendmmsg(2) on linux and runtime loop fallback on other targets.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.udp`.
///
/// # Replay
/// External, recordable.
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
///
/// Connect to an existing AF_UNIX endpoint address.
/// Address kind and endpoint type validation follow host AF_UNIX semantics.
///
/// # Platform
/// Unix and Windows only when AF_UNIX support is available at runtime.
/// Uses connect(2) on AF_UNIX sockets on Unix and AF_UNIX connect on Windows where available.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.connect`.
///
/// # Replay
/// External, recordable.
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
///
/// Place the bound socket into passive listen mode with the requested backlog semantics.
/// Path addresses are forwarded from `OsPath` without runtime normalization, while abstract and unnamed addresses follow host AF_UNIX rules.
///
/// # Platform
/// Unix and Windows only when AF_UNIX support is available at runtime.
/// Uses bind(2)+listen(2) on AF_UNIX sockets on Unix and AF_UNIX listen on Windows where available.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.listen`.
///
/// # Replay
/// External, recordable.
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
///
/// Allocate a connected AF_UNIX socket pair for local full-duplex messaging.
/// Pair semantics and descriptor inheritance follow host kernel behavior.
///
/// # Platform
/// Unix and Windows only when AF_UNIX support is available at runtime.
/// Uses socketpair(AF_UNIX) on Unix and runtime emulation on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.connect`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_uds_socket_pair(
    binding: &BindingCallContext,
    out: *mut SocketPair,
    sockettype: SocketType,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_uds_socket_pair(binding, out, sockettype) }
}

/// Resolve a host component from a resolve query.
fn resolve_host(
    _binding: &BindingCallContext,
    query: ResolveQuery,
) -> RuntimeResult<NativeStringRef> {
    let Some(host) = query.host else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "query",
            "host is required",
        ))
        .boxed());
    };

    Ok(host)
}

/// Resolve a service component from a resolve query.
fn resolve_port(query: ResolveQuery) -> RuntimeResult<u16> {
    let Some(service) = query.service else {
        return Ok(0);
    };

    let service = unsafe { service.as_str()? };
    service.parse::<u16>().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "query",
            "service must be a numeric port",
        ))
        .boxed()
    })
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
            Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsConnect")).boxed())
        }
    }
}

/// Read socket packet mark.
///
/// Read packet mark metadata from one socket endpoint.
/// Mark value interpretation is host-network-stack specific.
///
/// # Platform
/// Unix only.
/// Uses SO_MARK on Linux and returns `notSupported` on Unix targets without socket-mark support.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_packet_mark(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_packet_mark(binding, out, handle) }
}

/// Read one raw socket option payload.
///
/// Read one host socket option using raw level and name.
/// The returned byte payload is host-defined and must be decoded by the caller.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
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
///
/// Read timestamping controls from one socket endpoint.
/// Returned mode is normalized across host option variants.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses SO_TIMESTAMP families on Unix and host timestamping controls on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_timestamping(
    binding: &BindingCallContext,
    out: *mut SocketTimestampingMode,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_timestamping(binding, out, handle) }
}

/// Resolve an interface name to an index.
///
/// Maps a host interface name to its numeric index for route and multicast operations.
/// The mapping follows host network namespace rules.
///
/// # Platform
/// Unix and Windows.
/// Uses if_nametoindex on Unix and ConvertInterfaceAliasToLuid plus ConvertInterfaceLuidToIndex on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.interface`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_interface_index(
    binding: &BindingCallContext,
    out: *mut u32,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_interface_index(binding, out, name) }
}

/// Resolve an interface index to a name.
///
/// Maps a numeric host interface index to its canonical interface name.
/// The mapping follows host network namespace rules.
///
/// # Platform
/// Unix and Windows.
/// Uses if_indextoname on Unix and ConvertInterfaceIndexToLuid plus ConvertInterfaceLuidToAlias on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.interface`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_interface_name(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    index: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_interface_name(binding, out, index) }
}

/// List network interfaces with addresses and flags.
///
/// Enumerates host interfaces and returns their current address records.
/// Results are snapshots and may become stale immediately after the call.
///
/// # Platform
/// Unix and Windows.
/// Uses getifaddrs on Unix and iphlpapi adapter enumeration on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.interface`.
///
/// # Replay
/// External, recordable.
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
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let mut descriptors = Vec::new();

    #[cfg(target_os = "linux")]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::AfPacket,
            name: binding.store_string("af_packet"),
            available: true,
            priority: 100,
            capability_flags: PacketBackendCapabilityFlags(
                PACKET_BACKEND_CAP_CAPTURE.0
                    | PACKET_BACKEND_CAP_SEND.0
                    | PACKET_BACKEND_CAP_TIMESTAMP.0
                    | PACKET_BACKEND_CAP_FILTER.0
                    | PACKET_BACKEND_CAP_FANOUT.0
                    | PACKET_BACKEND_CAP_RING.0,
            ),
        });
    }

    #[cfg(target_os = "macos")]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::Bpf,
            name: binding.store_string("bpf"),
            available: true,
            priority: 100,
            capability_flags: PacketBackendCapabilityFlags(
                PACKET_BACKEND_CAP_CAPTURE.0
                    | PACKET_BACKEND_CAP_SEND.0
                    | PACKET_BACKEND_CAP_TIMESTAMP.0
                    | PACKET_BACKEND_CAP_FILTER.0,
            ),
        });
    }

    #[cfg(target_os = "windows")]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::WinRawSocket,
            name: binding.store_string("win_raw_socket"),
            available: true,
            priority: 100,
            capability_flags: PacketBackendCapabilityFlags(
                PACKET_BACKEND_CAP_CAPTURE.0
                    | PACKET_BACKEND_CAP_SEND.0
                    | PACKET_BACKEND_CAP_FILTER.0,
            ),
        });
    }

    #[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::Null,
            name: binding.store_string("null"),
            available: false,
            priority: 0,
            capability_flags: PacketBackendCapabilityFlags(0),
        });
    }

    #[cfg(not(any(unix, windows)))]
    {
        descriptors.push(PacketBackendDescriptor {
            backend: PacketBackend::Null,
            name: binding.store_string("null"),
            available: false,
            priority: 0,
            capability_flags: PacketBackendCapabilityFlags(0),
        });
    }

    unsafe {
        *out = binding.store_slice(descriptors);
    }

    Ok(())
}

/// Open a packet capture or inject endpoint.
///
/// Opens one host packet endpoint for packet capture and injection.
/// Frame shape and metadata are backend specific.
/// Host privilege checks and backend-specific limits are enforced by the kernel or driver.
///
/// # Platform
/// Unix and Windows.
/// Uses AF_PACKET on Linux and `/dev/bpf` packet devices on macOS.
/// Returns `notSupported` on Unix targets without a packet backend.
/// Uses one configured host packet backend on Windows.
/// Current Windows backend uses raw IPv4 sockets with `SIO_RCVALL`, payloads are IP packets rather than Ethernet frames.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_open(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    options: PacketCaptureOptions,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_open(binding, out, options) }
}

/// Receive one packet from a packet endpoint.
///
/// Reads one packet record into the provided payload buffer and returns packet metadata.
/// Truncation is reported explicitly when the payload buffer is smaller than the captured frame.
///
/// # Platform
/// Unix and Windows.
/// Uses AF_PACKET packet reads on Linux and BPF packet reads on macOS.
/// Returns `notSupported` on Unix targets without a packet backend.
/// Uses one configured host packet backend on Windows.
/// Current Windows backend reads raw IPv4 packets from `SOCK_RAW` capture lanes.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_receive(
    binding: &BindingCallContext,
    out: *mut PacketCaptureRecord,
    handle: SocketHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_receive(binding, out, handle, payload) }
}

/// Send one packet through a packet endpoint.
///
/// Writes one raw packet frame from the provided payload buffer.
/// Partial sends are reported through the returned byte count.
///
/// # Platform
/// Unix and Windows.
/// Uses AF_PACKET packet writes on Linux and BPF packet writes on macOS.
/// Returns `notSupported` on Unix targets without a packet backend.
/// Uses one configured host packet backend on Windows.
/// Current Windows backend sends raw IPv4 packets through `SOCK_RAW`.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_send(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_send(binding, out, handle, payload) }
}

/// Configure packet timestamp mode for a socket or packet endpoint.
///
/// Updates timestamping mode for packet metadata capture on supported backends.
/// Unsupported timestamp modes return notSupported.
///
/// # Platform
/// Unix and Windows.
/// Uses SO_TIMESTAMP families on Linux and BPF timestamp lanes on macOS.
/// Returns `notSupported` on Unix targets without timestamp-capable packet backends.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_set_timestamp_mode(
    binding: &BindingCallContext,
    handle: SocketHandle,
    mode: PacketTimestampMode,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_timestamp_mode(binding, handle, mode) }
}

/// Clear packet fanout from a packet endpoint.
///
/// Remove this endpoint from any active fanout group.
/// Group teardown behavior and packet redistribution follow host kernel semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_FANOUT reset on Linux and returns `notSupported` where fanout groups are unavailable.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_clear_fanout(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_clear_fanout(binding, handle) }
}

/// Clear the active packet filter program.
///
/// Removes any backend packet filter from the raw endpoint.
/// Filter teardown semantics are host defined.
///
/// # Platform
/// Unix and Windows.
/// Uses SO_DETACH_FILTER on Linux and BIOCSETF reset on macOS.
/// Returns `notSupported` on Unix targets without packet-filter backends.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_clear_filter(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_clear_filter(binding, handle) }
}

/// Clear packet rx and tx ring configuration.
///
/// Disable ring-backed packet queues and return to syscall-based send and receive.
/// Pending ring buffers are released according to host packet socket semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_RX_RING and PACKET_TX_RING reset on Linux and returns `notSupported` elsewhere.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_clear_ring(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_clear_ring(binding, handle) }
}

/// Set packet fanout on a packet endpoint.
///
/// Attach this endpoint to one kernel packet fanout group with the provided mode.
/// Fanout group behavior and mode-specific flags follow host packet socket semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_FANOUT on Linux and returns `notSupported` where fanout groups are unavailable.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_set_fanout(
    binding: &BindingCallContext,
    handle: SocketHandle,
    options: PacketFanoutOptions,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_fanout(binding, handle, options) }
}

/// Attach one packet filter program to a raw endpoint.
///
/// Installs one backend packet filter program for capture path filtering.
/// Filter verification and accepted instruction sets are host defined.
///
/// # Platform
/// Unix and Windows.
/// Uses SO_ATTACH_FILTER on Linux and BIOCSETF on macOS.
/// Returns `notSupported` on Unix targets without packet-filter backends.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_set_filter(
    binding: &BindingCallContext,
    handle: SocketHandle,
    filterprogram: NativeSlice<u8>,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_filter(binding, handle, filterprogram) }
}

/// Configure one packet rx ring for zero-copy capture.
///
/// Configure one receive ring so packet frames are delivered through kernel ring buffers.
/// Ring geometry is validated by the host kernel and may be clamped or rejected.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_RX_RING on Linux and returns `notSupported` where packet rings are unavailable.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_set_rx_ring(
    binding: &BindingCallContext,
    handle: SocketHandle,
    options: PacketRingOptions,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_rx_ring(binding, handle, options) }
}

/// Configure one packet tx ring for zero-copy transmit.
///
/// Configure one transmit ring so packet frames are queued through kernel ring buffers.
/// Ring geometry is validated by the host kernel and may be clamped or rejected.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_TX_RING on Linux and returns `notSupported` where packet rings are unavailable.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_set_tx_ring(
    binding: &BindingCallContext,
    handle: SocketHandle,
    options: PacketRingOptions,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_tx_ring(binding, handle, options) }
}

/// Read packet capture statistics from one endpoint.
///
/// Reads cumulative backend packet counters for the endpoint.
/// Counter units and reset behavior follow host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses packet socket stats on Linux and BPF stats on macOS.
/// Returns `notSupported` on Unix targets without packet stats backends.
/// Uses one configured host packet backend on Windows.
/// Returns `notSupported` on Windows when no packet backend is configured.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_packet_stats(
    binding: &BindingCallContext,
    out: *mut PacketCaptureStats,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_stats(binding, out, handle) }
}

/// Read the default IPv4 multicast interface for one socket.
///
/// Read the local interface address used for outgoing IPv4 multicast datagrams.
/// Returned address follows host socket option encoding rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IP_MULTICAST_IF) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_multicast_interface_v4(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_multicast_interface_v4(binding, out, handle) }
}

/// Read the default IPv6 multicast interface for one socket.
///
/// Read the local interface index used for outgoing IPv6 multicast datagrams.
/// Returned index follows host socket option encoding rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IPV6_MULTICAST_IF) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_multicast_interface_v6(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_multicast_interface_v6(binding, out, handle) }
}

/// Select the default IPv4 multicast interface for one socket.
///
/// Set the local interface used for outgoing IPv4 multicast datagrams.
/// Interface selection follows host route and socket option semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_MULTICAST_IF) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_multicast_interface_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    interfaceaddress: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_multicast_interface_v4(binding, handle, interfaceaddress) }
}

/// Select the default IPv6 multicast interface for one socket.
///
/// Set the local interface index used for outgoing IPv6 multicast datagrams.
/// Interface selection follows host route and socket option semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IPV6_MULTICAST_IF) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_multicast_interface_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    interfaceindex: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_multicast_interface_v6(binding, handle, interfaceindex) }
}

/// Read multicast loopback mode.
///
/// Read whether outgoing multicast packets are looped back to local receivers.
/// Returned state reflects host socket-option state at call time.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IP_MULTICAST_LOOP/IPV6_MULTICAST_LOOP) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_multicast_loop(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_multicast_loop(binding, out, handle) }
}

/// Read multicast TTL or hop-limit.
///
/// Read the active multicast TTL or IPv6 hop-limit used for outgoing datagrams.
/// Returned value follows host socket-option interpretation for the active family.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IP_MULTICAST_TTL/IPV6_MULTICAST_HOPS) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_multicast_ttl(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_get_multicast_ttl(binding, out, handle) }
}

/// Join one IPv4 source-specific multicast membership.
///
/// Join one IGMPv3 source-specific membership for the given group and source.
/// Membership installation is host scoped and can be rejected by kernel policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_ADD_SOURCE_MEMBERSHIP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_join_multicast_source_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV4,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_join_multicast_source_v4(binding, handle, membership) }
}

/// Join one IPv6 source-specific multicast membership.
///
/// Join one source-filtered IPv6 multicast membership for the given group and source.
/// Membership installation is host scoped and can be rejected by kernel policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses MCAST_JOIN_SOURCE_GROUP family socket options on Unix and equivalent host APIs on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_join_multicast_source_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV6,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_join_multicast_source_v6(binding, handle, membership) }
}

/// Leave one IPv4 source-specific multicast membership.
///
/// Leave one IGMPv3 source-specific membership for the given group and source.
/// Membership removal is host scoped and can be rejected by kernel policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_DROP_SOURCE_MEMBERSHIP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_leave_multicast_source_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV4,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_leave_multicast_source_v4(binding, handle, membership) }
}

/// Leave one IPv6 source-specific multicast membership.
///
/// Leave one source-filtered IPv6 multicast membership for the given group and source.
/// Membership removal is host scoped and can be rejected by kernel policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses MCAST_LEAVE_SOURCE_GROUP family socket options on Unix and equivalent host APIs on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_leave_multicast_source_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV6,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_leave_multicast_source_v6(binding, handle, membership) }
}

/// Enable or disable IP header inclusion on a raw socket.
///
/// Updates the raw socket header include mode.
/// Caller is responsible for writing valid protocol headers when enabled.
///
/// # Platform
/// Unix and Windows.
/// Uses setsockopt(IP_HDRINCL) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_raw_set_header_included(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_raw_set_header_included(binding, handle, enabled) }
}

/// Open a raw IP socket.
///
/// Creates a raw socket endpoint for protocol-level packet control.
/// Host privilege checks and protocol restrictions are enforced by the kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses socket(AF_INET/AF_INET6, SOCK_RAW) on Unix and WSASocketW raw mode on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_raw_socket(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
    protocol: i32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_raw_socket(binding, out, family, protocol) }
}

/// Add a route table entry.
///
/// Requests host route table insertion for the supplied entry.
/// Host privilege and policy checks are enforced by the kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses netlink route mutation on Linux and route sockets on macOS.
/// Returns `notSupported` on Unix targets without a route backend.
/// Uses iphlpapi route mutation APIs on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.route.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_route_add(
    binding: &BindingCallContext,
    route: RouteEntry,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_route_add(binding, route) }
}

/// Remove a route table entry.
///
/// Requests host route table deletion for the supplied entry.
/// Host privilege and policy checks are enforced by the kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses netlink route mutation on Linux and route sockets on macOS.
/// Returns `notSupported` on Unix targets without a route backend.
/// Uses iphlpapi route mutation APIs on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.route.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_route_delete(
    binding: &BindingCallContext,
    route: RouteEntry,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_route_delete(binding, route) }
}

/// List route table entries.
///
/// Reads the host route table and returns route entries for the selected family.
/// Results are snapshots and may become stale immediately after the call.
///
/// # Platform
/// Unix and Windows.
/// Uses netlink route tables on Linux and route sockets on macOS.
/// Returns `notSupported` on Unix targets without a route backend.
/// Uses iphlpapi route tables on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.route.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_route_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<RouteEntry>,
    family: SocketFamily,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_route_list(binding, out, family) }
}

/// Set socket packet mark.
///
/// Set packet mark metadata used by host routing and firewall policy.
/// Mark interpretation is host-network-stack specific.
///
/// # Platform
/// Unix only.
/// Uses SO_MARK on Linux and returns `notSupported` on Unix targets without socket-mark support.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_packet_mark(
    binding: &BindingCallContext,
    handle: SocketHandle,
    mark: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_packet_mark(binding, handle, mark) }
}

/// Set one raw socket option payload.
///
/// Set one host socket option using raw level, name, and byte payload.
/// This escape hatch covers options that do not yet have typed bindings.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
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
///
/// Configure timestamping controls on one socket endpoint.
/// Timestamp delivery channel and precision follow host kernel capabilities.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses SO_TIMESTAMP families on Unix and host timestamping controls on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_timestamping(
    binding: &BindingCallContext,
    handle: SocketHandle,
    mode: SocketTimestampingMode,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_timestamping(binding, handle, mode) }
}
