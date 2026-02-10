use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::OsPath;
use crate::platform::net::{
    AcceptFlags, Linger, ResolveFlags, SocketAddress, SocketControlBufferAbi, SocketCredentials,
    SocketFamily, SocketMessageFlags, SocketRecvMessage, SocketSendMessage, SocketShutdown,
    core as core_net,
};
use crate::platform::resource::{
    ListenerHandle, ResourceEntry, ResourceFinalizer, ResourceKind, SocketHandle, TransferredHandle,
};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, PlatformError, ResourceId, core as core_platform,
};
use crate::runtime::RuntimeCallContext;

use std::ffi::{CStr, CString};
use std::os::unix::io::RawFd;

#[cfg(any(target_os = "linux", target_os = "android"))]
const IPV6_JOIN_GROUP_OPT: libc::c_int = libc::IPV6_ADD_MEMBERSHIP;
#[cfg(any(target_os = "linux", target_os = "android"))]
const IPV6_LEAVE_GROUP_OPT: libc::c_int = libc::IPV6_DROP_MEMBERSHIP;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
const IPV6_JOIN_GROUP_OPT: libc::c_int = libc::IPV6_JOIN_GROUP;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
const IPV6_LEAVE_GROUP_OPT: libc::c_int = libc::IPV6_LEAVE_GROUP;

#[cfg(any(target_os = "linux", target_os = "android"))]
#[path = "linux.rs"]
mod linux;
#[cfg(any(target_os = "linux", target_os = "android"))]
use linux as os;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod macos;
#[cfg(target_os = "macos")]
use macos as os;

#[cfg(target_os = "ios")]
#[path = "ios.rs"]
mod ios;
#[cfg(target_os = "ios")]
use ios as os;

#[cfg(target_os = "freebsd")]
#[path = "freebsd.rs"]
mod freebsd;
#[cfg(target_os = "freebsd")]
use freebsd as os;

#[cfg(target_os = "openbsd")]
#[path = "openbsd.rs"]
mod openbsd;
#[cfg(target_os = "openbsd")]
use openbsd as os;

#[cfg(target_os = "netbsd")]
#[path = "netbsd.rs"]
mod netbsd;
#[cfg(target_os = "netbsd")]
use netbsd as os;

#[cfg(target_os = "dragonfly")]
#[path = "dragonfly.rs"]
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

/// Finalizer that closes a file descriptor.
#[derive(Debug)]
struct FileFinalizer {
    /// File descriptor to close.
    fd: RawFd,
}

impl ResourceFinalizer for FileFinalizer {
    /// Close the file descriptor when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// Build a raw socket address from a raw fd query.
fn socket_address_raw_from_fd(
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
        return Err(core_platform::net_error(syscall));
    }

    // decode the returned storage
    let storage = unsafe { storage.assume_init() };
    socket_address_raw_from_storage(context, &storage, length)
}

/// Decode a raw socket address from raw storage.
fn socket_address_raw_from_storage(
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

    // copy exact sockaddr bytes
    let bytes = unsafe {
        let pointer = storage as *const _ as *const u8;
        std::slice::from_raw_parts(pointer, length as usize)
    };
    let bytes = context.store_array(bytes.to_vec());

    // write the raw sockaddr payload
    Ok(SocketAddress {
        family: storage.ss_family as u16,
        length,
        bytes,
    })
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

    // copy exact sockaddr bytes
    let bytes = unsafe {
        let pointer = storage as *const _ as *const u8;
        std::slice::from_raw_parts(pointer, length as usize)
    };
    let bytes = context.store_array(bytes.to_vec());

    Ok(SocketAddress {
        family: storage.ss_family as u16,
        length,
        bytes,
    })
}

/// Build a sockaddr_un from a byte path.
fn sockaddr_un_from_path(path: &[u8]) -> RuntimeResult<(libc::sockaddr_un, libc::socklen_t, bool)> {
    // ensure the path fits in sockaddr_un
    let mut addr = unsafe { std::mem::zeroed::<libc::sockaddr_un>() };
    let max_len = addr.sun_path.len();
    if path.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "path is empty",
        ))
        .boxed());
    }
    if path.len() > max_len {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "path is too long",
        ))
        .boxed());
    }

    // determine abstract namespace usage
    let is_abstract = path[0] == 0;
    if !is_abstract && path.contains(&0) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "path contains nul byte",
        ))
        .boxed());
    }

    // populate the sockaddr
    addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
    unsafe {
        std::ptr::copy_nonoverlapping(
            path.as_ptr(),
            addr.sun_path.as_mut_ptr() as *mut u8,
            path.len(),
        );
    }
    if !is_abstract && path.len() < max_len {
        addr.sun_path[path.len()] = 0;
    }

    // compute the address length
    let base = std::mem::size_of::<libc::sa_family_t>();
    let length = if is_abstract {
        base + path.len()
    } else {
        base + path.len() + 1
    };

    Ok((addr, length as libc::socklen_t, is_abstract))
}

/// Resolve a socket descriptor from a handle.
fn socket_descriptor(context: &RuntimeCallContext, handle: SocketHandle) -> RuntimeResult<RawFd> {
    // resolve the socket resource
    core_net::require_resource(context, handle.0, ResourceKind::Socket, "socket", |entry| {
        entry.fd().ok_or_else(|| {
            RuntimeError::from(PlatformError::generic(
                None,
                "socket handle missing descriptor",
            ))
            .boxed()
        })
    })
}

