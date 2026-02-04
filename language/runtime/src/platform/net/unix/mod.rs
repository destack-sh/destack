use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{SocketAddress, SocketFamily, SocketShutdown, core as core_net};
use crate::platform::resource::{
    ListenerHandle, ResourceEntry, ResourceFinalizer, ResourceKind, SocketHandle,
};
use crate::platform::{NativeSlice, NativeStringRef, PlatformError, ResourceId};
use crate::runtime::RuntimeCallContext;

use std::net::{TcpListener, TcpStream};
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
use linux as os;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as os;

#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "ios")]
use ios as os;

#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "freebsd")]
use freebsd as os;

#[cfg(target_os = "openbsd")]
mod openbsd;
#[cfg(target_os = "openbsd")]
use openbsd as os;

#[cfg(target_os = "netbsd")]
mod netbsd;
#[cfg(target_os = "netbsd")]
use netbsd as os;

#[cfg(target_os = "dragonfly")]
mod dragonfly;
#[cfg(target_os = "dragonfly")]
use dragonfly as os;

/// Finalizer that closes a socket file descriptor.
#[derive(Debug)]
struct SocketFinalizer {
    /// Socket descriptor to close.
    fd: RawFd,
}

impl ResourceFinalizer for SocketFinalizer {
    /// Close the file descriptor when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// Build a socket address from a host string if it is numeric.
fn socket_address_from_host(host: &str, port: u16) -> Option<SocketAddr> {
    let ip = host.parse::<IpAddr>().ok()?;
    Some(SocketAddr::new(ip, port))
}

/// Build a socket address from a raw fd query.
fn socket_address_from_fd(
    context: &RuntimeCallContext,
    fd: RawFd,
    syscall: &str,
    query: unsafe extern "C" fn(RawFd, *mut libc::sockaddr, *mut libc::socklen_t) -> libc::c_int,
) -> RuntimeResult<SocketAddress> {
    // allocate space for the address
    let mut storage = std::mem::MaybeUninit::<libc::sockaddr_storage>::uninit();
    let mut length = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

    // query the socket address
    let result = unsafe { query(fd, storage.as_mut_ptr() as *mut libc::sockaddr, &mut length) };
    if result != 0 {
        return Err(last_os_error(syscall, None));
    }

    // decode the returned storage
    let storage = unsafe { storage.assume_init() };
    socket_address_from_storage(context, &storage, length)
}

/// Decode a socket address from raw storage.
fn socket_address_from_storage(
    context: &RuntimeCallContext,
    storage: &libc::sockaddr_storage,
    length: libc::socklen_t,
) -> RuntimeResult<SocketAddress> {
    // ensure the storage length is valid
    if length as usize > std::mem::size_of::<libc::sockaddr_storage>() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "socket address length out of range",
        ))
        .boxed());
    }

    // decode based on the family
    match storage.ss_family as libc::c_int {
        libc::AF_INET => {
            let addr = unsafe { &*(storage as *const _ as *const libc::sockaddr_in) };
            let ip = Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr));
            let host = ip.to_string();
            Ok(SocketAddress {
                host: context.store_string(&host),
                port: u16::from_be(addr.sin_port),
                family: SocketFamily::IPv4,
            })
        }
        libc::AF_INET6 => {
            let addr = unsafe { &*(storage as *const _ as *const libc::sockaddr_in6) };
            let ip = Ipv6Addr::from(addr.sin6_addr.s6_addr);
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
fn socket_descriptor(context: &RuntimeCallContext, handle: SocketHandle) -> RuntimeResult<RawFd> {
    core_net::require_resource(context, handle.0, ResourceKind::Socket, "socket", |entry| {
        entry.fd()
    })
}

/// Resolve a listener descriptor from a handle.
fn listener_descriptor(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<RawFd> {
    core_net::require_resource(
        context,
        handle.0,
        ResourceKind::Listener,
        "listener",
        |entry| entry.fd(),
    )
}

/// Map a SocketShutdown to a libc shutdown constant.
fn shutdown_how(how: SocketShutdown) -> libc::c_int {
    match how {
        SocketShutdown::Read => libc::SHUT_RD,
        SocketShutdown::Write => libc::SHUT_WR,
        SocketShutdown::ReadWrite => libc::SHUT_RDWR,
    }
}

/// Set a boolean socket option.
pub(super) fn set_socket_bool(
    fd: RawFd,
    level: libc::c_int,
    option: libc::c_int,
    enabled: bool,
) -> RuntimeResult<()> {
    let value: libc::c_int = if enabled { 1 } else { 0 };
    let rc = unsafe {
        libc::setsockopt(
            fd,
            level,
            option,
            &value as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(last_os_error("setsockopt", None));
    }

    Ok(())
}

/// Set a u32 socket option.
pub(super) fn set_socket_u32(
    fd: RawFd,
    level: libc::c_int,
    option: libc::c_int,
    value: u32,
) -> RuntimeResult<()> {
    let value = value as libc::c_int;
    let rc = unsafe {
        libc::setsockopt(
            fd,
            level,
            option,
            &value as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(last_os_error("setsockopt", None));
    }

    Ok(())
}

/// Build a runtime error from the last OS error.
fn last_os_error(syscall: &str, path: Option<&str>) -> Box<RuntimeError> {
    let error = std::io::Error::last_os_error();
    let errno = error.raw_os_error();
    let message = format!("{syscall} failed: {error}");
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        errno,
        Some(syscall.to_string()),
        path.map(|path| path.to_string()),
        message,
    ))
    .boxed()
}

/// Accept a new socket from a listener.
pub(crate) unsafe fn destack_net_accept(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // accept sockets on unix platforms
    {
        let fd = listener_descriptor(context, listener)?;
        let client_fd = unsafe { libc::accept(fd, std::ptr::null_mut(), std::ptr::null_mut()) };
        if client_fd < 0 {
            return Err(last_os_error("accept", None));
        }

        let entry = ResourceEntry::new(ResourceKind::Socket)
            .with_socket(client_fd)
            .with_finalizer(SocketFinalizer { fd: client_fd });
        let resource_id = context.runtime().resources.insert(entry);
        unsafe {
            *out = SocketHandle(resource_id);
        }

        Ok(())
    }
}

/// Close a socket handle.
pub(crate) unsafe fn destack_net_close(
    context: &RuntimeCallContext,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate the handle kind
    let is_socket = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| entry.kind == ResourceKind::Socket)
        .unwrap_or(false);
    if !is_socket {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown socket handle",
        ))
        .boxed());
    }

    // remove the resource and close it
    if !context.runtime().resources.remove_and_finalize(handle.0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown socket handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Connect to a remote host and return a socket handle.
pub(crate) unsafe fn destack_net_connect(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    host: NativeStringRef,
    port: u16,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // connect on unix platforms
    {
        let host = unsafe { host.as_str()? };
        if host.contains('\0') {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "host",
                "host contains nul byte",
            ))
            .boxed());
        }

        let stream = match socket_address_from_host(host, port) {
            Some(address) => TcpStream::connect(address),
            None => TcpStream::connect((host, port)),
        }
        .map_err(|error| {
            RuntimeError::from(PlatformError::io(format!("connect failed: {error}"))).boxed()
        })?;
        let fd = stream.into_raw_fd();
        let entry = ResourceEntry::new(ResourceKind::Socket)
            .with_socket(fd)
            .with_finalizer(SocketFinalizer { fd });
        let resource_id = context.runtime().resources.insert(entry);
        unsafe {
            *out = SocketHandle(resource_id);
        }

        Ok(())
    }
}

