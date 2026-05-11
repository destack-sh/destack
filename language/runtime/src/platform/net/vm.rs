use destack_vm;

use super::host;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeAbi, VmAbi};
use crate::platform::core::{
    allocate_vm_read_buffer as allocate_read_buffer,
    allocate_vm_read_buffers as allocate_read_buffers, bytes_array_to_vm, call_out,
    map_native_array_to_vm, map_native_slice_to_vm, store_bytes_array_from_vm,
    store_bytes_from_vm as buffer_from_vm, store_os_path_from_vm as path_ref_from_vm,
    store_string_from_vm as host_from_vm, store_values_array_from_vm,
    store_vm_byte_slices as buffers_from_vm, string_array_to_vm, values_array_to_vm,
    write_vm_read_buffer as write_read_buffer, write_vm_read_buffers as write_read_buffers,
};
use crate::platform::fs::{OsPath, OsPathBytes, PathBytesAbi};
use crate::platform::net::{
    AcceptFlags, KeepAliveConfig, Linger, NetInterface, NetInterfaceVm, PacketBackendDescriptor,
    PacketBackendDescriptorVm, PacketCaptureOptionsVm, PacketCaptureRecordVm, PacketCaptureStatsVm,
    PacketFanoutOptionsVm, PacketRingOptionsVm, PacketTimestampMode, ResolveFlags, ResolveQueryVm,
    ReverseLookupFlags, ReverseLookupNameVm, RouteEntry, RouteEntryVm, SocketAddress,
    SocketAddressVm, SocketControlBufferAbi, SocketCredentials, SocketCredentialsVm, SocketFamily,
    SocketMessageFlags, SocketOptionLevel, SocketOptionName, SocketPair, SocketPairVm,
    SocketProtocol, SocketRecvBatchRequestVm, SocketRecvFrom, SocketRecvFromVm, SocketRecvMessage,
    SocketRecvMessageVm, SocketSendBatchEntryVm, SocketSendMessage, SocketSendMessageVm,
    SocketSendTo, SocketSendToVm, SocketShutdown, SocketTimestampingMode, SocketType,
    UdpMessageFlags, UdpReceive, UdpReceiveVm, UdpSourceMembershipV4, UdpSourceMembershipV4Vm,
    UdpSourceMembershipV6, UdpSourceMembershipV6Vm, UdsAbstractAddress, UdsAddress, UdsAddressVm,
    UdsPathAddress, UdsUnnamedAddress, host as host_net,
};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeArray, PlatformError, VmArray, VmSlice};

use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::runtime::BindingCallContext;

/// Accept a new connection from a listener.
pub fn destack_net_accept(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    listener: ListenerHandle,
    flags: AcceptFlags,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_accept(binding, out, listener, flags) })
}

/// Close a socket handle.
pub fn destack_net_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_close(binding, handle) }
}

/// Connect to a remote socket address.
pub fn destack_net_connect(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let address = socket_address_from_vm(binding, context, address)?;
    unsafe { host_net::destack_net_connect_raw(binding, handle, address) }
}

/// Connect to a remote host and return a socket handle.
pub fn destack_net_connect_text(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    host: destack_vm::StringHandle,
    port: u16,
) -> RuntimeResult<SocketHandle> {
    // resolve all candidate remote endpoints from host and port text
    let addresses =
        resolve_text_addresses_from_vm(binding, context, host, port, "destack.net.connectText")?;
    let mut last_error = None;

    // try resolved addresses in order until one socket connects
    for address in addresses {
        let family = match socket_family_from_address(address, "destack.net.connectText") {
            Ok(family) => family,
            Err(error) => {
                last_error = Some(error);
                continue;
            }
        };

        let handle = match call_out(|out| unsafe {
            host_net::destack_net_socket(
                binding,
                out,
                family,
                tcp_stream_socket_type(),
                tcp_socket_protocol(),
            )
        }) {
            Ok(handle) => handle,
            Err(error) => {
                last_error = Some(error);
                continue;
            }
        };

        let result = unsafe { host_net::destack_net_connect_raw(binding, handle, address) };
        match result {
            Ok(()) => return Ok(handle),
            Err(error) => {
                let _ = unsafe { host_net::destack_net_close(binding, handle) };
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "destack.net.connectText: host and port resolved to no usable addresses",
        ))
        .boxed()
    }))
}

/// Start listening on a raw socket address.
pub fn destack_net_listen(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    address: SocketAddressVm,
    backlog: u32,
) -> RuntimeResult<ListenerHandle> {
    let address = socket_address_from_vm(binding, context, address)?;
    call_out(|out| unsafe { host_net::destack_net_listen_raw(binding, out, address, backlog) })
}

/// Start listening on a host and port.
pub fn destack_net_listen_text(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    host: destack_vm::StringHandle,
    port: u16,
    backlog: u32,
) -> RuntimeResult<ListenerHandle> {
    // resolve all candidate local endpoints from host and port text
    let addresses =
        resolve_text_addresses_from_vm(binding, context, host, port, "destack.net.listenText")?;
    let mut last_error = None;

    // try resolved addresses in order until one listener binds
    for address in addresses {
        match call_out(|out| unsafe {
            host_net::destack_net_listen_raw(binding, out, address, backlog)
        }) {
            Ok(listener) => return Ok(listener),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "destack.net.listenText: host and port resolved to no usable addresses",
        ))
        .boxed()
    }))
}

/// Read from a socket into the provided slice.
pub fn destack_net_read(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // allocate a native buffer for reads
    let native_buffer = allocate_read_buffer(binding, buffer);

    // perform the read
    let bytes_read =
        call_out(|out| unsafe { host_net::destack_net_read(binding, out, handle, native_buffer) })?;

    // write results back into the VM buffer
    write_read_buffer(context, buffer, native_buffer)?;

    Ok(bytes_read)
}

/// Write to a socket from the provided slice.
pub fn destack_net_write(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // resolve the VM buffer into native storage
    let native_buffer = buffer_from_vm(binding, context, buffer)?;

    // dispatch to the core binding
    call_out(|out| unsafe { host_net::destack_net_write(binding, out, handle, native_buffer) })
}

