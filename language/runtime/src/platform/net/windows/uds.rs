use std::mem;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use windows_sys::Win32::Foundation::MAX_PATH;
use windows_sys::Win32::Networking::WinSock::{
    AF_UNIX, INVALID_SOCKET, SOCK_DGRAM, SOCK_STREAM, SOCKADDR, SOCKADDR_UN, WSAEAFNOSUPPORT,
    WSAEINVAL, WSAEPROTONOSUPPORT, accept, bind, closesocket, connect, listen, socket,
};
use windows_sys::Win32::Storage::FileSystem::{DeleteFileW, GetTempPathW};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::OsPath;
use crate::platform::net::{AcceptFlags, SocketHandle, SocketPair, SocketType};
use crate::platform::resource::{ListenerHandle, ResourceEntry, ResourceKind};
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
        .worker()
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

/// Decode one utf8 UDS path for filesystem cleanup when possible.
fn uds_path_text(path: &[u8]) -> Option<String> {
    String::from_utf8(path.to_vec()).ok()
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
    let mut buffer = vec![0_u16; MAX_PATH as usize + 1];
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

/// Return whether one Winsock error means AF_UNIX is unavailable at runtime.
fn is_uds_runtime_not_supported(code: i32) -> bool {
    code == WSAEAFNOSUPPORT || code == WSAEPROTONOSUPPORT || code == WSAEINVAL
}

/// Build one UDS socket creation error with runtime support mapping.
fn uds_socket_error(operation: &'static str, code: i32) -> Box<RuntimeError> {
    if is_uds_runtime_not_supported(code) {
        return RuntimeError::from(PlatformError::not_supported(operation)).boxed();
    }

    core_platform::net_error_with_code("socket", code)
}

/// Register two sockets as one runtime socket pair.
fn register_socket_pair(
    binding: &BindingCallContext,
    out: *mut SocketPair,
    first_socket: usize,
    second_socket: usize,
) {
    // register the first socket
    let first_entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(first_socket as _)
        .with_finalizer(SocketFinalizer::new(first_socket));
    let first_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), first_entry, Some(binding.engine()));

    // register the second socket
    let second_entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(second_socket as _)
        .with_finalizer(SocketFinalizer::new(second_socket));
    let second_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), second_entry, Some(binding.engine()));

    // write the pair output
    unsafe {
        *out = SocketPair {
            first: SocketHandle(first_id),
            second: SocketHandle(second_id),
        };
    }
}

