use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{OsPath, OsPathVm, PathBytesAbi, PathEncoding, PathUtf16Abi};
use crate::platform::net::{
    AcceptFlags, KeepAliveConfig, Linger, ResolveFlags, SocketAddress, SocketAddressVm,
    SocketCredentials, SocketCredentialsVm, SocketMessageFlags, SocketPair, SocketPairVm,
    SocketProtocol, SocketRecvFromVm, SocketRecvMessage, SocketRecvMessageVm, SocketSendMessage,
    SocketSendMessageVm, SocketSendToVm, SocketShutdown, SocketType, UdpMessageFlags, UdpReceive,
    UdpReceiveVm, core as core_net,
};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError, VmArray, VmSlice};
use crate::runtime::RuntimeCallContext;

pub(super) use super::vm_missing::{
    destack_net_get_packet_mark, destack_net_get_timestamping, destack_net_interface_index,
    destack_net_interface_name, destack_net_list_interfaces, destack_net_packet_open,
    destack_net_packet_receive, destack_net_packet_send, destack_net_packet_set_timestamp_mode,
    destack_net_raw_set_header_included, destack_net_raw_socket, destack_net_route_add,
    destack_net_route_delete, destack_net_route_list, destack_net_route_set_namespace,
    destack_net_set_packet_mark, destack_net_set_timestamping,
};

/// Accept a new connection from a listener.
pub fn destack_net_accept(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    listener: ListenerHandle,
    flags: AcceptFlags,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { core_net::destack_net_accept(runtime, out, listener, flags) })
}

/// Close a socket handle.
pub fn destack_net_close(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_close(runtime, handle) }
}

/// Connect an existing socket handle to a raw remote address.
pub fn destack_net_connect(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    unsafe { core_net::destack_net_connect_raw(runtime, handle, address) }
}

/// Connect to a remote host and return a socket handle.
pub fn destack_net_connect_text(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _host: vm::StringHandle,
    _port: u16,
) -> RuntimeResult<SocketHandle> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.connectText")).boxed())
}

/// Start listening on a raw socket address.
pub fn destack_net_listen(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    address: SocketAddressVm,
    backlog: u32,
) -> RuntimeResult<ListenerHandle> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    call_out(|out| unsafe { core_net::destack_net_listen_raw(runtime, out, address, backlog) })
}

/// Start listening on a host and port.
pub fn destack_net_listen_text(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _host: vm::StringHandle,
    _port: u16,
    _backlog: u32,
) -> RuntimeResult<ListenerHandle> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.listenText")).boxed())
}

/// Read from a socket into the provided slice.
pub fn destack_net_read(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // allocate a native buffer for reads
    let native_buffer = allocate_read_buffer(runtime, buffer);

    // perform the read
    let bytes_read =
        call_out(|out| unsafe { core_net::destack_net_read(runtime, out, handle, native_buffer) })?;

    // write results back into the VM buffer
    write_read_buffer(context, buffer, native_buffer)?;

    Ok(bytes_read)
}

/// Write to a socket from the provided slice.
pub fn destack_net_write(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // resolve the VM buffer into native storage
    let native_buffer = buffer_from_vm(runtime, context, buffer)?;

    // dispatch to the core binding
    call_out(|out| unsafe { core_net::destack_net_write(runtime, out, handle, native_buffer) })
}

/// Receive a message with ancillary data.
pub fn destack_net_recv_msg(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    recv_flags: SocketMessageFlags,
    max_fds: u32,
    want_credentials: bool,
) -> RuntimeResult<SocketRecvMessageVm> {
    // allocate a native buffer for reads
    let native_buffer = allocate_read_buffer(runtime, buffer);

    // perform the receive
    let message = call_out(|out| unsafe {
        core_net::destack_net_recv_msg(
            runtime,
            out,
            handle,
            native_buffer,
            recv_flags,
            max_fds,
            want_credentials,
        )
    })?;

    // write results back into the VM buffer
    write_read_buffer(context, buffer, native_buffer)?;

    // convert the metadata into VM values
    socket_recv_message_to_vm(context, message)
}

