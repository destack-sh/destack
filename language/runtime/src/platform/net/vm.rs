use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeAbi, VmAbi};
use crate::platform::fs::{
    OsPath, OsPathVm, PathBytesAbi, PathEncoding, PathUtf16Abi, core as core_fs,
};
use crate::platform::net::{
    AcceptFlags, KeepAliveConfig, Linger, NetInterfaceVm, PacketCaptureOptionsVm,
    PacketCaptureRecordVm, PacketCaptureStatsVm, PacketFanoutOptionsVm, PacketRingOptionsVm,
    PacketTimestampMode, ResolveFlags, ResolveQueryVm, ReverseLookupFlags, ReverseLookupNameVm,
    RouteEntryVm, SocketAddress, SocketAddressVm, SocketControlBufferAbi, SocketCredentials,
    SocketCredentialsVm, SocketFamily, SocketMessageFlags, SocketOptionLevel, SocketOptionName,
    SocketPair, SocketPairVm, SocketProtocol, SocketRecvBatchRequestVm, SocketRecvFrom,
    SocketRecvFromVm, SocketRecvMessage, SocketRecvMessageVm, SocketSendBatchEntryVm,
    SocketSendMessage, SocketSendMessageVm, SocketSendTo, SocketSendToVm, SocketShutdown,
    SocketTimestampingMode, SocketType, UdpMessageFlags, UdpReceive, UdpReceiveVm,
    UdpSourceMembershipV4Vm, UdpSourceMembershipV6Vm, UdsAddress, UdsAddressKind, UdsAddressVm,
    host as host_net,
};
use crate::platform::resource::{ListenerHandle, SocketHandle};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError, VmArray, VmSlice};
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    listener: ListenerHandle,
    flags: AcceptFlags,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_accept(runtime, out, listener, flags) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_close(runtime, handle) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    unsafe { host_net::destack_net_connect_raw(runtime, handle, address) }
}

/// Connect to a remote host and return a socket handle.
pub fn destack_net_connect_text(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _host: vm::StringHandle,
    _port: u16,
) -> RuntimeResult<SocketHandle> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.connectText")).boxed())
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    address: SocketAddressVm,
    backlog: u32,
) -> RuntimeResult<ListenerHandle> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    call_out(|out| unsafe { host_net::destack_net_listen_raw(runtime, out, address, backlog) })
}