/// Start listening on the given address.
pub(crate) unsafe fn destack_net_listen(
    context: &RuntimeCallContext,
    out: *mut ListenerHandle,
    host: NativeStringRef,
    port: u16,
    backlog: u32,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // listen on unix platforms
    {
        let host = unsafe { host.as_str()? };
        if host.contains('\0') {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "host",
                "host contains nul byte",
            ))
            .boxed());
        }

        let listener = match socket_address_from_host(host, port) {
            Some(address) => TcpListener::bind(address),
            None => TcpListener::bind((host, port)),
        }
        .map_err(|error| {
            RuntimeError::from(PlatformError::io(format!("bind failed: {error}"))).boxed()
        })?;
        let backlog = backlog.min(i32::MAX as u32) as libc::c_int;
        if backlog > 0 {
            let rc = unsafe { libc::listen(listener.as_raw_fd(), backlog) };
            if rc != 0 {
                return Err(last_os_error("listen", None));
            }
        }

        let fd = listener.into_raw_fd();
        let entry = ResourceEntry::new(ResourceKind::Listener)
            .with_listener(fd)
            .with_finalizer(SocketFinalizer { fd });
        let resource_id = context.runtime().resources.insert(entry);
        unsafe {
            *out = ListenerHandle(resource_id);
        }

        Ok(())
    }
}

