use std::mem;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::sync::Once;

use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, IN_ADDR, IN_ADDR_0, IN6_ADDR, IN6_ADDR_0, SOCKADDR, SOCKADDR_IN,
    SOCKADDR_IN6, SOCKADDR_IN6_0, SOCKADDR_STORAGE, SOCKET, WSAGetLastError, closesocket,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{SocketAddress, SocketFamily, core as core_net};
use crate::platform::resource::{ListenerHandle, ResourceFinalizer, ResourceKind, SocketHandle};
use crate::platform::{PlatformError, ResourceId};
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

/// Initialise Winsock using the standard library.
pub(super) fn ensure_winsock() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = UdpSocket::bind("127.0.0.1:0");
    });
}

/// Build a runtime error from the last socket error.
pub(super) fn last_net_error(syscall: &str) -> Box<RuntimeError> {
    let errno = unsafe { WSAGetLastError() };
    let message = format!("{syscall} failed: {errno}");
    RuntimeError::from(PlatformError::net_with(
        None,
        None,
        Some(errno),
        Some(syscall.to_string()),
        None,
        None,
        message,
    ))
    .boxed()
}

/// Build a socket address from a host string if it is numeric.
pub(super) fn socket_address_from_host(host: &str, port: u16) -> Option<SocketAddr> {
    let ip = host.parse::<IpAddr>().ok()?;
    Some(SocketAddr::new(ip, port))
}

/// Address representation for Winsock calls.
#[repr(C)]
pub(super) union SocketAddrCRepr {
    v4: SOCKADDR_IN,
    v6: SOCKADDR_IN6,
}

impl SocketAddrCRepr {
    /// Access the sockaddr pointer for this union.
    pub(super) fn as_ptr(&self) -> *const SOCKADDR {
        self as *const _ as *const SOCKADDR
    }
}

/// Convert a SocketAddr to a Windows sockaddr representation.
pub(super) fn socket_addr(addr: &SocketAddr) -> (SocketAddrCRepr, i32) {
    match addr {
        SocketAddr::V4(addr) => {
            let sin_addr = unsafe {
                let mut s_un = mem::zeroed::<IN_ADDR_0>();
                s_un.S_addr = u32::from_ne_bytes(addr.ip().octets());
                IN_ADDR { S_un: s_un }
            };

            let sockaddr_in = SOCKADDR_IN {
                sin_family: AF_INET as u16,
                sin_port: addr.port().to_be(),
                sin_addr,
                sin_zero: [0; 8],
            };

            let sockaddr = SocketAddrCRepr { v4: sockaddr_in };
            (sockaddr, mem::size_of::<SOCKADDR_IN>() as i32)
        }
        SocketAddr::V6(addr) => {
            let sin6_addr = unsafe {
                let mut u = mem::zeroed::<IN6_ADDR_0>();
                u.Byte = addr.ip().octets();
                IN6_ADDR { u }
            };
            let u = unsafe {
                let mut u = mem::zeroed::<SOCKADDR_IN6_0>();
                u.sin6_scope_id = addr.scope_id();
                u
            };
            let sockaddr_in6 = SOCKADDR_IN6 {
                sin6_family: AF_INET6 as u16,
                sin6_port: addr.port().to_be(),
                sin6_flowinfo: addr.flowinfo(),
                sin6_addr,
                Anonymous: u,
            };
            let sockaddr = SocketAddrCRepr { v6: sockaddr_in6 };
            (sockaddr, mem::size_of::<SOCKADDR_IN6>() as i32)
        }
    }
}

/// Build a socket address from raw storage.
pub(super) fn socket_address_from_storage(
    context: &RuntimeCallContext,
    storage: &SOCKADDR_STORAGE,
) -> RuntimeResult<SocketAddress> {
    match storage.ss_family {
        family if family == AF_INET as u16 => {
            let addr = unsafe { &*(storage as *const _ as *const SOCKADDR_IN) };
            let ip = unsafe { Ipv4Addr::from(addr.sin_addr.S_un.S_addr.to_ne_bytes()) };
            let host = ip.to_string();
            Ok(SocketAddress {
                host: context.store_string(&host),
                port: u16::from_be(addr.sin_port),
                family: SocketFamily::IPv4,
            })
        }
        family if family == AF_INET6 as u16 => {
            let addr = unsafe { &*(storage as *const _ as *const SOCKADDR_IN6) };
            let ip = unsafe { Ipv6Addr::from(addr.sin6_addr.u.Byte) };
            let host = ip.to_string();
            Ok(SocketAddress {
                host: context.store_string(&host),
                port: u16::from_be(addr.sin6_port),
                family: SocketFamily::IPv6,
            })
        }
        _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "unsupported socket family",
        ))
        .boxed()),
    }
}

/// Resolve a socket descriptor from a handle.
pub(super) fn socket_descriptor(
    context: &RuntimeCallContext,
    handle: SocketHandle,
) -> RuntimeResult<SOCKET> {
    let socket =
        core_net::require_resource(context, handle.0, ResourceKind::Socket, "socket", |entry| {
            entry.socket()
        })?;

    Ok(socket as SOCKET)
}

/// Resolve a listener descriptor from a handle.
pub(super) fn listener_descriptor(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<SOCKET> {
    let socket = core_net::require_resource(
        context,
        handle.0,
        ResourceKind::Listener,
        "listener",
        |entry| entry.socket(),
    )?;

    Ok(socket as SOCKET)
}
