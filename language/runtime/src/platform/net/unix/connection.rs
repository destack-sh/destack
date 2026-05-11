use super::core::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::core::decode_accept_flags;
use crate::platform::net::*;
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::unix::io::RawFd;

#[cfg(any(target_os = "linux", target_os = "android"))]
const SOCKET_TYPE_FLAG_NONBLOCK: libc::c_int = libc::SOCK_NONBLOCK;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
const SOCKET_TYPE_FLAG_NONBLOCK: libc::c_int = 0;

#[cfg(any(target_os = "linux", target_os = "android"))]
const SOCKET_TYPE_FLAG_CLOEXEC: libc::c_int = libc::SOCK_CLOEXEC;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
const SOCKET_TYPE_FLAG_CLOEXEC: libc::c_int = 0;

fn close_socket(fd: RawFd) {
    let _ = unsafe { libc::close(fd) };
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn ipv4_loopback_address() -> libc::sockaddr_in {
    libc::sockaddr_in {
        sin_len: std::mem::size_of::<libc::sockaddr_in>() as u8,
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr {
            s_addr: u32::from(Ipv4Addr::LOCALHOST).to_be(),
        },
        sin_zero: [0; 8],
    }
}

#[cfg(not(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
)))]
fn ipv4_loopback_address() -> libc::sockaddr_in {
    libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr {
            s_addr: u32::from(Ipv4Addr::LOCALHOST).to_be(),
        },
        sin_zero: [0; 8],
    }
}

#[cfg(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn ipv6_loopback_address() -> libc::sockaddr_in6 {
    libc::sockaddr_in6 {
        sin6_len: std::mem::size_of::<libc::sockaddr_in6>() as u8,
        sin6_family: libc::AF_INET6 as libc::sa_family_t,
        sin6_port: 0,
        sin6_flowinfo: 0,
        sin6_addr: libc::in6_addr {
            s6_addr: Ipv6Addr::LOCALHOST.octets(),
        },
        sin6_scope_id: 0,
    }
}

#[cfg(not(any(
    target_vendor = "apple",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
)))]
fn ipv6_loopback_address() -> libc::sockaddr_in6 {
    libc::sockaddr_in6 {
        sin6_family: libc::AF_INET6 as libc::sa_family_t,
        sin6_port: 0,
        sin6_flowinfo: 0,
        sin6_addr: libc::in6_addr {
            s6_addr: Ipv6Addr::LOCALHOST.octets(),
        },
        sin6_scope_id: 0,
    }
}

fn bind_socket_loopback(fd: RawFd, family: libc::c_int) -> RuntimeResult<()> {
    if family == libc::AF_INET {
        let address = ipv4_loopback_address();
        let rc = unsafe {
            libc::bind(
                fd,
                &address as *const _ as *const libc::sockaddr,
                std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
            )
        };
        if rc != 0 {
            return Err(core_platform::net_error("bind"));
        }

        return Ok(());
    }

    if family == libc::AF_INET6 {
        let address = ipv6_loopback_address();
        let rc = unsafe {
            libc::bind(
                fd,
                &address as *const _ as *const libc::sockaddr,
                std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
            )
        };
        if rc != 0 {
            return Err(core_platform::net_error("bind"));
        }

        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "family",
        "unsupported socket family",
    ))
    .boxed())
}

fn local_socket_address(fd: RawFd) -> RuntimeResult<(libc::sockaddr_storage, libc::socklen_t)> {
    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    let mut length = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
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

    Ok((storage, length))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn apply_socket_pair_flags(fd: RawFd, requested_type: libc::c_int) -> RuntimeResult<()> {
    if (requested_type & SOCKET_TYPE_FLAG_NONBLOCK) != 0 {
        let current_flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if current_flags < 0 {
            return Err(core_platform::net_error("fcntl"));
        }

        let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, current_flags | libc::O_NONBLOCK) };
        if rc < 0 {
            return Err(core_platform::net_error("fcntl"));
        }
    }

    if (requested_type & SOCKET_TYPE_FLAG_CLOEXEC) != 0 {
        let current_flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        if current_flags < 0 {
            return Err(core_platform::net_error("fcntl"));
        }

        let rc = unsafe { libc::fcntl(fd, libc::F_SETFD, current_flags | libc::FD_CLOEXEC) };
        if rc < 0 {
            return Err(core_platform::net_error("fcntl"));
        }
    }

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn apply_socket_pair_flags(_fd: RawFd, _requested_type: libc::c_int) -> RuntimeResult<()> {
    Ok(())
}

