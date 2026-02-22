use std::mem;
use std::net::IpAddr;
use windows_sys::Win32::Networking::WinSock::{
    ADDRINFOW, AF_INET, AF_INET6, AF_UNSPEC, AI_ADDRCONFIG, AI_ALL, AI_CANONNAME, AI_NUMERICHOST,
    AI_NUMERICSERV, AI_PASSIVE, AI_V4MAPPED, FreeAddrInfoW, GetAddrInfoW, GetNameInfoW, NI_MAXHOST,
    NI_NAMEREQD, SOCKADDR, SOCKADDR_STORAGE, getpeername, getsockname,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{ResolveFlags, SocketAddress, SocketFamily, SocketHandle};
use crate::platform::{NativeArray, NativeStringRef, PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

/// Free one address-info chain on drop.
struct AddrInfoGuard {
    /// The address-info chain pointer.
    result: *mut ADDRINFOW,
}

impl Drop for AddrInfoGuard {
    /// Free the native address-info chain.
    fn drop(&mut self) {
        if !self.result.is_null() {
            unsafe {
                FreeAddrInfoW(self.result);
            }
        }
    }
}

/// Return the local address bytes for a socket.
pub(crate) unsafe fn destack_net_local_address_raw(
    context: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(context, handle)?;

    // query the local address
    let mut storage = mem::MaybeUninit::<SOCKADDR_STORAGE>::uninit();
    let mut length = mem::size_of::<SOCKADDR_STORAGE>() as i32;
    let rc = unsafe { getsockname(socket, storage.as_mut_ptr() as *mut SOCKADDR, &mut length) };
    if rc != 0 {
        return Err(last_net_error("getsockname"));
    }

    // decode and write the output
    let storage = unsafe { storage.assume_init() };
    let address = socket_address_raw_from_storage(context, &storage, length)?;
    unsafe {
        *out = address;
    }

    Ok(())
}

/// Return the peer address bytes for a socket.
pub(crate) unsafe fn destack_net_peer_address_raw(
    context: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(context, handle)?;

    // query the peer address
    let mut storage = mem::MaybeUninit::<SOCKADDR_STORAGE>::uninit();
    let mut length = mem::size_of::<SOCKADDR_STORAGE>() as i32;
    let rc = unsafe { getpeername(socket, storage.as_mut_ptr() as *mut SOCKADDR, &mut length) };
    if rc != 0 {
        return Err(last_net_error("getpeername"));
    }

    // decode and write the output
    let storage = unsafe { storage.assume_init() };
    let address = socket_address_raw_from_storage(context, &storage, length)?;
    unsafe {
        *out = address;
    }

    Ok(())
}

/// Resolve host and port into raw socket addresses.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_resolve_raw(
    context: &BindingCallContext,
    out: *mut NativeArray<SocketAddress>,
    host: NativeStringRef,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    ensure_winsock()?;

    // decode and validate the host
    let host = unsafe { host.as_str()? };
    if host.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "host contains nul byte",
        ))
        .boxed());
    }
    if flags.0 & 0x4 != 0 && host.parse::<IpAddr>().is_err() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "numeric host required",
        ))
        .boxed());
    }

    // build addrinfo hints
    let mut hints: ADDRINFOW = unsafe { std::mem::zeroed() };
    hints.ai_family = match family {
        SocketFamily::IPv4 => AF_INET as i32,
        SocketFamily::IPv6 => AF_INET6 as i32,
        SocketFamily::Unspecified => AF_UNSPEC as i32,
    };
    hints.ai_socktype = windows_sys::Win32::Networking::WinSock::SOCK_STREAM;
    hints.ai_protocol = windows_sys::Win32::Networking::WinSock::IPPROTO_TCP;
    hints.ai_flags = 0;
    if flags.0 & 0x1 != 0 {
        hints.ai_flags |= AI_PASSIVE as i32;
    }
    if flags.0 & 0x2 != 0 {
        hints.ai_flags |= AI_CANONNAME as i32;
    }
    if flags.0 & 0x4 != 0 {
        hints.ai_flags |= AI_NUMERICHOST as i32;
    }
    if flags.0 & 0x8 != 0 {
        hints.ai_flags |= AI_NUMERICSERV as i32;
    }
    if flags.0 & 0x10 != 0 {
        hints.ai_flags |= AI_V4MAPPED as i32;
    }
    if flags.0 & 0x20 != 0 {
        hints.ai_flags |= AI_ALL as i32;
    }
    if flags.0 & 0x40 != 0 {
        hints.ai_flags |= AI_ADDRCONFIG as i32;
    }

    // resolve addresses
    let host = core_platform::wide_with_nul(host);
    let service = core_platform::wide_with_nul(&port.to_string());
    let mut result: *mut ADDRINFOW = std::ptr::null_mut();
    let rc = unsafe { GetAddrInfoW(host.as_ptr(), service.as_ptr(), &hints, &mut result) };
    if rc != 0 {
        return Err(net_error_with_code("GetAddrInfoW", rc));
    }
    let _guard = AddrInfoGuard { result };

    // collect each raw sockaddr result
    let mut addresses = Vec::new();
    let mut current = result;
    while !current.is_null() {
        let info = unsafe { &*current };
        if !info.ai_addr.is_null() && info.ai_addrlen > 0 {
            let bytes = unsafe {
                std::slice::from_raw_parts(info.ai_addr as *const u8, info.ai_addrlen as usize)
            };
            let family = unsafe { (*info.ai_addr).sa_family };
            addresses.push(SocketAddress {
                family,
                length: info.ai_addrlen as u32,
                bytes: context.store_array(bytes.to_vec()),
            });
        }
        current = info.ai_next;
    }

    unsafe {
        *out = context.store_array(addresses);
    }

    Ok(())
}

/// Reverse lookup a raw socket address into hostnames.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_reverse_lookup_raw(
    context: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    ensure_winsock()?;

    // decode the raw socket address
    with_socket_address_raw(address, |sockaddr, length| {
        // resolve the host name from the socket address
        let mut host = vec![0u16; NI_MAXHOST as usize];
        let rc = unsafe {
            GetNameInfoW(
                sockaddr,
                length,
                host.as_mut_ptr(),
                host.len() as u32,
                std::ptr::null_mut(),
                0,
                NI_NAMEREQD as i32,
            )
        };
        if rc != 0 {
            return Err(net_error_with_code("GetNameInfoW", rc));
        }

        // decode the resolved host name
        let host_length = host
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(host.len());
        let host = String::from_utf16(&host[..host_length]).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "address",
                "resolved host is not valid utf16",
            ))
            .boxed()
        })?;

        // encode the output name list
        let host = context.store_string(&host);
        unsafe {
            *out = context.store_array(vec![host]);
        }

        Ok(())
    })
}
