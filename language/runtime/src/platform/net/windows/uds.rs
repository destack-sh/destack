use std::mem;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use windows_sys::Win32::Networking::WinSock::{
    AF_UNIX, INVALID_SOCKET, SOCK_STREAM, SOCKADDR, SOCKADDR_UN, accept, bind, closesocket,
    connect, listen, socket,
};
use windows_sys::Win32::Storage::FileSystem::{DeleteFileW, GetTempPathW};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::OsPath;
use crate::platform::net::{AcceptFlags, ListenerHandle, SocketHandle, SocketPair, SocketType};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

use super::connection::{destack_net_accept, destack_net_close_listener};

/// Monotonic suffix for temporary UDS socket-pair paths.
#[derive(Debug, Default)]
pub(crate) struct WindowsUdsRuntimeState {
    /// Monotonic suffix for temporary UDS socket-pair paths.
    next_socket_pair_id: AtomicU64,
}

/// Return runtime-owned state for windows UDS helpers.
fn windows_uds_runtime_state(binding: &BindingCallContext) -> Arc<WindowsUdsRuntimeState> {
    binding
        .agent()
        .platform_state
        .net
        .windows_uds_runtime_state(|| WindowsUdsRuntimeState {
            next_socket_pair_id: AtomicU64::new(1),
        })
}