fn register_socket_pair(
    binding: &BindingCallContext,
    out: *mut SocketPair,
    first_fd: RawFd,
    second_fd: RawFd,
) {
    let first_entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(first_fd)
        .with_finalizer(DescriptorFinalizer { fd: first_fd });
    let first_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), first_entry, Some(binding.engine()));

    let second_entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(second_fd)
        .with_finalizer(DescriptorFinalizer { fd: second_fd });
    let second_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), second_entry, Some(binding.engine()));

    unsafe {
        *out = SocketPair {
            first: SocketHandle(first_id),
            second: SocketHandle(second_id),
        };
    }
}

fn socket_pair_stream_loopback(
    binding: &BindingCallContext,
    out: *mut SocketPair,
    family: libc::c_int,
    socket_type: libc::c_int,
    protocol: libc::c_int,
) -> RuntimeResult<()> {
    let listener_fd = unsafe { libc::socket(family, libc::SOCK_STREAM, protocol) };
    if listener_fd < 0 {
        return Err(core_platform::net_error("socket"));
    }

    if let Err(error) = bind_socket_loopback(listener_fd, family) {
        close_socket(listener_fd);
        return Err(error);
    }

    let (listen_address, listen_length) = match local_socket_address(listener_fd) {
        Ok(address) => address,
        Err(error) => {
            close_socket(listener_fd);
            return Err(error);
        }
    };

    let rc = unsafe { libc::listen(listener_fd, 1) };
    if rc != 0 {
        close_socket(listener_fd);
        return Err(core_platform::net_error("listen"));
    }

    let client_fd = unsafe { libc::socket(family, libc::SOCK_STREAM, protocol) };
    if client_fd < 0 {
        close_socket(listener_fd);
        return Err(core_platform::net_error("socket"));
    }

    let rc = unsafe {
        libc::connect(
            client_fd,
            &listen_address as *const _ as *const libc::sockaddr,
            listen_length,
        )
    };
    if rc != 0 {
        close_socket(client_fd);
        close_socket(listener_fd);
        return Err(core_platform::net_error("connect"));
    }

    let server_fd =
        unsafe { libc::accept(listener_fd, std::ptr::null_mut(), std::ptr::null_mut()) };
    if server_fd < 0 {
        close_socket(client_fd);
        close_socket(listener_fd);
        return Err(core_platform::net_error("accept"));
    }

    close_socket(listener_fd);

    if let Err(error) = apply_socket_pair_flags(client_fd, socket_type) {
        close_socket(server_fd);
        close_socket(client_fd);
        return Err(error);
    }

    if let Err(error) = apply_socket_pair_flags(server_fd, socket_type) {
        close_socket(server_fd);
        close_socket(client_fd);
        return Err(error);
    }

    register_socket_pair(binding, out, client_fd, server_fd);

    Ok(())
}