/// Resolve a transferable file descriptor from a resource handle.
fn transferable_descriptor(
    context: &RuntimeCallContext,
    handle: TransferredHandle,
) -> RuntimeResult<RawFd> {
    let descriptor = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| entry.fd());
    match descriptor.flatten() {
        Some(descriptor) => Ok(descriptor),
        None => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "message.fds",
            "resource handle is not fd-backed",
        ))
        .boxed()),
    }
}

/// Classify a received descriptor into a resource kind.
fn kind_from_received_descriptor(descriptor: RawFd) -> RuntimeResult<ResourceKind> {
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    let status = unsafe { libc::fstat(descriptor, metadata.as_mut_ptr()) };
    if status != 0 {
        return Err(core_platform::io_error("fstat", None));
    }

    let metadata = unsafe { metadata.assume_init() };
    let file_type = metadata.st_mode & libc::S_IFMT;
    let kind = if file_type == libc::S_IFSOCK {
        ResourceKind::Socket
    } else {
        ResourceKind::File
    };

    Ok(kind)
}

/// Resolve a socket family from a socket descriptor.
fn socket_family_from_fd(fd: RawFd) -> RuntimeResult<SocketFamily> {
    // allocate address storage
    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    let mut length = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

    // query the socket address
    let rc = unsafe {
        libc::getsockname(
            fd,
            &mut storage as *mut _ as *mut libc::sockaddr,
            &mut length,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error("getsockname"));
    }

    // map the socket family
    match storage.ss_family as libc::c_int {
        libc::AF_INET => Ok(SocketFamily::IPv4),
        libc::AF_INET6 => Ok(SocketFamily::IPv6),
        _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unsupported socket family",
        ))
        .boxed()),
    }
}

/// Parse an IPv4 interface address for multicast operations.
fn parse_ipv4_interface(name: &str) -> RuntimeResult<Ipv4Addr> {
    // treat empty strings as INADDR_ANY
    if name.is_empty() {
        return Ok(Ipv4Addr::UNSPECIFIED);
    }

    // parse the interface address
    name.parse::<Ipv4Addr>().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "interfaceAddress",
            "invalid IPv4 interface address",
        ))
        .boxed()
    })
}

/// Resolve peer socket credentials for the current socket.
fn peer_socket_credentials(fd: RawFd) -> RuntimeResult<SocketCredentials> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        // query peer credentials with SO_PEERCRED
        let mut peer = std::mem::MaybeUninit::<libc::ucred>::uninit();
        let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        let rc = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                peer.as_mut_ptr() as *mut libc::c_void,
                &mut length,
            )
        };
        if rc != 0 {
            return Err(core_platform::net_error("getsockopt"));
        }

        let peer = unsafe { peer.assume_init() };
        return Ok(SocketCredentials {
            pid: peer.pid as u32,
            uid: peer.uid,
            gid: peer.gid,
        });
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        // query peer credentials with getpeereid
        let mut uid: libc::uid_t = 0;
        let mut gid: libc::gid_t = 0;
        let rc = unsafe { libc::getpeereid(fd, &mut uid, &mut gid) };
        if rc != 0 {
            return Err(core_platform::net_error("getpeereid"));
        }

        return Ok(SocketCredentials { pid: 0, uid, gid });
    }

    #[allow(unreachable_code)]
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.recvMsg")).boxed())
}

/// Resolve a listener descriptor from a handle.
fn listener_descriptor(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<RawFd> {
    // resolve the listener resource
    core_net::require_resource(
        context,
        handle.0,
        ResourceKind::Listener,
        "listener",
        |entry| {
            entry.fd().ok_or_else(|| {
                RuntimeError::from(PlatformError::generic(
                    None,
                    "listener handle missing descriptor",
                ))
                .boxed()
            })
        },
    )
}

/// Map a SocketShutdown to a libc shutdown constant.
fn shutdown_how(how: SocketShutdown) -> libc::c_int {
    // map the shutdown mode
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
    // encode the boolean value
    let value: libc::c_int = if enabled { 1 } else { 0 };

    // set the socket option
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
        return Err(core_platform::net_error("setsockopt"));
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
    // encode the value
    let value = value as libc::c_int;

    // set the socket option
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
        return Err(core_platform::net_error("setsockopt"));
    }

    Ok(())
}

/// Set a u8 socket option.
pub(super) fn set_socket_u8(
    fd: RawFd,
    level: libc::c_int,
    option: libc::c_int,
    value: u8,
) -> RuntimeResult<()> {
    // encode the value
    let value = value as libc::c_uchar;

    // set the socket option
    let rc = unsafe {
        libc::setsockopt(
            fd,
            level,
            option,
            &value as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_uchar>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error("setsockopt"));
    }

    Ok(())
}