/// Convert a OsPath into a UTF-8 byte buffer.
fn uds_path_bytes(path: OsPath) -> RuntimeResult<Vec<u8>> {
    match path {
        // decode bytes paths directly
        OsPath::OsPathBytes(path_bytes) => {
            let bytes = unsafe { path_bytes.bytes.0.as_slice()? };
            Ok(bytes.to_vec())
        }

        // decode utf16 paths into a UTF-8 byte path
        OsPath::OsPathUtf16(path_utf16) => {
            let utf16 = unsafe { path_utf16.utf16.0.as_slice()? };
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
fn uds_sockaddr_from_bytes(path: &[u8]) -> RuntimeResult<(SOCKADDR_UN, i32)> {
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

/// Build a sockaddr_un for a UNIX domain socket path.
fn uds_sockaddr(path: OsPath) -> RuntimeResult<(SOCKADDR_UN, i32)> {
    let path = uds_path_bytes(path)?;
    uds_sockaddr_from_bytes(&path)
}

/// Build one unique temporary path for UDS socket-pair emulation.
fn temporary_uds_socket_pair_path(binding: &BindingCallContext) -> String {
    // build a deterministic short filename suffix
    let runtime_state = windows_uds_runtime_state(binding);
    let pid = unsafe { GetCurrentProcessId() };
    let suffix = runtime_state
        .next_socket_pair_id
        .fetch_add(1, Ordering::Relaxed);
    let file_name = format!("destack-net-{pid}-{suffix}.sock");

    // prefer the system temp directory when it fits into sockaddr_un
    let temp_directory = windows_temp_directory();
    if !temp_directory.is_empty() {
        let separator = if temp_directory.ends_with('\\') || temp_directory.ends_with('/') {
            ""
        } else {
            "\\"
        };
        let temp_text = format!("{temp_directory}{separator}{file_name}");
        if temp_text.len() < unsafe { mem::zeroed::<SOCKADDR_UN>() }.sun_path.len() {
            return temp_text;
        }
    }

    // otherwise fall back to a short relative path
    file_name
}

/// Resolve the host temporary directory from Windows APIs.
fn windows_temp_directory() -> String {
    let mut buffer = vec![0_u16; 4096];
    let written = unsafe { GetTempPathW(buffer.len() as u32, buffer.as_mut_ptr()) };
    if written == 0 || written as usize >= buffer.len() {
        return String::new();
    }

    String::from_utf16_lossy(&buffer[..written as usize])
}

/// Best-effort cleanup for one temporary socket path.
fn delete_socket_path(path: &str) {
    let mut path_wide: Vec<u16> = path.encode_utf16().collect();
    path_wide.push(0);

    let _ = unsafe { DeleteFileW(path_wide.as_ptr()) };
}

/// Connect to a UDS endpoint.
///
/// Connect to an existing AF_UNIX endpoint address.
/// Address kind and endpoint type validation follow host AF_UNIX semantics.
///
/// # Platform
/// Unix and Windows only when AF_UNIX support is available at runtime.
/// Uses connect(2) on AF_UNIX sockets on Unix and AF_UNIX connect on Windows where available.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.connect`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_uds_connect(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    core_platform::ensure_winsock()?;

    // build the socket address
    let (addr, len) = uds_sockaddr(path)?;

    // create and connect the socket
    let socket = unsafe { socket(AF_UNIX as i32, SOCK_STREAM, 0) };
    if socket == INVALID_SOCKET {
        return Err(core_platform::net_error_with_code(
            "socket",
            core_platform::last_wsa_error_code(),
        ));
    }
    let rc = unsafe { connect(socket, &addr as *const _ as *const SOCKADDR, len) };
    if rc != 0 {
        unsafe {
            windows_sys::Win32::Networking::WinSock::closesocket(socket);
        }
        return Err(core_platform::net_error_with_code(
            "connect",
            core_platform::last_wsa_error_code(),
        ));
    }

    // register the socket
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(socket as _)
        .with_finalizer(SocketFinalizer::new(socket));
    let resource_id =
        binding
            .agent()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = SocketHandle(resource_id);
    }

    Ok(())
}

/// Listen on a UDS address.
///
/// Place the bound socket into passive listen mode with the requested backlog semantics.
/// Path addresses are forwarded from `OsPath` without runtime normalization, while abstract and unnamed addresses follow host AF_UNIX rules.
///
/// # Platform
/// Unix and Windows only when AF_UNIX support is available at runtime.
/// Uses bind(2)+listen(2) on AF_UNIX sockets on Unix and AF_UNIX listen on Windows where available.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.listen`.
///
/// # Replay
/// External, recordable.
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

    // ensure winsock is initialized
    core_platform::ensure_winsock()?;

    // build the socket address
    let (addr, len) = uds_sockaddr(path)?;

    // create the socket
    let socket = unsafe { socket(AF_UNIX as i32, SOCK_STREAM, 0) };
    if socket == INVALID_SOCKET {
        return Err(core_platform::net_error_with_code(
            "socket",
            core_platform::last_wsa_error_code(),
        ));
    }

    // bind the socket
    let rc = unsafe { bind(socket, &addr as *const _ as *const SOCKADDR, len) };
    if rc != 0 {
        unsafe {
            windows_sys::Win32::Networking::WinSock::closesocket(socket);
        }
        return Err(core_platform::net_error_with_code(
            "bind",
            core_platform::last_wsa_error_code(),
        ));
    }

    // listen for connections
    let backlog = i32::try_from(backlog).unwrap_or(i32::MAX);
    let rc = unsafe { listen(socket, backlog) };
    if rc != 0 {
        unsafe {
            windows_sys::Win32::Networking::WinSock::closesocket(socket);
        }
        return Err(core_platform::net_error_with_code(
            "listen",
            core_platform::last_wsa_error_code(),
        ));
    }

    // register the listener
    let entry = ResourceEntry::new(ResourceKind::Listener)
        .with_socket(socket as _)
        .with_finalizer(SocketFinalizer::new(socket));
    let resource_id =
        binding
            .agent()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = ListenerHandle(resource_id);
    }

    Ok(())
}

/// Accept a connection from a UDS listener.
///
/// Accept the next pending AF_UNIX stream connection.
/// Accept ordering and descriptor flags follow host kernel semantics.
///
/// # Platform
/// Unix and Windows only when AF_UNIX support is available at runtime.
/// Uses accept(2) on AF_UNIX sockets on Unix and accept on Windows where available.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.accept`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_uds_accept(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { destack_net_accept(binding, out, listener, AcceptFlags(0)) }
}

/// Close a UDS listener handle.
///
/// Close an AF_UNIX listener socket descriptor.
/// Pending accepts are interrupted according to host kernel semantics.
///
/// # Platform
/// Unix and Windows only when AF_UNIX support is available at runtime.
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
pub(crate) unsafe fn destack_net_uds_close_listener(
    binding: &BindingCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { destack_net_close_listener(binding, handle) }
}

