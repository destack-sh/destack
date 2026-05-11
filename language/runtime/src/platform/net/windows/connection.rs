use windows_sys::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};
use windows_sys::Win32::Networking::WinSock::{
    IN_ADDR, IN_ADDR_0, IN6_ADDR, IN6_ADDR_0, INVALID_SOCKET, IPPROTO_TCP, SO_REUSEADDR,
    SOCK_DGRAM, SOCK_STREAM, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6, SOCKADDR_STORAGE, SOCKET_ERROR,
    SOL_SOCKET, SOMAXCONN, accept, bind, closesocket, connect, getsockname, listen, setsockopt,
    socket,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::core::decode_accept_flags;
use crate::platform::net::{
    AcceptFlags, SocketAddress, SocketFamily, SocketPair, SocketProtocol, SocketType,
};
use crate::platform::resource::{ListenerHandle, ResourceEntry, ResourceKind, SocketHandle};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

const IPV4_LOOPBACK: [u8; 4] = [127, 0, 0, 1];
const IPV6_LOOPBACK: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

/// Bind one socket to an ephemeral loopback address.
fn bind_socket_loopback(socket: usize, family: i32) -> RuntimeResult<()> {
    // bind ipv4 sockets on loopback
    if family == windows_sys::Win32::Networking::WinSock::AF_INET as i32 {
        let mut address = SOCKADDR_IN {
            sin_family: windows_sys::Win32::Networking::WinSock::AF_INET,
            sin_port: 0,
            sin_addr: IN_ADDR {
                S_un: unsafe { std::mem::zeroed::<IN_ADDR_0>() },
            },
            sin_zero: [0; 8],
        };
        address.sin_addr.S_un.S_addr = u32::from_ne_bytes(IPV4_LOOPBACK);

        let rc = unsafe {
            bind(
                socket,
                &address as *const _ as *const SOCKADDR,
                std::mem::size_of::<SOCKADDR_IN>() as i32,
            )
        };
        if rc != 0 {
            return Err(last_net_error("bind"));
        }

        return Ok(());
    }

    // bind ipv6 sockets on loopback
    if family == windows_sys::Win32::Networking::WinSock::AF_INET6 as i32 {
        let mut address = SOCKADDR_IN6 {
            sin6_family: windows_sys::Win32::Networking::WinSock::AF_INET6,
            sin6_port: 0,
            sin6_flowinfo: 0,
            sin6_addr: IN6_ADDR {
                u: unsafe { std::mem::zeroed::<IN6_ADDR_0>() },
            },
            Anonymous: unsafe { std::mem::zeroed() },
        };
        address.sin6_addr.u.Byte = IPV6_LOOPBACK;

        let rc = unsafe {
            bind(
                socket,
                &address as *const _ as *const SOCKADDR,
                std::mem::size_of::<SOCKADDR_IN6>() as i32,
            )
        };
        if rc != 0 {
            return Err(last_net_error("bind"));
        }

        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "family",
        "unsupported socket family",
    ))
    .boxed())
}

/// Read one bound local socket address.
fn local_socket_address(socket: usize) -> RuntimeResult<(SOCKADDR_STORAGE, i32)> {
    // read the local socket endpoint
    let mut address = unsafe { std::mem::zeroed::<SOCKADDR_STORAGE>() };
    let mut length = std::mem::size_of::<SOCKADDR_STORAGE>() as i32;
    let rc = unsafe { getsockname(socket, &mut address as *mut _ as *mut SOCKADDR, &mut length) };
    if rc != 0 {
        return Err(last_net_error("getsockname"));
    }

    Ok((address, length))
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

    // return both handles
    unsafe {
        *out = SocketPair {
            first: SocketHandle(first_id),
            second: SocketHandle(second_id),
        };
    }
}