/// Receive a message with ancillary data.
pub fn destack_net_recv_msg(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    recv_flags: SocketMessageFlags,
    max_fds: u32,
    want_credentials: bool,
    max_control_bytes: u32,
) -> RuntimeResult<SocketRecvMessageVm> {
    // allocate a native receive buffer
    let native_buffer = allocate_read_buffer(binding, buffer);

    // receive one message through the os implementation
    let message = call_out(|out| unsafe {
        host::destack_net_recv_msg(
            binding,
            out,
            handle,
            native_buffer,
            recv_flags,
            max_fds,
            want_credentials,
            max_control_bytes,
        )
    })?;

    // write payload bytes back into the VM slice
    write_read_buffer(context, buffer, native_buffer)?;

    socket_recv_message_to_vm(context, message)
}

/// Receive multiple datagrams.
pub fn destack_net_recv_mmsg(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    requests: VmSlice<SocketRecvBatchRequestVm>,
    max_fds: u32,
    want_credentials: bool,
    max_control_bytes: u32,
) -> RuntimeResult<VmArray<SocketRecvMessageVm>> {
    // decode the per message request values
    let requests = requests.read_values(&context.read())?;
    let mut messages = Vec::with_capacity(requests.len());

    // receive each requested message through the host lane
    for request in requests {
        let native_buffer = allocate_read_buffer(binding, request.payload);
        let message = call_out(|out| unsafe {
            host::destack_net_recv_msg(
                binding,
                out,
                handle,
                native_buffer,
                request.recv_flags,
                max_fds,
                want_credentials,
                max_control_bytes,
            )
        })?;

        write_read_buffer(context, request.payload, native_buffer)?;
        messages.push(socket_recv_message_to_vm(context, message)?);
    }

    VmArray::from_values(&mut context.write(), &messages)
}

/// Send a message with ancillary data.
pub fn destack_net_send_msg(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    message: SocketSendMessageVm,
) -> RuntimeResult<u64> {
    // resolve the VM buffer into native storage
    let native_buffer = buffer_from_vm(binding, context, buffer)?;

    // convert the metadata into native values
    let message = socket_send_message_from_vm(binding, context, message)?;

    // dispatch to the core binding
    call_out(|out| unsafe {
        host_net::destack_net_send_msg(binding, out, handle, native_buffer, message)
    })
}

/// Receive a packet from a remote socket address.
pub fn destack_net_recv_from(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    recv_flags: SocketMessageFlags,
) -> RuntimeResult<SocketRecvFromVm> {
    let native = allocate_read_buffer(binding, buffer);
    let receive = call_out(|out| unsafe {
        host_net::destack_net_recv_from(binding, out, handle, native, recv_flags)
    })?;
    write_read_buffer(context, buffer, native)?;
    socket_recv_from_to_vm(context, receive)
}

/// Send a packet to a remote socket address.
pub fn destack_net_send_to(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    message: SocketSendToVm,
) -> RuntimeResult<u64> {
    let native_buffer = buffer_from_vm(binding, context, buffer)?;
    let native_message = socket_send_to_from_vm(binding, context, message)?;
    call_out(|out| unsafe {
        host_net::destack_net_send_to(binding, out, handle, native_buffer, native_message)
    })
}

/// Send multiple datagrams.
pub fn destack_net_send_mmsg(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    messages: VmSlice<SocketSendBatchEntryVm>,
) -> RuntimeResult<u64> {
    // decode the per message send entries
    let messages = messages.read_values(&context.read())?;
    let mut sent_count = 0u64;

    // send each entry via the host lane
    for message in messages {
        let native_buffer = buffer_from_vm(binding, context, message.payload)?;
        let native_message = socket_send_message_from_vm(binding, context, message.message)?;
        call_out(|out| unsafe {
            host_net::destack_net_send_msg(binding, out, handle, native_buffer, native_message)
        })?;
        sent_count = sent_count.saturating_add(1);
    }

    Ok(sent_count)
}

/// Shut down a socket for reads, writes, or both.
pub fn destack_net_shutdown(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_shutdown(binding, handle, how) }
}

/// Enable or disable nonblocking mode on a socket.
pub fn destack_net_set_nonblocking(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_nonblocking(binding, handle, enabled) }
}

/// Read the local socket address as raw bytes.
pub fn destack_net_local_address(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    // read the address via the core binding
    let address =
        call_out(|out| unsafe { host_net::destack_net_local_address_raw(binding, out, handle) })?;
    socket_address_to_vm(context, address)
}

/// Read the local socket address as normalized text metadata.
pub fn destack_net_local_address_text(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    // read the address via the core binding
    let address =
        call_out(|out| unsafe { host_net::destack_net_local_address_raw(binding, out, handle) })?;

    // map the raw address into vm value layout
    socket_address_to_vm(context, address)
}

/// Read the remote socket address as raw bytes.
pub fn destack_net_peer_address(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    // read the address via the core binding
    let address =
        call_out(|out| unsafe { host_net::destack_net_peer_address_raw(binding, out, handle) })?;
    socket_address_to_vm(context, address)
}

/// Read the remote socket address as normalized text metadata.
pub fn destack_net_peer_address_text(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    // read the address via the core binding
    let address =
        call_out(|out| unsafe { host_net::destack_net_peer_address_raw(binding, out, handle) })?;

    // map the raw address into vm value layout
    socket_address_to_vm(context, address)
}

/// Enable or disable TCP_NODELAY.
pub fn destack_net_set_no_delay(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_no_delay(binding, handle, enabled) }
}

/// Write full TCP keepalive parameters.
pub fn destack_net_set_keep_alive(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
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

/// Enable or disable SO_REUSEADDR.
pub fn destack_net_set_reuse_addr(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_reuse_addr(binding, handle, enabled) }
}

/// Enable or disable SO_REUSEPORT.
pub fn destack_net_set_reuse_port(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_reuse_port(binding, handle, enabled) }
}

/// Close a listener handle.
pub fn destack_net_close_listener(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_close_listener(binding, handle) }
}

