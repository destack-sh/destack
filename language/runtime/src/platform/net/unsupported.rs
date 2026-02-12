#![allow(dead_code)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::OsPath;
use crate::platform::net::{
    AcceptFlags, Linger, ResolveFlags, SocketAddress, SocketFamily, SocketMessageFlags,
    SocketRecvMessage, SocketSendMessage, SocketShutdown, UdpReceive,
};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Reject unsupported net accept.
pub(crate) unsafe fn destack_net_accept(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
    _flags: AcceptFlags,
) -> RuntimeResult<()> {
    let _ = (context, out, listener);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.accept")).boxed())
}

/// Reject unsupported net close.
pub(crate) unsafe fn destack_net_close(
    context: &RuntimeCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.close")).boxed())
}

/// Reject unsupported net connect.
pub(crate) unsafe fn destack_net_connect(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    host: NativeStringRef,
    port: u16,
) -> RuntimeResult<()> {
    let _ = (context, out, host, port);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.connect")).boxed())
}

/// Reject unsupported net listen.
pub(crate) unsafe fn destack_net_listen(
    context: &RuntimeCallContext,
    out: *mut ListenerHandle,
    host: NativeStringRef,
    port: u16,
    backlog: u32,
) -> RuntimeResult<()> {
    let _ = (context, out, host, port, backlog);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.listen")).boxed())
}

/// Reject unsupported net read.
pub(crate) unsafe fn destack_net_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.read")).boxed())
}

/// Reject unsupported net write.
pub(crate) unsafe fn destack_net_write(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.write")).boxed())
}

/// Reject unsupported net recvMsg.
pub(crate) unsafe fn destack_net_recv_msg(
    context: &RuntimeCallContext,
    out: *mut SocketRecvMessage,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    recv_flags: SocketMessageFlags,
    max_fds: u32,
    want_credentials: bool,
    max_control_bytes: u32,
) -> RuntimeResult<()> {
    let _ = (
        context,
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

/// Reject unsupported net recvMmsg.
pub(crate) unsafe fn destack_net_recv_mmsg(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u64>,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    recv_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffers, recv_flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.recvMmsg")).boxed())
}

/// Reject unsupported net sendMsg.
pub(crate) unsafe fn destack_net_send_msg(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    message: SocketSendMessage,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffer, message);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.sendMsg")).boxed())
}

/// Reject unsupported net sendMmsg.
pub(crate) unsafe fn destack_net_send_mmsg(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    send_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffers, send_flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.sendMmsg")).boxed())
}

/// Reject unsupported net shutdown.
pub(crate) unsafe fn destack_net_shutdown(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    let _ = (context, handle, how);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.shutdown")).boxed())
}

/// Reject unsupported net setNonblocking.
pub(crate) unsafe fn destack_net_set_nonblocking(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setNonblocking")).boxed())
}

/// Reject unsupported net localAddress.
pub(crate) unsafe fn destack_net_local_address(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.localAddress")).boxed())
}

/// Reject unsupported net localAddressRaw.
pub(crate) unsafe fn destack_net_local_address_raw(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.localAddressRaw")).boxed())
}

/// Reject unsupported net peerAddress.
pub(crate) unsafe fn destack_net_peer_address(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.peerAddress")).boxed())
}

/// Reject unsupported net peerAddressRaw.
pub(crate) unsafe fn destack_net_peer_address_raw(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.peerAddressRaw")).boxed())
}

/// Reject unsupported net setNoDelay.
pub(crate) unsafe fn destack_net_set_no_delay(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setNoDelay")).boxed())
}

/// Reject unsupported net setKeepAlive.
pub(crate) unsafe fn destack_net_set_keep_alive(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
    idle_seconds: u32,
    interval_seconds: u32,
    probe_count: u32,
) -> RuntimeResult<()> {
    let _ = (
        context,
        handle,
        enabled,
        idle_seconds,
        interval_seconds,
        probe_count,
    );
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setKeepAlive")).boxed())
}

/// Reject unsupported net setReuseAddr.
pub(crate) unsafe fn destack_net_set_reuse_addr(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReuseAddr")).boxed())
}

/// Reject unsupported net setReusePort.
pub(crate) unsafe fn destack_net_set_reuse_port(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReusePort")).boxed())
}

/// Reject unsupported net closeListener.
pub(crate) unsafe fn destack_net_close_listener(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.closeListener")).boxed())
}

/// Reject unsupported net join multicast.
pub(crate) unsafe fn destack_net_join_multicast(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, handle, group, interface_address);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.joinMulticast")).boxed())
}

/// Reject unsupported net joinMulticastV4.
pub(crate) unsafe fn destack_net_join_multicast_v4(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { destack_net_join_multicast(context, handle, group, interface_address) }
}

/// Reject unsupported net joinMulticastV6.
pub(crate) unsafe fn destack_net_join_multicast_v6(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    _interface_index: u32,
) -> RuntimeResult<()> {
    let interface_address = context.store_string("");
    unsafe { destack_net_join_multicast(context, handle, group, interface_address) }
}

