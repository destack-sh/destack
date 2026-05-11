#![allow(dead_code)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::fs::OsPath;
use crate::platform::net::{
    AcceptFlags, Linger, ResolveFlags, ReverseLookupFlags, ReverseLookupName, SocketAddress,
    SocketFamily, SocketMessageFlags, SocketRecvMessage, SocketSendMessage, SocketShutdown,
    UdpMessageFlags, UdpReceive,
};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeArray, PlatformError};
use crate::runtime::BindingCallContext;

pub(crate) use crate::platform::net::simulation::native::*;

/// Accept a new connection from a listener.
pub(crate) unsafe fn destack_net_accept(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
    _flags: AcceptFlags,
) -> RuntimeResult<()> {
    let _ = (binding, out, listener);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.accept")).boxed())
}

/// Close a socket handle.
pub(crate) unsafe fn destack_net_close(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.close")).boxed())
}

/// Connect to a remote socket address.
pub(crate) unsafe fn destack_net_connect(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    host: NativeStringRef,
    port: u16,
) -> RuntimeResult<()> {
    let _ = (binding, out, host, port);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.connect")).boxed())
}

/// Start listening on a raw socket address.
pub(crate) unsafe fn destack_net_listen(
    binding: &BindingCallContext,
    out: *mut ListenerHandle,
    host: NativeStringRef,
    port: u16,
    backlog: u32,
) -> RuntimeResult<()> {
    let _ = (binding, out, host, port, backlog);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.listen")).boxed())
}

/// Read from a socket into the provided slice.
pub(crate) unsafe fn destack_net_read(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.read")).boxed())
}

/// Write to a socket from the provided slice.
pub(crate) unsafe fn destack_net_write(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.write")).boxed())
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
    let _ = (
        binding,
        out,
        handle,
        buffer,
        recv_flags,
        max_fds,
        want_credentials,
        max_control_bytes,
    );
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.recvMsg")).boxed())
}

/// Receive multiple datagrams.
pub(crate) unsafe fn destack_net_recv_mmsg(
    binding: &BindingCallContext,
    out: *mut NativeArray<u64>,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    recv_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle, buffers, recv_flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.recvMmsg")).boxed())
}

/// Send a message with ancillary data.
pub(crate) unsafe fn destack_net_send_msg(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    message: SocketSendMessage,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle, buffer, message);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.sendMsg")).boxed())
}

/// Send multiple datagrams.
pub(crate) unsafe fn destack_net_send_mmsg(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    send_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle, buffers, send_flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.sendMmsg")).boxed())
}

/// Shut down a socket for reads, writes, or both.
pub(crate) unsafe fn destack_net_shutdown(
    binding: &BindingCallContext,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    let _ = (binding, handle, how);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.shutdown")).boxed())
}

/// Enable or disable nonblocking mode on a socket.
pub(crate) unsafe fn destack_net_set_nonblocking(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (binding, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setNonblocking")).boxed())
}

/// Read the local socket address as raw bytes.
pub(crate) unsafe fn destack_net_local_address(
    binding: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.localAddress")).boxed())
}

/// Reject unsupported net localAddressRaw.
pub(crate) unsafe fn destack_net_local_address_raw(
    binding: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.localAddressRaw")).boxed())
}

/// Read the remote socket address as raw bytes.
pub(crate) unsafe fn destack_net_peer_address(
    binding: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.peerAddress")).boxed())
}

/// Reject unsupported net peerAddressRaw.
pub(crate) unsafe fn destack_net_peer_address_raw(
    binding: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.peerAddressRaw")).boxed())
}

/// Enable or disable TCP_NODELAY.
pub(crate) unsafe fn destack_net_set_no_delay(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (binding, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setNoDelay")).boxed())
}

/// Write full TCP keepalive parameters.
pub(crate) unsafe fn destack_net_set_keep_alive(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
    idle_seconds: u32,
    interval_seconds: u32,
    probe_count: u32,
) -> RuntimeResult<()> {
    let _ = (
        binding,
        handle,
        enabled,
        idle_seconds,
        interval_seconds,
        probe_count,
    );
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setKeepAlive")).boxed())
}

/// Enable or disable SO_REUSEADDR.
pub(crate) unsafe fn destack_net_set_reuse_addr(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (binding, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReuseAddr")).boxed())
}

/// Enable or disable SO_REUSEPORT.
pub(crate) unsafe fn destack_net_set_reuse_port(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (binding, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReusePort")).boxed())
}

/// Close a listener handle.
pub(crate) unsafe fn destack_net_close_listener(
    binding: &BindingCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.closeListener")).boxed())
}

/// Reject unsupported net join multicast.
pub(crate) unsafe fn destack_net_join_multicast(
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (binding, handle, group, interface_address);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.joinMulticast")).boxed())
}

/// Join an IPv4 multicast group.
pub(crate) unsafe fn destack_net_join_multicast_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { destack_net_join_multicast(binding, handle, group, interface_address) }
}

/// Join an IPv6 multicast group.
pub(crate) unsafe fn destack_net_join_multicast_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    _interface_index: u32,
) -> RuntimeResult<()> {
    let interface_address = binding.store_string("");
    unsafe { destack_net_join_multicast(binding, handle, group, interface_address) }
}

/// Reject unsupported net leave multicast.
pub(crate) unsafe fn destack_net_leave_multicast(
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (binding, handle, group, interface_address);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.leaveMulticast")).boxed())
}

/// Leave an IPv4 multicast group.
pub(crate) unsafe fn destack_net_leave_multicast_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { destack_net_leave_multicast(binding, handle, group, interface_address) }
}

/// Leave an IPv6 multicast group.
pub(crate) unsafe fn destack_net_leave_multicast_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    _interface_index: u32,
) -> RuntimeResult<()> {
    let interface_address = binding.store_string("");
    unsafe { destack_net_leave_multicast(binding, handle, group, interface_address) }
}