/// Create a socket from a native family, type, and protocol.
pub fn destack_net_socket(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    family: SocketFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe {
        host_net::destack_net_socket(binding, out, family, socket_type, protocol)
    })
}

/// Create a connected socket pair.
pub fn destack_net_socket_pair(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    family: SocketFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<SocketPairVm> {
    call_out(|out| unsafe {
        host_net::destack_net_socket_pair(binding, out, family, socket_type, protocol)
    })
}

/// Bind an existing socket to a raw address.
pub fn destack_net_bind(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let address = socket_address_from_vm(binding, context, address)?;
    unsafe { host_net::destack_net_bind(binding, handle, address) }
}

/// Read into multiple buffers.
pub fn destack_net_readv(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let (native_buffers, vm_buffers) = allocate_read_buffers(binding, context, buffers, "buffers")?;
    let count = call_out(|out| unsafe {
        host_net::destack_net_readv(binding, out, handle, native_buffers)
    })?;
    write_read_buffers(context, vm_buffers, native_buffers, "buffers")?;
    Ok(count)
}

/// Write from multiple buffers.
pub fn destack_net_writev(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let native_buffers = buffers_from_vm(binding, context, buffers, "buffers")?;
    call_out(|out| unsafe { host_net::destack_net_writev(binding, out, handle, native_buffers) })
}

/// Resolve a host and service query into raw socket addresses.
pub fn destack_net_resolve(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    query: ResolveQueryVm,
) -> RuntimeResult<VmArray<SocketAddressVm>> {
    // decode query host and service values
    let (host, service) = resolve_query_parts_from_vm(binding, context, query)?;

    // resolve via raw resolver and map to VM addresses
    let addresses = call_out(|out| unsafe {
        host_net::destack_net_resolve_raw(binding, out, host, service, query.family, query.flags)
    })?;
    socket_address_array_to_vm(context, addresses)
}

/// Resolve a hostname and port into normalized socket addresses.
pub fn destack_net_resolve_text(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    host: destack_vm::StringHandle,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<VmArray<SocketAddressVm>> {
    // resolve the host string into native storage
    let host = host_from_vm(binding, context, host)?;
    let service = resolve_service_text(binding, port);

    // resolve via raw resolver and map to VM addresses
    let addresses = call_out(|out| unsafe {
        host_net::destack_net_resolve_raw(binding, out, Some(host), Some(service), family, flags)
    })?;
    socket_address_array_to_vm(context, addresses)
}

/// Reverse lookup a raw socket address into host and service names.
pub fn destack_net_reverse_lookup(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    address: SocketAddressVm,
    flags: ReverseLookupFlags,
) -> RuntimeResult<VmArray<ReverseLookupNameVm>> {
    // decode address and execute reverse lookup
    let address = socket_address_from_vm(binding, context, address)?;
    let names = call_out(|out| unsafe {
        host_net::destack_net_reverse_lookup_names_raw(binding, out, address, flags)
    })?;

    // map native lookup records into vm lookup records
    map_native_array_to_vm(context, names, |context, name| {
        let host = unsafe { name.host.as_str()? };
        let service = unsafe { name.service.as_str()? };

        let host = context
            .string_handle(host)
            .map_err(Box::<RuntimeError>::from)?;
        let service = context
            .string_handle(service)
            .map_err(Box::<RuntimeError>::from)?;
        Ok(ReverseLookupNameVm { host, service })
    })
}

/// Reverse lookup a normalized socket address into hostnames.
pub fn destack_net_reverse_lookup_text(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    address: SocketAddressVm,
) -> RuntimeResult<VmArray<destack_vm::StringHandle>> {
    // decode address and execute reverse lookup
    let address = socket_address_from_vm(binding, context, address)?;
    let hosts =
        call_out(|out| unsafe { host_net::destack_net_reverse_lookup_raw(binding, out, address) })?;
    string_array_to_vm(context, hosts)
}

/// Create a UDP socket.
pub fn destack_net_udp_socket(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    family: SocketFamily,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_udp_socket(binding, out, family) })
}

/// Bind a UDP socket to a raw local address.
pub fn destack_net_udp_bind(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let address = socket_address_from_vm(binding, context, address)?;
    unsafe { host_net::destack_net_udp_bind_raw(binding, handle, address) }
}

/// Bind a UDP socket to a host and port.
pub fn destack_net_udp_bind_text(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    host: destack_vm::StringHandle,
    port: u16,
) -> RuntimeResult<()> {
    // resolve all candidate local endpoints from host and port text
    let addresses =
        resolve_text_addresses_from_vm(binding, context, host, port, "destack.net.udpBindText")?;
    let mut last_error = None;

    // try resolved addresses in order until one bind succeeds
    for address in addresses {
        match unsafe { host_net::destack_net_udp_bind_raw(binding, handle, address) } {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "destack.net.udpBindText: host and port resolved to no usable addresses",
        ))
        .boxed()
    }))
}

/// Connect a UDP socket to a raw remote address.
pub fn destack_net_udp_connect(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let address = socket_address_from_vm(binding, context, address)?;
    unsafe { host_net::destack_net_udp_connect_raw(binding, handle, address) }
}

/// Connect a UDP socket to a host and port.
pub fn destack_net_udp_connect_text(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    host: destack_vm::StringHandle,
    port: u16,
) -> RuntimeResult<()> {
    // resolve all candidate remote endpoints from host and port text
    let addresses =
        resolve_text_addresses_from_vm(binding, context, host, port, "destack.net.udpConnectText")?;
    let mut last_error = None;

    // try resolved addresses in order until one connect succeeds
    for address in addresses {
        match unsafe { host_net::destack_net_udp_connect_raw(binding, handle, address) } {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "destack.net.udpConnectText: host and port resolved to no usable addresses",
        ))
        .boxed()
    }))
}

/// Receive a datagram from a remote address with raw address output.
pub fn destack_net_udp_recv_from(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    recv_flags: UdpMessageFlags,
) -> RuntimeResult<UdpReceiveVm> {
    let native = allocate_read_buffer(binding, buffer);
    let receive = call_out(|out| unsafe {
        host_net::destack_net_udp_recv_from_raw(binding, out, handle, native, recv_flags)
    })?;
    write_read_buffer(context, buffer, native)?;
    udp_receive_to_vm(context, receive)
}