/// Accept a new socket from a listener.
pub(crate) unsafe fn destack_net_accept(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
    _flags: AcceptFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // accept sockets on unix platforms
    {
        // resolve the listener descriptor
        let fd = listener_descriptor(context, listener)?;

        // accept the connection
        let client_fd = unsafe { libc::accept(fd, std::ptr::null_mut(), std::ptr::null_mut()) };
        if client_fd < 0 {
            return Err(core_platform::net_error("accept"));
        }

        // register the new socket
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
        // resolve the socket descriptor
        let fd = socket_descriptor(context, handle)?;

        // decode the buffer
        let buffer = unsafe { buffer.as_mut_slice()? };

        // read from the socket
        let rc = unsafe {
            libc::recv(
                fd,
                buffer.as_mut_ptr() as *mut libc::c_void,
                buffer.len(),
                0,
            )
        };
        if rc < 0 {
            return Err(core_platform::net_error("recv"));
        }

        // write the output
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
        // resolve the socket descriptor
        let fd = socket_descriptor(context, handle)?;

        // decode the buffer
        let buffer = unsafe { buffer.as_slice()? };

        // write to the socket
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
            return Err(core_platform::net_error("send"));
        }

        // write the output
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
        // resolve the socket descriptor
        let fd = socket_descriptor(context, handle)?;

        // issue the shutdown
        let rc = unsafe { libc::shutdown(fd, shutdown_how(how)) };
        if rc != 0 {
            return Err(core_platform::net_error("shutdown"));
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
        // resolve the socket descriptor
        let fd = socket_descriptor(context, handle)?;

        // fetch current flags
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(core_platform::net_error("fcntl"));
        }

        // update the nonblocking flag
        let new_flags = if enabled {
            flags | libc::O_NONBLOCK
        } else {
            flags & !libc::O_NONBLOCK
        };
        let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, new_flags) };
        if rc < 0 {
            return Err(core_platform::net_error("fcntl"));
        }

        Ok(())
    }
}

/// Read the local socket address as raw bytes.
pub(crate) unsafe fn destack_net_local_address_raw(
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
        // resolve the socket descriptor
        let fd = socket_descriptor(context, handle)?;

        // query the socket address
        let address = socket_address_raw_from_fd(context, fd, "getsockname", libc::getsockname)?;

        // write the output
        unsafe {
            *out = address;
        }

        Ok(())
    }
}

/// Read the remote socket address as raw bytes.
pub(crate) unsafe fn destack_net_peer_address_raw(
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
        // resolve the socket descriptor
        let fd = socket_descriptor(context, handle)?;

        // query the socket address
        let address = socket_address_raw_from_fd(context, fd, "getpeername", libc::getpeername)?;

        // write the output
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
        // resolve the socket descriptor
        let fd = socket_descriptor(context, handle)?;
        set_socket_bool(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, enabled)
    }
}

