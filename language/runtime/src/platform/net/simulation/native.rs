#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::BindingCallContext;

use crate::platform::net::{
    AcceptFlags, KeepAliveConfig, Linger, NetInterface, PacketCaptureOptions, PacketCaptureRecord,
    PacketCaptureStats, PacketFanoutOptions, PacketRingOptions, PacketTimestampMode, ResolveQuery,
    ReverseLookupFlags, ReverseLookupName, RouteEntry, SocketAddress, SocketFamily,
    SocketMessageFlags, SocketOptionLevel, SocketOptionName, SocketPair, SocketProtocol,
    SocketRecvBatchRequest, SocketRecvFrom, SocketRecvMessage, SocketSendBatchEntry,
    SocketSendMessage, SocketSendTo, SocketShutdown, SocketTimestampingMode, SocketType,
    UdpMessageFlags, UdpReceive, UdpSourceMembershipV4, UdpSourceMembershipV6, UdsAddress,
};
use crate::platform::{fs, resource};

/// Accept a new connection from a listener.
///
/// Accept the next pending connection from the listener queue.
/// Accept flags and queued-connection ordering follow host kernel semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses accept4/accept on Unix and accept/AcceptEx on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.accept`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_accept(
    _context: &BindingCallContext,
    out: *mut resource::SocketHandle,
    listener: resource::ListenerHandle,
    flags: AcceptFlags,
) -> RuntimeResult<()> {
    let _ = (out, listener, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.accept")).boxed())
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
    handle: resource::SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (handle, address);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.bind")).boxed())
}

/// Close a socket handle.
///
/// Close the target socket descriptor.
/// Close-on-pending-I/O behavior follows host kernel socket semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses close(2) on Unix and closesocket on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.close`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_close(
    _context: &BindingCallContext,
    _handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.close")).boxed())
}

/// Close a listener handle.
///
/// Close a listener socket descriptor.
/// Pending accepts are interrupted according to host kernel semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses close(2) on Unix and closesocket on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.close`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_close_listener(
    _context: &BindingCallContext,
    _handle: resource::ListenerHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.closeListener")).boxed())
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
    _context: &BindingCallContext,
    handle: resource::SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (handle, address);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.connect")).boxed())
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
    _context: &BindingCallContext,
    _out: *mut NativeArray<NetInterface>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.interface.listInterfaces",
    ))
    .boxed())
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
    _context: &BindingCallContext,
    out: *mut u32,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (out, name);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.interfaceIndex")).boxed())
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
    _context: &BindingCallContext,
    out: *mut NativeStringRef,
    index: u32,
) -> RuntimeResult<()> {
    let _ = (out, index);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.interfaceName")).boxed())
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
    _context: &BindingCallContext,
    out: *mut resource::ListenerHandle,
    address: SocketAddress,
    backlog: u32,
) -> RuntimeResult<()> {
    let _ = (out, address, backlog);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.listen")).boxed())
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
///
/// Read packet mark metadata from one socket endpoint.
/// Mark value interpretation is host-network-stack specific.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses SO_MARK on Linux and host route-marking controls on Windows where available.
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
    out: *mut u32,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.options.getTos")).boxed())
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
    _context: &BindingCallContext,
    out: *mut u32,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.options.getTtl")).boxed())
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
    _context: &BindingCallContext,
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
///
/// Enable or disable broadcast via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_BROADCAST) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_broadcast(
    _context: &BindingCallContext,
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
///
/// Set SO_LINGER parameters on the target socket.
/// The host kernel validates and applies linger policy exactly once for this call.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_LINGER) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_linger(
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
///
/// Set packet mark metadata used by host routing and firewall policy.
/// Mark interpretation is host-network-stack specific.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses SO_MARK on Linux and host route-marking controls on Windows where available.
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
    _context: &BindingCallContext,
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
///
/// Set SO_RCVTIMEO on the target socket.
/// Timeout interpretation and rounding follow host kernel socket-timeout semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_RCVTIMEO) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_read_timeout(
    _context: &BindingCallContext,
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
///
/// Set SO_RCVBUF on the target socket.
/// The host kernel clamps requested values according to platform limits.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_RCVBUF) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_recv_buffer(
    _context: &BindingCallContext,
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
///
/// Set SO_SNDBUF on the target socket.
/// The host kernel clamps requested values according to platform limits.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_SNDBUF) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_send_buffer(
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
///
/// Set the requested control value on the descriptor through the native option interface.
/// The binding performs one control transaction and returns the exact host outcome without policy retries.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_TOS/IPV6_TCLASS) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_tos(
    _context: &BindingCallContext,
    handle: resource::SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    let _ = (handle, tos);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.options.setTos")).boxed())
}

