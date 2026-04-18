use std::net::Ipv4Addr;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{
    SocketAddress, SocketCredentials, SocketFamily, SocketShutdown, core as core_net,
};
use crate::platform::resource::{
    ListenerHandle, ResourceFinalizer, ResourceKind, SocketHandle, TransferredHandle,
};
use crate::platform::{PlatformError, ResourceId, core as core_platform};
use crate::runtime::BindingCallContext;

use std::os::unix::io::RawFd;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(super) const IPV6_JOIN_GROUP_OPT: libc::c_int = libc::IPV6_ADD_MEMBERSHIP;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(super) const IPV6_LEAVE_GROUP_OPT: libc::c_int = libc::IPV6_DROP_MEMBERSHIP;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub(super) const IPV6_JOIN_GROUP_OPT: libc::c_int = libc::IPV6_JOIN_GROUP;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
pub(super) const IPV6_LEAVE_GROUP_OPT: libc::c_int = libc::IPV6_LEAVE_GROUP;

/// Normalize one host sockaddr family value into the ABI family type.
#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
))]
fn socket_family_from_storage(value: libc::sa_family_t) -> u16 {
    u16::from(value)
}

/// Normalize one host sockaddr family value into the ABI family type.
#[cfg(not(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
)))]
fn socket_family_from_storage(value: libc::sa_family_t) -> u16 {
    value
}

/// Return one mutable unix-domain path buffer pointer as bytes.
fn unix_socket_path_pointer(address: &mut libc::sockaddr_un) -> *mut u8 {
    #[cfg(target_os = "android")]
    {
        address.sun_path.as_mut_ptr()
    }

    #[cfg(not(target_os = "android"))]
    {
        address.sun_path.as_mut_ptr().cast::<u8>()
    }
}

pub(super) struct SocketFinalizer {
    /// Socket descriptor to close.
    pub(super) fd: RawFd,
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
pub(super) struct FileFinalizer {
    /// File descriptor to close.
    pub(super) fd: RawFd,
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
pub(super) fn socket_address_raw_from_fd(
    binding: &BindingCallContext,
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
    socket_address_raw_from_storage(binding, &storage, length)
}

/// Decode a raw socket address from raw storage.
pub(super) fn socket_address_raw_from_storage(
    binding: &BindingCallContext,
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
    let bytes = binding.store_array_copy(bytes);

    // write the raw sockaddr payload
    Ok(SocketAddress {
        family: socket_family_from_storage(storage.ss_family),
        length,
        bytes,
    })
}

/// Decode a socket address from raw storage.
pub(super) fn socket_address_from_storage(
    binding: &BindingCallContext,
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
    let bytes = binding.store_array_copy(bytes);

    Ok(SocketAddress {
        family: socket_family_from_storage(storage.ss_family),
        length,
        bytes,
    })
}

/// Build a sockaddr_un from a byte path.
pub(super) fn sockaddr_un_from_path(
    path: &[u8],
) -> RuntimeResult<(libc::sockaddr_un, libc::socklen_t, bool)> {
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
            unix_socket_path_pointer(&mut addr),
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
pub(super) fn socket_descriptor(
    binding: &BindingCallContext,
    handle: SocketHandle,
) -> RuntimeResult<RawFd> {
    // resolve the socket resource
    core_net::require_resource(binding, handle.0, ResourceKind::Socket, "socket", |entry| {
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
pub(super) fn transferable_descriptor(
    binding: &BindingCallContext,
    handle: TransferredHandle,
) -> RuntimeResult<RawFd> {
    let descriptor = binding
        .worker()
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
pub(super) fn kind_from_received_descriptor(descriptor: RawFd) -> RuntimeResult<ResourceKind> {
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
pub(super) fn socket_family_from_fd(fd: RawFd) -> RuntimeResult<SocketFamily> {
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
pub(super) fn parse_ipv4_interface(name: &str) -> RuntimeResult<Ipv4Addr> {
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
pub(super) fn peer_socket_credentials(fd: RawFd) -> RuntimeResult<SocketCredentials> {
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
pub(super) fn listener_descriptor(
    binding: &BindingCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<RawFd> {
    // resolve the listener resource
    core_net::require_resource(
        binding,
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
pub(super) fn shutdown_how(how: SocketShutdown) -> libc::c_int {
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

/// Finalizer that closes a unix file descriptor.
#[derive(Debug)]
pub(super) struct DescriptorFinalizer {
    /// The descriptor to close.
    pub(super) fd: RawFd,
}

impl ResourceFinalizer for DescriptorFinalizer {
    /// Close the descriptor when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// Convert a socket family enum into a platform address family.
pub(super) fn socket_family_to_raw(family: SocketFamily) -> libc::c_int {
    match family {
        SocketFamily::Unspecified => libc::AF_INET,
        SocketFamily::IPv4 => libc::AF_INET,
        SocketFamily::IPv6 => libc::AF_INET6,
    }
}

/// Convert a raw socket address payload into syscall arguments.
pub(super) fn with_socket_address_raw<T>(
    address: SocketAddress,
    with_sockaddr: impl FnOnce(*const libc::sockaddr, libc::socklen_t) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    // validate raw address bytes
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

    // dispatch to the syscall closure
    let pointer = bytes.as_ptr() as *const libc::sockaddr;
    let length = bytes.len() as libc::socklen_t;
    with_sockaddr(pointer, length)
}

/// Read a boolean socket option.
pub(super) fn get_socket_bool(
    fd: RawFd,
    level: libc::c_int,
    option: libc::c_int,
    syscall: &str,
) -> RuntimeResult<bool> {
    let mut value: libc::c_int = 0;
    let mut length = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            fd,
            level,
            option,
            &mut value as *mut _ as *mut libc::c_void,
            &mut length,
        )
    };
    if result != 0 {
        return Err(core_platform::net_error(syscall));
    }

    Ok(value != 0)
}

/// Read a u32 socket option.
pub(super) fn get_socket_u32(
    fd: RawFd,
    level: libc::c_int,
    option: libc::c_int,
    syscall: &str,
) -> RuntimeResult<u32> {
    let mut value: libc::c_int = 0;
    let mut length = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            fd,
            level,
            option,
            &mut value as *mut _ as *mut libc::c_void,
            &mut length,
        )
    };
    if result != 0 {
        return Err(core_platform::net_error(syscall));
    }

    Ok(value as u32)
}