/// Receive a UDP packet with normalized sender metadata.
pub fn destack_net_udp_recv_from_text(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<UdpReceiveVm> {
    // allocate a native read buffer for the datagram payload
    let native = allocate_read_buffer(binding, buffer);

    // receive one datagram and decode sender metadata
    let receive = call_out(|out| unsafe {
        host_net::destack_net_udp_recv_from_raw(binding, out, handle, native, UdpMessageFlags(0))
    })?;

    // write the payload back into the VM buffer
    write_read_buffer(context, buffer, native)?;

    // map receive metadata to VM representation
    udp_receive_to_vm(context, receive)
}

/// Send a datagram to a raw remote address.
pub fn destack_net_udp_send_to(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
    buffer: VmSlice<u8>,
    send_flags: UdpMessageFlags,
) -> RuntimeResult<u64> {
    let address = socket_address_from_vm(binding, context, address)?;
    let native = buffer_from_vm(binding, context, buffer)?;
    call_out(|out| unsafe {
        host_net::destack_net_udp_send_to_raw(binding, out, handle, address, native, send_flags)
    })
}

/// Send a UDP packet to a host and port.
pub fn destack_net_udp_send_to_text(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    host: destack_vm::StringHandle,
    port: u16,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // resolve VM payload bytes into native storage
    let native = buffer_from_vm(binding, context, buffer)?;

    // resolve all candidate remote endpoints from host and port text
    let addresses =
        resolve_text_addresses_from_vm(binding, context, host, port, "destack.net.udpSendToText")?;
    let mut last_error = None;

    // try resolved addresses in order until one send succeeds
    for address in addresses {
        match call_out(|out| unsafe {
            host_net::destack_net_udp_send_to_raw(
                binding,
                out,
                handle,
                address,
                native,
                UdpMessageFlags(0),
            )
        }) {
            Ok(bytes_sent) => return Ok(bytes_sent),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "destack.net.udpSendToText: host and port resolved to no usable addresses",
        ))
        .boxed()
    }))
}

/// Accept a connection from a UDS listener.
pub fn destack_net_uds_accept(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    listener: ListenerHandle,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_uds_accept(binding, out, listener) })
}

/// Close a UDS listener handle.
pub fn destack_net_uds_close_listener(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_uds_close_listener(binding, handle) }
}

/// Connect to a UDS endpoint.
pub fn destack_net_uds_connect(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    address: UdsAddressVm,
) -> RuntimeResult<SocketHandle> {
    // decode the uds address and delegate to the os implementation
    let address = uds_address_from_vm(binding, context, address)?;
    let path = uds_path(binding, address)?;
    call_out(|out| unsafe { host::destack_net_uds_connect(binding, out, path) })
}

/// Listen on a UDS address.
pub fn destack_net_uds_listen(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    address: UdsAddressVm,
    backlog: u32,
) -> RuntimeResult<ListenerHandle> {
    // decode the uds address and delegate to the os implementation
    let address = uds_address_from_vm(binding, context, address)?;
    let path = uds_path(binding, address)?;
    call_out(|out| unsafe { host::destack_net_uds_listen(binding, out, path, backlog) })
}

/// Create a connected UDS socket pair.
pub fn destack_net_uds_socket_pair(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    socket_type: SocketType,
) -> RuntimeResult<SocketPair> {
    call_out(|out| unsafe { host_net::destack_net_uds_socket_pair(binding, out, socket_type) })
}

/// Set socket linger settings.
pub fn destack_net_set_linger(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    linger: Linger,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_linger(binding, handle, linger) }
}

/// Set the receive buffer size.
pub fn destack_net_set_recv_buffer(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_recv_buffer(binding, handle, size) }
}

/// Set the send buffer size.
pub fn destack_net_set_send_buffer(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_send_buffer(binding, handle, size) }
}

/// Enable or disable broadcast.
pub fn destack_net_set_broadcast(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_broadcast(binding, handle, enabled) }
}

/// Join an IPv4 multicast group.
pub fn destack_net_join_multicast_v4(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    group: destack_vm::StringHandle,
    interface_address: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    // resolve the multicast group
    let group = host_from_vm(binding, context, group)?;

    // resolve the interface address
    let interface_address = host_from_vm(binding, context, interface_address)?;

    // dispatch to the core binding
    unsafe { host_net::destack_net_join_multicast_v4(binding, handle, group, interface_address) }
}

/// Join an IPv6 multicast group.
pub fn destack_net_join_multicast_v6(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    group: destack_vm::StringHandle,
    interface_index: u32,
) -> RuntimeResult<()> {
    // resolve the multicast group
    let group = host_from_vm(binding, context, group)?;

    // dispatch to the core binding
    unsafe { host_net::destack_net_join_multicast_v6(binding, handle, group, interface_index) }
}

/// Leave an IPv4 multicast group.
pub fn destack_net_leave_multicast_v4(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    group: destack_vm::StringHandle,
    interface_address: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    // resolve the multicast group
    let group = host_from_vm(binding, context, group)?;

    // resolve the interface address
    let interface_address = host_from_vm(binding, context, interface_address)?;

    // dispatch to the core binding
    unsafe { host_net::destack_net_leave_multicast_v4(binding, handle, group, interface_address) }
}

/// Leave an IPv6 multicast group.
pub fn destack_net_leave_multicast_v6(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    group: destack_vm::StringHandle,
    interface_index: u32,
) -> RuntimeResult<()> {
    // resolve the multicast group
    let group = host_from_vm(binding, context, group)?;

    // dispatch to the core binding
    unsafe { host_net::destack_net_leave_multicast_v6(binding, handle, group, interface_index) }
}

/// Enable or disable multicast loopback.
pub fn destack_net_set_multicast_loop(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // dispatch to the core binding
    unsafe { host_net::destack_net_set_multicast_loop(binding, handle, enabled) }
}

/// Set multicast TTL.
pub fn destack_net_set_multicast_ttl(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    // dispatch to the core binding
    unsafe { host_net::destack_net_set_multicast_ttl(binding, handle, ttl) }
}