/// Read into multiple buffers.
pub(crate) unsafe fn destack_net_readv(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.readv")).boxed())
}

/// Resolve a host and service query into raw socket addresses.
pub(crate) unsafe fn destack_net_resolve(
    binding: &BindingCallContext,
    out: *mut NativeArray<SocketAddress>,
    host: NativeStringRef,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<()> {
    let _ = (binding, out, host, port, family, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.resolve")).boxed())
}

/// Reverse lookup a raw socket address into host and service names.
pub(crate) unsafe fn destack_net_reverse_lookup(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (binding, out, address);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.reverseLookup")).boxed())
}

/// Enable or disable broadcast.
pub(crate) unsafe fn destack_net_set_broadcast(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (binding, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setBroadcast")).boxed())
}

/// Set socket linger settings.
pub(crate) unsafe fn destack_net_set_linger(
    binding: &BindingCallContext,
    handle: SocketHandle,
    linger: Linger,
) -> RuntimeResult<()> {
    let _ = (binding, handle, linger);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setLinger")).boxed())
}

/// Enable or disable multicast loopback.
pub(crate) unsafe fn destack_net_set_multicast_loop(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (binding, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setMulticastLoop")).boxed())
}

/// Set multicast TTL.
pub(crate) unsafe fn destack_net_set_multicast_ttl(
    binding: &BindingCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    let _ = (binding, handle, ttl);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setMulticastTtl")).boxed())
}

/// Set the read timeout in milliseconds.
pub(crate) unsafe fn destack_net_set_read_timeout(
    binding: &BindingCallContext,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    let _ = (binding, handle, timeout_ms);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReadTimeout")).boxed())
}

/// Set the receive buffer size.
pub(crate) unsafe fn destack_net_set_recv_buffer(
    binding: &BindingCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let _ = (binding, handle, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setRecvBuffer")).boxed())
}

/// Set the send buffer size.
pub(crate) unsafe fn destack_net_set_send_buffer(
    binding: &BindingCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let _ = (binding, handle, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setSendBuffer")).boxed())
}

/// Set the IP type-of-service field.
pub(crate) unsafe fn destack_net_set_tos(
    binding: &BindingCallContext,
    handle: SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    let _ = (binding, handle, tos);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setTos")).boxed())
}

/// Set the IP time-to-live.
pub(crate) unsafe fn destack_net_set_ttl(
    binding: &BindingCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    let _ = (binding, handle, ttl);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setTtl")).boxed())
}

/// Set the write timeout in milliseconds.
pub(crate) unsafe fn destack_net_set_write_timeout(
    binding: &BindingCallContext,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    let _ = (binding, handle, timeout_ms);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setWriteTimeout")).boxed())
}

/// Create a UDP socket.
pub(crate) unsafe fn destack_net_udp_socket(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
) -> RuntimeResult<()> {
    let _ = (binding, out, family);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpSocket")).boxed())
}

/// Bind a UDP socket to a raw local address.
pub(crate) unsafe fn destack_net_udp_bind(
    binding: &BindingCallContext,
    handle: SocketHandle,
    host: NativeStringRef,
    port: u16,
) -> RuntimeResult<()> {
    let _ = (binding, handle, host, port);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpBind")).boxed())
}

/// Connect a UDP socket to a raw remote address.
pub(crate) unsafe fn destack_net_udp_connect(
    binding: &BindingCallContext,
    handle: SocketHandle,
    host: NativeStringRef,
    port: u16,
) -> RuntimeResult<()> {
    let _ = (binding, handle, host, port);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpConnect")).boxed())
}

/// Send a datagram to a raw remote address.
pub(crate) unsafe fn destack_net_udp_send_to(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    host: NativeStringRef,
    port: u16,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle, host, port, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpSendTo")).boxed())
}

/// Receive a datagram from a remote address with raw address output.
pub(crate) unsafe fn destack_net_udp_recv_from(
    binding: &BindingCallContext,
    out: *mut UdpReceive,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpRecvFrom")).boxed())
}

/// Connect to a UDS endpoint.
pub(crate) unsafe fn destack_net_uds_connect(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (binding, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsConnect")).boxed())
}

/// Listen on a UDS address.
pub(crate) unsafe fn destack_net_uds_listen(
    binding: &BindingCallContext,
    out: *mut ListenerHandle,
    path: OsPath,
    backlog: u32,
) -> RuntimeResult<()> {
    let _ = (binding, out, path, backlog);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsListen")).boxed())
}

/// Connect one socket to one raw remote address.
pub(crate) unsafe fn destack_net_connect_raw(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (binding, handle, address);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.connect")).boxed())
}

/// Start one listener on one raw local address.
pub(crate) unsafe fn destack_net_listen_raw(
    binding: &BindingCallContext,
    out: *mut ListenerHandle,
    address: SocketAddress,
    backlog: u32,
) -> RuntimeResult<()> {
    let _ = (binding, out, address, backlog);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.listen")).boxed())
}

/// Resolve one host query into raw socket addresses.
pub(crate) unsafe fn destack_net_resolve_raw(
    binding: &BindingCallContext,
    out: *mut NativeArray<SocketAddress>,
    host: Option<NativeStringRef>,
    service: Option<NativeStringRef>,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<()> {
    let _ = (binding, out, host, service, family, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.resolve")).boxed())
}

/// Resolve one raw socket address into host names.
pub(crate) unsafe fn destack_net_reverse_lookup_raw(
    binding: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (binding, out, address);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.reverseLookup")).boxed())
}

/// Resolve one raw socket address into host and service names.
pub(crate) unsafe fn destack_net_reverse_lookup_names_raw(
    binding: &BindingCallContext,
    out: *mut NativeArray<ReverseLookupName>,
    address: SocketAddress,
    flags: ReverseLookupFlags,
) -> RuntimeResult<()> {
    let _ = (binding, out, address, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.reverseLookup")).boxed())
}

/// Bind one udp socket to one raw local address.
pub(crate) unsafe fn destack_net_udp_bind_raw(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (binding, handle, address);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpBind")).boxed())
}

/// Connect one udp socket to one raw remote address.
pub(crate) unsafe fn destack_net_udp_connect_raw(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (binding, handle, address);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpConnect")).boxed())
}

/// Receive one udp datagram with one raw source address.
pub(crate) unsafe fn destack_net_udp_recv_from_raw(
    binding: &BindingCallContext,
    out: *mut UdpReceive,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    recv_flags: UdpMessageFlags,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle, buffer, recv_flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpRecvFrom")).boxed())
}

/// Send one udp datagram to one raw destination address.
pub(crate) unsafe fn destack_net_udp_send_to_raw(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    address: SocketAddress,
    buffer: NativeSlice<u8>,
    send_flags: UdpMessageFlags,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle, address, buffer, send_flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpSendTo")).boxed())
}

/// Accept a connection from a UDS listener.
pub(crate) unsafe fn destack_net_uds_accept(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
) -> RuntimeResult<()> {
    let _ = (binding, out, listener);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsAccept")).boxed())
}

/// Close a UDS listener handle.
pub(crate) unsafe fn destack_net_uds_close_listener(
    binding: &BindingCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsCloseListener")).boxed())
}

/// Write from multiple buffers.
pub(crate) unsafe fn destack_net_writev(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (binding, out, handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.writev")).boxed())
}
