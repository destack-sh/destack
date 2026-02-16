#![allow(unused_imports)]

use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::OsPath;
use crate::platform::net::{core as core_net, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::RuntimeCallContext;

use std::ffi::{CStr, CString};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::unix::io::RawFd;

/// Read the local socket address as raw bytes.
pub(crate) unsafe fn destack_net_local_address_raw(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the local address on unix platforms
    // resolve the socket descriptor
    let fd = socket_descriptor(context, handle)?;

    // query the socket address
    let address = socket_address_raw_from_fd(context, fd, "getsockname", libc::getsockname)?;

    // write the output
    unsafe {
        *out = address;
    }

    Ok(())
}

/// Read the remote socket address as raw bytes.
pub(crate) unsafe fn destack_net_peer_address_raw(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the peer address on unix platforms
    // resolve the socket descriptor
    let fd = socket_descriptor(context, handle)?;

    // query the socket address
    let address = socket_address_raw_from_fd(context, fd, "getpeername", libc::getpeername)?;

    // write the output
    unsafe {
        *out = address;
    }

    Ok(())
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
pub(crate) unsafe fn destack_net_resolve(
    context: &RuntimeCallContext,
    out: *mut NativeArray<SocketAddress>,
    host: NativeStringRef,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the host string
    let host = unsafe { host.as_str()? };
    if host.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "host contains nul byte",
        ))
        .boxed());
    }

    // enforce numeric-only resolution when requested
    if flags.0 & 0x4 != 0 && host.parse::<IpAddr>().is_err() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "numeric host required",
        ))
        .boxed());
    }

    // build addrinfo hints
    let mut hints: libc::addrinfo = unsafe { std::mem::zeroed() };
    hints.ai_family = match family {
        SocketFamily::IPv4 => libc::AF_INET,
        SocketFamily::IPv6 => libc::AF_INET6,
        SocketFamily::Unspecified => libc::AF_UNSPEC,
    };
    hints.ai_socktype = libc::SOCK_STREAM;
    hints.ai_protocol = libc::IPPROTO_TCP;
    hints.ai_flags = 0;
    if flags.0 & 0x1 != 0 {
        hints.ai_flags |= libc::AI_PASSIVE;
    }
    if flags.0 & 0x2 != 0 {
        hints.ai_flags |= libc::AI_CANONNAME;
    }
    if flags.0 & 0x4 != 0 {
        hints.ai_flags |= libc::AI_NUMERICHOST;
    }
    if flags.0 & 0x8 != 0 {
        hints.ai_flags |= libc::AI_NUMERICSERV;
    }
    if flags.0 & 0x10 != 0 {
        hints.ai_flags |= libc::AI_V4MAPPED;
    }
    if flags.0 & 0x20 != 0 {
        hints.ai_flags |= libc::AI_ALL;
    }
    if flags.0 & 0x40 != 0 {
        hints.ai_flags |= libc::AI_ADDRCONFIG;
    }

    // resolve addresses
    let host_c = CString::new(host).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "host contains nul byte",
        ))
        .boxed()
    })?;
    let service = CString::new(port.to_string()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "port",
            "port conversion failed",
        ))
        .boxed()
    })?;
    let mut result: *mut libc::addrinfo = std::ptr::null_mut();
    let rc = unsafe { libc::getaddrinfo(host_c.as_ptr(), service.as_ptr(), &hints, &mut result) };
    if rc != 0 {
        let error = unsafe { CStr::from_ptr(libc::gai_strerror(rc)) }
            .to_string_lossy()
            .to_string();
        return Err(
            RuntimeError::from(PlatformError::io(format!("getaddrinfo failed: {error}"))).boxed(),
        );
    }

    struct AddrInfoGuard(*mut libc::addrinfo);
    impl Drop for AddrInfoGuard {
        fn drop(&mut self) {
            unsafe {
                libc::freeaddrinfo(self.0);
            }
        }
    }
    let _guard = AddrInfoGuard(result);

    // collect addresses
    let mut addresses = Vec::new();
    let mut current = result;
    while !current.is_null() {
        let info = unsafe { &*current };
        let storage = unsafe { &*(info.ai_addr as *const libc::sockaddr_storage) };
        let address = socket_address_from_storage(context, storage, info.ai_addrlen)?;
        addresses.push(address);
        current = info.ai_next;
    }

    unsafe {
        *out = context.store_array(addresses);
    }

    Ok(())
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
pub(crate) unsafe fn destack_net_reverse_lookup(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the raw socket address
    let bytes = unsafe { address.bytes.as_slice()? };
    if bytes.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "raw address is empty",
        ))
        .boxed());
    }
    if bytes.len() > std::mem::size_of::<libc::sockaddr_storage>() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "raw address is too large",
        ))
        .boxed());
    }

    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            &mut storage as *mut _ as *mut u8,
            bytes.len(),
        );
    }
    let length = bytes.len() as libc::socklen_t;

    let mut host = [0 as libc::c_char; libc::NI_MAXHOST as usize];
    let rc = unsafe {
        libc::getnameinfo(
            &storage as *const _ as *const libc::sockaddr,
            length,
            host.as_mut_ptr(),
            host.len() as libc::socklen_t,
            std::ptr::null_mut(),
            0,
            libc::NI_NAMEREQD,
        )
    };
    if rc != 0 {
        let error = unsafe { CStr::from_ptr(libc::gai_strerror(rc)) }
            .to_string_lossy()
            .to_string();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "reverse lookup failed: {error}"
        )))
        .boxed());
    }
    let name = unsafe { CStr::from_ptr(host.as_ptr()) }
        .to_string_lossy()
        .to_string();
    unsafe {
        *out = context.store_array(vec![context.store_string(&name)]);
    }

    Ok(())
}

/// Resolve host and port into raw socket addresses.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_resolve_raw(
    context: &RuntimeCallContext,
    out: *mut NativeArray<SocketAddress>,
    host: NativeStringRef,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<()> {
    unsafe { super::destack_net_resolve(context, out, host, port, family, flags) }
}

/// Reverse lookup a raw socket address into hostnames.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_reverse_lookup_raw(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { super::destack_net_reverse_lookup(context, out, address) }
}
