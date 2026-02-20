use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, INVALID_SOCKET, IPPROTO_UDP, SOCK_DGRAM, SOCKADDR, SOCKADDR_STORAGE,
    SOCKET_ERROR, bind, connect, recvfrom, sendto, socket,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{
    SocketAddress, SocketFamily, SocketHandle, UdpMessageFlags, UdpReceive,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeSlice, PlatformError};
use crate::runtime::BindingCallContext;

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
    context: &BindingCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    ensure_winsock()?;

    // select the socket family
    let family = match family {
        SocketFamily::IPv4 => AF_INET as i32,
        SocketFamily::IPv6 => AF_INET6 as i32,
        SocketFamily::Unspecified => AF_INET as i32,
    };

    // create the socket
    let socket = unsafe { socket(family, SOCK_DGRAM, IPPROTO_UDP) };
    if socket == INVALID_SOCKET {
        return Err(last_net_error("socket"));
    }

    // register the socket
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(socket as _)
        .with_finalizer(SocketFinalizer::new(socket));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = SocketHandle(resource_id);
    }

    Ok(())
}

/// Bind a UDP socket to a raw local address.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_udp_bind_raw(
    context: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // ensure winsock is initialized
    ensure_winsock()?;

    // resolve the socket descriptor
    let socket = socket_descriptor(context, handle)?;

    // bind using the raw address payload
    with_socket_address_raw(address, |sockaddr, length| {
        let rc = unsafe { bind(socket, sockaddr, length) };
        if rc != 0 {
            return Err(last_net_error("bind"));
        }

        Ok(())
    })
}

/// Connect a UDP socket to a raw remote address.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_udp_connect_raw(
    context: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // ensure winsock is initialized
    ensure_winsock()?;

    // resolve the socket descriptor
    let socket = socket_descriptor(context, handle)?;

    // connect using the raw address payload
    with_socket_address_raw(address, |sockaddr, length| {
        let rc = unsafe { connect(socket, sockaddr, length) };
        if rc != 0 {
            return Err(last_net_error("connect"));
        }

        Ok(())
    })
}

/// Receive a UDP datagram with raw sender metadata.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_udp_recv_from_raw(
    context: &BindingCallContext,
    out: *mut UdpReceive,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    recv_flags: UdpMessageFlags,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    ensure_winsock()?;

    // resolve runtime values
    let socket = socket_descriptor(context, handle)?;
    let buffer = unsafe { buffer.as_mut_slice()? };
    let buffer_len = i32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large",
        ))
        .boxed()
    })?;
    let mut address = unsafe { std::mem::zeroed::<SOCKADDR_STORAGE>() };
    let mut address_length = std::mem::size_of::<SOCKADDR_STORAGE>() as i32;

    // receive one datagram
    let bytes = unsafe {
        recvfrom(
            socket,
            buffer.as_mut_ptr() as *mut _,
            buffer_len,
            recv_flags.0 as i32,
            &mut address as *mut _ as *mut SOCKADDR,
            &mut address_length,
        )
    };
    if bytes == SOCKET_ERROR {
        return Err(last_net_error("recvfrom"));
    }

    // encode sender metadata and payload length
    let address = socket_address_raw_from_storage(context, &address, address_length)?;
    unsafe {
        *out = UdpReceive {
            address,
            bytes: bytes as u64,
            recv_flags: UdpMessageFlags(0),
        };
    }

    Ok(())
}

/// Send a UDP datagram to a raw destination address.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_udp_send_to_raw(
    context: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    address: SocketAddress,
    buffer: NativeSlice<u8>,
    send_flags: UdpMessageFlags,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    ensure_winsock()?;

    // resolve runtime values
    let socket = socket_descriptor(context, handle)?;
    let buffer = unsafe { buffer.as_slice()? };
    let buffer_len = i32::try_from(buffer.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "buffer",
            "buffer too large",
        ))
        .boxed()
    })?;

    // send one datagram
    with_socket_address_raw(address, |sockaddr, length| {
        let bytes = unsafe {
            sendto(
                socket,
                buffer.as_ptr() as *const _,
                buffer_len,
                send_flags.0 as i32,
                sockaddr,
                length,
            )
        };
        if bytes == SOCKET_ERROR {
            return Err(last_net_error("sendto"));
        }

        unsafe {
            *out = bytes as u64;
        }

        Ok(())
    })
}
