#![allow(unused_imports)]

use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::OsPath;
use crate::platform::net::{core as core_net, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::{BindingCallContext, NativeSlice};

use std::ffi::{CStr, CString};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::unix::io::RawFd;

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
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // select the socket family
    let family = match family {
        SocketFamily::IPv4 => libc::AF_INET,
        SocketFamily::IPv6 => libc::AF_INET6,
        SocketFamily::Unspecified => libc::AF_INET,
    };

    // create the socket
    let fd = unsafe { libc::socket(family, libc::SOCK_DGRAM, libc::IPPROTO_UDP) };
    if fd < 0 {
        return Err(core_platform::net_error("socket"));
    }

    // register the socket
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(fd)
        .with_finalizer(SocketFinalizer { fd });
    let resource_id = binding
        .agent()
        .resources
        .insert(entry, Some(binding.engine()));
    unsafe {
        *out = SocketHandle(resource_id);
    }

    Ok(())
}

/// Bind a UDP socket to a raw local address.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_udp_bind_raw(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let fd = socket_descriptor(binding, handle)?;

    // bind using the provided raw sockaddr
    with_socket_address_raw(address, |sockaddr, length| {
        let result = unsafe { libc::bind(fd, sockaddr, length) };
        if result != 0 {
            return Err(RuntimeError::from(PlatformError::io("bind failed".to_string())).boxed());
        }

        Ok(())
    })
}

/// Connect a UDP socket to a raw remote address.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_udp_connect_raw(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let fd = socket_descriptor(binding, handle)?;

    // connect using the provided raw sockaddr
    with_socket_address_raw(address, |sockaddr, length| {
        let result = unsafe { libc::connect(fd, sockaddr, length) };
        if result != 0 {
            return Err(
                RuntimeError::from(PlatformError::io("connect failed".to_string())).boxed(),
            );
        }

        Ok(())
    })
}

/// Receive a UDP datagram with raw sender metadata.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_udp_recv_from_raw(
    binding: &BindingCallContext,
    out: *mut UdpReceive,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    recv_flags: UdpMessageFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve runtime values
    let fd = socket_descriptor(binding, handle)?;
    let buffer = unsafe { buffer.as_mut_slice()? };
    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    let mut length = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

    // receive the datagram
    let bytes = unsafe {
        libc::recvfrom(
            fd,
            buffer.as_mut_ptr() as *mut libc::c_void,
            buffer.len(),
            recv_flags.0 as libc::c_int,
            &mut storage as *mut _ as *mut libc::sockaddr,
            &mut length,
        )
    };
    if bytes < 0 {
        return Err(RuntimeError::from(PlatformError::io("recvfrom failed".to_string())).boxed());
    }

    // encode the sender address and payload
    let address = socket_address_raw_from_storage(binding, &storage, length)?;
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
#[cfg(unix)]
pub(crate) unsafe fn destack_net_udp_send_to_raw(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: SocketHandle,
    address: SocketAddress,
    buffer: NativeSlice<u8>,
    send_flags: UdpMessageFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve runtime values
    let fd = socket_descriptor(binding, handle)?;
    let buffer = unsafe { buffer.as_slice()? };

    // send the datagram
    with_socket_address_raw(address, |sockaddr, length| {
        let bytes = unsafe {
            libc::sendto(
                fd,
                buffer.as_ptr() as *const libc::c_void,
                buffer.len(),
                send_flags.0 as libc::c_int,
                sockaddr,
                length,
            )
        };
        if bytes < 0 {
            return Err(RuntimeError::from(PlatformError::io("sendto failed".to_string())).boxed());
        }

        unsafe {
            *out = bytes as u64;
        }

        Ok(())
    })
}