/// Receive multiple messages into multiple buffers.
pub fn destack_net_recv_mmsg(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
    recv_flags: SocketMessageFlags,
) -> RuntimeResult<VmArray<u64>> {
    // allocate native buffers for reads
    let (native_buffers, vm_buffers) = allocate_read_buffers(runtime, context, buffers)?;

    // perform the receive operation
    let counts = call_out(|out| unsafe {
        core_net::destack_net_recv_mmsg(runtime, out, handle, native_buffers, recv_flags)
    })?;

    // write read buffers back into VM slices
    write_read_buffers(context, vm_buffers, native_buffers)?;

    // convert result counts into VM values
    u64_array_to_vm(context, counts)
}

/// Send a message with ancillary data.
pub fn destack_net_send_msg(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    message: SocketSendMessageVm,
) -> RuntimeResult<u64> {
    // resolve the VM buffer into native storage
    let native_buffer = buffer_from_vm(runtime, context, buffer)?;

    // convert the metadata into native values
    let message = socket_send_message_from_vm(runtime, context, message)?;

    // dispatch to the core binding
    call_out(|out| unsafe {
        core_net::destack_net_send_msg(runtime, out, handle, native_buffer, message)
    })
}

/// Receive a packet with raw source address metadata.
pub fn destack_net_recv_from(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    recv_flags: SocketMessageFlags,
) -> RuntimeResult<SocketRecvFromVm> {
    let native = allocate_read_buffer(runtime, buffer);
    let receive = call_out(|out| unsafe {
        core_net::destack_net_recv_from(runtime, out, handle, native, recv_flags)
    })?;
    write_read_buffer(context, buffer, native)?;
    socket_recv_from_to_vm(context, receive)
}

/// Send a packet to a raw destination address.
pub fn destack_net_send_to(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    message: SocketSendToVm,
) -> RuntimeResult<u64> {
    let native_buffer = buffer_from_vm(runtime, context, buffer)?;
    let native_message = socket_send_to_from_vm(runtime, context, message)?;
    call_out(|out| unsafe {
        core_net::destack_net_send_to(runtime, out, handle, native_buffer, native_message)
    })
}

/// Send multiple messages from multiple buffers.
pub fn destack_net_send_mmsg(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
    send_flags: SocketMessageFlags,
) -> RuntimeResult<u64> {
    // resolve VM buffers into native storage
    let native_buffers = buffers_from_vm(runtime, context, buffers)?;

    // dispatch to the core binding
    call_out(|out| unsafe {
        core_net::destack_net_send_mmsg(runtime, out, handle, native_buffers, send_flags)
    })
}

/// Shut down a socket for reads, writes, or both.
pub fn destack_net_shutdown(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_shutdown(runtime, handle, how) }
}

/// Enable or disable nonblocking mode on a socket.
pub fn destack_net_set_nonblocking(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_nonblocking(runtime, handle, enabled) }
}

/// Read the local socket address as raw bytes.
pub fn destack_net_local_address(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    // read the address via the core binding
    let address =
        call_out(|out| unsafe { core_net::destack_net_local_address_raw(runtime, out, handle) })?;
    socket_address_raw_to_vm(context, address)
}

/// Read the local socket address as normalized text metadata.
pub fn destack_net_local_address_text(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.localAddressText")).boxed())
}

/// Read the remote socket address as raw bytes.
pub fn destack_net_peer_address(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    // read the address via the core binding
    let address =
        call_out(|out| unsafe { core_net::destack_net_peer_address_raw(runtime, out, handle) })?;
    socket_address_raw_to_vm(context, address)
}

/// Read the remote socket address as normalized text metadata.
pub fn destack_net_peer_address_text(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.peerAddressText")).boxed())
}

/// Enable or disable TCP_NODELAY.
pub fn destack_net_set_no_delay(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_no_delay(runtime, handle, enabled) }
}

/// Write full TCP keepalive settings.
pub fn destack_net_set_keep_alive(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    config: KeepAliveConfig,
) -> RuntimeResult<()> {
    unsafe {
        core_net::destack_net_set_keep_alive(
            runtime,
            handle,
            config.enabled,
            config.idle_seconds,
            config.interval_seconds,
            config.probe_count,
        )
    }
}

