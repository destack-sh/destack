use windows_sys::Win32::Networking::WinSock::{SOCKET, closesocket};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::core as core_net;
use crate::platform::resource::{ListenerHandle, ResourceFinalizer, ResourceKind, SocketHandle};
use crate::platform::{PlatformError, ResourceId, core as core_platform};
use crate::runtime::RuntimeCallContext;

/// Finalizer that closes a socket handle.
#[derive(Debug)]
pub(super) struct SocketFinalizer {
    /// Raw socket descriptor to close.
    socket: SOCKET,
}

impl SocketFinalizer {
    /// Create a socket finalizer for a raw socket.
    pub(super) fn new(socket: SOCKET) -> Self {
        Self { socket }
    }
}

impl ResourceFinalizer for SocketFinalizer {
    /// Close the socket when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            closesocket(self.socket);
        }
    }
}

/// Ensure Winsock is initialized for networking operations.
pub(super) fn ensure_winsock() -> RuntimeResult<()> {
    core_platform::ensure_winsock()
}

/// Build a runtime error from the last socket error.
pub(super) fn last_net_error(syscall: &str) -> Box<RuntimeError> {
    core_platform::net_error_with_code(syscall, core_platform::last_wsa_error_code())
}

/// Resolve a socket descriptor from a handle.
pub(super) fn socket_descriptor(
    context: &RuntimeCallContext,
    handle: SocketHandle,
) -> RuntimeResult<SOCKET> {
    // resolve the socket resource
    let socket =
        core_net::require_resource(context, handle.0, ResourceKind::Socket, "socket", |entry| {
            entry.socket().ok_or_else(|| {
                RuntimeError::from(PlatformError::generic(
                    None,
                    "socket handle missing payload",
                ))
                .boxed()
            })
        })?;

    // cast to a raw socket
    Ok(socket as SOCKET)
}

/// Resolve a listener descriptor from a handle.
pub(super) fn listener_descriptor(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<SOCKET> {
    // resolve the listener resource
    let socket = core_net::require_resource(
        context,
        handle.0,
        ResourceKind::Listener,
        "listener",
        |entry| {
            entry.socket().ok_or_else(|| {
                RuntimeError::from(PlatformError::generic(
                    None,
                    "listener handle missing payload",
                ))
                .boxed()
            })
        },
    )?;

    // cast to a raw socket
    Ok(socket as SOCKET)
}