/// Enable or disable TCP keepalive.
pub(crate) unsafe fn destack_net_set_keep_alive(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
    idle_seconds: u32,
    interval_seconds: u32,
    probe_count: u32,
) -> RuntimeResult<()> {
    // update keepalive on unix platforms
    {
        // resolve the socket descriptor
        let fd = socket_descriptor(context, handle)?;

        // update keepalive flags
        set_socket_bool(fd, libc::SOL_SOCKET, libc::SO_KEEPALIVE, enabled)?;

        // apply keepalive tuning values
        if enabled && idle_seconds > 0 {
            os::set_keepalive_delay(fd, idle_seconds)?;
        }
        if enabled && interval_seconds > 0 {
            set_socket_u32(fd, libc::IPPROTO_TCP, libc::TCP_KEEPINTVL, interval_seconds)?;
        }
        if enabled && probe_count > 0 {
            set_socket_u32(fd, libc::IPPROTO_TCP, libc::TCP_KEEPCNT, probe_count)?;
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
        // resolve the socket descriptor
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
        // resolve the socket descriptor
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

/// Read from a socket into multiple buffers.
pub(crate) unsafe fn destack_net_readv(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor and buffer list
    let fd = socket_descriptor(context, handle)?;
    let buffers = unsafe { buffers.as_slice()? };
    let mut iovecs = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let slice = unsafe { buffer.as_mut_slice()? };
        iovecs.push(libc::iovec {
            iov_base: slice.as_mut_ptr() as *mut libc::c_void,
            iov_len: slice.len(),
        });
    }

    // read from the socket
    let rc = unsafe { libc::readv(fd, iovecs.as_ptr(), iovecs.len() as libc::c_int) };
    if rc < 0 {
        return Err(core_platform::net_error("readv"));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Write to a socket from multiple buffers.
pub(crate) unsafe fn destack_net_writev(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor and buffer list
    let fd = socket_descriptor(context, handle)?;
    let buffers = unsafe { buffers.as_slice()? };
    let mut iovecs = Vec::with_capacity(buffers.len());
    for buffer in buffers {
        let slice = unsafe { buffer.as_slice()? };
        iovecs.push(libc::iovec {
            iov_base: slice.as_ptr() as *mut libc::c_void,
            iov_len: slice.len(),
        });
    }

    // write to the socket
    let rc = unsafe { libc::writev(fd, iovecs.as_ptr(), iovecs.len() as libc::c_int) };
    if rc < 0 {
        return Err(core_platform::net_error("writev"));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Receive multiple messages into multiple buffers.
#[allow(dead_code)]
pub(crate) unsafe fn destack_net_recv_mmsg(
    context: &RuntimeCallContext,
    out: *mut NativeArray<u64>,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    recv_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate receive flags
    if recv_flags.0 > i32::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "recvFlags",
            "flags value is out of range",
        ))
        .boxed());
    }

    // resolve socket and buffers
    let fd = socket_descriptor(context, handle)?;
    let buffers = unsafe { buffers.as_slice()? };
    let mut counts = Vec::with_capacity(buffers.len());

    // receive into each buffer
    for buffer in buffers {
        let slice = unsafe { buffer.as_mut_slice()? };
        let rc = unsafe {
            libc::recv(
                fd,
                slice.as_mut_ptr() as *mut libc::c_void,
                slice.len(),
                recv_flags.0 as libc::c_int,
            )
        };

        // stop on would-block once at least one packet is received
        if rc < 0 {
            let errno = core_platform::get_errno();
            if !counts.is_empty() && (errno == libc::EAGAIN || errno == libc::EWOULDBLOCK) {
                break;
            }

            return Err(core_platform::net_error("recv"));
        }

        // stop on orderly shutdown
        if rc == 0 {
            break;
        }

        counts.push(rc as u64);
    }

    // write the output array
    unsafe {
        *out = context.store_array(counts);
    }

    Ok(())
}

/// Receive a message with ancillary data.
pub(crate) unsafe fn destack_net_recv_msg(
    context: &RuntimeCallContext,
    out: *mut SocketRecvMessage,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    recv_flags: SocketMessageFlags,
    max_fds: u32,
    want_credentials: bool,
    max_control_bytes: u32,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor and buffer
    let fd = socket_descriptor(context, handle)?;
    let buffer = unsafe { buffer.as_mut_slice()? };

    // build the iovec
    let mut iovec = libc::iovec {
        iov_base: buffer.as_mut_ptr() as *mut libc::c_void,
        iov_len: buffer.len(),
    };

    // compute control buffer length
    let mut control_len = 0usize;
    if max_fds > 0 {
        let fd_bytes = (max_fds as usize)
            .checked_mul(std::mem::size_of::<RawFd>())
            .ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "maxFds",
                    "fd count is too large",
                ))
                .boxed()
            })?;
        if fd_bytes > u32::MAX as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "maxFds",
                "fd count is too large",
            ))
            .boxed());
        }
        control_len =
            control_len.saturating_add(unsafe { libc::CMSG_SPACE(fd_bytes as u32) } as usize);
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if want_credentials {
        control_len = control_len.saturating_add(unsafe {
            libc::CMSG_SPACE(std::mem::size_of::<libc::ucred>() as u32)
        } as usize);
    }

    // cap control extraction to caller budget
    if max_control_bytes > 0 {
        control_len = control_len.min(max_control_bytes as usize);
    }

    // allocate the control buffer
    let mut control = vec![0u8; control_len];

    // build the message header
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iovec;
    message.msg_iovlen = 1;
    if !control.is_empty() {
        let control_len = control.len();
        if control_len > u32::MAX as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "buffer",
                "control buffer too large",
            ))
            .boxed());
        }
        message.msg_control = control.as_mut_ptr() as *mut libc::c_void;
        message.msg_controllen = control_len as _;
    }

    // validate receive flags
    if recv_flags.0 > i32::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "recvFlags",
            "flags value is out of range",
        ))
        .boxed());
    }

    // receive the message
    let rc = unsafe { libc::recvmsg(fd, &mut message, recv_flags.0 as libc::c_int) };
    if rc < 0 {
        return Err(core_platform::net_error("recvmsg"));
    }

    // decode recvmsg flags
    let recv_flags = SocketMessageFlags(message.msg_flags as u32);
    let payload_truncated = (message.msg_flags & libc::MSG_TRUNC) != 0;
    let control_truncated = (message.msg_flags & libc::MSG_CTRUNC) != 0;

    // decode ancillary data
    let mut received_fds: Vec<RawFd> = Vec::new();
    let mut credentials: Option<SocketCredentials> = None;
    let mut cmsg = unsafe { libc::CMSG_FIRSTHDR(&message) };
    while !cmsg.is_null() {
        let header = unsafe { &*cmsg };
        if header.cmsg_level == libc::SOL_SOCKET && header.cmsg_type == libc::SCM_RIGHTS {
            let data_len = header.cmsg_len as usize - unsafe { libc::CMSG_LEN(0) as usize };
            let count = data_len / std::mem::size_of::<RawFd>();
            let data = unsafe { libc::CMSG_DATA(cmsg) as *const RawFd };
            for index in 0..count {
                received_fds.push(unsafe { *data.add(index) });
            }
        }
        if header.cmsg_level == libc::SOL_SOCKET {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            if header.cmsg_type == libc::SCM_CREDENTIALS {
                let data = unsafe { libc::CMSG_DATA(cmsg) as *const libc::ucred };
                let ucred = unsafe { *data };
                if want_credentials {
                    credentials = Some(SocketCredentials {
                        pid: ucred.pid as u32,
                        uid: ucred.uid,
                        gid: ucred.gid,
                    });
                }
            }
        }
        cmsg = unsafe { libc::CMSG_NXTHDR(&message, cmsg) };
    }

    // register received file descriptors
    let mut handles = Vec::with_capacity(received_fds.len().min(max_fds as usize));
    for (index, descriptor) in received_fds.into_iter().enumerate() {
        if index >= max_fds as usize {
            unsafe {
                libc::close(descriptor);
            }
            continue;
        }
        let kind = kind_from_received_descriptor(descriptor)?;
        let entry = ResourceEntry::new(kind)
            .with_fd(descriptor)
            .with_finalizer(FileFinalizer { fd: descriptor });
        let id = context.runtime().resources.insert(entry);
        handles.push(TransferredHandle(id));
    }

    // resolve missing credentials through peer socket metadata
    if want_credentials && credentials.is_none() {
        credentials = Some(peer_socket_credentials(fd)?);
    }

    // build the response payload
    let fds = context.store_array(handles);
    let control = context.store_array(Vec::<u8>::new());
    let address = SocketAddress {
        family: 0,
        length: 0,
        bytes: context.store_array(Vec::new()),
    };
    let has_credentials = credentials.is_some();
    let credentials = if let Some(credentials) = credentials {
        credentials
    } else {
        SocketCredentials {
            pid: 0,
            uid: 0,
            gid: 0,
        }
    };
    unsafe {
        *out = SocketRecvMessage {
            bytes: rc as u64,
            has_address: false,
            address,
            recv_flags,
            payload_truncated,
            control_truncated,
            control: SocketControlBufferAbi(control),
            fds,
            has_credentials,
            credentials,
        };
    }

    Ok(())
}

