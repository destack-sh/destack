use std::mem;

use windows_sys::Win32::Networking::WinSock::{
    AF_UNIX, INVALID_SOCKET, SOCK_STREAM, SOCKADDR, SOCKADDR_UN, bind, connect, listen, socket,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::{OsPath, PathEncoding};
use crate::platform::net::{AcceptFlags, ListenerHandle, SocketHandle};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::runtime::RuntimeCallContext;

/// Convert a OsPath into a UTF-8 byte buffer.
fn uds_path_bytes(path: OsPath) -> RuntimeResult<Vec<u8>> {
    match path.encoding {
        // decode bytes paths directly
        PathEncoding::Bytes => {
            let bytes = unsafe { path.bytes.0.as_slice()? };
            Ok(bytes.to_vec())
        }

        // decode utf16 paths into a UTF-8 byte path
        PathEncoding::Utf16 => {
            let utf16 = unsafe { path.utf16.0.as_slice()? };
            let string = String::from_utf16(utf16).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "path",
                    "path contains invalid utf16",
                ))
                .boxed()
            })?;
            Ok(string.into_bytes())
        }
    }
}

/// Build a sockaddr_un for a UNIX domain socket path.
fn uds_sockaddr(path: OsPath) -> RuntimeResult<(SOCKADDR_UN, i32)> {
    let path = uds_path_bytes(path)?;
    let mut addr: SOCKADDR_UN = unsafe { mem::zeroed() };
    addr.sun_family = AF_UNIX;
    let max_len = addr.sun_path.len();
    if path.len() >= max_len {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "path too long for unix domain socket",
        ))
        .boxed());
    }
    for (idx, byte) in path.iter().enumerate() {
        addr.sun_path[idx] = *byte;
    }
    Ok((addr, mem::size_of::<SOCKADDR_UN>() as i32))
}

/// Connect to a UNIX domain socket.
pub(crate) unsafe fn destack_net_uds_connect(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    ensure_winsock()?;

    // build the socket address
    let (addr, len) = uds_sockaddr(path)?;

    // create and connect the socket
    let socket = unsafe { socket(AF_UNIX as i32, SOCK_STREAM, 0) };
    if socket == INVALID_SOCKET {
        return Err(last_net_error("socket"));
    }
    let rc = unsafe { connect(socket, &addr as *const _ as *const SOCKADDR, len) };
    if rc != 0 {
        unsafe {
            windows_sys::Win32::Networking::WinSock::closesocket(socket);
        }
        return Err(last_net_error("connect"));
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

/// Listen on a UNIX domain socket.
pub(crate) unsafe fn destack_net_uds_listen(
    context: &RuntimeCallContext,
    out: *mut ListenerHandle,
    path: OsPath,
    backlog: u32,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    ensure_winsock()?;

    // build the socket address
    let (addr, len) = uds_sockaddr(path)?;

    // create the socket
    let socket = unsafe { socket(AF_UNIX as i32, SOCK_STREAM, 0) };
    if socket == INVALID_SOCKET {
        return Err(last_net_error("socket"));
    }

    // bind the socket
    let rc = unsafe { bind(socket, &addr as *const _ as *const SOCKADDR, len) };
    if rc != 0 {
        unsafe {
            windows_sys::Win32::Networking::WinSock::closesocket(socket);
        }
        return Err(last_net_error("bind"));
    }

    // listen for connections
    let backlog = i32::try_from(backlog).unwrap_or(i32::MAX);
    let rc = unsafe { listen(socket, backlog) };
    if rc != 0 {
        unsafe {
            windows_sys::Win32::Networking::WinSock::closesocket(socket);
        }
        return Err(last_net_error("listen"));
    }

    // register the listener
    let entry = ResourceEntry::new(ResourceKind::Listener)
        .with_socket(socket as _)
        .with_finalizer(SocketFinalizer::new(socket));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = ListenerHandle(resource_id);
    }

    Ok(())
}

/// Accept a UNIX domain socket connection.
pub(crate) unsafe fn destack_net_uds_accept(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { super::destack_net_accept(context, out, listener, AcceptFlags(0)) }
}

/// Close a UNIX domain socket listener.
pub(crate) unsafe fn destack_net_uds_close_listener(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { super::destack_net_close_listener(context, handle) }
}