fn socket_pair_dgram_loopback(
    binding: &BindingCallContext,
    out: *mut SocketPair,
    family: libc::c_int,
    socket_type: libc::c_int,
    protocol: libc::c_int,
) -> RuntimeResult<()> {
    let first_fd = unsafe { libc::socket(family, libc::SOCK_DGRAM, protocol) };
    if first_fd < 0 {
        return Err(core_platform::net_error("socket"));
    }

    let second_fd = unsafe { libc::socket(family, libc::SOCK_DGRAM, protocol) };
    if second_fd < 0 {
        close_socket(first_fd);
        return Err(core_platform::net_error("socket"));
    }

    if let Err(error) = bind_socket_loopback(first_fd, family) {
        close_socket(second_fd);
        close_socket(first_fd);
        return Err(error);
    }

    if let Err(error) = bind_socket_loopback(second_fd, family) {
        close_socket(second_fd);
        close_socket(first_fd);
        return Err(error);
    }

    let (first_address, first_length) = match local_socket_address(first_fd) {
        Ok(address) => address,
        Err(error) => {
            close_socket(second_fd);
            close_socket(first_fd);
            return Err(error);
        }
    };

    let (second_address, second_length) = match local_socket_address(second_fd) {
        Ok(address) => address,
        Err(error) => {
            close_socket(second_fd);
            close_socket(first_fd);
            return Err(error);
        }
    };

    let rc = unsafe {
        libc::connect(
            first_fd,
            &second_address as *const _ as *const libc::sockaddr,
            second_length,
        )
    };
    if rc != 0 {
        close_socket(second_fd);
        close_socket(first_fd);
        return Err(core_platform::net_error("connect"));
    }

    let rc = unsafe {
        libc::connect(
            second_fd,
            &first_address as *const _ as *const libc::sockaddr,
            first_length,
        )
    };
    if rc != 0 {
        close_socket(second_fd);
        close_socket(first_fd);
        return Err(core_platform::net_error("connect"));
    }

    if let Err(error) = apply_socket_pair_flags(first_fd, socket_type) {
        close_socket(second_fd);
        close_socket(first_fd);
        return Err(error);
    }

    if let Err(error) = apply_socket_pair_flags(second_fd, socket_type) {
        close_socket(second_fd);
        close_socket(first_fd);
        return Err(error);
    }

    register_socket_pair(binding, out, first_fd, second_fd);

    Ok(())
}

/// Accept a new connection from a listener.
pub(crate) unsafe fn destack_net_accept(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    listener: ListenerHandle,
    flags: AcceptFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the listener descriptor and decode accept behavior
    let fd = listener_descriptor(binding, listener)?;
    let accept_behavior = decode_accept_flags(flags)?;

    // accept the connection first
    let client_fd = unsafe { libc::accept(fd, std::ptr::null_mut(), std::ptr::null_mut()) };
    if client_fd < 0 {
        return Err(core_platform::net_error("accept"));
    }

    // apply requested file-status and descriptor flags
    if accept_behavior.nonblocking {
        let current_flags = unsafe { libc::fcntl(client_fd, libc::F_GETFL) };
        if current_flags < 0 {
            unsafe {
                libc::close(client_fd);
            }
            return Err(core_platform::net_error("fcntl"));
        }

        let rc = unsafe { libc::fcntl(client_fd, libc::F_SETFL, current_flags | libc::O_NONBLOCK) };
        if rc < 0 {
            unsafe {
                libc::close(client_fd);
            }
            return Err(core_platform::net_error("fcntl"));
        }
    }

    // apply close-on-exec after accept on unix targets that do not expose a shared normalized path
    if accept_behavior.cloexec {
        let current_flags = unsafe { libc::fcntl(client_fd, libc::F_GETFD) };
        if current_flags < 0 {
            unsafe {
                libc::close(client_fd);
            }
            return Err(core_platform::net_error("fcntl"));
        }

        let rc = unsafe { libc::fcntl(client_fd, libc::F_SETFD, current_flags | libc::FD_CLOEXEC) };
        if rc < 0 {
            unsafe {
                libc::close(client_fd);
            }
            return Err(core_platform::net_error("fcntl"));
        }
    }

    // register the new socket
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(client_fd)
        .with_finalizer(SocketFinalizer { fd: client_fd });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
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
    // validate the handle kind
    let is_socket = binding
        .worker()
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
    if !binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown socket handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Close a listener handle.
pub(crate) unsafe fn destack_net_close_listener(
    binding: &BindingCallContext,
    handle: ListenerHandle,
) -> RuntimeResult<()> {
    // validate the handle kind
    let is_listener = binding
        .worker()
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
    if !binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown listener handle",
        ))
        .boxed());
    }

    Ok(())
}