/// Create a connected UDS socket pair.
///
/// Allocate a connected AF_UNIX socket pair for local full-duplex messaging.
/// Pair semantics and descriptor inheritance follow host kernel behavior.
///
/// # Platform
/// Unix and Windows only when AF_UNIX support is available at runtime.
/// Uses socketpair(AF_UNIX) on Unix and runtime emulation on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.connect`.
///
/// # Replay
/// External, recordable.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_uds_socket_pair(
    binding: &BindingCallContext,
    out: *mut SocketPair,
    socket_type: SocketType,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // only stream sockets are currently emulatable
    if socket_type.0 as i32 != SOCK_STREAM {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.net.udsSocketPair")).boxed(),
        );
    }

    // ensure winsock is initialized
    core_platform::ensure_winsock()?;

    // allocate one temporary filesystem path for the pair setup
    let socket_path = temporary_uds_socket_pair_path(binding);
    let (address, address_length) = uds_sockaddr_from_bytes(socket_path.as_bytes())?;

    // create one listener socket
    let listener_socket = unsafe { socket(AF_UNIX as i32, SOCK_STREAM, 0) };
    if listener_socket == INVALID_SOCKET {
        return Err(core_platform::net_error_with_code(
            "socket",
            core_platform::last_wsa_error_code(),
        ));
    }

    // bind the listener on the temporary path
    let rc = unsafe {
        bind(
            listener_socket,
            &address as *const _ as *const SOCKADDR,
            address_length,
        )
    };
    if rc != 0 {
        unsafe {
            closesocket(listener_socket);
        }
        return Err(core_platform::net_error_with_code(
            "bind",
            core_platform::last_wsa_error_code(),
        ));
    }

    // listen for one incoming connection
    let rc = unsafe { listen(listener_socket, 1) };
    if rc != 0 {
        unsafe {
            closesocket(listener_socket);
        }
        delete_socket_path(&socket_path);
        return Err(core_platform::net_error_with_code(
            "listen",
            core_platform::last_wsa_error_code(),
        ));
    }

    // connect one client socket
    let client_socket = unsafe { socket(AF_UNIX as i32, SOCK_STREAM, 0) };
    if client_socket == INVALID_SOCKET {
        unsafe {
            closesocket(listener_socket);
        }
        delete_socket_path(&socket_path);
        return Err(core_platform::net_error_with_code(
            "socket",
            core_platform::last_wsa_error_code(),
        ));
    }
    let rc = unsafe {
        connect(
            client_socket,
            &address as *const _ as *const SOCKADDR,
            address_length,
        )
    };
    if rc != 0 {
        unsafe {
            closesocket(client_socket);
            closesocket(listener_socket);
        }
        delete_socket_path(&socket_path);
        return Err(core_platform::net_error_with_code(
            "connect",
            core_platform::last_wsa_error_code(),
        ));
    }

    // accept one server-side socket
    let server_socket =
        unsafe { accept(listener_socket, std::ptr::null_mut(), std::ptr::null_mut()) };
    if server_socket == INVALID_SOCKET {
        unsafe {
            closesocket(client_socket);
            closesocket(listener_socket);
        }
        delete_socket_path(&socket_path);
        return Err(core_platform::net_error_with_code(
            "accept",
            core_platform::last_wsa_error_code(),
        ));
    }

    // close and cleanup listener resources
    unsafe {
        closesocket(listener_socket);
    }
    delete_socket_path(&socket_path);

    // register the first socket
    let first_entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(client_socket as _)
        .with_finalizer(SocketFinalizer::new(client_socket));
    let first_id =
        binding
            .agent()
            .resources
            .insert(binding.world(), first_entry, Some(binding.engine()));

    // register the second socket
    let second_entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(server_socket as _)
        .with_finalizer(SocketFinalizer::new(server_socket));
    let second_id =
        binding
            .agent()
            .resources
            .insert(binding.world(), second_entry, Some(binding.engine()));

    // write the pair output
    unsafe {
        *out = SocketPair {
            first: SocketHandle(first_id),
            second: SocketHandle(second_id),
        };
    }

    Ok(())
}
