use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, INVALID_SOCKET, IPPROTO_TCP, SOCK_STREAM, accept, bind, closesocket,
    connect, listen, socket,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::resource::{ListenerHandle, ResourceEntry, ResourceKind, SocketHandle};
use crate::platform::{NativeStringRef, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Accept a pending connection.
pub(crate) unsafe fn destack_net_accept(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    ensure_winsock();

    let socket = listener_descriptor(context, listener)?;
    let client = unsafe { accept(socket, std::ptr::null_mut(), std::ptr::null_mut()) };
    if client == INVALID_SOCKET {
        return Err(last_net_error("accept"));
    }

    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(client as _)
        .with_finalizer(SocketFinalizer::new(client));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = SocketHandle(resource_id);
    }
    Ok(())
}

/// Close a socket handle.
pub(crate) unsafe fn destack_net_close(
    context: &RuntimeCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    let entry = context
        .runtime()
        .resources
        .remove(handle.0)
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                "unknown socket handle",
            ))
            .boxed()
        })?;
    entry.finalize(handle.0);
    Ok(())
}

/// Close a listener handle.
pub(crate) unsafe fn destack_net_close_listener(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    let entry = context
        .runtime()
        .resources
        .remove(handle.0)
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                "unknown listener handle",
            ))
            .boxed()
        })?;
    entry.finalize(handle.0);
    Ok(())
}

/// Connect to a TCP socket.
pub(crate) unsafe fn destack_net_connect(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    host: NativeStringRef,
    port: u16,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    ensure_winsock();

    let host = unsafe { host.as_str()? };
    let addr = socket_address_from_host(host, port).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "expected numeric ip address",
        ))
        .boxed()
    })?;
    let (sockaddr, len) = socket_addr(&addr);
    let domain = match addr {
        std::net::SocketAddr::V4(_) => AF_INET,
        std::net::SocketAddr::V6(_) => AF_INET6,
    };

    let socket = unsafe { socket(domain.into(), SOCK_STREAM, IPPROTO_TCP) };
    if socket == INVALID_SOCKET {
        return Err(last_net_error("socket"));
    }

    let rc = unsafe { connect(socket, sockaddr.as_ptr(), len) };
    if rc != 0 {
        unsafe {
            closesocket(socket);
        }
        return Err(last_net_error("connect"));
    }

    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(socket as _)
        .with_finalizer(SocketFinalizer::new(socket));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = SocketHandle(resource_id);
    }
    Ok(())
}

/// Listen on a TCP socket.
pub(crate) unsafe fn destack_net_listen(
    context: &RuntimeCallContext,
    out: *mut ListenerHandle,
    host: NativeStringRef,
    port: u16,
    backlog: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    ensure_winsock();

    let host = unsafe { host.as_str()? };
    let addr = socket_address_from_host(host, port).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "expected numeric ip address",
        ))
        .boxed()
    })?;
    let (sockaddr, len) = socket_addr(&addr);
    let domain = match addr {
        std::net::SocketAddr::V4(_) => AF_INET,
        std::net::SocketAddr::V6(_) => AF_INET6,
    };

    let socket = unsafe { socket(domain.into(), SOCK_STREAM, IPPROTO_TCP) };
    if socket == INVALID_SOCKET {
        return Err(last_net_error("socket"));
    }

    let rc = unsafe { bind(socket, sockaddr.as_ptr(), len) };
    if rc != 0 {
        unsafe {
            closesocket(socket);
        }
        return Err(last_net_error("bind"));
    }

    let rc = unsafe { listen(socket, backlog as i32) };
    if rc != 0 {
        unsafe {
            closesocket(socket);
        }
        return Err(last_net_error("listen"));
    }

    let entry = ResourceEntry::new(ResourceKind::Listener)
        .with_listener(socket as _)
        .with_finalizer(SocketFinalizer::new(socket));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = ListenerHandle(resource_id);
    }
    Ok(())
}