/// Read from a socket into the provided slice.
pub(crate) unsafe fn destack_net_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read on unix platforms
    {
        let fd = socket_descriptor(context, handle)?;
        let buffer = unsafe { buffer.as_mut_slice()? };
        let rc = unsafe {
            libc::recv(
                fd,
                buffer.as_mut_ptr() as *mut libc::c_void,
                buffer.len(),
                0,
            )
        };
        if rc < 0 {
            return Err(last_os_error("recv", None));
        }
        unsafe {
            *out = rc as u64;
        }
        Ok(())
    }
}

/// Write to a socket from the provided slice.
pub(crate) unsafe fn destack_net_write(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // write on unix platforms
    {
        let fd = socket_descriptor(context, handle)?;
        let buffer = unsafe { buffer.as_slice()? };
        let flags = os::send_flags();
        let rc = unsafe {
            libc::send(
                fd,
                buffer.as_ptr() as *const libc::c_void,
                buffer.len(),
                flags,
            )
        };
        if rc < 0 {
            return Err(last_os_error("send", None));
        }
        unsafe {
            *out = rc as u64;
        }
        Ok(())
    }
}

/// Shut down a socket for reads, writes, or both.
pub(crate) unsafe fn destack_net_shutdown(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    how: SocketShutdown,
) -> RuntimeResult<()> {
    // shutdown sockets on unix platforms
    {
        let fd = socket_descriptor(context, handle)?;
        let rc = unsafe { libc::shutdown(fd, shutdown_how(how)) };
        if rc != 0 {
            return Err(last_os_error("shutdown", None));
        }
        Ok(())
    }
}

/// Enable or disable nonblocking mode on a socket.
pub(crate) unsafe fn destack_net_set_nonblocking(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // update nonblocking mode on unix platforms
    {
        let fd = socket_descriptor(context, handle)?;
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(last_os_error("fcntl", None));
        }
        let new_flags = if enabled {
            flags | libc::O_NONBLOCK
        } else {
            flags & !libc::O_NONBLOCK
        };
        let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, new_flags) };
        if rc < 0 {
            return Err(last_os_error("fcntl", None));
        }
        Ok(())
    }
}

/// Read the local socket address.
pub(crate) unsafe fn destack_net_local_address(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the local address on unix platforms
    {
        let fd = socket_descriptor(context, handle)?;
        let address = socket_address_from_fd(context, fd, "getsockname", libc::getsockname)?;
        unsafe {
            *out = address;
        }
        Ok(())
    }
}

/// Read the remote socket address.
pub(crate) unsafe fn destack_net_peer_address(
    context: &RuntimeCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the peer address on unix platforms
    {
        let fd = socket_descriptor(context, handle)?;
        let address = socket_address_from_fd(context, fd, "getpeername", libc::getpeername)?;
        unsafe {
            *out = address;
        }
        Ok(())
    }
}

/// Enable or disable TCP_NODELAY.
pub(crate) unsafe fn destack_net_set_no_delay(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // update TCP_NODELAY on unix platforms
    {
        let fd = socket_descriptor(context, handle)?;
        set_socket_bool(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, enabled)
    }
}

/// Enable or disable TCP keepalive.
pub(crate) unsafe fn destack_net_set_keep_alive(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
    delay_seconds: u32,
) -> RuntimeResult<()> {
    // update keepalive on unix platforms
    {
        let fd = socket_descriptor(context, handle)?;
        set_socket_bool(fd, libc::SOL_SOCKET, libc::SO_KEEPALIVE, enabled)?;

        if enabled && delay_seconds > 0 {
            os::set_keepalive_delay(fd, delay_seconds)?;
        }

        Ok(())
    }
}

/// Enable or disable SO_REUSEADDR.
pub(crate) unsafe fn destack_net_set_reuse_addr(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // update SO_REUSEADDR on unix platforms
    {
        let fd = socket_descriptor(context, handle)?;
        set_socket_bool(fd, libc::SOL_SOCKET, libc::SO_REUSEADDR, enabled)
    }
}

/// Enable or disable SO_REUSEPORT.
pub(crate) unsafe fn destack_net_set_reuse_port(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // update SO_REUSEPORT on unix platforms
    {
        let fd = socket_descriptor(context, handle)?;
        os::set_reuse_port(fd, enabled)
    }
}

/// Close a listener handle.
pub(crate) unsafe fn destack_net_close_listener(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    // validate the handle kind
    let is_listener = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| entry.kind == ResourceKind::Listener)
        .unwrap_or(false);
    if !is_listener {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown listener handle",
        ))
        .boxed());
    }

    // remove the resource and close it
    if !context.runtime().resources.remove_and_finalize(handle.0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown listener handle",
        ))
        .boxed());
    }

    Ok(())
}