/// Set the IP time-to-live.
pub fn destack_net_set_ttl(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_ttl(binding, handle, ttl) }
}

/// Set the IP type-of-service field.
pub fn destack_net_set_tos(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_tos(binding, handle, tos) }
}

/// Set the read timeout in milliseconds.
pub fn destack_net_set_read_timeout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_read_timeout(binding, handle, timeout_ms) }
}

/// Set the write timeout in milliseconds.
pub fn destack_net_set_write_timeout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_write_timeout(binding, handle, timeout_ms) }
}

/// Restrict an IPv6 socket to IPv6 traffic only.
pub fn destack_net_set_only_v6(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_only_v6(binding, handle, enabled) }
}

/// Read full TCP keepalive parameters.
pub fn destack_net_get_keep_alive(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<KeepAliveConfig> {
    call_out(|out| unsafe { host_net::destack_net_get_keep_alive(binding, out, handle) })
}

/// Read TCP_NODELAY.
pub fn destack_net_get_no_delay(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_no_delay(binding, out, handle) })
}

/// Read SO_REUSEADDR.
pub fn destack_net_get_reuse_addr(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_reuse_addr(binding, out, handle) })
}

/// Read SO_REUSEPORT.
pub fn destack_net_get_reuse_port(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_reuse_port(binding, out, handle) })
}

/// Read socket linger settings.
pub fn destack_net_get_linger(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<Linger> {
    call_out(|out| unsafe { host_net::destack_net_get_linger(binding, out, handle) })
}

/// Read the receive buffer size.
pub fn destack_net_get_recv_buffer(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_recv_buffer(binding, out, handle) })
}

/// Read the send buffer size.
pub fn destack_net_get_send_buffer(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_send_buffer(binding, out, handle) })
}

/// Read broadcast mode.
pub fn destack_net_get_broadcast(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_broadcast(binding, out, handle) })
}

/// Read the IP time-to-live.
pub fn destack_net_get_ttl(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_ttl(binding, out, handle) })
}

/// Read the IP type-of-service field.
pub fn destack_net_get_tos(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_tos(binding, out, handle) })
}

/// Read the read timeout in milliseconds.
pub fn destack_net_get_read_timeout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_read_timeout(binding, out, handle) })
}

/// Read the write timeout in milliseconds.
pub fn destack_net_get_write_timeout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_write_timeout(binding, out, handle) })
}

/// Read IPv6-only mode.
pub fn destack_net_get_only_v6(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_only_v6(binding, out, handle) })
}

fn socket_address_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    address: SocketAddress,
) -> RuntimeResult<SocketAddressVm> {
    // encode the raw address bytes
    let bytes = bytes_array_to_vm(context, address.bytes)?;

    // build the VM socket address
    Ok(SocketAddressVm {
        family: address.family,
        length: address.length,
        bytes,
    })
}

fn socket_address_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    address: SocketAddressVm,
) -> RuntimeResult<SocketAddress> {
    Ok(SocketAddress {
        family: address.family,
        length: address.length,
        bytes: store_bytes_array_from_vm(binding, context, address.bytes)?,
    })
}

fn socket_address_array_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    array: NativeArray<SocketAddress>,
) -> RuntimeResult<VmArray<SocketAddressVm>> {
    map_native_array_to_vm(context, array, |context, item| {
        socket_address_to_vm(context, *item)
    })
}

fn net_interface_array_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    array: NativeArray<NetInterface>,
) -> RuntimeResult<VmArray<NetInterfaceVm>> {
    map_native_array_to_vm(context, array, |context, value| {
        let name = unsafe { value.name.as_str()? };
        let name = context
            .string_handle(name)
            .map_err(Box::<RuntimeError>::from)?;
        let mac_address = bytes_array_to_vm(context, value.mac_address)?;
        let addresses = socket_address_array_to_vm(context, value.addresses)?;

        Ok(NetInterfaceVm {
            name,
            index: value.index,
            flags: value.flags,
            mtu: value.mtu,
            mac_address,
            addresses,
        })
    })
}

fn packet_backend_descriptor_slice_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    value: NativeSlice<PacketBackendDescriptor>,
) -> RuntimeResult<VmSlice<PacketBackendDescriptorVm>> {
    map_native_slice_to_vm(context, value, |context, value| {
        let name = unsafe { value.name.as_str()? };
        Ok(PacketBackendDescriptorVm {
            backend: value.backend,
            name: context
                .string_handle(name)
                .map_err(Box::<RuntimeError>::from)?,
            available: value.available,
            priority: value.priority,
            capability_flags: value.capability_flags,
        })
    })
}

fn route_entry_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    route: RouteEntryVm,
) -> RuntimeResult<RouteEntry> {
    let destination = socket_address_from_vm(binding, context, route.destination)?;
    let gateway = socket_address_from_vm(binding, context, route.gateway)?;

    Ok(RouteEntry {
        family: route.family,
        destination,
        prefix_length: route.prefix_length,
        gateway,
        interface_index: route.interface_index,
        metric: route.metric,
        kind: route.kind,
    })
}

fn route_entry_array_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    array: NativeArray<RouteEntry>,
) -> RuntimeResult<VmArray<RouteEntryVm>> {
    map_native_array_to_vm(context, array, |context, route| {
        let destination = socket_address_to_vm(context, route.destination)?;
        let gateway = socket_address_to_vm(context, route.gateway)?;
        Ok(RouteEntryVm {
            family: route.family,
            destination,
            prefix_length: route.prefix_length,
            gateway,
            interface_index: route.interface_index,
            metric: route.metric,
            kind: route.kind,
        })
    })
}

fn udp_receive_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    receive: UdpReceive,
) -> RuntimeResult<UdpReceiveVm> {
    let address = socket_address_to_vm(context, receive.address)?;
    Ok(UdpReceiveVm {
        address,
        bytes: receive.bytes,
        recv_flags: receive.recv_flags,
    })
}

