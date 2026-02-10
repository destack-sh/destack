use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{OsPath, PathBytesAbi, PathEncoding};
use crate::platform::net::{
    KeepAliveConfig, Linger, PacketCaptureOptions, PacketCaptureRecord, PacketTimestampMode,
    ResolveQuery, ReverseLookupFlags, ReverseLookupName, RouteEntry, SocketAddress, SocketFamily,
    SocketMessageFlags, SocketOptionLevel, SocketOptionName, SocketPair, SocketProtocol,
    SocketRecvBatchRequest, SocketRecvFrom, SocketRecvMessage, SocketSendBatchEntry, SocketSendTo,
    SocketTimestampingMode, SocketType, UdpMessageFlags, UdpReceive, UdsAddress, core as core_net,
};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};
use crate::runtime::RuntimeCallContext;

pub(crate) use core_net::{
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
    query: ResolveQuery,
) -> RuntimeResult<()> {
    // validate query components
    let host = resolve_host(context, query)?;
    let port = resolve_port(query)?;

    // delegate to the raw resolver
    unsafe {
        core_net::destack_net_resolve_raw(context, out, host, port, query.family, query.flags)
    }
}

/// Reverse lookup a raw socket address.
pub(crate) unsafe fn destack_net_reverse_lookup(
    context: &RuntimeCallContext,
    out: *mut NativeArray<ReverseLookupName>,
    address: SocketAddress,
    flags: ReverseLookupFlags,
) -> RuntimeResult<()> {
    // reserve non default behavior until flags are implemented
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.net.reverseLookup")).boxed(),
        );
    }

    // resolve hostnames from the address
    let mut hosts =
        std::mem::MaybeUninit::<NativeArray<crate::platform::NativeStringRef>>::uninit();
    unsafe { core_net::destack_net_reverse_lookup_raw(context, hosts.as_mut_ptr(), address) }?;
    let hosts = unsafe { hosts.assume_init() };
    let hosts = unsafe { hosts.as_slice()? };

    // map hostnames into lookup records
    let empty_service = context.store_string("");
    let mut names = Vec::with_capacity(hosts.len());
    for host in hosts {
        names.push(ReverseLookupName {
            host: *host,
            service: empty_service,
        });
    }

    unsafe {
        *out = context.store_array(names);
    }

    Ok(())
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

/// Receive a message with ancillary metadata.
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
    unsafe {
        super::os::destack_net_recv_msg(
            context,
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

/// Receive multiple messages with ancillary metadata.
pub(crate) unsafe fn destack_net_recv_mmsg(
    context: &RuntimeCallContext,
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
            super::os::destack_net_recv_msg(
                context,
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
        *out = context.store_array(messages);
    }

    Ok(())
}

/// Send multiple messages with ancillary metadata.
pub(crate) unsafe fn destack_net_send_mmsg(
    context: &RuntimeCallContext,
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
            core_net::destack_net_send_msg(
                context,
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

/// Connect to a unix-domain socket endpoint.
pub(crate) unsafe fn destack_net_uds_connect(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    address: UdsAddress,
) -> RuntimeResult<()> {
    // resolve the low level uds address into a path encoding
    let path = uds_path(context, address)?;

    // delegate to the os implementation
    unsafe { super::os::destack_net_uds_connect(context, out, path) }
}

/// Listen on a unix-domain socket endpoint.
pub(crate) unsafe fn destack_net_uds_listen(
    context: &RuntimeCallContext,
    out: *mut ListenerHandle,
    address: UdsAddress,
    backlog: u32,
) -> RuntimeResult<()> {
    // resolve the low level uds address into a path encoding
    let path = uds_path(context, address)?;

    // delegate to the os implementation
    unsafe { super::os::destack_net_uds_listen(context, out, path, backlog) }
}

/// Create a connected unix domain socket pair.
pub(crate) unsafe fn destack_net_uds_socket_pair(
    context: &RuntimeCallContext,
    out: *mut SocketPair,
    sockettype: SocketType,
) -> RuntimeResult<()> {
    unsafe { core_net::destack_net_uds_socket_pair(context, out, sockettype) }
}

/// Resolve a host component from a resolve query.
fn resolve_host(
    _context: &RuntimeCallContext,
    query: ResolveQuery,
) -> RuntimeResult<crate::platform::NativeStringRef> {
    if !query.has_host {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "query",
            "host is required",
        ))
        .boxed());
    }

    Ok(query.host)
}

/// Resolve a service component from a resolve query.
fn resolve_port(query: ResolveQuery) -> RuntimeResult<u16> {
    if !query.has_service {
        return Ok(0);
    }

    let service = unsafe { query.service.as_str()? };
    service.parse::<u16>().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "query",
            "service must be a numeric port",
        ))
        .boxed()
    })
}

/// Map a UDS address into the path shape expected by OS backends.
fn uds_path(context: &RuntimeCallContext, address: UdsAddress) -> RuntimeResult<OsPath> {
    match address.kind {
        crate::platform::net::UdsAddressKind::Path => Ok(address.path),
        crate::platform::net::UdsAddressKind::Abstract => {
            let name = unsafe { address.abstract_name.as_slice()? };
            let mut bytes = Vec::with_capacity(name.len().saturating_add(1));
            bytes.push(0);
            bytes.extend_from_slice(name);
            let data = PathBytesAbi(context.store_array(bytes));
            Ok(OsPath {
                encoding: PathEncoding::Bytes,
                data,
            })
        }
        crate::platform::net::UdsAddressKind::Unnamed => {
            Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsConnect")).boxed())
        }
    }
}