/// Set the IP time-to-live.
///
/// Set the requested control value on the descriptor through the native option interface.
/// The binding performs one control transaction and returns the exact host outcome without policy retries.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_TTL/IPV6_UNICAST_HOPS) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_ttl(
    _context: &BindingCallContext,
    handle: resource::SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    let _ = (handle, ttl);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.options.setTtl")).boxed())
}

/// Set the write timeout in milliseconds.
///
/// Set SO_SNDTIMEO on the target socket.
/// Timeout interpretation and rounding follow host kernel socket-timeout semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_SNDTIMEO) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_write_timeout(
    _context: &BindingCallContext,
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
///
/// Remove this endpoint from any active fanout group.
/// Group teardown behavior and packet redistribution follow host kernel semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_FANOUT reset on Linux and returns notSupported where fanout groups are unavailable.
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
    _context: &BindingCallContext,
    _handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetClearFanout",
    ))
    .boxed())
}

/// Clear the active packet filter program.
///
/// Removes any backend packet filter from the raw endpoint.
/// Filter teardown semantics are host defined.
///
/// # Platform
/// Unix and Windows.
/// Uses SO_DETACH_FILTER or BPF detach APIs on Unix and equivalent packet filter APIs on Windows.
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
    _context: &BindingCallContext,
    _handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetClearFilter",
    ))
    .boxed())
}

/// Clear packet rx and tx ring configuration.
///
/// Disable ring-backed packet queues and return to syscall-based send and receive.
/// Pending ring buffers are released according to host packet socket semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_RX_RING and PACKET_TX_RING reset on Linux and returns notSupported elsewhere.
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
    _context: &BindingCallContext,
    _handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetClearRing",
    ))
    .boxed())
}

/// Open a packet capture or inject endpoint.
///
/// Opens a link-layer packet endpoint for packet capture and injection.
/// Host privilege checks and backend-specific limits are enforced by the kernel or driver.
///
/// # Platform
/// Unix and Windows.
/// Uses AF_PACKET on Linux, BPF devices on BSD, and packet capture drivers on Windows.
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
    _context: &BindingCallContext,
    out: *mut resource::SocketHandle,
    options: PacketCaptureOptions,
) -> RuntimeResult<()> {
    let _ = (out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.raw.packetOpen")).boxed())
}

/// Receive one packet from a packet endpoint.
///
/// Reads one packet record into the provided payload buffer and returns packet metadata.
/// Truncation is reported explicitly when the payload buffer is smaller than the captured frame.
///
/// # Platform
/// Unix and Windows.
/// Uses AF_PACKET or BPF packet reads on Unix and packet capture driver reads on Windows.
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
    _context: &BindingCallContext,
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
///
/// Writes one raw packet frame from the provided payload buffer.
/// Partial sends are reported through the returned byte count.
///
/// # Platform
/// Unix and Windows.
/// Uses AF_PACKET or BPF packet writes on Unix and packet injection driver writes on Windows.
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
    _context: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    argument_payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, argument_payload);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.raw.packetSend")).boxed())
}

/// Set packet fanout on a packet endpoint.
///
/// Attach this endpoint to one kernel packet fanout group with the provided mode.
/// Fanout group behavior and mode-specific flags follow host packet socket semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_FANOUT on Linux and returns notSupported where fanout groups are unavailable.
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
    _context: &BindingCallContext,
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
///
/// Installs one backend packet filter program for capture path filtering.
/// Filter verification and accepted instruction sets are host defined.
///
/// # Platform
/// Unix and Windows.
/// Uses SO_ATTACH_FILTER or BPF attach APIs on Unix and equivalent packet filter APIs on Windows.
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
    _context: &BindingCallContext,
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
///
/// Configure one receive ring so packet frames are delivered through kernel ring buffers.
/// Ring geometry is validated by the host kernel and may be clamped or rejected.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_RX_RING on Linux and returns notSupported where packet rings are unavailable.
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
    _context: &BindingCallContext,
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
///
/// Updates timestamping mode for packet metadata capture on supported backends.
/// Unsupported timestamp modes return notSupported.
///
/// # Platform
/// Unix and Windows.
/// Uses SO_TIMESTAMP families on Unix and socket timestamp controls on Windows where available.
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
    _context: &BindingCallContext,
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
///
/// Configure one transmit ring so packet frames are queued through kernel ring buffers.
/// Ring geometry is validated by the host kernel and may be clamped or rejected.
///
/// # Platform
/// Unix and Windows.
/// Uses PACKET_TX_RING on Linux and returns notSupported where packet rings are unavailable.
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
    _context: &BindingCallContext,
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
///
/// Reads cumulative backend packet counters for the endpoint.
/// Counter units and reset behavior follow host backend semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses packet socket stats on Linux, BPF stats on BSD, and equivalent packet backend stats on Windows.
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
    _context: &BindingCallContext,
    out: *mut PacketCaptureStats,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.raw.packetStats")).boxed())
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
    out: *mut resource::SocketHandle,
    family: SocketFamily,
    protocol: i32,
) -> RuntimeResult<()> {
    let _ = (out, family, protocol);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.raw.socket")).boxed())
}