/// Start listening on a host and port.
pub fn destack_net_listen_text(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _host: vm::StringHandle,
    _port: u16,
    _backlog: u32,
) -> RuntimeResult<ListenerHandle> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.listenText")).boxed())
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // allocate a native buffer for reads
    let native_buffer = allocate_read_buffer(runtime, buffer);

    // perform the read
    let bytes_read =
        call_out(|out| unsafe { host_net::destack_net_read(runtime, out, handle, native_buffer) })?;

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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    // resolve the VM buffer into native storage
    let native_buffer = buffer_from_vm(runtime, context, buffer)?;

    // dispatch to the core binding
    call_out(|out| unsafe { host_net::destack_net_write(runtime, out, handle, native_buffer) })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    recv_flags: SocketMessageFlags,
    max_fds: u32,
    want_credentials: bool,
    max_control_bytes: u32,
) -> RuntimeResult<SocketRecvMessageVm> {
    // allocate a native receive buffer
    let native_buffer = allocate_read_buffer(runtime, buffer);

    // receive one message through the os implementation
    let message = call_out(|out| unsafe {
        super::host::destack_net_recv_msg(
            runtime,
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _requests: VmSlice<SocketRecvBatchRequestVm>,
    _max_fds: u32,
    _want_credentials: bool,
    _max_control_bytes: u32,
) -> RuntimeResult<VmArray<SocketRecvMessageVm>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.recvMmsg")).boxed())
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
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
        host_net::destack_net_send_msg(runtime, out, handle, native_buffer, message)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    recv_flags: SocketMessageFlags,
) -> RuntimeResult<SocketRecvFromVm> {
    let native = allocate_read_buffer(runtime, buffer);
    let receive = call_out(|out| unsafe {
        host_net::destack_net_recv_from(runtime, out, handle, native, recv_flags)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    message: SocketSendToVm,
) -> RuntimeResult<u64> {
    let native_buffer = buffer_from_vm(runtime, context, buffer)?;
    let native_message = socket_send_to_from_vm(runtime, context, message)?;
    call_out(|out| unsafe {
        host_net::destack_net_send_to(runtime, out, handle, native_buffer, native_message)
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _messages: VmSlice<SocketSendBatchEntryVm>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.sendMmsg")).boxed())
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_shutdown(runtime, handle, how) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_nonblocking(runtime, handle, enabled) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    // read the address via the core binding
    let address =
        call_out(|out| unsafe { host_net::destack_net_local_address_raw(runtime, out, handle) })?;
    socket_address_raw_to_vm(context, address)
}

/// Read the local socket address as normalized text metadata.
pub fn destack_net_local_address_text(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.localAddressText")).boxed())
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    // read the address via the core binding
    let address =
        call_out(|out| unsafe { host_net::destack_net_peer_address_raw(runtime, out, handle) })?;
    socket_address_raw_to_vm(context, address)
}

/// Read the remote socket address as normalized text metadata.
pub fn destack_net_peer_address_text(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<SocketAddressVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.peerAddressText")).boxed())
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_no_delay(runtime, handle, enabled) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    config: KeepAliveConfig,
) -> RuntimeResult<()> {
    unsafe {
        host_net::destack_net_set_keep_alive(
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_reuse_addr(runtime, handle, enabled) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_reuse_port(runtime, handle, enabled) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_close_listener(runtime, handle) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    family: SocketFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe {
        host_net::destack_net_socket(runtime, out, family, socket_type, protocol)
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    family: SocketFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<SocketPairVm> {
    call_out(|out| unsafe {
        host_net::destack_net_socket_pair(runtime, out, family, socket_type, protocol)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    unsafe { host_net::destack_net_bind(runtime, handle, address) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let (native_buffers, vm_buffers) = allocate_read_buffers(runtime, context, buffers)?;
    let count = call_out(|out| unsafe {
        host_net::destack_net_readv(runtime, out, handle, native_buffers)
    })?;
    write_read_buffers(context, vm_buffers, native_buffers)?;
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let native_buffers = buffers_from_vm(runtime, context, buffers)?;
    call_out(|out| unsafe { host_net::destack_net_writev(runtime, out, handle, native_buffers) })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    query: ResolveQueryVm,
) -> RuntimeResult<VmArray<SocketAddressVm>> {
    // decode query host and service values
    let host = resolve_host_from_vm(runtime, context, query)?;
    let port = resolve_port_from_vm(context, query)?;

    // resolve via raw resolver and map to VM addresses
    let addresses = call_out(|out| unsafe {
        host_net::destack_net_resolve_raw(runtime, out, host, port, query.family, query.flags)
    })?;
    socket_address_raw_array_to_vm(context, addresses)
}

/// Resolve a hostname and port into normalized socket addresses.
pub fn destack_net_resolve_text(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    host: vm::StringHandle,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<VmArray<SocketAddressVm>> {
    // resolve the host string into native storage
    let host = host_from_vm(runtime, context, host)?;

    // resolve via raw resolver and map to VM addresses
    let addresses = call_out(|out| unsafe {
        host_net::destack_net_resolve_raw(runtime, out, host, port, family, flags)
    })?;
    socket_address_raw_array_to_vm(context, addresses)
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    address: SocketAddressVm,
    flags: ReverseLookupFlags,
) -> RuntimeResult<VmArray<ReverseLookupNameVm>> {
    // reserve non default behavior until flags are implemented
    if flags.0 != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.net.reverseLookup")).boxed(),
        );
    }

    // decode address and execute reverse lookup
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    let hosts =
        call_out(|out| unsafe { host_net::destack_net_reverse_lookup_raw(runtime, out, address) })?;
    let hosts = unsafe { hosts.as_slice()? };

    // map hostnames into lookup records
    let empty_service = vm::StringHandle::new(context.intern_string(""));
    let mut names = Vec::with_capacity(hosts.len());
    for host in hosts {
        let host = unsafe { host.as_str()? };
        let host = vm::StringHandle::new(context.intern_string(host));
        names.push(ReverseLookupNameVm {
            host,
            service: empty_service,
        });
    }

    let mut values = Vec::with_capacity(names.len());
    for name in names {
        let value = context.allocate_aggregate(vec![name.host.value(), name.service.value()]);
        values.push(value);
    }
    let data = context.allocate_raw_values(values);
    Ok(VmArray {
        data,
        len: hosts.len() as u32,
        capacity: hosts.len() as u32,
        _marker: std::marker::PhantomData::<ReverseLookupNameVm>,
    })
}

/// Reverse lookup a normalized socket address into hostnames.
pub fn destack_net_reverse_lookup_text(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    address: SocketAddressVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    // decode address and execute reverse lookup
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    let hosts =
        call_out(|out| unsafe { host_net::destack_net_reverse_lookup_raw(runtime, out, address) })?;
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    family: SocketFamily,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_udp_socket(runtime, out, family) })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    unsafe { host_net::destack_net_udp_bind_raw(runtime, handle, address) }
}

/// Bind a UDP socket to a host and port.
pub fn destack_net_udp_bind_text(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _host: vm::StringHandle,
    _port: u16,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpBindText")).boxed())
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
) -> RuntimeResult<()> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    unsafe { host_net::destack_net_udp_connect_raw(runtime, handle, address) }
}

/// Connect a UDP socket to a host and port.
pub fn destack_net_udp_connect_text(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _host: vm::StringHandle,
    _port: u16,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpConnectText")).boxed())
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    buffer: VmSlice<u8>,
    recv_flags: UdpMessageFlags,
) -> RuntimeResult<UdpReceiveVm> {
    let native = allocate_read_buffer(runtime, buffer);
    let receive = call_out(|out| unsafe {
        host_net::destack_net_udp_recv_from_raw(runtime, out, handle, native, recv_flags)
    })?;
    write_read_buffer(context, buffer, native)?;
    udp_receive_raw_to_vm(context, receive)
}

/// Receive a UDP packet with normalized sender metadata.
pub fn destack_net_udp_recv_from_text(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _buffer: VmSlice<u8>,
) -> RuntimeResult<UdpReceiveVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpRecvFromText")).boxed())
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    address: SocketAddressVm,
    buffer: VmSlice<u8>,
    send_flags: UdpMessageFlags,
) -> RuntimeResult<u64> {
    let address = socket_address_raw_from_vm(runtime, context, address)?;
    let native = buffer_from_vm(runtime, context, buffer)?;
    call_out(|out| unsafe {
        host_net::destack_net_udp_send_to_raw(runtime, out, handle, address, native, send_flags)
    })
}

/// Send a UDP packet to a host and port.
pub fn destack_net_udp_send_to_text(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _host: vm::StringHandle,
    _port: u16,
    _buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpSendToText")).boxed())
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    listener: ListenerHandle,
) -> RuntimeResult<SocketHandle> {
    call_out(|out| unsafe { host_net::destack_net_uds_accept(runtime, out, listener) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_uds_close_listener(runtime, handle) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    address: UdsAddressVm,
) -> RuntimeResult<SocketHandle> {
    // decode the uds address and delegate to the os implementation
    let address = uds_address_from_vm(runtime, context, address)?;
    let path = uds_path(runtime, address)?;
    call_out(|out| unsafe { super::host::destack_net_uds_connect(runtime, out, path) })
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    address: UdsAddressVm,
    backlog: u32,
) -> RuntimeResult<ListenerHandle> {
    // decode the uds address and delegate to the os implementation
    let address = uds_address_from_vm(runtime, context, address)?;
    let path = uds_path(runtime, address)?;
    call_out(|out| unsafe { super::host::destack_net_uds_listen(runtime, out, path, backlog) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    socket_type: SocketType,
) -> RuntimeResult<SocketPair> {
    call_out(|out| unsafe { host_net::destack_net_uds_socket_pair(runtime, out, socket_type) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    linger: Linger,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_linger(runtime, handle, linger) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_recv_buffer(runtime, handle, size) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_send_buffer(runtime, handle, size) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_broadcast(runtime, handle, enabled) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    group: vm::StringHandle,
    interface_address: vm::StringHandle,
) -> RuntimeResult<()> {
    // resolve the multicast group
    let group = host_from_vm(runtime, context, group)?;

    // resolve the interface address
    let interface_address = host_from_vm(runtime, context, interface_address)?;

    // dispatch to the core binding
    unsafe { host_net::destack_net_join_multicast_v4(runtime, handle, group, interface_address) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    group: vm::StringHandle,
    interface_index: u32,
) -> RuntimeResult<()> {
    // resolve the multicast group
    let group = host_from_vm(runtime, context, group)?;

    // dispatch to the core binding
    unsafe { host_net::destack_net_join_multicast_v6(runtime, handle, group, interface_index) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    group: vm::StringHandle,
    interface_address: vm::StringHandle,
) -> RuntimeResult<()> {
    // resolve the multicast group
    let group = host_from_vm(runtime, context, group)?;

    // resolve the interface address
    let interface_address = host_from_vm(runtime, context, interface_address)?;

    // dispatch to the core binding
    unsafe { host_net::destack_net_leave_multicast_v4(runtime, handle, group, interface_address) }
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    group: vm::StringHandle,
    interface_index: u32,
) -> RuntimeResult<()> {
    // resolve the multicast group
    let group = host_from_vm(runtime, context, group)?;

    // dispatch to the core binding
    unsafe { host_net::destack_net_leave_multicast_v6(runtime, handle, group, interface_index) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // dispatch to the core binding
    unsafe { host_net::destack_net_set_multicast_loop(runtime, handle, enabled) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    // dispatch to the core binding
    unsafe { host_net::destack_net_set_multicast_ttl(runtime, handle, ttl) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_ttl(runtime, handle, ttl) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_tos(runtime, handle, tos) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_read_timeout(runtime, handle, timeout_ms) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_write_timeout(runtime, handle, timeout_ms) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    unsafe { host_net::destack_net_set_only_v6(runtime, handle, enabled) }
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<KeepAliveConfig> {
    call_out(|out| unsafe { host_net::destack_net_get_keep_alive(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_no_delay(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_reuse_addr(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_reuse_port(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<Linger> {
    call_out(|out| unsafe { host_net::destack_net_get_linger(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_recv_buffer(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_send_buffer(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_broadcast(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_ttl(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_tos(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_read_timeout(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<u32> {
    call_out(|out| unsafe { host_net::destack_net_get_write_timeout(runtime, out, handle) })
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
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: SocketHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { host_net::destack_net_get_only_v6(runtime, out, handle) })
}

fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    // allocate space for the output and invoke the call
    let mut value = std::mem::MaybeUninit::<T>::uninit();
    call(value.as_mut_ptr())?;
    Ok(unsafe { value.assume_init() })
}

fn host_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<NativeSlice<u8>> {
    // copy the VM buffer into runtime storage
    let bytes = buffer.read_bytes(context)?;
    Ok(runtime.store_slice(bytes))
}

fn allocate_read_buffer(runtime: &BindingCallContext, buffer: VmSlice<u8>) -> NativeSlice<u8> {
    // allocate a native buffer for reads
    let length = buffer.len as usize;
    runtime.store_slice(vec![0u8; length])
}

fn write_read_buffer(
    context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
    native: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // copy bytes back into the VM buffer
    let bytes = unsafe { native.as_slice()? };
    buffer.write_bytes(context, bytes)
}

fn decode_buffer_slices(
    context: &mut vm::ExternalCallContext<'_>,
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
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
    context: &mut vm::ExternalCallContext<'_>,
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
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
    context: &mut vm::ExternalCallContext<'_>,
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
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
    context: &mut vm::ExternalCallContext<'_>,
    address: SocketAddress,
) -> RuntimeResult<SocketAddressVm> {
    socket_address_to_vm(context, address)
}

fn socket_address_raw_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    address: SocketAddressVm,
) -> RuntimeResult<SocketAddress> {
    socket_address_from_vm(runtime, context, address)
}

fn socket_address_raw_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
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

fn string_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
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
    context: &mut vm::ExternalCallContext<'_>,
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
    context: &mut vm::ExternalCallContext<'_>,
    receive: UdpReceive,
) -> RuntimeResult<UdpReceiveVm> {
    udp_receive_to_vm(context, receive)
}

fn socket_send_to_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    send_to: SocketSendToVm,
) -> RuntimeResult<SocketSendTo> {
    let address = socket_address_raw_from_vm(runtime, context, send_to.address)?;
    Ok(SocketSendTo {
        address,
        flags: send_to.flags,
    })
}

fn socket_recv_from_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    recv_from: SocketRecvFrom,
) -> RuntimeResult<SocketRecvFromVm> {
    let address = socket_address_raw_to_vm(context, recv_from.address)?;
    Ok(SocketRecvFromVm {
        bytes: recv_from.bytes,
        address,
        recv_flags: recv_from.recv_flags,
    })
}

fn socket_send_message_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    message: SocketSendMessageVm,
) -> RuntimeResult<SocketSendMessage> {
    let fds = message.fds.read_values(context)?;
    let fds = runtime.store_array(fds);
    let address = socket_address_raw_from_vm(runtime, context, message.address)?;
    let control = message.control.0.read_bytes(context)?;
    let control = SocketControlBufferAbi::<NativeAbi>(runtime.store_array(control));
    Ok(SocketSendMessage {
        has_address: message.has_address,
        address,
        fds,
        control,
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
    context: &mut vm::ExternalCallContext<'_>,
    message: SocketRecvMessage,
) -> RuntimeResult<SocketRecvMessageVm> {
    let address = socket_address_raw_to_vm(context, message.address)?;
    let control = unsafe { message.control.0.as_slice()? };
    let control = VmArray::from_bytes(context, control);
    let control = SocketControlBufferAbi::<VmAbi>(control);
    let fds = unsafe { message.fds.as_slice()? };
    let fds = VmArray::from_values(context, fds)?;
    Ok(SocketRecvMessageVm {
        bytes: message.bytes,
        has_address: message.has_address,
        address,
        recv_flags: message.recv_flags,
        payload_truncated: message.payload_truncated,
        control_truncated: message.control_truncated,
        control,
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
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<OsPath> {
    match path.encoding {
        PathEncoding::Bytes => {
            let bytes = path.bytes.0.read_bytes(context)?;
            let bytes = PathBytesAbi::<NativeAbi>(runtime.store_array(bytes));
            Ok(OsPath {
                encoding: PathEncoding::Bytes,
                bytes,
                utf16: core_fs::empty_path_utf16(),
            })
        }
        PathEncoding::Utf16 => {
            let units = path.utf16.0.read_values(context)?;
            let utf16 = PathUtf16Abi::<NativeAbi>(runtime.store_array(units));
            Ok(OsPath {
                encoding: PathEncoding::Utf16,
                bytes: core_fs::empty_path_bytes(),
                utf16,
            })
        }
    }
}

fn uds_address_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    address: UdsAddressVm,
) -> RuntimeResult<UdsAddress> {
    let path = path_ref_from_vm(runtime, context, address.path)?;
    let abstract_name = address.abstract_name.read_bytes(context)?;
    let abstract_name = runtime.store_array(abstract_name);

    Ok(UdsAddress {
        kind: address.kind,
        path,
        abstract_name,
    })
}

fn uds_path(runtime: &BindingCallContext, address: UdsAddress) -> RuntimeResult<OsPath> {
    match address.kind {
        UdsAddressKind::Path => Ok(address.path),
        UdsAddressKind::Abstract => {
            let name = unsafe { address.abstract_name.as_slice()? };
            let mut bytes = Vec::with_capacity(name.len().saturating_add(1));
            bytes.push(0);
            bytes.extend_from_slice(name);
            let bytes = PathBytesAbi::<NativeAbi>(runtime.store_array(bytes));
            Ok(OsPath {
                encoding: PathEncoding::Bytes,
                bytes,
                utf16: core_fs::empty_path_utf16(),
            })
        }
        UdsAddressKind::Unnamed => {
            Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsConnect")).boxed())
        }
    }
}

fn resolve_host_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    query: ResolveQueryVm,
) -> RuntimeResult<NativeStringRef> {
    if !query.has_host {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "query",
            "host is required",
        ))
        .boxed());
    }

    host_from_vm(runtime, context, query.host)
}

fn resolve_port_from_vm(
    context: &mut vm::ExternalCallContext<'_>,
    query: ResolveQueryVm,
) -> RuntimeResult<u16> {
    if !query.has_service {
        return Ok(0);
    }

    let service = context
        .string_ref(query.service)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    let service = service.as_str();
    service.parse::<u16>().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "query",
            "service must be a numeric port",
        ))
        .boxed()
    })
}

/// Build an unsupported error for VM net bindings that are not implemented yet.
fn not_supported_binding(binding_name: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(binding_name)).boxed()
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
pub(super) fn destack_net_get_packet_mark(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<u32> {
    Err(not_supported_binding("destack.net.getPacketMark"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _level: SocketOptionLevel,
    _name: SocketOptionName,
    _maxbytes: u32,
) -> RuntimeResult<VmArray<u8>> {
    Err(not_supported_binding("destack.net.getSockOptRaw"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<SocketTimestampingMode> {
    Err(not_supported_binding("destack.net.getTimestamping"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _name: vm::StringHandle,
) -> RuntimeResult<u32> {
    Err(not_supported_binding("destack.net.interfaceIndex"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _index: u32,
) -> RuntimeResult<vm::StringHandle> {
    Err(not_supported_binding("destack.net.interfaceName"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<NetInterfaceVm>> {
    Err(not_supported_binding("destack.net.listInterfaces"))
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
pub(super) fn destack_net_packet_open(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _options: PacketCaptureOptionsVm,
) -> RuntimeResult<SocketHandle> {
    Err(not_supported_binding("destack.net.packetOpen"))
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
pub(super) fn destack_net_packet_receive(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _payload: VmSlice<u8>,
) -> RuntimeResult<PacketCaptureRecordVm> {
    Err(not_supported_binding("destack.net.packetReceive"))
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
pub(super) fn destack_net_packet_send(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _payload: VmSlice<u8>,
) -> RuntimeResult<u64> {
    Err(not_supported_binding("destack.net.packetSend"))
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
pub(super) fn destack_net_packet_set_timestamp_mode(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _mode: PacketTimestampMode,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.packetSetTimestampMode"))
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
pub(super) fn destack_net_packet_clear_fanout(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.packetClearFanout"))
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
pub(super) fn destack_net_packet_clear_filter(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.packetClearFilter"))
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
pub(super) fn destack_net_packet_clear_ring(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.packetClearRing"))
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
pub(super) fn destack_net_packet_set_fanout(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _options: PacketFanoutOptionsVm,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.packetSetFanout"))
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
pub(super) fn destack_net_packet_set_filter(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _filterprogram: VmSlice<u8>,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.packetSetFilter"))
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
pub(super) fn destack_net_packet_set_rx_ring(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _options: PacketRingOptionsVm,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.packetSetRxRing"))
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
pub(super) fn destack_net_packet_set_tx_ring(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _options: PacketRingOptionsVm,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.packetSetTxRing"))
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
pub(super) fn destack_net_packet_stats(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<PacketCaptureStatsVm> {
    Err(not_supported_binding("destack.net.packetStats"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _enabled: bool,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.rawSetHeaderIncluded"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _family: SocketFamily,
    _protocol: i32,
) -> RuntimeResult<SocketHandle> {
    Err(not_supported_binding("destack.net.rawSocket"))
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
pub(super) fn destack_net_route_add(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _route: RouteEntryVm,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.routeAdd"))
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
pub(super) fn destack_net_route_delete(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _route: RouteEntryVm,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.routeDelete"))
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
pub(super) fn destack_net_route_list(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _family: SocketFamily,
) -> RuntimeResult<VmArray<RouteEntryVm>> {
    Err(not_supported_binding("destack.net.routeList"))
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
pub(super) fn destack_net_set_packet_mark(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _mark: u32,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.setPacketMark"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _level: SocketOptionLevel,
    _name: SocketOptionName,
    _value: VmSlice<u8>,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.setSockOptRaw"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _mode: SocketTimestampingMode,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.setTimestamping"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<vm::StringHandle> {
    Err(not_supported_binding("destack.net.getMulticastInterfaceV4"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<u32> {
    Err(not_supported_binding("destack.net.getMulticastInterfaceV6"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<bool> {
    Err(not_supported_binding("destack.net.getMulticastLoop"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
) -> RuntimeResult<u32> {
    Err(not_supported_binding("destack.net.getMulticastTtl"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _interfaceaddress: vm::StringHandle,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.setMulticastInterfaceV4"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _interfaceindex: u32,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.setMulticastInterfaceV6"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _membership: UdpSourceMembershipV4Vm,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.joinMulticastSourceV4"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _membership: UdpSourceMembershipV6Vm,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.joinMulticastSourceV6"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _membership: UdpSourceMembershipV4Vm,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.leaveMulticastSourceV4"))
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
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: SocketHandle,
    _membership: UdpSourceMembershipV6Vm,
) -> RuntimeResult<()> {
    Err(not_supported_binding("destack.net.leaveMulticastSourceV6"))
}
