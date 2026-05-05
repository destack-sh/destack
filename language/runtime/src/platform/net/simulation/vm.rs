#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{
    AcceptFlags, KeepAliveConfigVm, LingerVm, NetInterfaceVm, PacketBackendDescriptorVm,
    PacketCaptureOptionsVm, PacketCaptureRecordVm, PacketCaptureStatsVm, PacketFanoutOptionsVm,
    PacketRingOptionsVm, PacketTimestampMode, ResolveQueryVm, ReverseLookupFlags,
    ReverseLookupNameVm, RouteEntryVm, SocketAddressVm, SocketFamily, SocketMessageFlags,
    SocketOptionLevel, SocketOptionName, SocketPairVm, SocketProtocol, SocketRecvBatchRequestVm,
    SocketRecvFromVm, SocketRecvMessageVm, SocketSendBatchEntryVm, SocketSendMessageVm,
    SocketSendToVm, SocketShutdown, SocketTimestampingMode, SocketType, UdpMessageFlags,
    UdpReceiveVm, UdpSourceMembershipV4Vm, UdpSourceMembershipV6Vm, UdsAddressVm,
};
use crate::platform::{PlatformError, VmArray, VmSlice, fs, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// List host packet backends.
pub(crate) fn destack_net_packet_backend_list(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<PacketBackendDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetBackendList",
    ))
    .boxed())
}

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
pub(crate) fn destack_net_accept(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    listener: resource::ListenerHandle,
    flags: AcceptFlags,
) -> RuntimeResult<resource::SocketHandle> {
    let _ = (listener, flags);
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
pub(crate) fn destack_net_local_address(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
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
pub(crate) fn destack_net_peer_address(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
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
pub(crate) fn destack_net_bind(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    address: SocketAddressVm,
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
pub(crate) fn destack_net_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_close_listener(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_connect(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    address: SocketAddressVm,
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
pub(crate) fn destack_net_list_interfaces(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmArray<NetInterfaceVm>> {
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
pub(crate) fn destack_net_interface_index(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _name: vm::StringHandle,
) -> RuntimeResult<u32> {
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
pub(crate) fn destack_net_interface_name(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _index: u32,
) -> RuntimeResult<vm::StringHandle> {
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
pub(crate) fn destack_net_listen(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: SocketAddressVm,
    backlog: u32,
) -> RuntimeResult<resource::ListenerHandle> {
    let _ = (address, backlog);
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
pub(crate) fn destack_net_get_broadcast(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<bool> {
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
pub(crate) fn destack_net_get_linger(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<LingerVm> {
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
pub(crate) fn destack_net_get_only_v6(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<bool> {
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
pub(crate) fn destack_net_get_packet_mark(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
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
pub(crate) fn destack_net_get_read_timeout(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
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
pub(crate) fn destack_net_get_recv_buffer(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
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
pub(crate) fn destack_net_get_send_buffer(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
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
pub(crate) fn destack_net_get_sock_opt_raw(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    maxbytes: u32,
) -> RuntimeResult<VmArray<u8>> {
    let _ = (handle, level, name, maxbytes);
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
pub(crate) fn destack_net_get_timestamping(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<SocketTimestampingMode> {
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
pub(crate) fn destack_net_get_tos(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
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
pub(crate) fn destack_net_get_ttl(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
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
pub(crate) fn destack_net_get_write_timeout(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
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
pub(crate) fn destack_net_set_broadcast(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_set_linger(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    linger: LingerVm,
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
pub(crate) fn destack_net_set_only_v6(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_set_packet_mark(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_set_read_timeout(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_set_recv_buffer(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_set_send_buffer(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_set_sock_opt_raw(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    argument_value: VmSlice<u8>,
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
pub(crate) fn destack_net_set_timestamping(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_set_tos(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_set_ttl(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_set_write_timeout(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_packet_clear_fanout(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_packet_clear_filter(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_packet_clear_ring(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.raw.packetClearRing",
    ))
    .boxed())
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
pub(crate) fn destack_net_packet_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _options: PacketCaptureOptionsVm,
) -> RuntimeResult<resource::SocketHandle> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.raw.packetOpen")).boxed())
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
pub(crate) fn destack_net_packet_receive(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<PacketCaptureRecordVm> {
    let _ = (handle, argument_payload);
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
pub(crate) fn destack_net_packet_send(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    argument_payload: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, argument_payload);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.raw.packetSend")).boxed())
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
pub(crate) fn destack_net_packet_set_fanout(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    options: PacketFanoutOptionsVm,
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
pub(crate) fn destack_net_packet_set_filter(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    filterprogram: VmSlice<u8>,
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
pub(crate) fn destack_net_packet_set_rx_ring(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    options: PacketRingOptionsVm,
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
pub(crate) fn destack_net_packet_set_timestamp_mode(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_packet_set_tx_ring(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    options: PacketRingOptionsVm,
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
pub(crate) fn destack_net_packet_stats(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<PacketCaptureStatsVm> {
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
pub(crate) fn destack_net_raw_set_header_included(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_raw_socket(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    family: SocketFamily,
    protocol: i32,
) -> RuntimeResult<resource::SocketHandle> {
    let _ = (family, protocol);
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
pub(crate) fn destack_net_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
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
pub(crate) fn destack_net_readv(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers);
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
pub(crate) fn destack_net_recv_from(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
    recvflags: SocketMessageFlags,
) -> RuntimeResult<SocketRecvFromVm> {
    let _ = (handle, buffer, recvflags);
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
pub(crate) fn destack_net_recv_mmsg(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    requests: VmSlice<SocketRecvBatchRequestVm>,
    maxfds: u32,
    wantcredentials: bool,
    maxcontrolbytes: u32,
) -> RuntimeResult<VmArray<SocketRecvMessageVm>> {
    let _ = (handle, requests, maxfds, wantcredentials, maxcontrolbytes);
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
pub(crate) fn destack_net_recv_msg(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
    recvflags: SocketMessageFlags,
    maxfds: u32,
    wantcredentials: bool,
    maxcontrolbytes: u32,
) -> RuntimeResult<SocketRecvMessageVm> {
    let _ = (
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
pub(crate) fn destack_net_resolve(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _query: ResolveQueryVm,
) -> RuntimeResult<VmArray<SocketAddressVm>> {
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
pub(crate) fn destack_net_reverse_lookup(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: SocketAddressVm,
    flags: ReverseLookupFlags,
) -> RuntimeResult<VmArray<ReverseLookupNameVm>> {
    let _ = (address, flags);
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
pub(crate) fn destack_net_get_reuse_addr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<bool> {
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
pub(crate) fn destack_net_get_reuse_port(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<bool> {
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
pub(crate) fn destack_net_set_reuse_addr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_set_reuse_port(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_route_add(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _route: RouteEntryVm,
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
pub(crate) fn destack_net_route_delete(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _route: RouteEntryVm,
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
pub(crate) fn destack_net_route_list(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _family: SocketFamily,
) -> RuntimeResult<VmArray<RouteEntryVm>> {
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
pub(crate) fn destack_net_send_mmsg(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    messages: VmSlice<SocketSendBatchEntryVm>,
) -> RuntimeResult<u64> {
    let _ = (handle, messages);
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
pub(crate) fn destack_net_send_msg(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
    message: SocketSendMessageVm,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer, message);
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
pub(crate) fn destack_net_send_to(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
    message: SocketSendToVm,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer, message);
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
pub(crate) fn destack_net_set_nonblocking(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_shutdown(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_socket(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    family: SocketFamily,
    sockettype: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<resource::SocketHandle> {
    let _ = (family, sockettype, protocol);
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
pub(crate) fn destack_net_socket_pair(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    family: SocketFamily,
    sockettype: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<SocketPairVm> {
    let _ = (family, sockettype, protocol);
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
pub(crate) fn destack_net_get_keep_alive(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<KeepAliveConfigVm> {
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
pub(crate) fn destack_net_get_no_delay(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<bool> {
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
pub(crate) fn destack_net_set_keep_alive(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    config: KeepAliveConfigVm,
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
pub(crate) fn destack_net_set_no_delay(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_udp_bind(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    address: SocketAddressVm,
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
pub(crate) fn destack_net_udp_connect(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    address: SocketAddressVm,
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
pub(crate) fn destack_net_get_multicast_interface_v4(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<vm::StringHandle> {
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
pub(crate) fn destack_net_get_multicast_interface_v6(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
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
pub(crate) fn destack_net_get_multicast_loop(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<bool> {
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
pub(crate) fn destack_net_get_multicast_ttl(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
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
pub(crate) fn destack_net_join_multicast_source_v4(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    membership: UdpSourceMembershipV4Vm,
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
pub(crate) fn destack_net_join_multicast_source_v6(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    membership: UdpSourceMembershipV6Vm,
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
pub(crate) fn destack_net_join_multicast_v4(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    group: vm::StringHandle,
    interfaceaddress: vm::StringHandle,
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
pub(crate) fn destack_net_join_multicast_v6(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    group: vm::StringHandle,
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
pub(crate) fn destack_net_leave_multicast_source_v4(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    membership: UdpSourceMembershipV4Vm,
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
pub(crate) fn destack_net_leave_multicast_source_v6(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    membership: UdpSourceMembershipV6Vm,
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
pub(crate) fn destack_net_leave_multicast_v4(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    group: vm::StringHandle,
    interfaceaddress: vm::StringHandle,
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
pub(crate) fn destack_net_leave_multicast_v6(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    group: vm::StringHandle,
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
pub(crate) fn destack_net_udp_recv_from(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
    recvflags: UdpMessageFlags,
) -> RuntimeResult<UdpReceiveVm> {
    let _ = (handle, buffer, recvflags);
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
pub(crate) fn destack_net_udp_send_to(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    address: SocketAddressVm,
    buffer: VmSlice<u8>,
    sendflags: UdpMessageFlags,
) -> RuntimeResult<u64> {
    let _ = (handle, address, buffer, sendflags);
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
pub(crate) fn destack_net_set_multicast_interface_v4(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    interfaceaddress: vm::StringHandle,
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
pub(crate) fn destack_net_set_multicast_interface_v6(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_set_multicast_loop(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_set_multicast_ttl(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_udp_socket(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _family: SocketFamily,
) -> RuntimeResult<resource::SocketHandle> {
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
pub(crate) fn destack_net_uds_accept(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _listener: resource::ListenerHandle,
) -> RuntimeResult<resource::SocketHandle> {
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
pub(crate) fn destack_net_uds_close_listener(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_net_uds_connect(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _address: UdsAddressVm,
) -> RuntimeResult<resource::SocketHandle> {
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
pub(crate) fn destack_net_uds_listen(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    address: UdsAddressVm,
    backlog: u32,
) -> RuntimeResult<resource::ListenerHandle> {
    let _ = (address, backlog);
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
pub(crate) fn destack_net_uds_socket_pair(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _sockettype: SocketType,
) -> RuntimeResult<SocketPairVm> {
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
pub(crate) fn destack_net_write(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
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
pub(crate) fn destack_net_writev(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.writev")).boxed())
}