/// Read from a socket into the provided slice.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses read(2)/recv(2) on Unix and recv on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.connect`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_read(
    _context: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.read")).boxed())
}

/// Read into multiple buffers.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses readv(2)/recvmsg(2) on Unix and WSARecv on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.connect`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_readv(
    _context: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.readv")).boxed())
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
    _context: &BindingCallContext,
    out: *mut SocketRecvFrom,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
    recvflags: SocketMessageFlags,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer, recvflags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.recvFrom")).boxed())
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
    out: *mut NativeArray<SocketAddress>,
    query: ResolveQuery,
) -> RuntimeResult<()> {
    let _ = (out, query);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.resolve.lookup")).boxed())
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
///
/// Enable or disable SO_REUSEADDR via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_REUSEADDR) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_reuse_addr(
    _context: &BindingCallContext,
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
///
/// Enable or disable SO_REUSEPORT via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix only. This operation returns `notSupported` on Windows.
/// Uses setsockopt(SO_REUSEPORT) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_reuse_port(
    _context: &BindingCallContext,
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
///
/// Requests host route table insertion for the supplied entry.
/// Host privilege and policy checks are enforced by the kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses netlink or routing sockets on Unix and iphlpapi route mutation APIs on Windows.
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
    _context: &BindingCallContext,
    _route: RouteEntry,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.routeAdd")).boxed())
}

/// Remove a route table entry.
///
/// Requests host route table deletion for the supplied entry.
/// Host privilege and policy checks are enforced by the kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses netlink or routing sockets on Unix and iphlpapi route mutation APIs on Windows.
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
    _context: &BindingCallContext,
    _route: RouteEntry,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.routeDelete")).boxed())
}

/// List route table entries.
///
/// Reads the host route table and returns route entries for the selected family.
/// Results are snapshots and may become stale immediately after the call.
///
/// # Platform
/// Unix and Windows.
/// Uses netlink or routing sockets on Unix and iphlpapi route tables on Windows.
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
    _context: &BindingCallContext,
    out: *mut NativeArray<RouteEntry>,
    family: SocketFamily,
) -> RuntimeResult<()> {
    let _ = (out, family);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.routeList")).boxed())
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
    _context: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    messages: NativeSlice<SocketSendBatchEntry>,
) -> RuntimeResult<()> {
    let _ = (out, handle, messages);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.sendMmsg")).boxed())
}

/// Send a message with ancillary data.
///
/// Send a message with ancillary data via host kernel APIs.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses sendmsg(2) on Unix and WSASendMsg on Windows where available.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_send_msg(
    _context: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
    message: SocketSendMessage,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer, message);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.sendMsg")).boxed())
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
    _context: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
    message: SocketSendTo,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer, message);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.sendTo")).boxed())
}

/// Enable or disable nonblocking mode on a socket.
///
/// Enable or disable nonblocking mode on a socket descriptor.
/// Subsequent I/O blocking behavior follows host kernel descriptor state.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses fcntl/ioctl on Unix and ioctlsocket on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_nonblocking(
    _context: &BindingCallContext,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setNonblocking")).boxed())
}

/// Shut down a socket for reads, writes, or both.
///
/// Shut down read, write, or both directions on a connected socket.
/// Half-close and peer-observed behavior follow host kernel shutdown semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses shutdown(2) on Unix and shutdown on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.close`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_shutdown(
    _context: &BindingCallContext,
    handle: resource::SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    let _ = (handle, how);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.shutdown")).boxed())
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
    _context: &BindingCallContext,
    out: *mut resource::SocketHandle,
    family: SocketFamily,
    sockettype: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    let _ = (out, family, sockettype, protocol);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.socket")).boxed())
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
    _context: &BindingCallContext,
    out: *mut SocketPair,
    family: SocketFamily,
    sockettype: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    let _ = (out, family, sockettype, protocol);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.socketPair")).boxed())
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
    _context: &BindingCallContext,
    out: *mut KeepAliveConfig,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.tcp.getKeepAlive")).boxed())
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
    _context: &BindingCallContext,
    out: *mut bool,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.tcp.getNoDelay")).boxed())
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
    _context: &BindingCallContext,
    handle: resource::SocketHandle,
    config: KeepAliveConfig,
) -> RuntimeResult<()> {
    let _ = (handle, config);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.tcp.setKeepAlive")).boxed())
}