/// Connect an existing socket to a raw remote address.
pub(crate) unsafe fn destack_net_connect_raw(
    binding: &BindingCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let fd = socket_descriptor(binding, handle)?;

    // connect using the provided raw sockaddr
    with_socket_address_raw(address, |sockaddr, length| {
        let result = unsafe { libc::connect(fd, sockaddr, length) };
        if result != 0 {
            return Err(core_platform::net_error("connect"));
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
    // resolve the socket descriptor
    let fd = socket_descriptor(binding, handle)?;

    // bind using the provided raw sockaddr
    with_socket_address_raw(address, |sockaddr, length| {
        let result = unsafe { libc::bind(fd, sockaddr, length) };
        if result != 0 {
            return Err(core_platform::net_error("bind"));
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

    // select the backlog value
    let backlog = if backlog == 0 {
        libc::SOMAXCONN
    } else {
        backlog.min(i32::MAX as u32) as libc::c_int
    };

    // resolve the socket family from the raw address metadata
    let family = address.family as libc::c_int;

    // create and bind a new listening socket
    with_socket_address_raw(address, |sockaddr, length| {
        let fd = unsafe { libc::socket(family, libc::SOCK_STREAM, libc::IPPROTO_TCP) };
        if fd < 0 {
            return Err(core_platform::net_error("socket"));
        }

        let enabled: libc::c_int = 1;
        let result = unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_REUSEADDR,
                &enabled as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        if result != 0 {
            unsafe {
                libc::close(fd);
            }
            return Err(core_platform::net_error("setsockopt"));
        }

        let result = unsafe { libc::bind(fd, sockaddr, length) };
        if result != 0 {
            unsafe {
                libc::close(fd);
            }
            return Err(core_platform::net_error("bind"));
        }

        let result = unsafe { libc::listen(fd, backlog) };
        if result != 0 {
            unsafe {
                libc::close(fd);
            }
            return Err(core_platform::net_error("listen"));
        }

        let entry = ResourceEntry::new(ResourceKind::Listener)
            .with_listener(fd)
            .with_finalizer(DescriptorFinalizer { fd });
        let resource_id =
            binding
                .worker()
                .resources
                .insert(&binding.world(), entry, Some(binding.engine()));
        unsafe {
            *out = ListenerHandle(resource_id);
        }

        Ok(())
    })
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

    // create the requested socket
    let fd = unsafe {
        libc::socket(
            socket_family_to_raw(family),
            socket_type.0 as libc::c_int,
            protocol.0 as libc::c_int,
        )
    };
    if fd < 0 {
        return Err(core_platform::net_error("socket"));
    }

    // register the socket handle
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(fd)
        .with_finalizer(DescriptorFinalizer { fd });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
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

    if family == SocketFamily::Unspecified {
        let mut pair = [0 as RawFd; 2];
        let result = unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                socket_type.0 as libc::c_int,
                protocol.0 as libc::c_int,
                pair.as_mut_ptr(),
            )
        };
        if result != 0 {
            return Err(core_platform::net_error("socketpair"));
        }

        register_socket_pair(binding, out, pair[0], pair[1]);

        return Ok(());
    }

    let family = socket_family_to_raw(family);
    let requested_type = socket_type.0 as libc::c_int;
    let base_type = requested_type & !(SOCKET_TYPE_FLAG_NONBLOCK | SOCKET_TYPE_FLAG_CLOEXEC);

    if base_type == libc::SOCK_STREAM {
        return socket_pair_stream_loopback(binding, out, family, requested_type, protocol.0);
    }

    if base_type == libc::SOCK_DGRAM {
        return socket_pair_dgram_loopback(binding, out, family, requested_type, protocol.0);
    }

    Err(RuntimeError::from(PlatformError::not_supported("destack.net.socketPair")).boxed())
}