/// Send a message with ancillary data.
pub(crate) unsafe fn destack_net_send_msg(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: NativeSlice<u8>,
    message: SocketSendMessage,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor and buffer
    let fd = socket_descriptor(context, handle)?;
    let buffer = unsafe { buffer.as_slice()? };

    // resolve file descriptors to send
    let fds = unsafe { message.fds.as_slice()? };
    let mut raw_fds = Vec::with_capacity(fds.len());
    for handle in fds {
        raw_fds.push(transferable_descriptor(context, *handle)?);
    }

    // validate credentials support
    if message.has_credentials {
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            return Err(
                RuntimeError::from(PlatformError::not_supported("destack.net.sendMsg")).boxed(),
            );
        }
    }

    // validate send flags before dispatch
    if message.flags.0 > i32::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "message.flags",
            "flags value is out of range",
        ))
        .boxed());
    }

    // build the iovec
    let mut iovec = libc::iovec {
        iov_base: buffer.as_ptr() as *mut libc::c_void,
        iov_len: buffer.len(),
    };

    // compute control buffer length
    let mut control_len = 0usize;
    if !raw_fds.is_empty() {
        let fd_bytes = raw_fds
            .len()
            .checked_mul(std::mem::size_of::<RawFd>())
            .ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "message.fds",
                    "fd count is too large",
                ))
                .boxed()
            })?;
        if fd_bytes > u32::MAX as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "message.fds",
                "fd count is too large",
            ))
            .boxed());
        }
        control_len =
            control_len.saturating_add(unsafe { libc::CMSG_SPACE(fd_bytes as u32) } as usize);
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if message.has_credentials {
        control_len = control_len.saturating_add(unsafe {
            libc::CMSG_SPACE(std::mem::size_of::<libc::ucred>() as u32)
        } as usize);
    }

    // allocate the control buffer
    let mut control = vec![0u8; control_len];

    // build the message header
    let mut hdr: libc::msghdr = unsafe { std::mem::zeroed() };
    hdr.msg_iov = &mut iovec;
    hdr.msg_iovlen = 1;
    if !control.is_empty() {
        let control_len = control.len();
        if control_len > u32::MAX as usize {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "buffer",
                "control buffer too large",
            ))
            .boxed());
        }
        hdr.msg_control = control.as_mut_ptr() as *mut libc::c_void;
        hdr.msg_controllen = control_len as _;
    }

    // fill control messages
    #[cfg(any(target_os = "linux", target_os = "android"))]
    let mut cmsg = unsafe { libc::CMSG_FIRSTHDR(&hdr) };
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    let cmsg = unsafe { libc::CMSG_FIRSTHDR(&hdr) };
    if !raw_fds.is_empty() {
        let header = unsafe { &mut *cmsg };
        header.cmsg_level = libc::SOL_SOCKET;
        header.cmsg_type = libc::SCM_RIGHTS;
        let cmsg_len =
            unsafe { libc::CMSG_LEN((raw_fds.len() * std::mem::size_of::<RawFd>()) as u32) }
                as usize;
        header.cmsg_len = cmsg_len as _;
        let data = unsafe { libc::CMSG_DATA(cmsg) as *mut RawFd };
        unsafe {
            std::ptr::copy_nonoverlapping(raw_fds.as_ptr(), data, raw_fds.len());
        }
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            cmsg = unsafe { libc::CMSG_NXTHDR(&hdr, cmsg) };
        }
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            let _ = cmsg;
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    if message.has_credentials {
        if cmsg.is_null() {
            return Err(RuntimeError::from(PlatformError::io("missing control buffer")).boxed());
        }
        let header = unsafe { &mut *cmsg };
        header.cmsg_level = libc::SOL_SOCKET;
        header.cmsg_type = libc::SCM_CREDENTIALS;
        let cmsg_len =
            unsafe { libc::CMSG_LEN(std::mem::size_of::<libc::ucred>() as u32) } as usize;
        header.cmsg_len = cmsg_len as _;
        let data = unsafe { libc::CMSG_DATA(cmsg) as *mut libc::ucred };
        unsafe {
            *data = libc::ucred {
                pid: message.credentials.pid as libc::pid_t,
                uid: message.credentials.uid,
                gid: message.credentials.gid,
            };
        }
    }

    // send the message
    let rc = unsafe { libc::sendmsg(fd, &hdr, message.flags.0 as libc::c_int) };
    if rc < 0 {
        return Err(core_platform::net_error("sendmsg"));
    }

    unsafe {
        *out = rc as u64;
    }

    Ok(())
}

