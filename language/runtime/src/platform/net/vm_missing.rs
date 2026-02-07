#![allow(dead_code)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{
    AcceptFlags, KeepAliveConfigVm, LingerVm, NetInterfaceVm, PacketCaptureOptionsVm,
    PacketCaptureRecordVm, PacketTimestampMode, ResolveFlags, RouteEntryVm, SocketAddressVm,
    SocketFamily, SocketPairVm, SocketProtocol, SocketRecvFromVm, SocketRecvMessageVm,
    SocketSendMessageVm, SocketSendToVm, SocketShutdown, SocketTimestampingMode, SocketType,
    UdpReceiveVm,
};
use crate::platform::{PlatformError, VmArray, VmSlice, fs, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.net.accept.
pub(super) fn destack_net_accept(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    listener: resource::ListenerHandle,
    flags: AcceptFlags,
) -> RuntimeResult<resource::SocketHandle> {
    let _ = (listener, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.accept is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.bind.
pub(super) fn destack_net_bind(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let _ = (handle, address);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.bind is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.close.
pub(super) fn destack_net_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.closeListener.
pub(super) fn destack_net_close_listener(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ListenerHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.closeListener is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.connect.
pub(super) fn destack_net_connect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let _ = (handle, address);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.connect is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getBroadcast.
pub(super) fn destack_net_get_broadcast(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<bool> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getBroadcast is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getKeepAlive.
pub(super) fn destack_net_get_keep_alive(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<KeepAliveConfigVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getKeepAlive is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getLinger.
pub(super) fn destack_net_get_linger(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<LingerVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getLinger is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getNoDelay.
pub(super) fn destack_net_get_no_delay(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<bool> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getNoDelay is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getOnlyV6.
pub(super) fn destack_net_get_only_v6(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<bool> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getOnlyV6 is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getPacketMark.
pub(super) fn destack_net_get_packet_mark(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getPacketMark is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getReadTimeout.
pub(super) fn destack_net_get_read_timeout(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getReadTimeout is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getRecvBuffer.
pub(super) fn destack_net_get_recv_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getRecvBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getReuseAddr.
pub(super) fn destack_net_get_reuse_addr(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<bool> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getReuseAddr is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getReusePort.
pub(super) fn destack_net_get_reuse_port(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<bool> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getReusePort is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getSendBuffer.
pub(super) fn destack_net_get_send_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getSendBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getTimestamping.
pub(super) fn destack_net_get_timestamping(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<SocketTimestampingMode> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getTimestamping is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getTos.
pub(super) fn destack_net_get_tos(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getTos is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getTtl.
pub(super) fn destack_net_get_ttl(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getTtl is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.getWriteTimeout.
pub(super) fn destack_net_get_write_timeout(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<u32> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.getWriteTimeout is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.interfaceIndex.
pub(super) fn destack_net_interface_index(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    name: vm::StringHandle,
) -> RuntimeResult<u32> {
    let _ = name;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.interfaceIndex is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.interfaceName.
pub(super) fn destack_net_interface_name(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    index: u32,
) -> RuntimeResult<vm::StringHandle> {
    let _ = index;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.interfaceName is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.joinMulticastV4.
pub(super) fn destack_net_join_multicast_v4(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    group: vm::StringHandle,
    interfaceaddress: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (handle, group, interfaceaddress);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.joinMulticastV4 is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.joinMulticastV6.
pub(super) fn destack_net_join_multicast_v6(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    group: vm::StringHandle,
    interfaceindex: u32,
) -> RuntimeResult<()> {
    let _ = (handle, group, interfaceindex);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.joinMulticastV6 is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.leaveMulticastV4.
pub(super) fn destack_net_leave_multicast_v4(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    group: vm::StringHandle,
    interfaceaddress: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (handle, group, interfaceaddress);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.leaveMulticastV4 is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.leaveMulticastV6.
pub(super) fn destack_net_leave_multicast_v6(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    group: vm::StringHandle,
    interfaceindex: u32,
) -> RuntimeResult<()> {
    let _ = (handle, group, interfaceindex);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.leaveMulticastV6 is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.listInterfaces.
pub(super) fn destack_net_list_interfaces(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<VmArray<NetInterfaceVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.listInterfaces is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.listen.
pub(super) fn destack_net_listen(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    address: SocketAddressVm,
    backlog: u32,
) -> RuntimeResult<resource::ListenerHandle> {
    let _ = (address, backlog);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.listen is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.localAddress.
pub(super) fn destack_net_local_address(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.localAddress is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.packetOpen.
pub(super) fn destack_net_packet_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    options: PacketCaptureOptionsVm,
) -> RuntimeResult<resource::SocketHandle> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.packetOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.packetReceive.
pub(super) fn destack_net_packet_receive(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    payload: VmSlice<u8>,
) -> RuntimeResult<PacketCaptureRecordVm> {
    let _ = (handle, payload);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.packetReceive is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.packetSend.
pub(super) fn destack_net_packet_send(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    payload: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, payload);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.packetSend is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.packetSetTimestampMode.
pub(super) fn destack_net_packet_set_timestamp_mode(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    mode: PacketTimestampMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.packetSetTimestampMode is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.peerAddress.
pub(super) fn destack_net_peer_address(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.peerAddress is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.rawSetHeaderIncluded.
pub(super) fn destack_net_raw_set_header_included(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.rawSetHeaderIncluded is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.rawSocket.
pub(super) fn destack_net_raw_socket(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    family: SocketFamily,
    protocol: i32,
) -> RuntimeResult<resource::SocketHandle> {
    let _ = (family, protocol);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.rawSocket is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.read.
pub(super) fn destack_net_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.read is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.readv.
pub(super) fn destack_net_readv(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.readv is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.recvFrom.
pub(super) fn destack_net_recv_from(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
    recvflags: u32,
) -> RuntimeResult<SocketRecvFromVm> {
    let _ = (handle, buffer, recvflags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.recvFrom is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.recvMmsg.
pub(super) fn destack_net_recv_mmsg(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
    recvflags: u32,
) -> RuntimeResult<VmArray<u64>> {
    let _ = (handle, buffers, recvflags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.recvMmsg is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.recvMsg.
pub(super) fn destack_net_recv_msg(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
    recvflags: u32,
    maxfds: u32,
    wantcredentials: bool,
) -> RuntimeResult<SocketRecvMessageVm> {
    let _ = (handle, buffer, recvflags, maxfds, wantcredentials);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.recvMsg is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.resolve.
pub(super) fn destack_net_resolve(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    host: vm::StringHandle,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<VmArray<SocketAddressVm>> {
    let _ = (host, port, family, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.resolve is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.reverseLookup.
pub(super) fn destack_net_reverse_lookup(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    address: SocketAddressVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let _ = address;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.reverseLookup is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.routeAdd.
pub(super) fn destack_net_route_add(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    route: RouteEntryVm,
) -> RuntimeResult<()> {
    let _ = route;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.routeAdd is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.routeDelete.
pub(super) fn destack_net_route_delete(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    route: RouteEntryVm,
) -> RuntimeResult<()> {
    let _ = route;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.routeDelete is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.routeList.
pub(super) fn destack_net_route_list(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    family: SocketFamily,
) -> RuntimeResult<VmArray<RouteEntryVm>> {
    let _ = family;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.routeList is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.routeSetNamespace.
pub(super) fn destack_net_route_set_namespace(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.routeSetNamespace is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.sendMmsg.
pub(super) fn destack_net_send_mmsg(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
    sendflags: u32,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers, sendflags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.sendMmsg is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.sendMsg.
pub(super) fn destack_net_send_msg(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
    message: SocketSendMessageVm,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer, message);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.sendMsg is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.sendTo.
pub(super) fn destack_net_send_to(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
    message: SocketSendToVm,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer, message);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.sendTo is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setBroadcast.
pub(super) fn destack_net_set_broadcast(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setBroadcast is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setKeepAlive.
pub(super) fn destack_net_set_keep_alive(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    config: KeepAliveConfigVm,
) -> RuntimeResult<()> {
    let _ = (handle, config);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setKeepAlive is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setLinger.
pub(super) fn destack_net_set_linger(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    linger: LingerVm,
) -> RuntimeResult<()> {
    let _ = (handle, linger);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setLinger is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setMulticastLoop.
pub(super) fn destack_net_set_multicast_loop(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setMulticastLoop is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setMulticastTtl.
pub(super) fn destack_net_set_multicast_ttl(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    let _ = (handle, ttl);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setMulticastTtl is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setNoDelay.
pub(super) fn destack_net_set_no_delay(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setNoDelay is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setNonblocking.
pub(super) fn destack_net_set_nonblocking(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setNonblocking is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setOnlyV6.
pub(super) fn destack_net_set_only_v6(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setOnlyV6 is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setPacketMark.
pub(super) fn destack_net_set_packet_mark(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    mark: u32,
) -> RuntimeResult<()> {
    let _ = (handle, mark);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setPacketMark is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setReadTimeout.
pub(super) fn destack_net_set_read_timeout(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    timeoutms: u32,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutms);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setReadTimeout is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setRecvBuffer.
pub(super) fn destack_net_set_recv_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let _ = (handle, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setRecvBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setReuseAddr.
pub(super) fn destack_net_set_reuse_addr(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setReuseAddr is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setReusePort.
pub(super) fn destack_net_set_reuse_port(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = (handle, enabled);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setReusePort is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setSendBuffer.
pub(super) fn destack_net_set_send_buffer(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let _ = (handle, size);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setSendBuffer is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setTimestamping.
pub(super) fn destack_net_set_timestamping(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    mode: SocketTimestampingMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setTimestamping is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setTos.
pub(super) fn destack_net_set_tos(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    let _ = (handle, tos);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setTos is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setTtl.
pub(super) fn destack_net_set_ttl(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    let _ = (handle, ttl);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setTtl is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.setWriteTimeout.
pub(super) fn destack_net_set_write_timeout(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    timeoutms: u32,
) -> RuntimeResult<()> {
    let _ = (handle, timeoutms);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.setWriteTimeout is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.shutdown.
pub(super) fn destack_net_shutdown(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    let _ = (handle, how);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.shutdown is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.socket.
pub(super) fn destack_net_socket(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    family: SocketFamily,
    sockettype: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<resource::SocketHandle> {
    let _ = (family, sockettype, protocol);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.socket is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.socketPair.
pub(super) fn destack_net_socket_pair(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    family: SocketFamily,
    sockettype: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<SocketPairVm> {
    let _ = (family, sockettype, protocol);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.socketPair is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.udpBind.
pub(super) fn destack_net_udp_bind(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let _ = (handle, address);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udpBind is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.udpConnect.
pub(super) fn destack_net_udp_connect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let _ = (handle, address);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udpConnect is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.udpRecvFrom.
pub(super) fn destack_net_udp_recv_from(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
    recvflags: u32,
) -> RuntimeResult<UdpReceiveVm> {
    let _ = (handle, buffer, recvflags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udpRecvFrom is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.udpSendTo.
pub(super) fn destack_net_udp_send_to(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    address: SocketAddressVm,
    buffer: VmSlice<u8>,
    sendflags: u32,
) -> RuntimeResult<u64> {
    let _ = (handle, address, buffer, sendflags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udpSendTo is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.udpSocket.
pub(super) fn destack_net_udp_socket(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    family: SocketFamily,
) -> RuntimeResult<resource::SocketHandle> {
    let _ = family;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udpSocket is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.udsAccept.
pub(super) fn destack_net_uds_accept(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    listener: resource::ListenerHandle,
) -> RuntimeResult<resource::SocketHandle> {
    let _ = listener;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udsAccept is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.udsCloseListener.
pub(super) fn destack_net_uds_close_listener(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::ListenerHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udsCloseListener is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.udsConnect.
pub(super) fn destack_net_uds_connect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<resource::SocketHandle> {
    let _ = path;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udsConnect is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.udsListen.
pub(super) fn destack_net_uds_listen(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: fs::OsPathVm,
    backlog: u32,
) -> RuntimeResult<resource::ListenerHandle> {
    let _ = (path, backlog);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udsListen is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.udsSocketPair.
pub(super) fn destack_net_uds_socket_pair(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    sockettype: SocketType,
) -> RuntimeResult<SocketPairVm> {
    let _ = sockettype;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.udsSocketPair is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.write.
pub(super) fn destack_net_write(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.write is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.net.writev.
pub(super) fn destack_net_writev(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.writev is not available in the VM yet",
    ))
    .boxed())
}
