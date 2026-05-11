use super::core::*;
use super::io::destack_net_recv_from as recv_from_socket;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::net::*;
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

/// Create a UDP socket.
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
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = SocketHandle(resource_id);
    }

    Ok(())
}

/// Bind a UDP socket to a raw local address.
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
            return Err(core_platform::net_error("bind"));
        }

        Ok(())
    })
}

/// Connect a UDP socket to a raw remote address.
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
            return Err(core_platform::net_error("connect"));
        }

        Ok(())
    })
}

/// Receive a UDP datagram with raw sender metadata.
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

    // reuse recvfrom projection so udp recvFlags match socket recvFlags
    let mut receive = std::mem::MaybeUninit::<SocketRecvFrom>::uninit();
    unsafe {
        recv_from_socket(
            binding,
            receive.as_mut_ptr(),
            handle,
            buffer,
            SocketMessageFlags(recv_flags.0),
        )
    }?;
    let receive = unsafe { receive.assume_init() };

    // write the udp-specific projection
    unsafe {
        *out = UdpReceive {
            address: receive.address,
            bytes: receive.bytes,
            recv_flags: UdpMessageFlags(receive.recv_flags.0),
        };
    }

    Ok(())
}

/// Send a UDP datagram to a raw destination address.
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
            return Err(core_platform::net_error("sendto"));
        }

        unsafe {
            *out = bytes as u64;
        }

        Ok(())
    })
}