/// Send multiple messages from multiple buffers.
#[allow(dead_code)]
pub(crate) unsafe fn destack_net_send_mmsg(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    send_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate send flags
    if send_flags.0 > i32::MAX as u32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "sendFlags",
            "flags value is out of range",
        ))
        .boxed());
    }

    // resolve socket and buffers
    let fd = socket_descriptor(context, handle)?;
    let buffers = unsafe { buffers.as_slice()? };
    let mut sent_count = 0u64;

    // send each buffer as an individual message
    for buffer in buffers {
        let slice = unsafe { buffer.as_slice()? };
        let rc = unsafe {
            libc::send(
                fd,
                slice.as_ptr() as *const libc::c_void,
                slice.len(),
                send_flags.0 as libc::c_int,
            )
        };

        // stop after partial progress on transient send errors
        if rc < 0 {
            let errno = core_platform::get_errno();
            if sent_count > 0 && (errno == libc::EAGAIN || errno == libc::EWOULDBLOCK) {
                break;
            }

            return Err(core_platform::net_error("send"));
        }

        sent_count = sent_count.saturating_add(1);
    }

    // write the output count
    unsafe {
        *out = sent_count;
    }

    Ok(())
}

/// Resolve a hostname and port into socket addresses.
pub(crate) unsafe fn destack_net_resolve(
    context: &RuntimeCallContext,
    out: *mut NativeArray<SocketAddress>,
    host: NativeStringRef,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the host string
    let host = unsafe { host.as_str()? };
    if host.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "host contains nul byte",
        ))
        .boxed());
    }

    // enforce numeric-only resolution when requested
    if flags.0 & 0x4 != 0 && host.parse::<IpAddr>().is_err() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "numeric host required",
        ))
        .boxed());
    }

    // build addrinfo hints
    let mut hints: libc::addrinfo = unsafe { std::mem::zeroed() };
    hints.ai_family = match family {
        SocketFamily::IPv4 => libc::AF_INET,
        SocketFamily::IPv6 => libc::AF_INET6,
        SocketFamily::Unspecified => libc::AF_UNSPEC,
    };
    hints.ai_socktype = libc::SOCK_STREAM;
    hints.ai_protocol = libc::IPPROTO_TCP;
    hints.ai_flags = 0;
    if flags.0 & 0x1 != 0 {
        hints.ai_flags |= libc::AI_PASSIVE;
    }
    if flags.0 & 0x2 != 0 {
        hints.ai_flags |= libc::AI_CANONNAME;
    }
    if flags.0 & 0x4 != 0 {
        hints.ai_flags |= libc::AI_NUMERICHOST;
    }
    if flags.0 & 0x8 != 0 {
        hints.ai_flags |= libc::AI_NUMERICSERV;
    }
    if flags.0 & 0x10 != 0 {
        hints.ai_flags |= libc::AI_V4MAPPED;
    }
    if flags.0 & 0x20 != 0 {
        hints.ai_flags |= libc::AI_ALL;
    }
    if flags.0 & 0x40 != 0 {
        hints.ai_flags |= libc::AI_ADDRCONFIG;
    }

    // resolve addresses
    let host_c = CString::new(host).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "host contains nul byte",
        ))
        .boxed()
    })?;
    let service = CString::new(port.to_string()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "port",
            "port conversion failed",
        ))
        .boxed()
    })?;
    let mut result: *mut libc::addrinfo = std::ptr::null_mut();
    let rc = unsafe { libc::getaddrinfo(host_c.as_ptr(), service.as_ptr(), &hints, &mut result) };
    if rc != 0 {
        let error = unsafe { CStr::from_ptr(libc::gai_strerror(rc)) }
            .to_string_lossy()
            .to_string();
        return Err(
            RuntimeError::from(PlatformError::io(format!("getaddrinfo failed: {error}"))).boxed(),
        );
    }

    struct AddrInfoGuard(*mut libc::addrinfo);
    impl Drop for AddrInfoGuard {
        fn drop(&mut self) {
            unsafe {
                libc::freeaddrinfo(self.0);
            }
        }
    }
    let _guard = AddrInfoGuard(result);

    // collect addresses
    let mut addresses = Vec::new();
    let mut current = result;
    while !current.is_null() {
        let info = unsafe { &*current };
        let storage = unsafe { &*(info.ai_addr as *const libc::sockaddr_storage) };
        let address = socket_address_from_storage(context, storage, info.ai_addrlen)?;
        addresses.push(address);
        current = info.ai_next;
    }

    unsafe {
        *out = context.store_array(addresses);
    }

    Ok(())
}

