use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, INVALID_SOCKET, IPPROTO_UDP, SOCK_DGRAM, SOCKET_ERROR, bind, connect,
    sendto, socket,
};

use super::io::destack_net_recv_from as recv_from_socket;
use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::net::{
    SocketAddress, SocketFamily, SocketHandle, SocketMessageFlags, SocketRecvFrom, UdpMessageFlags,
    UdpReceive,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, core as core_platform};
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

    // ensure winsock is initialized
    core_platform::ensure_winsock()?;

    // select the socket family
    let family = match family {
        SocketFamily::IPv4 => AF_INET as i32,
        SocketFamily::IPv6 => AF_INET6 as i32,
        SocketFamily::Unspecified => AF_INET as i32,
    };

    // create the socket
    let socket = unsafe { socket(family, SOCK_DGRAM, IPPROTO_UDP) };
    if socket == INVALID_SOCKET {
        return Err(core_platform::net_error_with_code(
            "socket",
            core_platform::last_wsa_error_code(),
        ));
    }

    // register the socket
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(socket as _)
        .with_finalizer(SocketFinalizer::new(socket));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));
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
    // ensure winsock is initialized
    core_platform::ensure_winsock()?;

    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

    // bind using the raw address payload
    with_socket_address_raw(address, |sockaddr, length| {
        let rc = unsafe { bind(socket, sockaddr, length) };
        if rc != 0 {
            return Err(core_platform::net_error_with_code(
                "bind",
                core_platform::last_wsa_error_code(),
            ));
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
    // ensure winsock is initialized
    core_platform::ensure_winsock()?;

    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

    // connect using the raw address payload
    with_socket_address_raw(address, |sockaddr, length| {
        let rc = unsafe { connect(socket, sockaddr, length) };
        if rc != 0 {
            return Err(core_platform::net_error_with_code(
                "connect",
                core_platform::last_wsa_error_code(),
            ));
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
    // validate the output pointer
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
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    core_platform::ensure_winsock()?;

    // resolve runtime values
    let socket = socket_descriptor(binding, handle)?;
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
            return Err(core_platform::net_error_with_code(
                "sendto",
                core_platform::last_wsa_error_code(),
            ));
        }

        unsafe {
            *out = bytes as u64;
        }

        Ok(())
    })
}