/// Enable or disable SO_REUSEADDR.
pub fn destack_net_set_reuse_addr(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_reuse_addr(runtime, handle, enabled) }
}

/// Enable or disable SO_REUSEPORT.
pub fn destack_net_set_reuse_port(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_reuse_port(runtime, handle, enabled) }
}

/// Close a listener handle.
pub fn destack_net_close_listener(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_close_listener(runtime, handle) }
}

/// Create a socket from family, type, and protocol.
pub fn destack_net_socket(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    family: crate::platform::net::SocketFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe {
        core_net::destack_net_socket(runtime, out, family, socket_type, protocol)
    })
}

/// Create a connected socket pair.
pub fn destack_net_socket_pair(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    family: crate::platform::net::SocketFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<SocketPairVm> {
    call_out(|out| unsafe {
        core_net::destack_net_socket_pair(runtime, out, family, socket_type, protocol)
    })
}

/// Bind an existing socket to a raw address.
pub fn destack_net_bind(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    unsafe { core_net::destack_net_bind(runtime, handle, address) }
}

/// Read from a socket into multiple buffers.
pub fn destack_net_readv(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let (native_buffers, vm_buffers) = allocate_read_buffers(runtime, context, buffers)?;
    let count = call_out(|out| unsafe {
        core_net::destack_net_readv(runtime, out, handle, native_buffers)
    })?;
    write_read_buffers(context, vm_buffers, native_buffers)?;
    Ok(count)
}

/// Write to a socket from multiple buffers.
pub fn destack_net_writev(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let native_buffers = buffers_from_vm(runtime, context, buffers)?;
    call_out(|out| unsafe { core_net::destack_net_writev(runtime, out, handle, native_buffers) })
}

/// Resolve a hostname and port into raw socket addresses.
pub fn destack_net_resolve(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    host: vm::StringHandle,
    port: u16,
    family: crate::platform::net::SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<VmArray<SocketAddressVm>> {
    let host = host_from_vm(runtime, context, host)?;
    let addresses = call_out(|out| unsafe {
        core_net::destack_net_resolve_raw(runtime, out, host, port, family, flags)
    })?;
    socket_address_raw_array_to_vm(context, addresses)
}

/// Resolve a hostname and port into normalized socket addresses.
pub fn destack_net_resolve_text(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _host: vm::StringHandle,
    _port: u16,
    _family: crate::platform::net::SocketFamily,
    _flags: ResolveFlags,
) -> RuntimeResult<VmArray<SocketAddressVm>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.resolveText")).boxed())
}

/// Reverse lookup a raw socket address into hostnames.
pub fn destack_net_reverse_lookup(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    address: SocketAddressVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    let names =
        call_out(|out| unsafe { core_net::destack_net_reverse_lookup_raw(runtime, out, address) })?;
    string_array_to_vm(context, names)
}

/// Reverse lookup a normalized socket address into hostnames.
pub fn destack_net_reverse_lookup_text(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _address: SocketAddressVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.net.reverseLookupText",
    ))
    .boxed())
}

/// Create a UDP socket.
pub fn destack_net_udp_socket(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    family: crate::platform::net::SocketFamily,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { core_net::destack_net_udp_socket(runtime, out, family) })
}

/// Bind a UDP socket to a raw address.
pub fn destack_net_udp_bind(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    unsafe { core_net::destack_net_udp_bind_raw(runtime, handle, address) }
}

/// Bind a UDP socket to a host and port.
pub fn destack_net_udp_bind_text(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: SocketHandle,
    _host: vm::StringHandle,
    _port: u16,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpBindText")).boxed())
}

/// Connect a UDP socket to a raw address.
pub fn destack_net_udp_connect(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    unsafe { core_net::destack_net_udp_connect_raw(runtime, handle, address) }
}

/// Connect a UDP socket to a host and port.
pub fn destack_net_udp_connect_text(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: SocketHandle,
    _host: vm::StringHandle,
    _port: u16,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpConnectText")).boxed())
}