/// Return a standard not supported error for one net binding.
fn missing_binding(binding_name: &'static str) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(binding_name)).boxed())
}

/// Validate one required output pointer.
unsafe fn check_out_pointer<T>(out: *mut T, name: &'static str) -> RuntimeResult<()> {
    // reject null pointers explicitly
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer(name)).boxed());
    }

    Ok(())
}

/// Read the packet mark for one socket.
pub(crate) unsafe fn destack_net_get_packet_mark(
    _context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, handle);

    missing_binding("destack.net.getPacketMark")
}

/// Read one raw socket option.
pub(crate) unsafe fn destack_net_get_sock_opt_raw(
    _context: &RuntimeCallContext,
    out: *mut NativeArray<u8>,
    handle: SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    maxbytes: u32,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, handle, level, name, maxbytes);

    missing_binding("destack.net.getSockOptRaw")
}

/// Read the timestamping mode for one socket.
pub(crate) unsafe fn destack_net_get_timestamping(
    _context: &RuntimeCallContext,
    out: *mut SocketTimestampingMode,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, handle);

    missing_binding("destack.net.getTimestamping")
}

/// Resolve one network interface name to an index.
pub(crate) unsafe fn destack_net_interface_index(
    _context: &RuntimeCallContext,
    out: *mut u32,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, name);

    missing_binding("destack.net.interfaceIndex")
}

/// Resolve one interface index to a name.
pub(crate) unsafe fn destack_net_interface_name(
    _context: &RuntimeCallContext,
    out: *mut NativeStringRef,
    index: u32,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, index);

    missing_binding("destack.net.interfaceName")
}

/// List network interfaces.
pub(crate) unsafe fn destack_net_list_interfaces(
    _context: &RuntimeCallContext,
    out: *mut NativeArray<crate::platform::net::NetInterface>,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = out;

    missing_binding("destack.net.listInterfaces")
}

/// Open one packet socket.
pub(crate) unsafe fn destack_net_packet_open(
    _context: &RuntimeCallContext,
    out: *mut SocketHandle,
    options: PacketCaptureOptions,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, options);

    missing_binding("destack.net.packetOpen")
}

/// Receive one packet capture record.
pub(crate) unsafe fn destack_net_packet_receive(
    _context: &RuntimeCallContext,
    out: *mut PacketCaptureRecord,
    handle: SocketHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, handle, payload);

    missing_binding("destack.net.packetReceive")
}

/// Send one packet through a packet socket.
pub(crate) unsafe fn destack_net_packet_send(
    _context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    payload: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, handle, payload);

    missing_binding("destack.net.packetSend")
}

/// Configure packet timestamping mode.
pub(crate) unsafe fn destack_net_packet_set_timestamp_mode(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    mode: PacketTimestampMode,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = (handle, mode);

    missing_binding("destack.net.packetSetTimestampMode")
}

/// Configure raw socket header included mode.
pub(crate) unsafe fn destack_net_raw_set_header_included(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = (handle, enabled);

    missing_binding("destack.net.rawSetHeaderIncluded")
}

/// Open one raw socket.
pub(crate) unsafe fn destack_net_raw_socket(
    _context: &RuntimeCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
    protocol: i32,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, family, protocol);

    missing_binding("destack.net.rawSocket")
}

/// Add one route entry.
pub(crate) unsafe fn destack_net_route_add(
    _context: &RuntimeCallContext,
    route: RouteEntry,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = route;

    missing_binding("destack.net.routeAdd")
}

/// Delete one route entry.
pub(crate) unsafe fn destack_net_route_delete(
    _context: &RuntimeCallContext,
    route: RouteEntry,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = route;

    missing_binding("destack.net.routeDelete")
}

/// List route entries for one family.
pub(crate) unsafe fn destack_net_route_list(
    _context: &RuntimeCallContext,
    out: *mut NativeArray<RouteEntry>,
    family: SocketFamily,
) -> RuntimeResult<()> {
    // validate pointer and keep arguments used
    unsafe { check_out_pointer(out, "out")? };
    let _ = (out, family);

    missing_binding("destack.net.routeList")
}

/// Set the packet mark for one socket.
pub(crate) unsafe fn destack_net_set_packet_mark(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    mark: u32,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = (handle, mark);

    missing_binding("destack.net.setPacketMark")
}

/// Write one raw socket option.
pub(crate) unsafe fn destack_net_set_sock_opt_raw(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = (handle, level, name, value);

    missing_binding("destack.net.setSockOptRaw")
}

/// Set the timestamping mode for one socket.
pub(crate) unsafe fn destack_net_set_timestamping(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    mode: SocketTimestampingMode,
) -> RuntimeResult<()> {
    // keep arguments used
    let _ = (handle, mode);

    missing_binding("destack.net.setTimestamping")
}