/// Reject unsupported net leave multicast.
pub(crate) unsafe fn destack_net_leave_multicast(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, handle, group, interface_address);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.leaveMulticast")).boxed())
}

/// Reject unsupported net leaveMulticastV4.
pub(crate) unsafe fn destack_net_leave_multicast_v4(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { destack_net_leave_multicast(context, handle, group, interface_address) }
}

/// Reject unsupported net leaveMulticastV6.
pub(crate) unsafe fn destack_net_leave_multicast_v6(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    _interface_index: u32,
) -> RuntimeResult<()> {
    let interface_address = context.store_string("");
    unsafe { destack_net_leave_multicast(context, handle, group, interface_address) }
}

/// Reject unsupported net readv.
pub(crate) unsafe fn destack_net_readv(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.readv")).boxed())
}

/// Reject unsupported net resolve.
pub(crate) unsafe fn destack_net_resolve(
    context: &RuntimeCallContext,
    out: *mut NativeArray<SocketAddress>,
    host: NativeStringRef,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<()> {
    let _ = (context, out, host, port, family, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.resolve")).boxed())
}

/// Reject unsupported net reverse lookup.
pub(crate) unsafe fn destack_net_reverse_lookup(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    address: SocketAddress,
) -> RuntimeResult<()> {
    let _ = (context, out, address);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.reverseLookup")).boxed())
}

/// Reject unsupported net set broadcast.
pub(crate) unsafe fn destack_net_set_broadcast(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setBroadcast")).boxed())
}

/// Reject unsupported net set linger.
pub(crate) unsafe fn destack_net_set_linger(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    linger: Linger,
) -> RuntimeResult<()> {
    let _ = (context, handle, linger);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setLinger")).boxed())
}

/// Reject unsupported net set multicast loop.
pub(crate) unsafe fn destack_net_set_multicast_loop(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (context, handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setMulticastLoop")).boxed())
}

/// Reject unsupported net set multicast ttl.
pub(crate) unsafe fn destack_net_set_multicast_ttl(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, ttl);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setMulticastTtl")).boxed())
}

/// Reject unsupported net set read timeout.
pub(crate) unsafe fn destack_net_set_read_timeout(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, timeout_ms);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReadTimeout")).boxed())
}

/// Reject unsupported net set receive buffer.
pub(crate) unsafe fn destack_net_set_recv_buffer(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setRecvBuffer")).boxed())
}

/// Reject unsupported net set send buffer.
pub(crate) unsafe fn destack_net_set_send_buffer(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setSendBuffer")).boxed())
}

/// Reject unsupported net set tos.
pub(crate) unsafe fn destack_net_set_tos(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, tos);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setTos")).boxed())
}

/// Reject unsupported net set ttl.
pub(crate) unsafe fn destack_net_set_ttl(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, ttl);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setTtl")).boxed())
}

/// Reject unsupported net set write timeout.
pub(crate) unsafe fn destack_net_set_write_timeout(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, timeout_ms);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setWriteTimeout")).boxed())
}

/// Reject unsupported net udp socket.
pub(crate) unsafe fn destack_net_udp_socket(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
) -> RuntimeResult<()> {
    let _ = (context, out, family);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpSocket")).boxed())
}

/// Reject unsupported net udp bind.
pub(crate) unsafe fn destack_net_udp_bind(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    host: NativeStringRef,
    port: u16,
) -> RuntimeResult<()> {
    let _ = (context, handle, host, port);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpBind")).boxed())
}

/// Reject unsupported net udp connect.
pub(crate) unsafe fn destack_net_udp_connect(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    host: NativeStringRef,
    port: u16,
) -> RuntimeResult<()> {
    let _ = (context, handle, host, port);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpConnect")).boxed())
}

/// Reject unsupported net udp send.
pub(crate) unsafe fn destack_net_udp_send_to(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    host: NativeStringRef,
    port: u16,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, host, port, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpSendTo")).boxed())
}

/// Reject unsupported net udp recv.
pub(crate) unsafe fn destack_net_udp_recv_from(
    context: &RuntimeCallContext,
    out: *mut UdpReceive,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpRecvFrom")).boxed())
}

/// Reject unsupported net uds connect.
pub(crate) unsafe fn destack_net_uds_connect(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsConnect")).boxed())
}

/// Reject unsupported net uds listen.
pub(crate) unsafe fn destack_net_uds_listen(
    context: &RuntimeCallContext,
    out: *mut ListenerHandle,
    path: OsPath,
    backlog: u32,
) -> RuntimeResult<()> {
    let _ = (context, out, path, backlog);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsListen")).boxed())
}

/// Reject unsupported net uds accept.
pub(crate) unsafe fn destack_net_uds_accept(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, listener);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsAccept")).boxed())
}

/// Reject unsupported net uds close listener.
pub(crate) unsafe fn destack_net_uds_close_listener(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsCloseListener")).boxed())
}

/// Reject unsupported net writev.
pub(crate) unsafe fn destack_net_writev(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.writev")).boxed())
}