fn udp_source_membership_v4_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    membership: UdpSourceMembershipV4Vm,
) -> RuntimeResult<UdpSourceMembershipV4> {
    Ok(UdpSourceMembershipV4 {
        group: host_from_vm(binding, context, membership.group)?,
        source: host_from_vm(binding, context, membership.source)?,
        interface_address: host_from_vm(binding, context, membership.interface_address)?,
    })
}

fn udp_source_membership_v6_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    membership: UdpSourceMembershipV6Vm,
) -> RuntimeResult<UdpSourceMembershipV6> {
    Ok(UdpSourceMembershipV6 {
        group: host_from_vm(binding, context, membership.group)?,
        source: host_from_vm(binding, context, membership.source)?,
        interface_index: membership.interface_index,
    })
}

fn socket_send_to_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    send_to: SocketSendToVm,
) -> RuntimeResult<SocketSendTo> {
    let address = socket_address_from_vm(binding, context, send_to.address)?;
    Ok(SocketSendTo {
        address,
        flags: send_to.flags,
    })
}

fn socket_recv_from_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    recv_from: SocketRecvFrom,
) -> RuntimeResult<SocketRecvFromVm> {
    let address = socket_address_to_vm(context, recv_from.address)?;
    Ok(SocketRecvFromVm {
        bytes: recv_from.bytes,
        address,
        recv_flags: recv_from.recv_flags,
    })
}

fn socket_send_message_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    message: SocketSendMessageVm,
) -> RuntimeResult<SocketSendMessage> {
    let fds = store_values_array_from_vm(binding, context, message.fds)?;
    let address = if let Some(address) = message.address {
        let address = socket_address_from_vm(binding, context, address)?;
        Some(address)
    } else {
        None
    };
    let control = store_bytes_array_from_vm(binding, context, message.control.0)?;
    let control = SocketControlBufferAbi::<NativeAbi>(control);
    let credentials = message.credentials.map(|credentials| SocketCredentials {
        pid: credentials.pid,
        uid: credentials.uid,
        gid: credentials.gid,
    });

    Ok(SocketSendMessage {
        address,
        fds,
        control,
        flags: message.flags,
        credentials,
    })
}

fn socket_recv_message_to_vm(
    context: &mut destack_vm::BindingContext<'_>,
    message: SocketRecvMessage,
) -> RuntimeResult<SocketRecvMessageVm> {
    let address = if let Some(address) = message.address {
        let address = socket_address_to_vm(context, address)?;
        Some(address)
    } else {
        None
    };
    let control = bytes_array_to_vm(context, message.control.0)?;
    let control = SocketControlBufferAbi::<VmAbi>(control);
    let fds = values_array_to_vm(context, message.fds)?;
    let credentials = message.credentials.map(|credentials| SocketCredentialsVm {
        pid: credentials.pid,
        uid: credentials.uid,
        gid: credentials.gid,
    });

    Ok(SocketRecvMessageVm {
        bytes: message.bytes,
        address,
        recv_flags: message.recv_flags,
        payload_truncated: message.payload_truncated,
        control_truncated: message.control_truncated,
        control,
        fds,
        credentials,
    })
}

fn uds_address_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    address: UdsAddressVm,
) -> RuntimeResult<UdsAddress> {
    match address {
        UdsAddressVm::UdsPathAddress(path_address) => {
            let path = path_ref_from_vm(binding, context, path_address.path)?;
            Ok(UdsAddress::UdsPathAddress(UdsPathAddress {
                kind: binding.store_string("path"),
                path,
            }))
        }
        UdsAddressVm::UdsAbstractAddress(abstract_address) => {
            let abstract_name =
                store_bytes_array_from_vm(binding, context, abstract_address.abstract_name)?;
            Ok(UdsAddress::UdsAbstractAddress(UdsAbstractAddress {
                kind: binding.store_string("abstract"),
                abstract_name,
            }))
        }
        UdsAddressVm::UdsUnnamedAddress(_) => {
            Ok(UdsAddress::UdsUnnamedAddress(UdsUnnamedAddress {
                kind: binding.store_string("unnamed"),
            }))
        }
    }
}

fn uds_path(binding: &BindingCallContext, address: UdsAddress) -> RuntimeResult<OsPath> {
    match address {
        UdsAddress::UdsPathAddress(UdsPathAddress { path, .. }) => Ok(path),
        UdsAddress::UdsAbstractAddress(UdsAbstractAddress { abstract_name, .. }) => {
            let name = unsafe { abstract_name.as_slice()? };
            let mut bytes = Vec::with_capacity(name.len().saturating_add(1));
            bytes.push(0);
            bytes.extend_from_slice(name);
            let bytes = PathBytesAbi::<NativeAbi>(binding.store_array(bytes));
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

fn resolve_query_parts_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    query: ResolveQueryVm,
) -> RuntimeResult<(Option<NativeStringRef>, Option<NativeStringRef>)> {
    // require at least one query component
    if query.host.is_none() && query.service.is_none() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "query",
            "host or service is required",
        ))
        .boxed());
    }

    // decode the optional host string
    let host = match query.host {
        Some(host) => Some(host_from_vm(binding, context, host)?),
        None => None,
    };

    // decode the optional service string
    let service = match query.service {
        Some(service) => Some(host_from_vm(binding, context, service)?),
        None => None,
    };

    Ok((host, service))
}

fn resolve_service_text(binding: &BindingCallContext, port: u16) -> NativeStringRef {
    binding.store_string(&port.to_string())
}

fn resolve_text_addresses_from_vm(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    host: destack_vm::StringHandle,
    port: u16,
    binding_name: &'static str,
) -> RuntimeResult<Vec<SocketAddress>> {
    // resolve the host string into binding storage
    let host = host_from_vm(binding, context, host)?;
    let service = resolve_service_text(binding, port);

    // resolve host and service to raw addresses
    let addresses = call_out(|out| unsafe {
        host_net::destack_net_resolve_raw(
            binding,
            out,
            Some(host),
            Some(service),
            SocketFamily::Unspecified,
            ResolveFlags(0),
        )
    })?;

    // pick the first usable address from resolver output
    let addresses = unsafe { addresses.as_slice()? };
    if addresses.is_empty() {
        let message = format!("{binding_name}: host and port resolved to no addresses");
        return Err(
            RuntimeError::from(PlatformError::invalid_argument_value("host", message)).boxed(),
        );
    }

    Ok(addresses.to_vec())
}