/// Build a connected stream socket pair over loopback.
fn socket_pair_stream_loopback(
    binding: &BindingCallContext,
    out: *mut SocketPair,
    family: i32,
    protocol: i32,
) -> RuntimeResult<()> {
    // create one temporary listener socket
    let listener = unsafe { socket(family, SOCK_STREAM, protocol) };
    if listener == INVALID_SOCKET {
        return Err(last_net_error("socket"));
    }

    // set SO_REUSEADDR on the listener
    let reuse: u32 = 1;
    let rc = unsafe {
        setsockopt(
            listener,
            SOL_SOCKET,
            SO_REUSEADDR,
            &reuse as *const _ as *const u8,
            std::mem::size_of::<u32>() as i32,
        )
    };
    if rc == SOCKET_ERROR {
        unsafe {
            closesocket(listener);
        }
        return Err(last_net_error("setsockopt(SO_REUSEADDR)"));
    }

    // bind the listener on loopback
    if let Err(error) = bind_socket_loopback(listener, family) {
        unsafe {
            closesocket(listener);
        }
        return Err(error);
    }

    // resolve the listener endpoint
    let (listen_address, listen_length) = match local_socket_address(listener) {
        Ok(address) => address,
        Err(error) => {
            unsafe {
                closesocket(listener);
            }
            return Err(error);
        }
    };

    // place the listener into accept mode
    let rc = unsafe { listen(listener, 1) };
    if rc != 0 {
        unsafe {
            closesocket(listener);
        }
        return Err(last_net_error("listen"));
    }

    // connect one client socket to the listener endpoint
    let client_socket = unsafe { socket(family, SOCK_STREAM, protocol) };
    if client_socket == INVALID_SOCKET {
        unsafe {
            closesocket(listener);
        }
        return Err(last_net_error("socket"));
    }
    let rc = unsafe {
        connect(
            client_socket,
            &listen_address as *const _ as *const SOCKADDR,
            listen_length,
        )
    };
    if rc != 0 {
        unsafe {
            closesocket(client_socket);
            closesocket(listener);
        }
        return Err(last_net_error("connect"));
    }

    // accept one server socket
    let server_socket = unsafe { accept(listener, std::ptr::null_mut(), std::ptr::null_mut()) };
    if server_socket == INVALID_SOCKET {
        unsafe {
            closesocket(client_socket);
            closesocket(listener);
        }
        return Err(last_net_error("accept"));
    }

    // close the listener after establishing the pair
    unsafe {
        closesocket(listener);
    }

    // publish both endpoints
    register_socket_pair(binding, out, client_socket, server_socket);

    Ok(())
}

/// Build a connected datagram socket pair over loopback.
fn socket_pair_dgram_loopback(
    binding: &BindingCallContext,
    out: *mut SocketPair,
    family: i32,
    protocol: i32,
) -> RuntimeResult<()> {
    // create the first datagram socket
    let first_socket = unsafe { socket(family, SOCK_DGRAM, protocol) };
    if first_socket == INVALID_SOCKET {
        return Err(last_net_error("socket"));
    }

    // create the second datagram socket
    let second_socket = unsafe { socket(family, SOCK_DGRAM, protocol) };
    if second_socket == INVALID_SOCKET {
        unsafe {
            closesocket(first_socket);
        }
        return Err(last_net_error("socket"));
    }

    // bind the first socket to loopback
    if let Err(error) = bind_socket_loopback(first_socket, family) {
        unsafe {
            closesocket(second_socket);
            closesocket(first_socket);
        }
        return Err(error);
    }

    // bind the second socket to loopback
    if let Err(error) = bind_socket_loopback(second_socket, family) {
        unsafe {
            closesocket(second_socket);
            closesocket(first_socket);
        }
        return Err(error);
    }

    // resolve both local endpoints
    let (first_address, first_length) = match local_socket_address(first_socket) {
        Ok(address) => address,
        Err(error) => {
            unsafe {
                closesocket(second_socket);
                closesocket(first_socket);
            }
            return Err(error);
        }
    };
    let (second_address, second_length) = match local_socket_address(second_socket) {
        Ok(address) => address,
        Err(error) => {
            unsafe {
                closesocket(second_socket);
                closesocket(first_socket);
            }
            return Err(error);
        }
    };

    // connect first to second
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
        return Err(last_net_error("connect"));
    }

    // connect second to first
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
        return Err(last_net_error("connect"));
    }

    // publish both endpoints
    register_socket_pair(binding, out, first_socket, second_socket);

    Ok(())
}

