use windows_sys::Win32::Networking::WinSock::{INVALID_SOCKET, accept};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::net::AcceptFlags;
use crate::platform::resource::{ListenerHandle, ResourceEntry, ResourceKind, SocketHandle};
use crate::runtime::RuntimeCallContext;

/// Accept a pending connection.
pub(crate) unsafe fn destack_net_accept(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
    _flags: AcceptFlags,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    ensure_winsock()?;

    // accept the connection
    let socket = listener_descriptor(context, listener)?;
    let client = unsafe { accept(socket, std::ptr::null_mut(), std::ptr::null_mut()) };
    if client == INVALID_SOCKET {
        return Err(last_net_error("accept"));
    }

    // register the socket
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
    // remove the resource entry
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

    // finalize the socket
    entry.finalize(handle.0);

    Ok(())
}

/// Close a listener handle.
pub(crate) unsafe fn destack_net_close_listener(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    // remove the resource entry
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

    // finalize the listener
    entry.finalize(handle.0);

    Ok(())
}