fn socket_family_from_address(
    address: SocketAddress,
    binding_name: &'static str,
) -> RuntimeResult<SocketFamily> {
    // map raw family numbers into socket family variants
    match address.family {
        0 => Ok(SocketFamily::Unspecified),
        #[cfg(unix)]
        family if family == libc::AF_INET as u16 => Ok(SocketFamily::IPv4),
        #[cfg(unix)]
        family if family == libc::AF_INET6 as u16 => Ok(SocketFamily::IPv6),
        #[cfg(windows)]
        family if family == windows_sys::Win32::Networking::WinSock::AF_INET => {
            Ok(SocketFamily::IPv4)
        }
        #[cfg(windows)]
        family if family == windows_sys::Win32::Networking::WinSock::AF_INET6 => {
            Ok(SocketFamily::IPv6)
        }
        _ => {
            let message = format!(
                "{binding_name}: unsupported socket family {}",
                address.family
            );
            Err(RuntimeError::from(PlatformError::invalid_argument_value("host", message)).boxed())
        }
    }
}

/// Return the native socket type constant for one TCP stream socket.
#[cfg(unix)]
fn tcp_stream_socket_type() -> SocketType {
    SocketType(libc::SOCK_STREAM as u32)
}

/// Return the native socket type constant for one TCP stream socket.
#[cfg(windows)]
fn tcp_stream_socket_type() -> SocketType {
    SocketType(windows_sys::Win32::Networking::WinSock::SOCK_STREAM as u32)
}

/// Return the native protocol constant for TCP sockets.
#[cfg(unix)]
fn tcp_socket_protocol() -> SocketProtocol {
    SocketProtocol(libc::IPPROTO_TCP)
}

/// Return the native protocol constant for TCP sockets.
#[cfg(windows)]
fn tcp_socket_protocol() -> SocketProtocol {
    SocketProtocol(windows_sys::Win32::Networking::WinSock::IPPROTO_TCP)
}

/// Return the POSIX-compatible socket type constant for one TCP stream socket.
#[cfg(not(any(unix, windows)))]
fn tcp_stream_socket_type() -> SocketType {
    SocketType(1)
}

/// Return the POSIX-compatible protocol constant for TCP sockets.
#[cfg(not(any(unix, windows)))]
fn tcp_socket_protocol() -> SocketProtocol {
    SocketProtocol(6)
}

/// Read socket packet mark.
pub(super) fn destack_net_get_packet_mark(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_packet_mark(binding, out, handle) })
}

/// Read one raw socket option payload.
pub(super) fn destack_net_get_sock_opt_raw(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    maxbytes: u32,
) -> RuntimeResult<VmArray<u8>> {
    let value = call_out(|out| unsafe {
        host_net::destack_net_get_sock_opt_raw(binding, out, handle, level, name, maxbytes)
    })?;

    bytes_array_to_vm(context, value)
}

/// Read packet timestamping mode.
pub(super) fn destack_net_get_timestamping(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketTimestampingMode> {
    call_out(|out| unsafe { host_net::destack_net_get_timestamping(binding, out, handle) })
}

/// Resolve an interface name to an index.
pub(super) fn destack_net_interface_index(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
) -> RuntimeResult<u32> {
    let name = host_from_vm(binding, context, name)?;
    call_out(|out| unsafe { host_net::destack_net_interface_index(binding, out, name) })
}

/// Resolve an interface index to a name.
pub(super) fn destack_net_interface_name(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    index: u32,
) -> RuntimeResult<destack_vm::StringHandle> {
    let value =
        call_out(|out| unsafe { host_net::destack_net_interface_name(binding, out, index) })?;
    let value = unsafe { value.as_str()? };
    context
        .string_handle(value)
        .map_err(Box::<RuntimeError>::from)
}

/// List network interfaces with addresses and flags.
pub(super) fn destack_net_list_interfaces(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<VmArray<NetInterfaceVm>> {
    let values = call_out(|out| unsafe { host_net::destack_net_list_interfaces(binding, out) })?;
    net_interface_array_to_vm(context, values)
}

/// List host packet backends.
pub(super) fn destack_net_packet_backend_list(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
) -> RuntimeResult<VmSlice<PacketBackendDescriptorVm>> {
    let values =
        call_out(|out| unsafe { super::core::destack_net_packet_backend_list(binding, out) })?;
    packet_backend_descriptor_slice_to_vm(context, values)
}

/// Open a packet capture or inject endpoint.
pub(super) fn destack_net_packet_open(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    options: PacketCaptureOptionsVm,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_packet_open(binding, out, options) })
}

/// Receive one packet from a packet endpoint.
pub(super) fn destack_net_packet_receive(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    payload: VmSlice<u8>,
) -> RuntimeResult<PacketCaptureRecordVm> {
    let native = allocate_read_buffer(binding, payload);
    let record = call_out(|out| unsafe {
        host_net::destack_net_packet_receive(binding, out, handle, native)
    })?;
    write_read_buffer(context, payload, native)?;
    Ok(record)
}

/// Send one packet through a packet endpoint.
pub(super) fn destack_net_packet_send(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    payload: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let native = buffer_from_vm(binding, context, payload)?;
    call_out(|out| unsafe { host_net::destack_net_packet_send(binding, out, handle, native) })
}

/// Configure packet timestamp mode for a socket or packet endpoint.
pub(super) fn destack_net_packet_set_timestamp_mode(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    mode: PacketTimestampMode,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_timestamp_mode(binding, handle, mode) }
}

/// Clear packet fanout from a packet endpoint.
pub(super) fn destack_net_packet_clear_fanout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_clear_fanout(binding, handle) }
}

/// Clear the active packet filter program.
pub(super) fn destack_net_packet_clear_filter(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_clear_filter(binding, handle) }
}

/// Clear packet rx and tx ring configuration.
pub(super) fn destack_net_packet_clear_ring(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_clear_ring(binding, handle) }
}