/// Receive a UDP packet with raw sender metadata.
pub fn destack_net_udp_recv_from(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    recv_flags: UdpMessageFlags,
) -> RuntimeResult<UdpReceiveVm> {
    let native = allocate_read_buffer(runtime, buffer);
    let receive = call_out(|out| unsafe {
        core_net::destack_net_udp_recv_from_raw(runtime, out, handle, native, recv_flags)
    })?;
    write_read_buffer(context, buffer, native)?;
    udp_receive_raw_to_vm(context, receive)
}

/// Receive a UDP packet with normalized sender metadata.
pub fn destack_net_udp_recv_from_text(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: SocketHandle,
    _buffer: VmSlice<u8>,
) -> RuntimeResult<UdpReceiveVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpRecvFromText")).boxed())
}

/// Send a UDP packet to a raw address.
pub fn destack_net_udp_send_to(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
    buffer: VmSlice<u8>,
    send_flags: UdpMessageFlags,
) -> RuntimeResult<u64> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    let native = buffer_from_vm(runtime, context, buffer)?;
    call_out(|out| unsafe {
        core_net::destack_net_udp_send_to_raw(runtime, out, handle, address, native, send_flags)
    })
}

/// Send a UDP packet to a host and port.
pub fn destack_net_udp_send_to_text(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: SocketHandle,
    _host: vm::StringHandle,
    _port: u16,
    _buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpSendToText")).boxed())
}

/// Accept a UNIX domain socket connection.
pub fn destack_net_uds_accept(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    listener: ListenerHandle,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { core_net::destack_net_uds_accept(runtime, out, listener) })
}

/// Close a UNIX domain socket listener.
pub fn destack_net_uds_close_listener(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_uds_close_listener(runtime, handle) }
}

/// Connect to a UNIX domain socket.
pub fn destack_net_uds_connect(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<SocketHandle> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_net::destack_net_uds_connect(runtime, out, path) })
}

/// Listen on a UNIX domain socket.
pub fn destack_net_uds_listen(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    backlog: u32,
) -> RuntimeResult<ListenerHandle> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_net::destack_net_uds_listen(runtime, out, path, backlog) })
}

/// Create a connected unix domain socket pair.
pub fn destack_net_uds_socket_pair(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    socket_type: SocketType,
) -> RuntimeResult<SocketPair> {
    call_out(|out| unsafe { core_net::destack_net_uds_socket_pair(runtime, out, socket_type) })
}

/// Set socket linger settings.
pub fn destack_net_set_linger(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    linger: Linger,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_linger(runtime, handle, linger) }
}

/// Set the receive buffer size.
pub fn destack_net_set_recv_buffer(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_recv_buffer(runtime, handle, size) }
}

/// Set the send buffer size.
pub fn destack_net_set_send_buffer(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_send_buffer(runtime, handle, size) }
}

/// Enable or disable broadcast.
pub fn destack_net_set_broadcast(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_broadcast(runtime, handle, enabled) }
}

/// Join an IPv4 multicast group.
pub fn destack_net_join_multicast_v4(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    group: vm::StringHandle,
    interface_address: vm::StringHandle,
) -> RuntimeResult<()> {
    // resolve the multicast group
    let group = host_from_vm(runtime, context, group)?;

    // resolve the interface address
    let interface_address = host_from_vm(runtime, context, interface_address)?;

    // dispatch to the core binding
    unsafe { core_net::destack_net_join_multicast_v4(runtime, handle, group, interface_address) }
}

/// Join an IPv6 multicast group.
pub fn destack_net_join_multicast_v6(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    group: vm::StringHandle,
    interface_index: u32,
) -> RuntimeResult<()> {
    // resolve the multicast group
    let group = host_from_vm(runtime, context, group)?;

    // dispatch to the core binding
    unsafe { core_net::destack_net_join_multicast_v6(runtime, handle, group, interface_index) }
}

/// Leave an IPv4 multicast group.
pub fn destack_net_leave_multicast_v4(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    group: vm::StringHandle,
    interface_address: vm::StringHandle,
) -> RuntimeResult<()> {
    // resolve the multicast group
    let group = host_from_vm(runtime, context, group)?;

    // resolve the interface address
    let interface_address = host_from_vm(runtime, context, interface_address)?;

    // dispatch to the core binding
    unsafe { core_net::destack_net_leave_multicast_v4(runtime, handle, group, interface_address) }
}