/// Reverse lookup a socket address into hostnames.
pub(crate) unsafe fn destack_net_reverse_lookup(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the raw socket address
    let bytes = unsafe { address.bytes.as_slice()? };
    if bytes.is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "raw address is empty",
        ))
        .boxed());
    }
    if bytes.len() > std::mem::size_of::<libc::sockaddr_storage>() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "raw address is too large",
        ))
        .boxed());
    }

    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            &mut storage as *mut _ as *mut u8,
            bytes.len(),
        );
    }
    let length = bytes.len() as libc::socklen_t;

    let mut host = [0 as libc::c_char; libc::NI_MAXHOST as usize];
    let rc = unsafe {
        libc::getnameinfo(
            &storage as *const _ as *const libc::sockaddr,
            length,
            host.as_mut_ptr(),
            host.len() as libc::socklen_t,
            std::ptr::null_mut(),
            0,
            libc::NI_NAMEREQD,
        )
    };
    if rc != 0 {
        let error = unsafe { CStr::from_ptr(libc::gai_strerror(rc)) }
            .to_string_lossy()
            .to_string();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "reverse lookup failed: {error}"
        )))
        .boxed());
    }
    let name = unsafe { CStr::from_ptr(host.as_ptr()) }
        .to_string_lossy()
        .to_string();
    unsafe {
        *out = context.store_array(vec![context.store_string(&name)]);
    }

    Ok(())
}

/// Create a UDP socket.
pub(crate) unsafe fn destack_net_udp_socket(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // select the socket family
    let family = match family {
        SocketFamily::IPv4 => libc::AF_INET,
        SocketFamily::IPv6 => libc::AF_INET6,
        SocketFamily::Unspecified => libc::AF_INET,
    };

    // create the socket
    let fd = unsafe { libc::socket(family, libc::SOCK_DGRAM, libc::IPPROTO_UDP) };
    if fd < 0 {
        return Err(core_platform::net_error("socket"));
    }

    // register the socket
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(fd)
        .with_finalizer(SocketFinalizer { fd });
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = SocketHandle(resource_id);
    }

    Ok(())
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
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = ListenerHandle(resource_id);
    }

    Ok(())
}

/// Accept a connection from a UNIX domain socket listener.
pub(crate) unsafe fn destack_net_uds_accept(
    context: &RuntimeCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // accept via the raw file descriptor
    let fd = listener_descriptor(context, listener)?;
    let fd = unsafe { libc::accept(fd, std::ptr::null_mut(), std::ptr::null_mut()) };
    if fd < 0 {
        return Err(core_platform::net_error("accept"));
    }
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(fd)
        .with_finalizer(SocketFinalizer { fd });
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = SocketHandle(resource_id);
    }

    Ok(())
}

/// Close a UNIX domain socket listener.
pub(crate) unsafe fn destack_net_uds_close_listener(
    context: &RuntimeCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    unsafe { destack_net_close_listener(context, handle) }
}

/// Set socket linger settings.
pub(crate) unsafe fn destack_net_set_linger(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    linger: Linger,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    let value = libc::linger {
        l_onoff: if linger.enabled { 1 } else { 0 },
        l_linger: linger.seconds as libc::c_int,
    };
    let rc = unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_LINGER,
            &value as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::linger>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error("setsockopt"));
    }
    Ok(())
}

/// Set the receive buffer size.
pub(crate) unsafe fn destack_net_set_recv_buffer(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    set_socket_u32(fd, libc::SOL_SOCKET, libc::SO_RCVBUF, size)
}

/// Set the send buffer size.
pub(crate) unsafe fn destack_net_set_send_buffer(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    set_socket_u32(fd, libc::SOL_SOCKET, libc::SO_SNDBUF, size)
}

/// Enable or disable broadcast.
pub(crate) unsafe fn destack_net_set_broadcast(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    set_socket_bool(fd, libc::SOL_SOCKET, libc::SO_BROADCAST, enabled)
}

/// Join an IPv4 multicast group.
pub(crate) unsafe fn destack_net_join_multicast_v4(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve socket descriptor
    let fd = socket_descriptor(context, handle)?;
    let group = unsafe { group.as_str()? };
    let interface_address = unsafe { interface_address.as_str()? };

    // parse and apply ipv4 membership
    let group_addr = group.parse::<Ipv4Addr>().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "group",
            "invalid IPv4 multicast address",
        ))
        .boxed()
    })?;
    let interface_addr = parse_ipv4_interface(interface_address)?;
    let request = libc::ip_mreq {
        imr_multiaddr: libc::in_addr {
            s_addr: u32::from_ne_bytes(group_addr.octets()).to_be(),
        },
        imr_interface: libc::in_addr {
            s_addr: u32::from_ne_bytes(interface_addr.octets()).to_be(),
        },
    };
    let rc = unsafe {
        libc::setsockopt(
            fd,
            libc::IPPROTO_IP,
            libc::IP_ADD_MEMBERSHIP,
            &request as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::ip_mreq>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error("setsockopt(IP_ADD_MEMBERSHIP)"));
    }

    Ok(())
}

/// Leave an IPv4 multicast group.
pub(crate) unsafe fn destack_net_leave_multicast_v4(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve socket descriptor
    let fd = socket_descriptor(context, handle)?;
    let group = unsafe { group.as_str()? };
    let interface_address = unsafe { interface_address.as_str()? };

    // parse and apply ipv4 membership
    let group_addr = group.parse::<Ipv4Addr>().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "group",
            "invalid IPv4 multicast address",
        ))
        .boxed()
    })?;
    let interface_addr = parse_ipv4_interface(interface_address)?;
    let request = libc::ip_mreq {
        imr_multiaddr: libc::in_addr {
            s_addr: u32::from_ne_bytes(group_addr.octets()).to_be(),
        },
        imr_interface: libc::in_addr {
            s_addr: u32::from_ne_bytes(interface_addr.octets()).to_be(),
        },
    };
    let rc = unsafe {
        libc::setsockopt(
            fd,
            libc::IPPROTO_IP,
            libc::IP_DROP_MEMBERSHIP,
            &request as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::ip_mreq>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error("setsockopt(IP_DROP_MEMBERSHIP)"));
    }

    Ok(())
}