/// Set packet fanout on a packet endpoint.
pub(super) fn destack_net_packet_set_fanout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    options: PacketFanoutOptionsVm,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_fanout(binding, handle, options) }
}

/// Attach one packet filter program to a raw endpoint.
pub(super) fn destack_net_packet_set_filter(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    filterprogram: VmSlice<u8>,
) -> RuntimeResult<()> {
    let filterprogram = buffer_from_vm(binding, context, filterprogram)?;
    unsafe { host_net::destack_net_packet_set_filter(binding, handle, filterprogram) }
}

/// Configure one packet rx ring for zero-copy capture.
pub(super) fn destack_net_packet_set_rx_ring(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    options: PacketRingOptionsVm,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_rx_ring(binding, handle, options) }
}

/// Configure one packet tx ring for zero-copy transmit.
pub(super) fn destack_net_packet_set_tx_ring(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    options: PacketRingOptionsVm,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_packet_set_tx_ring(binding, handle, options) }
}

/// Read packet capture statistics from one endpoint.
pub(super) fn destack_net_packet_stats(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<PacketCaptureStatsVm> {
    call_out(|out| unsafe { host_net::destack_net_packet_stats(binding, out, handle) })
}

/// Enable or disable IP header inclusion on a raw socket.
pub(super) fn destack_net_raw_set_header_included(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_raw_set_header_included(binding, handle, enabled) }
}

/// Open a raw IP socket.
pub(super) fn destack_net_raw_socket(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    family: SocketFamily,
    protocol: i32,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_raw_socket(binding, out, family, protocol) })
}

/// Add a route table entry.
pub(super) fn destack_net_route_add(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    route: RouteEntryVm,
) -> RuntimeResult<()> {
    let route = route_entry_from_vm(binding, context, route)?;
    unsafe { host_net::destack_net_route_add(binding, route) }
}

/// Remove a route table entry.
pub(super) fn destack_net_route_delete(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    route: RouteEntryVm,
) -> RuntimeResult<()> {
    let route = route_entry_from_vm(binding, context, route)?;
    unsafe { host_net::destack_net_route_delete(binding, route) }
}

/// List route table entries.
pub(super) fn destack_net_route_list(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    family: SocketFamily,
) -> RuntimeResult<VmArray<RouteEntryVm>> {
    let routes = call_out(|out| unsafe { host_net::destack_net_route_list(binding, out, family) })?;
    route_entry_array_to_vm(context, routes)
}

/// Set socket packet mark.
pub(super) fn destack_net_set_packet_mark(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    mark: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_packet_mark(binding, handle, mark) }
}

/// Set one raw socket option payload.
pub(super) fn destack_net_set_sock_opt_raw(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    value: VmSlice<u8>,
) -> RuntimeResult<()> {
    let value = buffer_from_vm(binding, context, value)?;
    unsafe { host_net::destack_net_set_sock_opt_raw(binding, handle, level, name, value) }
}

/// Set packet timestamping mode.
pub(super) fn destack_net_set_timestamping(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    mode: SocketTimestampingMode,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_timestamping(binding, handle, mode) }
}

/// Read the default IPv4 multicast interface for one socket.
pub(super) fn destack_net_get_multicast_interface_v4(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<destack_vm::StringHandle> {
    let value = call_out(|out| unsafe {
        host_net::destack_net_get_multicast_interface_v4(binding, out, handle)
    })?;
    let value = unsafe { value.as_str()? };
    context
        .string_handle(value)
        .map_err(Box::<RuntimeError>::from)
}

/// Read the default IPv6 multicast interface for one socket.
pub(super) fn destack_net_get_multicast_interface_v6(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe {
        host_net::destack_net_get_multicast_interface_v6(binding, out, handle)
    })
}

/// Read multicast loopback mode.
pub(super) fn destack_net_get_multicast_loop(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_multicast_loop(binding, out, handle) })
}

/// Read multicast TTL or hop-limit.
pub(super) fn destack_net_get_multicast_ttl(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_multicast_ttl(binding, out, handle) })
}

/// Select the default IPv4 multicast interface for one socket.
pub(super) fn destack_net_set_multicast_interface_v4(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    interfaceaddress: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    let interfaceaddress = host_from_vm(binding, context, interfaceaddress)?;
    unsafe { host_net::destack_net_set_multicast_interface_v4(binding, handle, interfaceaddress) }
}

/// Select the default IPv6 multicast interface for one socket.
pub(super) fn destack_net_set_multicast_interface_v6(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    interfaceindex: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_multicast_interface_v6(binding, handle, interfaceindex) }
}

/// Join one IPv4 source-specific multicast membership.
pub(super) fn destack_net_join_multicast_source_v4(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    membership: UdpSourceMembershipV4Vm,
) -> RuntimeResult<()> {
    let membership = udp_source_membership_v4_from_vm(binding, context, membership)?;
    unsafe { host_net::destack_net_join_multicast_source_v4(binding, handle, membership) }
}

/// Join one IPv6 source-specific multicast membership.
pub(super) fn destack_net_join_multicast_source_v6(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    membership: UdpSourceMembershipV6Vm,
) -> RuntimeResult<()> {
    let membership = udp_source_membership_v6_from_vm(binding, context, membership)?;
    unsafe { host_net::destack_net_join_multicast_source_v6(binding, handle, membership) }
}

/// Leave one IPv4 source-specific multicast membership.
pub(super) fn destack_net_leave_multicast_source_v4(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    membership: UdpSourceMembershipV4Vm,
) -> RuntimeResult<()> {
    let membership = udp_source_membership_v4_from_vm(binding, context, membership)?;
    unsafe { host_net::destack_net_leave_multicast_source_v4(binding, handle, membership) }
}

/// Leave one IPv6 source-specific multicast membership.
pub(super) fn destack_net_leave_multicast_source_v6(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    membership: UdpSourceMembershipV6Vm,
) -> RuntimeResult<()> {
    let membership = udp_source_membership_v6_from_vm(binding, context, membership)?;
    unsafe { host_net::destack_net_leave_multicast_source_v6(binding, handle, membership) }
}