/// Leave an IPv6 multicast group.
pub fn destack_net_leave_multicast_v6(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    group: vm::StringHandle,
    interface_index: u32,
) -> RuntimeResult<()> {
    // resolve the multicast group
    let group = host_from_vm(runtime, context, group)?;

    // dispatch to the core binding
    unsafe { core_net::destack_net_leave_multicast_v6(runtime, handle, group, interface_index) }
}

/// Enable or disable multicast loopback.
pub fn destack_net_set_multicast_loop(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // dispatch to the core binding
    unsafe { core_net::destack_net_set_multicast_loop(runtime, handle, enabled) }
}

/// Set multicast TTL.
pub fn destack_net_set_multicast_ttl(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    // dispatch to the core binding
    unsafe { core_net::destack_net_set_multicast_ttl(runtime, handle, ttl) }
}

/// Set the IP time-to-live.
pub fn destack_net_set_ttl(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_ttl(runtime, handle, ttl) }
}

/// Set the IP type-of-service field.
pub fn destack_net_set_tos(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_tos(runtime, handle, tos) }
}

/// Set the read timeout in milliseconds.
pub fn destack_net_set_read_timeout(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_read_timeout(runtime, handle, timeout_ms) }
}

/// Set the write timeout in milliseconds.
pub fn destack_net_set_write_timeout(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_write_timeout(runtime, handle, timeout_ms) }
}

/// Restrict an IPv6 socket to IPv6 traffic only.
pub fn destack_net_set_only_v6(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_set_only_v6(runtime, handle, enabled) }
}

/// Read TCP keepalive settings.
pub fn destack_net_get_keep_alive(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<KeepAliveConfig> {
    call_out(|out| unsafe { core_net::destack_net_get_keep_alive(runtime, out, handle) })
}

/// Read TCP_NODELAY.
pub fn destack_net_get_no_delay(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { core_net::destack_net_get_no_delay(runtime, out, handle) })
}

/// Read SO_REUSEADDR.
pub fn destack_net_get_reuse_addr(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { core_net::destack_net_get_reuse_addr(runtime, out, handle) })
}

/// Read SO_REUSEPORT.
pub fn destack_net_get_reuse_port(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { core_net::destack_net_get_reuse_port(runtime, out, handle) })
}

/// Read socket linger settings.
pub fn destack_net_get_linger(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<Linger> {
    call_out(|out| unsafe { core_net::destack_net_get_linger(runtime, out, handle) })
}

/// Read receive buffer size.
pub fn destack_net_get_recv_buffer(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { core_net::destack_net_get_recv_buffer(runtime, out, handle) })
}

/// Read send buffer size.
pub fn destack_net_get_send_buffer(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { core_net::destack_net_get_send_buffer(runtime, out, handle) })
}

/// Read broadcast mode.
pub fn destack_net_get_broadcast(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { core_net::destack_net_get_broadcast(runtime, out, handle) })
}

/// Read IP time to live.
pub fn destack_net_get_ttl(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { core_net::destack_net_get_ttl(runtime, out, handle) })
}

/// Read IP type of service.
pub fn destack_net_get_tos(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { core_net::destack_net_get_tos(runtime, out, handle) })
}

/// Read read timeout in milliseconds.
pub fn destack_net_get_read_timeout(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { core_net::destack_net_get_read_timeout(runtime, out, handle) })
}

/// Read write timeout in milliseconds.
pub fn destack_net_get_write_timeout(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { core_net::destack_net_get_write_timeout(runtime, out, handle) })
}

/// Read IPv6-only mode.
pub fn destack_net_get_only_v6(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { core_net::destack_net_get_only_v6(runtime, out, handle) })
}

fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    // allocate space for the output and invoke the call
    let mut value = std::mem::MaybeUninit::<T>::uninit();
    call(value.as_mut_ptr())?;
    Ok(unsafe { value.assume_init() })
}

