use super::core::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::OsPath;
use crate::platform::net::{core as core_net, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

use super::{destack_net_close_listener, destack_net_socket_pair};

use std::ffi::CString;

/// Connect to a UDS endpoint.
pub(crate) unsafe fn destack_net_uds_connect(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the path
    let path = core_net::unix_path_bytes(path, "path")?;
    let path = path.as_slice();

    // build the socket address
    let (addr, length, _) = sockaddr_un_from_path(path)?;

    // open and connect the socket
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(core_platform::net_error("socket"));
    }
    let rc = unsafe { libc::connect(fd, &addr as *const _ as *const libc::sockaddr, length) };
    if rc != 0 {
        unsafe {
            libc::close(fd);
        }
        return Err(core_platform::net_error("connect"));
    }

    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(fd)
        .with_finalizer(SocketFinalizer { fd });
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

/// Listen on a UDS address.
pub(crate) unsafe fn destack_net_uds_listen(
    binding: &BindingCallContext,
    out: *mut ListenerHandle,
    path: OsPath,
    backlog: u32,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the path
    let path = core_net::unix_path_bytes(path, "path")?;
    let path = path.as_slice();

    // build the socket address
    let (addr, length, is_abstract) = sockaddr_un_from_path(path)?;

    // remove any pre-existing socket file
    if !is_abstract {
        let path_c = CString::new(path).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path contains nul byte",
            ))
            .boxed()
        })?;
        unsafe {
            libc::unlink(path_c.as_ptr());
        }
    }

    // open and bind the socket
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(core_platform::net_error("socket"));
    }
    let rc = unsafe { libc::bind(fd, &addr as *const _ as *const libc::sockaddr, length) };
    if rc != 0 {
        unsafe {
            libc::close(fd);
        }
        return Err(core_platform::net_error("bind"));
    }

    // start listening on the socket
    let backlog = if backlog == 0 {
        libc::SOMAXCONN
    } else {
        backlog.min(i32::MAX as u32) as libc::c_int
    };
    let rc = unsafe { libc::listen(fd, backlog) };
    if rc != 0 {
        unsafe {
            libc::close(fd);
        }
        return Err(core_platform::net_error("listen"));
    }
    let entry = ResourceEntry::new(ResourceKind::Listener)
        .with_listener(fd)
        .with_finalizer(SocketFinalizer { fd });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = ListenerHandle(resource_id);
    }

    Ok(())
}

/// Accept a connection from a UDS listener.
pub(crate) unsafe fn destack_net_uds_accept(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // accept via the raw file descriptor
    let fd = listener_descriptor(binding, listener)?;
    let fd = unsafe { libc::accept(fd, std::ptr::null_mut(), std::ptr::null_mut()) };
    if fd < 0 {
        return Err(core_platform::net_error("accept"));
    }
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(fd)
        .with_finalizer(SocketFinalizer { fd });
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

/// Close a UDS listener handle.
pub(crate) unsafe fn destack_net_uds_close_listener(
    binding: &BindingCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { destack_net_close_listener(binding, handle) }
}

/// Create a connected UDS socket pair.
pub(crate) unsafe fn destack_net_uds_socket_pair(
    binding: &BindingCallContext,
    out: *mut SocketPair,
    socket_type: SocketType,
) -> RuntimeResult<()> {
    unsafe {
        destack_net_socket_pair(
            binding,
            out,
            SocketFamily::Unspecified,
            socket_type,
            SocketProtocol(0),
        )
    }
}