/// Accept a new connection from a listener.
pub(crate) unsafe fn destack_net_accept(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
    flags: AcceptFlags,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized and decode accept behavior
    ensure_winsock()?;
    let accept_behavior = decode_accept_flags(flags)?;

    // accept the connection
    let socket = listener_descriptor(binding, listener)?;
    let client = unsafe { accept(socket, std::ptr::null_mut(), std::ptr::null_mut()) };
    if client == INVALID_SOCKET {
        return Err(last_net_error("accept"));
    }

    // apply requested nonblocking mode before publishing the socket
    if accept_behavior.nonblocking {
        let mut value: u32 = 1;
        let rc = unsafe {
            windows_sys::Win32::Networking::WinSock::ioctlsocket(
                client,
                windows_sys::Win32::Networking::WinSock::FIONBIO,
                &mut value,
            )
        };
        if rc != 0 {
            unsafe {
                closesocket(client);
            }
            return Err(last_net_error("ioctlsocket"));
        }
    }

    // clear handle inheritance when close-on-exec semantics were requested
    if accept_behavior.cloexec {
        let rc = unsafe { SetHandleInformation(client as isize, HANDLE_FLAG_INHERIT, 0) };
        if rc == 0 {
            unsafe {
                closesocket(client);
            }
            return Err(core_platform::io_error_with_code(
                "SetHandleInformation",
                core_platform::last_error_code(),
            ));
        }
    }

    // register the socket
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(client as _)
        .with_finalizer(SocketFinalizer::new(client));
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

/// Close a socket handle.
pub(crate) unsafe fn destack_net_close(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // remove the resource entry
    let entry = binding
        .worker()
        .resources
        .remove(binding.world(), handle.0, Some(binding.engine()))
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
    binding: &BindingCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    // remove the resource entry
    let entry = binding
        .worker()
        .resources
        .remove(binding.world(), handle.0, Some(binding.engine()))
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

/// Connect an existing socket to a raw remote address.
pub(crate) unsafe fn destack_net_connect_raw(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // ensure winsock is initialized
    ensure_winsock()?;

    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

    // connect using the provided raw address
    with_socket_address_raw(address, |sockaddr, length| {
        let rc = unsafe { connect(socket, sockaddr, length) };
        if rc != 0 {
            return Err(last_net_error("connect"));
        }

        Ok(())
    })
}

/// Bind an existing socket to a raw address.
pub(crate) unsafe fn destack_net_bind(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // ensure winsock is initialized
    ensure_winsock()?;

    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

    // bind using the provided raw address
    with_socket_address_raw(address, |sockaddr, length| {
        let rc = unsafe { bind(socket, sockaddr, length) };
        if rc != 0 {
            return Err(last_net_error("bind"));
        }

        Ok(())
    })
}

/// Start listening on a raw local socket address.
pub(crate) unsafe fn destack_net_listen_raw(
    binding: &BindingCallContext,
    out: *mut ListenerHandle,
    address: SocketAddress,
    backlog: u32,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    ensure_winsock()?;

    // resolve the socket family from the raw address bytes
    let mut family = address.family as i32;
    if family == 0 {
        family = with_socket_address_raw(address, |sockaddr, _length| {
            let family = unsafe { (*sockaddr).sa_family as i32 };
            Ok(family)
        })?;
    }

    // create the listening socket
    let listener = unsafe { socket(family, SOCK_STREAM, IPPROTO_TCP) };
    if listener == INVALID_SOCKET {
        return Err(last_net_error("socket"));
    }

    // set SO_REUSEADDR for quick local test rebinding
    let reuse: u32 = 1;
    let rc = unsafe {
        setsockopt(
            listener,
            SOL_SOCKET,
            SO_REUSEADDR,
            &reuse as *const _ as *const u8,
            std::mem::size_of::<u32>() as i32,
        )
    };
    if rc == SOCKET_ERROR {
        unsafe {
            closesocket(listener);
        }
        return Err(last_net_error("setsockopt(SO_REUSEADDR)"));
    }

    // bind and listen using the provided address
    let bind_result = with_socket_address_raw(address, |sockaddr, length| {
        let rc = unsafe { bind(listener, sockaddr, length) };
        if rc != 0 {
            return Err(last_net_error("bind"));
        }

        Ok(())
    });
    if let Err(error) = bind_result {
        unsafe {
            closesocket(listener);
        }
        return Err(error);
    }

    let backlog = if backlog == 0 {
        SOMAXCONN as i32
    } else {
        backlog.min(i32::MAX as u32) as i32
    };
    let rc = unsafe { listen(listener, backlog) };
    if rc != 0 {
        unsafe {
            closesocket(listener);
        }
        return Err(last_net_error("listen"));
    }

    // register the listener handle
    let entry = ResourceEntry::new(ResourceKind::Listener)
        .with_listener(listener as _)
        .with_finalizer(SocketFinalizer::new(listener));
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

/// Create a socket from a native family, type, and protocol.
pub(crate) unsafe fn destack_net_socket(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    ensure_winsock()?;

    // create the socket
    let af = socket_family_to_raw(family);
    let socket = unsafe { socket(af, socket_type.0 as i32, protocol.0) };
    if socket == INVALID_SOCKET {
        return Err(last_net_error("socket"));
    }

    // register the socket handle
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

/// Create a connected socket pair.
pub(crate) unsafe fn destack_net_socket_pair(
    binding: &BindingCallContext,
    out: *mut SocketPair,
    family: SocketFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    ensure_winsock()?;

    // choose the address family
    let family = socket_family_to_raw(family);
    let protocol = protocol.0;
    let socket_type = socket_type.0 as i32;

    // emulate stream socket pairs through loopback connect+accept
    if socket_type == SOCK_STREAM {
        return socket_pair_stream_loopback(binding, out, family, protocol);
    }

    // emulate datagram socket pairs through connected loopback sockets
    if socket_type == SOCK_DGRAM {
        return socket_pair_dgram_loopback(binding, out, family, protocol);
    }

    // reject socket types that are not safely emulatable
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.socketPair")).boxed())
}