fn host_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    host: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    // resolve the VM string
    let host_ref = context
        .string_ref(host)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    // store it in the runtime arena
    Ok(runtime.store_string(host_ref.as_str()))
}

fn buffer_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<NativeSlice<u8>> {
    // copy the VM buffer into runtime storage
    let bytes = buffer.read_bytes(context)?;
    Ok(runtime.store_slice(bytes))
}

fn allocate_read_buffer(runtime: &RuntimeCallContext, buffer: VmSlice<u8>) -> NativeSlice<u8> {
    // allocate a native buffer for reads
    let length = buffer.len as usize;
    runtime.store_slice(vec![0u8; length])
}

fn write_read_buffer(
    context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
    native: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // copy bytes back into the VM buffer
    let bytes = unsafe { native.as_slice()? };
    buffer.write_bytes(context, bytes)
}

fn decode_buffer_slices(
    context: &mut vm::RuntimeContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<Vec<VmSlice<u8>>> {
    let values = buffers.raw_values(context)?;
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        decoded.push(VmSlice::from_value(
            context,
            value,
            "buffers",
            "Slice<uint8>",
        )?);
    }
    Ok(decoded)
}

#[allow(clippy::type_complexity)]
fn allocate_read_buffers(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<(NativeSlice<NativeSlice<u8>>, Vec<VmSlice<u8>>)> {
    let vm_buffers = decode_buffer_slices(context, buffers)?;
    let mut native_buffers = Vec::with_capacity(vm_buffers.len());
    for buffer in vm_buffers.iter() {
        let length = buffer.len as usize;
        native_buffers.push(runtime.store_slice(vec![0u8; length]));
    }
    let native_slice = runtime.store_slice(native_buffers);

    Ok((native_slice, vm_buffers))
}

fn write_read_buffers(
    context: &mut vm::RuntimeContext<'_>,
    vm_buffers: Vec<VmSlice<u8>>,
    native_buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let native_buffers = unsafe { native_buffers.as_slice()? };
    if native_buffers.len() != vm_buffers.len() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "buffers",
            "buffer length mismatch",
        ))
        .boxed());
    }

    for (vm_buffer, native_buffer) in vm_buffers.into_iter().zip(native_buffers.iter()) {
        let bytes = unsafe { native_buffer.as_slice()? };
        vm_buffer.write_bytes(context, bytes)?;
    }

    Ok(())
}

fn buffers_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<NativeSlice<NativeSlice<u8>>> {
    let vm_buffers = decode_buffer_slices(context, buffers)?;
    let mut native_buffers = Vec::with_capacity(vm_buffers.len());
    for buffer in vm_buffers {
        let bytes = buffer.read_bytes(context)?;
        native_buffers.push(runtime.store_slice(bytes));
    }
    Ok(runtime.store_slice(native_buffers))
}

fn socket_address_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    address: SocketAddress,
) -> RuntimeResult<SocketAddressVm> {
    // decode raw address bytes
    let bytes = unsafe { address.bytes.as_slice()? };
    let bytes = VmArray::from_values(context, bytes)?;

    // build the VM socket address
    Ok(SocketAddressVm {
        family: address.family,
        length: address.length,
        bytes,
    })
}

fn socket_address_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    address: SocketAddressVm,
) -> RuntimeResult<SocketAddress> {
    let bytes = address.bytes.read_bytes(context)?;

    Ok(SocketAddress {
        family: address.family,
        length: address.length,
        bytes: runtime.store_array(bytes),
    })
}

fn socket_address_raw_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    address: SocketAddress,
) -> RuntimeResult<SocketAddressVm> {
    socket_address_to_vm(context, address)
}

fn socket_address_raw_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    address: SocketAddressVm,
) -> RuntimeResult<SocketAddress> {
    socket_address_from_vm(runtime, context, address)
}

fn socket_address_raw_array_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    array: NativeArray<SocketAddress>,
) -> RuntimeResult<VmArray<SocketAddressVm>> {
    let items = unsafe { array.as_slice()? };
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let raw_value = socket_address_raw_to_vm(context, *item)?;
        let bytes = raw_value.bytes.to_value(context);
        let family = vm::Value::uint(raw_value.family as u64, 16);
        let value = context.allocate_aggregate(vec![family, bytes]);
        values.push(value);
    }
    let pointer = context.allocate_raw_values(values);
    Ok(VmArray {
        data: pointer,
        len: items.len() as u32,
        capacity: items.len() as u32,
        _marker: std::marker::PhantomData::<SocketAddressVm>,
    })
}

