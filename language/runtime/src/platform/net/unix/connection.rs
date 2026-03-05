#![allow(unused_imports)]

use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::OsPath;
use crate::platform::net::{core as core_net, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

use std::ffi::{CStr, CString};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::unix::io::RawFd;

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
pub(crate) unsafe fn destack_net_accept(
    context: &BindingCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
    _flags: AcceptFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // accept sockets on unix platforms
    // resolve the listener descriptor
    let fd = listener_descriptor(context, listener)?;

    // accept the connection
    let client_fd = unsafe { libc::accept(fd, std::ptr::null_mut(), std::ptr::null_mut()) };
    if client_fd < 0 {
        return Err(core_platform::net_error("accept"));
    }

    // register the new socket
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(client_fd)
        .with_finalizer(SocketFinalizer { fd: client_fd });
    let resource_id = context
        .agent()
        .resources
        .insert(entry, Some(context.engine()));
    unsafe {
        *out = SocketHandle(resource_id);
    }

    Ok(())
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
pub(crate) unsafe fn destack_net_close(
    context: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate the handle kind
    let is_socket = context
        .agent()
        .resources
        .with_entry(handle.0, |entry| entry.kind == ResourceKind::Socket)
        .unwrap_or(false);
    if !is_socket {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown socket handle",
        ))
        .boxed());
    }

    // remove the resource and close it
    if !context
        .agent()
        .resources
        .remove_and_finalize(handle.0, Some(context.engine()))
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown socket handle",
        ))
        .boxed());
    }

    Ok(())
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
pub(crate) unsafe fn destack_net_close_listener(
    context: &BindingCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    // validate the handle kind
    let is_listener = context
        .agent()
        .resources
        .with_entry(handle.0, |entry| entry.kind == ResourceKind::Listener)
        .unwrap_or(false);
    if !is_listener {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown listener handle",
        ))
        .boxed());
    }

    // remove the resource and close it
    if !context
        .agent()
        .resources
        .remove_and_finalize(handle.0, Some(context.engine()))
    {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown listener handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Connect an existing socket to a raw remote address.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_connect_raw(
    context: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let fd = socket_descriptor(context, handle)?;

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
#[cfg(unix)]
pub(crate) unsafe fn destack_net_bind(
    context: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let fd = socket_descriptor(context, handle)?;

    // bind using the provided raw sockaddr
    with_socket_address_raw(address, |sockaddr, length| {
        let result = unsafe { libc::bind(fd, sockaddr, length) };
        if result != 0 {
            return Err(RuntimeError::from(PlatformError::io("bind failed".to_string())).boxed());
        }

        Ok(())
    })
}

/// Start listening on a raw local socket address.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_listen_raw(
    context: &BindingCallContext,
    out: *mut ListenerHandle,
    address: SocketAddress,
    backlog: u32,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // select the backlog value
    let backlog = if backlog == 0 {
        libc::SOMAXCONN
    } else {
        backlog.min(i32::MAX as u32) as libc::c_int
    };

    // resolve the socket family from the raw address metadata
    let family = address.family as libc::c_int;

    // create and bind a new listening socket
    with_socket_address_raw(address, |sockaddr, length| {
        let fd = unsafe { libc::socket(family, libc::SOCK_STREAM, libc::IPPROTO_TCP) };
        if fd < 0 {
            let error = std::io::Error::last_os_error();
            return Err(
                RuntimeError::from(PlatformError::io(format!("socket failed: {error}"))).boxed(),
            );
        }

        let enabled: libc::c_int = 1;
        let result = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_REUSEADDR,
                &enabled as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        if result != 0 {
            unsafe {
                libc::close(fd);
            }
            let error = std::io::Error::last_os_error();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "setsockopt failed: {error}"
            )))
            .boxed());
        }

        let result = unsafe { libc::bind(fd, sockaddr, length) };
        if result != 0 {
            unsafe {
                libc::close(fd);
            }
            let error = std::io::Error::last_os_error();
            return Err(
                RuntimeError::from(PlatformError::io(format!("bind failed: {error}"))).boxed(),
            );
        }

        let result = unsafe { libc::listen(fd, backlog) };
        if result != 0 {
            unsafe {
                libc::close(fd);
            }
            let error = std::io::Error::last_os_error();
            return Err(
                RuntimeError::from(PlatformError::io(format!("listen failed: {error}"))).boxed(),
            );
        }

        let entry = ResourceEntry::new(ResourceKind::Listener)
            .with_listener(fd)
            .with_finalizer(DescriptorFinalizer { fd });
        let resource_id = context
            .agent()
            .resources
            .insert(entry, Some(context.engine()));
        unsafe {
            *out = ListenerHandle(resource_id);
        }

        Ok(())
    })
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
#[cfg(unix)]
pub(crate) unsafe fn destack_net_socket(
    context: &BindingCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // create the requested socket
    let fd = unsafe {
        libc::socket(
            socket_family_to_raw(family),
            socket_type.0 as libc::c_int,
            protocol.0 as libc::c_int,
        )
    };
    if fd < 0 {
        return Err(RuntimeError::from(PlatformError::io("socket failed".to_string())).boxed());
    }

    // register the socket handle
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(fd)
        .with_finalizer(DescriptorFinalizer { fd });
    let resource_id = context
        .agent()
        .resources
        .insert(entry, Some(context.engine()));
    unsafe {
        *out = SocketHandle(resource_id);
    }

    Ok(())
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
#[cfg(unix)]
pub(crate) unsafe fn destack_net_socket_pair(
    context: &BindingCallContext,
    out: *mut SocketPair,
    family: SocketFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // map the family to socketpair-compatible values
    let family = match family {
        SocketFamily::Unspecified => libc::AF_UNIX,
        SocketFamily::IPv4 | SocketFamily::IPv6 => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "family",
                "socketPair requires SocketFamily.Unspecified",
            ))
            .boxed());
        }
    };

    // create the pair
    let mut pair = [0 as RawFd; 2];
    let result = unsafe {
        libc::socketpair(
            family,
            socket_type.0 as libc::c_int,
            protocol.0 as libc::c_int,
            pair.as_mut_ptr(),
        )
    };
    if result != 0 {
        return Err(RuntimeError::from(PlatformError::io("socketpair failed".to_string())).boxed());
    }

    // register both sockets
    let first_entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(pair[0])
        .with_finalizer(DescriptorFinalizer { fd: pair[0] });
    let first_id = context
        .agent()
        .resources
        .insert(first_entry, Some(context.engine()));

    let second_entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(pair[1])
        .with_finalizer(DescriptorFinalizer { fd: pair[1] });
    let second_id = context
        .agent()
        .resources
        .insert(second_entry, Some(context.engine()));

    unsafe {
        *out = SocketPair {
            first: SocketHandle(first_id),
            second: SocketHandle(second_id),
        };
    }

    Ok(())
}
