use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, SOCKADDR, SOCKADDR_STORAGE, SOCKET, WSAGetLastError, closesocket,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{SocketAddress, SocketFamily, core as core_net};
use crate::platform::resource::{ListenerHandle, ResourceFinalizer, ResourceKind, SocketHandle};
use crate::platform::{PlatformError, ResourceId, core as core_platform};
use crate::runtime::BindingCallContext;

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

/// Ensure that WinSock is initialized for the current process.
pub(super) fn ensure_winsock() -> RuntimeResult<()> {
    core_platform::ensure_winsock()
}

/// Return one network error for the specified WinSock code.
pub(super) fn net_error_with_code(syscall: &'static str, code: i32) -> Box<RuntimeError> {
    core_platform::net_error_with_code(syscall, code)
}

/// Return one network error from the current WinSock error code.
pub(super) fn last_net_error(syscall: &'static str) -> Box<RuntimeError> {
    core_platform::net_error_with_code(syscall, unsafe { WSAGetLastError() })
}

/// Resolve a socket descriptor from a handle.
pub(super) fn socket_descriptor(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<SOCKET> {
    // resolve the socket resource
    let socket =
        core_net::require_resource(binding, handle.0, ResourceKind::Socket, "socket", |entry| {
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

/// Convert a socket family enum into a WinSock address family.
pub(super) fn socket_family_to_raw(family: SocketFamily) -> i32 {
    match family {
        SocketFamily::Unspecified => AF_INET as i32,
        SocketFamily::IPv4 => AF_INET as i32,
        SocketFamily::IPv6 => AF_INET6 as i32,
    }
}

/// Convert one raw address payload into a sockaddr pointer and length.
pub(super) fn with_socket_address_raw<T>(
    address: SocketAddress,
    with_sockaddr: impl FnOnce(*const SOCKADDR, i32) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    // validate the raw address bytes
    let bytes = unsafe { address.bytes.as_slice()? };
    if bytes.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "raw address is empty",
        ))
        .boxed());
    }
    if bytes.len() > std::mem::size_of::<SOCKADDR_STORAGE>() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "raw address is too large",
        ))
        .boxed());
    }

    // choose the length from explicit metadata or payload size
    let address_length = if address.length == 0 {
        bytes.len()
    } else {
        address.length as usize
    };
    if address_length == 0 || address_length > bytes.len() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "raw address length is invalid",
        ))
        .boxed());
    }

    // dispatch to the syscall closure
    let pointer = bytes.as_ptr() as *const SOCKADDR;
    with_sockaddr(pointer, address_length as i32)
}

/// Encode raw sockaddr storage into a socket address payload.
pub(super) fn socket_address_raw_from_storage(
    binding: &BindingCallContext,
    storage: &SOCKADDR_STORAGE,
    length: i32,
) -> RuntimeResult<SocketAddress> {
    // validate the returned length
    if length <= 0 || length as usize > std::mem::size_of::<SOCKADDR_STORAGE>() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "socket address length out of range",
        ))
        .boxed());
    }

    // copy the exact sockaddr bytes
    let bytes = unsafe {
        let pointer = storage as *const _ as *const u8;
        std::slice::from_raw_parts(pointer, length as usize)
    };
    let bytes = binding.store_array_copy(bytes);

    // build the raw address payload
    Ok(SocketAddress {
        family: storage.ss_family,
        length: length as u32,
        bytes,
    })
}

/// Resolve a listener descriptor from a handle.
pub(super) fn listener_descriptor(
    binding: &BindingCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<SOCKET> {
    // resolve the listener resource
    let socket = core_net::require_resource(
        binding,
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