/// Enable or disable TCP_NODELAY.
///
/// Set TCP_NODELAY on the target TCP socket.
/// Nagle aggregation behavior changes immediately according to host TCP stack semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(TCP_NODELAY) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_no_delay(
    _context: &BindingCallContext,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.tcp.setNoDelay")).boxed())
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
    _context: &BindingCallContext,
    handle: resource::SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (handle, address);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udp.bind")).boxed())
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
    _context: &BindingCallContext,
    handle: resource::SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (handle, address);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udp.connect")).boxed())
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
///
/// Join an IPv4 multicast membership on the selected interface.
/// Group membership tracking and validation follow host IGMP implementation rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_ADD_MEMBERSHIP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_join_multicast_v4(
    _context: &BindingCallContext,
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
///
/// Join an IPv6 multicast membership on the selected interface index.
/// Group membership tracking and validation follow host MLD implementation rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IPV6_JOIN_GROUP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_join_multicast_v6(
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
///
/// Leave an IPv4 multicast membership on the selected interface.
/// Group membership tracking and validation follow host IGMP implementation rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_DROP_MEMBERSHIP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_leave_multicast_v4(
    _context: &BindingCallContext,
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
///
/// Leave an IPv6 multicast membership on the selected interface index.
/// Group membership tracking and validation follow host MLD implementation rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IPV6_LEAVE_GROUP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_leave_multicast_v6(
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
    out: *mut UdpReceive,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
    recvflags: UdpMessageFlags,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer, recvflags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udp.recvFrom")).boxed())
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
    _context: &BindingCallContext,
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
///
/// Enable or disable local loopback delivery for outgoing multicast packets.
/// Loopback behavior follows host multicast socket-option semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_MULTICAST_LOOP/IPV6_MULTICAST_LOOP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_multicast_loop(
    _context: &BindingCallContext,
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
///
/// Set multicast TTL or hop-limit for outgoing datagrams.
/// Hop-limit interpretation follows host IP stack option semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_MULTICAST_TTL/IPV6_MULTICAST_HOPS) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_multicast_ttl(
    _context: &BindingCallContext,
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
///
/// Allocate a UDP datagram socket for the requested address family.
/// Datagram behavior and protocol defaults follow host UDP stack semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses socket(AF_INET/AF_INET6, SOCK_DGRAM) on Unix and WSASocketW on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.udp`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_udp_socket(
    _context: &BindingCallContext,
    out: *mut resource::SocketHandle,
    family: SocketFamily,
) -> RuntimeResult<()> {
    let _ = (out, family);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udp.socket")).boxed())
}

/// Accept a connection from a UDS listener.
///
/// Accept the next pending AF_UNIX stream connection.
/// Accept ordering and descriptor flags follow host kernel semantics.
///
/// # Platform
/// Unix and Windows only when AF_UNIX support is available at runtime.
/// Uses accept(2) on AF_UNIX sockets on Unix and accept on Windows where available.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.accept`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_uds_accept(
    _context: &BindingCallContext,
    out: *mut resource::SocketHandle,
    listener: resource::ListenerHandle,
) -> RuntimeResult<()> {
    let _ = (out, listener);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsAccept")).boxed())
}

/// Close a UDS listener handle.
///
/// Close an AF_UNIX listener socket descriptor.
/// Pending accepts are interrupted according to host kernel semantics.
///
/// # Platform
/// Unix and Windows only when AF_UNIX support is available at runtime.
/// Uses close(2) on Unix and closesocket on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.close`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_uds_close_listener(
    _context: &BindingCallContext,
    _handle: resource::ListenerHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsCloseListener")).boxed())
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
    _context: &BindingCallContext,
    out: *mut resource::SocketHandle,
    address: UdsAddress,
) -> RuntimeResult<()> {
    let _ = (out, address);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsConnect")).boxed())
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
    _context: &BindingCallContext,
    out: *mut resource::ListenerHandle,
    address: UdsAddress,
    backlog: u32,
) -> RuntimeResult<()> {
    let _ = (out, address, backlog);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsListen")).boxed())
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
    _context: &BindingCallContext,
    out: *mut SocketPair,
    sockettype: SocketType,
) -> RuntimeResult<()> {
    let _ = (out, sockettype);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsSocketPair")).boxed())
}

/// Write to a socket from the provided slice.
///
/// Write data directly from caller provided buffers to the target descriptor using native transfer semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses write(2)/send(2) on Unix and send on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.connect`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_write(
    _context: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.write")).boxed())
}

/// Write from multiple buffers.
///
/// Write data directly from caller provided buffers to the target descriptor using native transfer semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses writev(2)/sendmsg(2) on Unix and WSASend on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.connect`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_writev(
    _context: &BindingCallContext,
    out: *mut u64,
    handle: resource::SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers);

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.writev")).boxed())
}