/// Join an IPv6 multicast group.
pub(crate) unsafe fn destack_net_join_multicast_v6(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_index: u32,
) -> RuntimeResult<()> {
    // resolve socket descriptor
    let fd = socket_descriptor(context, handle)?;
    let group = unsafe { group.as_str()? };

    // parse and apply ipv6 membership
    let group_addr = group.parse::<Ipv6Addr>().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "group",
            "invalid IPv6 multicast address",
        ))
        .boxed()
    })?;
    let request = libc::ipv6_mreq {
        ipv6mr_multiaddr: libc::in6_addr {
            s6_addr: group_addr.octets(),
        },
        ipv6mr_interface: interface_index,
    };
    let rc = unsafe {
        libc::setsockopt(
            fd,
            libc::IPPROTO_IPV6,
            IPV6_JOIN_GROUP_OPT,
            &request as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::ipv6_mreq>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error("setsockopt(IPV6_JOIN_GROUP)"));
    }

    Ok(())
}

/// Leave an IPv6 multicast group.
pub(crate) unsafe fn destack_net_leave_multicast_v6(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_index: u32,
) -> RuntimeResult<()> {
    // resolve socket descriptor
    let fd = socket_descriptor(context, handle)?;
    let group = unsafe { group.as_str()? };

    // parse and apply ipv6 membership
    let group_addr = group.parse::<Ipv6Addr>().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "group",
            "invalid IPv6 multicast address",
        ))
        .boxed()
    })?;
    let request = libc::ipv6_mreq {
        ipv6mr_multiaddr: libc::in6_addr {
            s6_addr: group_addr.octets(),
        },
        ipv6mr_interface: interface_index,
    };
    let rc = unsafe {
        libc::setsockopt(
            fd,
            libc::IPPROTO_IPV6,
            IPV6_LEAVE_GROUP_OPT,
            &request as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::ipv6_mreq>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error("setsockopt(IPV6_LEAVE_GROUP)"));
    }

    Ok(())
}

/// Enable or disable multicast loopback.
pub(crate) unsafe fn destack_net_set_multicast_loop(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve socket metadata
    let fd = socket_descriptor(context, handle)?;
    let family = socket_family_from_fd(fd)?;

    // dispatch by socket family
    match family {
        SocketFamily::IPv4 => {
            set_socket_u8(fd, libc::IPPROTO_IP, libc::IP_MULTICAST_LOOP, enabled as u8)
        }
        SocketFamily::IPv6 => set_socket_u32(
            fd,
            libc::IPPROTO_IPV6,
            libc::IPV6_MULTICAST_LOOP,
            enabled as u32,
        ),
        SocketFamily::Unspecified => Err(RuntimeError::from(
            PlatformError::invalid_argument_value("handle", "unsupported socket family"),
        )
        .boxed()),
    }
}

/// Set multicast TTL.
pub(crate) unsafe fn destack_net_set_multicast_ttl(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    // resolve socket metadata
    let fd = socket_descriptor(context, handle)?;
    let family = socket_family_from_fd(fd)?;

    // dispatch by socket family
    match family {
        SocketFamily::IPv4 => {
            set_socket_u8(fd, libc::IPPROTO_IP, libc::IP_MULTICAST_TTL, ttl as u8)
        }
        SocketFamily::IPv6 => {
            set_socket_u32(fd, libc::IPPROTO_IPV6, libc::IPV6_MULTICAST_HOPS, ttl)
        }
        SocketFamily::Unspecified => Err(RuntimeError::from(
            PlatformError::invalid_argument_value("handle", "unsupported socket family"),
        )
        .boxed()),
    }
}

/// Set the IP time-to-live.
pub(crate) unsafe fn destack_net_set_ttl(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    set_socket_u32(fd, libc::IPPROTO_IP, libc::IP_TTL, ttl)
}

/// Set the IP type-of-service field.
pub(crate) unsafe fn destack_net_set_tos(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    set_socket_u32(fd, libc::IPPROTO_IP, libc::IP_TOS, tos)
}

/// Set the read timeout in milliseconds.
pub(crate) unsafe fn destack_net_set_read_timeout(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    let timeout = libc::timeval {
        tv_sec: (timeout_ms / 1000) as libc::time_t,
        tv_usec: ((timeout_ms % 1000) * 1000) as libc::suseconds_t,
    };
    let rc = unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            &timeout as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::timeval>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error("setsockopt"));
    }
    Ok(())
}

/// Set the write timeout in milliseconds.
pub(crate) unsafe fn destack_net_set_write_timeout(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    let timeout = libc::timeval {
        tv_sec: (timeout_ms / 1000) as libc::time_t,
        tv_usec: ((timeout_ms % 1000) * 1000) as libc::suseconds_t,
    };
    let rc = unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_SNDTIMEO,
            &timeout as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::timeval>() as libc::socklen_t,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error("setsockopt"));
    }
    Ok(())
}