fn u64_array_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    values: NativeArray<u64>,
) -> RuntimeResult<VmArray<u64>> {
    let values = unsafe { values.as_slice()? };
    VmArray::from_values(context, values)
}

fn string_array_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    array: NativeArray<NativeStringRef>,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let values = unsafe { array.as_slice()? };
    let mut handles = Vec::with_capacity(values.len());
    for value in values {
        let name = unsafe { value.as_str()? };
        let handle = vm::StringHandle::new(context.intern_string(name));
        handles.push(handle);
    }
    VmArray::from_values(context, &handles)
}

fn udp_receive_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    receive: UdpReceive,
) -> RuntimeResult<UdpReceiveVm> {
    let address = socket_address_raw_to_vm(context, receive.address)?;
    Ok(UdpReceiveVm {
        address,
        bytes: receive.bytes,
        recv_flags: receive.recv_flags,
    })
}

fn udp_receive_raw_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    receive: crate::platform::net::UdpReceive,
) -> RuntimeResult<UdpReceiveVm> {
    udp_receive_to_vm(context, receive)
}

fn socket_send_to_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    send_to: SocketSendToVm,
) -> RuntimeResult<crate::platform::net::SocketSendTo> {
    let address = socket_address_raw_from_vm(runtime, context, send_to.address)?;
    Ok(crate::platform::net::SocketSendTo {
        address,
        flags: send_to.flags,
    })
}

fn socket_recv_from_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    recv_from: crate::platform::net::SocketRecvFrom,
) -> RuntimeResult<SocketRecvFromVm> {
    let address = socket_address_raw_to_vm(context, recv_from.address)?;
    Ok(SocketRecvFromVm {
        bytes: recv_from.bytes,
        address,
        recv_flags: recv_from.recv_flags,
    })
}

fn socket_send_message_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    message: SocketSendMessageVm,
) -> RuntimeResult<SocketSendMessage> {
    let fds = message.fds.read_values(context)?;
    let fds = runtime.store_array(fds);
    Ok(SocketSendMessage {
        fds,
        flags: message.flags,
        has_credentials: message.has_credentials,
        credentials: SocketCredentials {
            pid: message.credentials.pid,
            uid: message.credentials.uid,
            gid: message.credentials.gid,
        },
    })
}

fn socket_recv_message_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    message: SocketRecvMessage,
) -> RuntimeResult<SocketRecvMessageVm> {
    let fds = unsafe { message.fds.as_slice()? };
    let fds = VmArray::from_values(context, fds)?;
    Ok(SocketRecvMessageVm {
        bytes: message.bytes,
        recv_flags: message.recv_flags,
        payload_truncated: message.payload_truncated,
        control_truncated: message.control_truncated,
        fds,
        has_credentials: message.has_credentials,
        credentials: SocketCredentialsVm {
            pid: message.credentials.pid,
            uid: message.credentials.uid,
            gid: message.credentials.gid,
        },
    })
}

fn path_ref_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<OsPath> {
    match path.encoding {
        PathEncoding::Bytes => {
            let bytes = path.bytes.0.read_bytes(context)?;
            let bytes = PathBytesAbi::<NativeAbi>(runtime.store_array(bytes));
            let utf16 = PathUtf16Abi::<NativeAbi>(runtime.store_array(Vec::new()));
            Ok(OsPath {
                encoding: PathEncoding::Bytes,
                bytes,
                utf16,
            })
        }
        PathEncoding::Utf16 => {
            let units = path.utf16.0.read_values(context)?;
            let utf16 = PathUtf16Abi::<NativeAbi>(runtime.store_array(units));
            let bytes = PathBytesAbi::<NativeAbi>(runtime.store_array(Vec::new()));
            Ok(OsPath {
                encoding: PathEncoding::Utf16,
                bytes,
                utf16,
            })
        }
    }
}
