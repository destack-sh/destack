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
pub fn destack_net_accept(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    listener: ListenerHandle,
    flags: AcceptFlags,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_accept(binding, out, listener, flags) })
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
pub fn destack_net_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_close(binding, handle) }
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
pub fn destack_net_shutdown(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_shutdown(binding, handle, how) }
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
pub fn destack_net_set_nonblocking(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_nonblocking(binding, handle, enabled) }
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
pub fn destack_net_set_no_delay(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_no_delay(binding, handle, enabled) }
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
pub fn destack_net_set_reuse_addr(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_reuse_addr(binding, handle, enabled) }
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
pub fn destack_net_set_reuse_port(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_reuse_port(binding, handle, enabled) }
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
pub fn destack_net_close_listener(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_close_listener(binding, handle) }
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
pub fn destack_net_udp_socket(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    family: SocketFamily,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_udp_socket(binding, out, family) })
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
pub fn destack_net_uds_accept(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    listener: ListenerHandle,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_uds_accept(binding, out, listener) })
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
pub fn destack_net_uds_close_listener(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_uds_close_listener(binding, handle) }
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
pub fn destack_net_uds_socket_pair(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    socket_type: SocketType,
) -> RuntimeResult<SocketPair> {
    call_out(|out| unsafe { host_net::destack_net_uds_socket_pair(binding, out, socket_type) })
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
pub fn destack_net_set_linger(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    linger: Linger,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_linger(binding, handle, linger) }
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
pub fn destack_net_set_recv_buffer(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_recv_buffer(binding, handle, size) }
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
pub fn destack_net_set_send_buffer(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_send_buffer(binding, handle, size) }
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
pub fn destack_net_set_broadcast(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_broadcast(binding, handle, enabled) }
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
pub fn destack_net_set_ttl(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_ttl(binding, handle, ttl) }
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
pub fn destack_net_set_tos(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_tos(binding, handle, tos) }
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
pub fn destack_net_set_read_timeout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_read_timeout(binding, handle, timeout_ms) }
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
pub fn destack_net_set_write_timeout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_write_timeout(binding, handle, timeout_ms) }
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
pub fn destack_net_set_only_v6(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_only_v6(binding, handle, enabled) }
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
pub fn destack_net_get_keep_alive(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<KeepAliveConfig> {
    call_out(|out| unsafe { host_net::destack_net_get_keep_alive(binding, out, handle) })
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
pub fn destack_net_get_no_delay(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_no_delay(binding, out, handle) })
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
pub fn destack_net_get_reuse_addr(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_reuse_addr(binding, out, handle) })
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
pub fn destack_net_get_reuse_port(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_reuse_port(binding, out, handle) })
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
pub fn destack_net_get_linger(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<Linger> {
    call_out(|out| unsafe { host_net::destack_net_get_linger(binding, out, handle) })
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
pub fn destack_net_get_recv_buffer(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_recv_buffer(binding, out, handle) })
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
pub fn destack_net_get_send_buffer(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_send_buffer(binding, out, handle) })
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
pub fn destack_net_get_broadcast(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_broadcast(binding, out, handle) })
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
pub fn destack_net_get_ttl(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_ttl(binding, out, handle) })
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
pub fn destack_net_get_tos(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_tos(binding, out, handle) })
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
pub fn destack_net_get_read_timeout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_read_timeout(binding, out, handle) })
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
pub fn destack_net_get_write_timeout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_write_timeout(binding, out, handle) })
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
pub(super) fn destack_net_get_packet_mark(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_packet_mark(binding, out, handle) })
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
pub(super) fn destack_net_get_timestamping(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketTimestampingMode> {
    call_out(|out| unsafe { host_net::destack_net_get_timestamping(binding, out, handle) })
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
pub(super) fn destack_net_interface_index(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    name: destack_vm::StringHandle,
) -> RuntimeResult<u32> {
    let name = host_from_vm(binding, context, name)?;
    call_out(|out| unsafe { host_net::destack_net_interface_index(binding, out, name) })
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
pub(super) fn destack_net_packet_open(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    options: PacketCaptureOptionsVm,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_packet_open(binding, out, options) })
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
pub(super) fn destack_net_packet_set_timestamp_mode(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
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
pub(super) fn destack_net_packet_clear_fanout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
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
pub(super) fn destack_net_packet_clear_filter(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
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
pub(super) fn destack_net_packet_clear_ring(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
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
pub(super) fn destack_net_packet_set_fanout(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    options: PacketFanoutOptionsVm,
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
pub(super) fn destack_net_packet_set_rx_ring(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    options: PacketRingOptionsVm,
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
pub(super) fn destack_net_packet_set_tx_ring(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    options: PacketRingOptionsVm,
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
pub(super) fn destack_net_packet_stats(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<PacketCaptureStatsVm> {
    call_out(|out| unsafe { host_net::destack_net_packet_stats(binding, out, handle) })
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
pub(super) fn destack_net_raw_set_header_included(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
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
pub(super) fn destack_net_raw_socket(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    family: SocketFamily,
    protocol: i32,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_raw_socket(binding, out, family, protocol) })
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
pub(super) fn destack_net_route_add(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    route: RouteEntryVm,
) -> RuntimeResult<()> {
    let route = route_entry_from_vm(binding, context, route)?;
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
pub(super) fn destack_net_route_delete(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    route: RouteEntryVm,
) -> RuntimeResult<()> {
    let route = route_entry_from_vm(binding, context, route)?;
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
pub(super) fn destack_net_route_list(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    family: SocketFamily,
) -> RuntimeResult<VmArray<RouteEntryVm>> {
    let routes = call_out(|out| unsafe { host_net::destack_net_route_list(binding, out, family) })?;
    route_entry_array_to_vm(context, routes)
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
pub(super) fn destack_net_set_packet_mark(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
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
pub(super) fn destack_net_set_timestamping(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    mode: SocketTimestampingMode,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_timestamping(binding, handle, mode) }
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
pub(super) fn destack_net_get_multicast_loop(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_multicast_loop(binding, out, handle) })
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
pub(super) fn destack_net_get_multicast_ttl(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_multicast_ttl(binding, out, handle) })
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
pub(super) fn destack_net_set_multicast_interface_v6(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    interfaceindex: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_multicast_interface_v6(binding, handle, interfaceindex) }
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
pub(super) fn destack_net_leave_multicast_source_v6(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: SocketHandle,
    membership: UdpSourceMembershipV6Vm,
) -> RuntimeResult<()> {
    let membership = udp_source_membership_v6_from_vm(binding, context, membership)?;
    unsafe { host_net::destack_net_leave_multicast_source_v6(binding, handle, membership) }
}