/// Build one connected datagram UDS pair through two bound loopback-style endpoints.
fn uds_datagram_socket_pair(
    binding: &BindingCallContext,
    out: *mut SocketPair,
) -> RuntimeResult<()> {
    // allocate two temporary paths for the datagram endpoints
    let first_path = temporary_uds_socket_pair_path(binding);
    let second_path = temporary_uds_socket_pair_path(binding);
    let (first_address, first_length) = uds_sockaddr_from_bytes(first_path.as_bytes())?;
    let (second_address, second_length) = uds_sockaddr_from_bytes(second_path.as_bytes())?;

    // create the first endpoint
    let first_socket = unsafe { socket(AF_UNIX as i32, SOCK_DGRAM, 0) };
    if first_socket == INVALID_SOCKET {
        let code = core_platform::last_wsa_error_code();
        return Err(uds_socket_error("destack.net.udsSocketPair", code));
    }

    // create the second endpoint
    let second_socket = unsafe { socket(AF_UNIX as i32, SOCK_DGRAM, 0) };
    if second_socket == INVALID_SOCKET {
        unsafe {
            closesocket(first_socket);
        }
        let code = core_platform::last_wsa_error_code();
        return Err(uds_socket_error("destack.net.udsSocketPair", code));
    }

    // bind the first endpoint
    let rc = unsafe {
        bind(
            first_socket,
            &first_address as *const _ as *const SOCKADDR,
            first_length,
        )
    };
    if rc != 0 {
        unsafe {
            closesocket(second_socket);
            closesocket(first_socket);
        }
        delete_socket_path(&first_path);
        return Err(core_platform::net_error_with_code(
            "bind",
            core_platform::last_wsa_error_code(),
        ));
    }

    // bind the second endpoint
    let rc = unsafe {
        bind(
            second_socket,
            &second_address as *const _ as *const SOCKADDR,
            second_length,
        )
    };
    if rc != 0 {
        unsafe {
            closesocket(second_socket);
            closesocket(first_socket);
        }
        delete_socket_path(&second_path);
        delete_socket_path(&first_path);
        return Err(core_platform::net_error_with_code(
            "bind",
            core_platform::last_wsa_error_code(),
        ));
    }

    // connect the first endpoint to the second path
    let rc = unsafe {
        connect(
            first_socket,
            &second_address as *const _ as *const SOCKADDR,
            second_length,
        )
    };
    if rc != 0 {
        unsafe {
            closesocket(second_socket);
            closesocket(first_socket);
        }
        delete_socket_path(&second_path);
        delete_socket_path(&first_path);
        return Err(core_platform::net_error_with_code(
            "connect",
            core_platform::last_wsa_error_code(),
        ));
    }

    // connect the second endpoint to the first path
    let rc = unsafe {
        connect(
            second_socket,
            &first_address as *const _ as *const SOCKADDR,
            first_length,
        )
    };
    if rc != 0 {
        unsafe {
            closesocket(second_socket);
            closesocket(first_socket);
        }
        delete_socket_path(&second_path);
        delete_socket_path(&first_path);
        return Err(core_platform::net_error_with_code(
            "connect",
            core_platform::last_wsa_error_code(),
        ));
    }

    // unlink both filesystem paths after the pair is established
    delete_socket_path(&second_path);
    delete_socket_path(&first_path);

    // publish both endpoints
    register_socket_pair(binding, out, first_socket as usize, second_socket as usize);

    Ok(())
}

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

    // ensure winsock is initialized
    core_platform::ensure_winsock()?;

    // build the socket address
    let (addr, len) = uds_sockaddr(path)?;

    // create and connect the socket
    let socket = unsafe { socket(AF_UNIX as i32, SOCK_STREAM, 0) };
    if socket == INVALID_SOCKET {
        let code = core_platform::last_wsa_error_code();
        return Err(uds_socket_error("destack.net.udsConnect", code));
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

    // ensure winsock is initialized
    core_platform::ensure_winsock()?;

    // decode the path so bind can clean up one stale socket path first
    let path = uds_path_bytes(path)?;
    let stale_path = uds_path_text(&path);
    let (addr, len) = uds_sockaddr_from_bytes(&path)?;

    // create the socket
    let socket = unsafe { socket(AF_UNIX as i32, SOCK_STREAM, 0) };
    if socket == INVALID_SOCKET {
        let code = core_platform::last_wsa_error_code();
        return Err(uds_socket_error("destack.net.udsListen", code));
    }

    // remove one pre-existing filesystem socket path before bind
    if let Some(path) = stale_path.as_deref() {
        delete_socket_path(path);
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
    unsafe { destack_net_accept(binding, out, listener, AcceptFlags(0)) }
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
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // emulate datagram pairs by binding and cross-connecting two endpoints
    if socket_type.0 as i32 == SOCK_DGRAM {
        return uds_datagram_socket_pair(binding, out);
    }

    // reject socket kinds that are not currently emulatable
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
        let code = core_platform::last_wsa_error_code();
        return Err(uds_socket_error("destack.net.udsSocketPair", code));
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
        let code = core_platform::last_wsa_error_code();
        return Err(uds_socket_error("destack.net.udsSocketPair", code));
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

    // publish both endpoints
    register_socket_pair(binding, out, client_socket as usize, server_socket as usize);

    Ok(())
}
