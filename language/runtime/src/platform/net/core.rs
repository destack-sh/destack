use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(unix)]
use crate::platform::fs::{OsPath, PathEncoding, core as core_fs};
use crate::platform::net::{
    KeepAliveConfig, Linger, ResolveFlags, SocketAddress, SocketFamily, SocketMessageFlags,
    SocketPair, SocketProtocol, SocketRecvFrom, SocketSendTo, SocketType, UdpMessageFlags,
    UdpReceive,
};
use crate::platform::resource::{ListenerHandle, ResourceEntry, ResourceKind, SocketHandle};
use crate::platform::{NativeArray, NativeStringRef, PlatformError, ResourceId};
use crate::runtime::RuntimeCallContext;

#[cfg(unix)]
use crate::platform::resource::ResourceFinalizer;
#[cfg(unix)]
use std::os::unix::io::RawFd;

/// Resolve a socket handle to its resource entry.
#[cfg_attr(not(any(unix, windows)), allow(dead_code))]
pub(crate) fn require_resource<T>(
    context: &RuntimeCallContext,
    id: ResourceId,
    kind: ResourceKind,
    label: &str,
    with_entry: impl FnOnce(&ResourceEntry) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let resolved = context
        .runtime()
        .resources
        .with_entry(id, |entry| {
            if entry.kind != kind {
                return None;
            }
            Some(with_entry(entry))
        })
        .flatten();

    match resolved {
        Some(Ok(value)) => Ok(value),
        Some(Err(error)) => Err(error),
        None => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            format!("unknown {label} handle"),
        ))
        .boxed()),
    }
}

/// Finalizer that closes a unix file descriptor.
#[cfg(unix)]
#[derive(Debug)]
struct DescriptorFinalizer {
    /// The descriptor to close.
    fd: RawFd,
}

#[cfg(unix)]
impl ResourceFinalizer for DescriptorFinalizer {
    /// Close the descriptor when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// Resolve a socket handle into a unix descriptor.
#[cfg(unix)]
fn socket_descriptor(context: &RuntimeCallContext, handle: SocketHandle) -> RuntimeResult<RawFd> {
    require_resource(context, handle.0, ResourceKind::Socket, "socket", |entry| {
        let Some(fd) = entry.fd() else {
            return Err(RuntimeError::Internal {
                message: "socket payload missing descriptor".to_string(),
            }
            .boxed());
        };

        Ok(fd)
    })
}

/// Convert a socket family enum into a platform address family.
#[cfg(unix)]
fn socket_family_to_raw(family: SocketFamily) -> libc::c_int {
    match family {
        SocketFamily::Unspecified => libc::AF_INET,
        SocketFamily::IPv4 => libc::AF_INET,
        SocketFamily::IPv6 => libc::AF_INET6,
    }
}

/// Convert a raw socket address payload into syscall arguments.
#[cfg(unix)]
fn with_socket_address_raw<T>(
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

/// Convert a raw sockaddr storage buffer into a `SocketAddress`.
#[cfg(unix)]
fn socket_address_raw_from_storage(
    context: &RuntimeCallContext,
    storage: &libc::sockaddr_storage,
    length: libc::socklen_t,
) -> RuntimeResult<SocketAddress> {
    // validate the reported length
    if length as usize > std::mem::size_of::<libc::sockaddr_storage>() {
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
    let bytes = context.store_array(bytes.to_vec());

    Ok(SocketAddress {
        family: storage.ss_family as u16,
        length,
        bytes,
    })
}

/// Read a boolean socket option.
#[cfg(unix)]
fn get_socket_bool(
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
        return Err(RuntimeError::from(PlatformError::io(format!("{syscall} failed"))).boxed());
    }

    Ok(value != 0)
}

/// Read a u32 socket option.
#[cfg(unix)]
fn get_socket_u32(
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
        return Err(RuntimeError::from(PlatformError::io(format!("{syscall} failed"))).boxed());
    }

    Ok(value as u32)
}

/// Connect an existing socket to a raw remote address.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_connect_raw(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let fd = socket_descriptor(context, handle)?;

    // connect using the provided raw sockaddr
    with_socket_address_raw(address, |sockaddr, length| {
        let result = unsafe { libc::connect(fd, sockaddr, length) };
        if result != 0 {
            return Err(
                RuntimeError::from(PlatformError::io("connect failed".to_string())).boxed(),
            );
        }

        Ok(())
    })
}

/// Connect an existing socket to a raw remote address.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_connect_raw(
    _context: &RuntimeCallContext,
    _handle: SocketHandle,
    _address: SocketAddress,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.connect")).boxed())
}

/// Bind an existing socket to a raw local address.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_bind(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let fd = socket_descriptor(context, handle)?;

    // bind using the provided raw sockaddr
    with_socket_address_raw(address, |sockaddr, length| {
        let result = unsafe { libc::bind(fd, sockaddr, length) };
        if result != 0 {
            return Err(RuntimeError::from(PlatformError::io("bind failed".to_string())).boxed());
        }

        Ok(())
    })
}

/// Bind an existing socket to a raw local address.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_bind(
    _context: &RuntimeCallContext,
    _handle: SocketHandle,
    _address: SocketAddress,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.bind")).boxed())
}

/// Start listening on a raw local socket address.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_listen_raw(
    context: &RuntimeCallContext,
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
            let error = std::io::Error::last_os_error();
            return Err(
                RuntimeError::from(PlatformError::io(format!("socket failed: {error}"))).boxed(),
            );
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
            let error = std::io::Error::last_os_error();
            return Err(RuntimeError::from(PlatformError::io(format!(
                "setsockopt failed: {error}"
            )))
            .boxed());
        }

        let result = unsafe { libc::bind(fd, sockaddr, length) };
        if result != 0 {
            unsafe {
                libc::close(fd);
            }
            let error = std::io::Error::last_os_error();
            return Err(
                RuntimeError::from(PlatformError::io(format!("bind failed: {error}"))).boxed(),
            );
        }

        let result = unsafe { libc::listen(fd, backlog) };
        if result != 0 {
            unsafe {
                libc::close(fd);
            }
            let error = std::io::Error::last_os_error();
            return Err(
                RuntimeError::from(PlatformError::io(format!("listen failed: {error}"))).boxed(),
            );
        }

        let entry = ResourceEntry::new(ResourceKind::Listener)
            .with_listener(fd)
            .with_finalizer(DescriptorFinalizer { fd });
        let resource_id = context.runtime().resources.insert(entry);
        unsafe {
            *out = ListenerHandle(resource_id);
        }

        Ok(())
    })
}

/// Start listening on a raw local socket address.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_listen_raw(
    _context: &RuntimeCallContext,
    _out: *mut ListenerHandle,
    _address: SocketAddress,
    _backlog: u32,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.listen")).boxed())
}

/// Create a socket from family, type, and protocol.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_socket(
    context: &RuntimeCallContext,
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
        return Err(RuntimeError::from(PlatformError::io("socket failed".to_string())).boxed());
    }

    // register the socket handle
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(fd)
        .with_finalizer(DescriptorFinalizer { fd });
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = SocketHandle(resource_id);
    }

    Ok(())
}

/// Create a socket from family, type, and protocol.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_socket(
    _context: &RuntimeCallContext,
    _out: *mut SocketHandle,
    _family: SocketFamily,
    _socket_type: SocketType,
    _protocol: SocketProtocol,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.socket")).boxed())
}

/// Create a connected socket pair.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_socket_pair(
    context: &RuntimeCallContext,
    out: *mut SocketPair,
    family: SocketFamily,
    socket_type: SocketType,
    protocol: SocketProtocol,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // map the family to socketpair-compatible values
    let family = match family {
        SocketFamily::Unspecified => libc::AF_UNIX,
        SocketFamily::IPv4 | SocketFamily::IPv6 => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "family",
                "socketPair requires SocketFamily.Unspecified",
            ))
            .boxed());
        }
    };

    // create the pair
    let mut pair = [0 as RawFd; 2];
    let result = unsafe {
        libc::socketpair(
            family,
            socket_type.0 as libc::c_int,
            protocol.0 as libc::c_int,
            pair.as_mut_ptr(),
        )
    };
    if result != 0 {
        return Err(RuntimeError::from(PlatformError::io("socketpair failed".to_string())).boxed());
    }

    // register both sockets
    let first_entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(pair[0])
        .with_finalizer(DescriptorFinalizer { fd: pair[0] });
    let first_id = context.runtime().resources.insert(first_entry);

    let second_entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(pair[1])
        .with_finalizer(DescriptorFinalizer { fd: pair[1] });
    let second_id = context.runtime().resources.insert(second_entry);

    unsafe {
        *out = SocketPair {
            first: SocketHandle(first_id),
            second: SocketHandle(second_id),
        };
    }

    Ok(())
}

/// Create a connected socket pair.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_socket_pair(
    _context: &RuntimeCallContext,
    _out: *mut SocketPair,
    _family: SocketFamily,
    _socket_type: SocketType,
    _protocol: SocketProtocol,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.socketPair")).boxed())
}

/// Receive a packet from a socket with raw sender metadata.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_recv_from(
    context: &RuntimeCallContext,
    out: *mut SocketRecvFrom,
    handle: SocketHandle,
    buffer: crate::platform::NativeSlice<u8>,
    recv_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve runtime values
    let fd = socket_descriptor(context, handle)?;
    let buffer = unsafe { buffer.as_mut_slice()? };
    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    let mut length = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

    // receive the datagram and source address
    let bytes = unsafe {
        libc::recvfrom(
            fd,
            buffer.as_mut_ptr() as *mut libc::c_void,
            buffer.len(),
            recv_flags.0 as libc::c_int,
            &mut storage as *mut _ as *mut libc::sockaddr,
            &mut length,
        )
    };
    if bytes < 0 {
        return Err(RuntimeError::from(PlatformError::io("recvfrom failed".to_string())).boxed());
    }

    // encode the source address and output payload
    let address = socket_address_raw_from_storage(context, &storage, length)?;
    unsafe {
        *out = SocketRecvFrom {
            bytes: bytes as u64,
            address,
            recv_flags: SocketMessageFlags(0),
        };
    }

    Ok(())
}

/// Receive a packet from a socket with raw sender metadata.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_recv_from(
    _context: &RuntimeCallContext,
    _out: *mut SocketRecvFrom,
    _handle: SocketHandle,
    _buffer: crate::platform::NativeSlice<u8>,
    _recv_flags: SocketMessageFlags,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.recvFrom")).boxed())
}

/// Send a packet to a raw destination address.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_send_to(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    buffer: crate::platform::NativeSlice<u8>,
    message: SocketSendTo,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve runtime values
    let fd = socket_descriptor(context, handle)?;
    let buffer = unsafe { buffer.as_slice()? };

    // send the datagram to the raw destination
    with_socket_address_raw(message.address, |sockaddr, length| {
        let bytes = unsafe {
            libc::sendto(
                fd,
                buffer.as_ptr() as *const libc::c_void,
                buffer.len(),
                message.flags.0 as libc::c_int,
                sockaddr,
                length,
            )
        };
        if bytes < 0 {
            return Err(RuntimeError::from(PlatformError::io("sendto failed".to_string())).boxed());
        }

        unsafe {
            *out = bytes as u64;
        }

        Ok(())
    })
}

/// Send a packet to a raw destination address.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_send_to(
    _context: &RuntimeCallContext,
    _out: *mut u64,
    _handle: SocketHandle,
    _buffer: crate::platform::NativeSlice<u8>,
    _message: SocketSendTo,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.sendTo")).boxed())
}

/// Resolve host and port into raw socket addresses.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_resolve_raw(
    context: &RuntimeCallContext,
    out: *mut NativeArray<SocketAddress>,
    host: NativeStringRef,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<()> {
    unsafe { super::os::destack_net_resolve(context, out, host, port, family, flags) }
}

/// Resolve host and port into raw socket addresses.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_resolve_raw(
    _context: &RuntimeCallContext,
    _out: *mut NativeArray<SocketAddress>,
    _host: NativeStringRef,
    _port: u16,
    _family: SocketFamily,
    _flags: ResolveFlags,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.resolve")).boxed())
}

/// Reverse lookup a raw socket address into hostnames.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_reverse_lookup_raw(
    context: &RuntimeCallContext,
    out: *mut NativeArray<NativeStringRef>,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { super::os::destack_net_reverse_lookup(context, out, address) }
}

/// Reverse lookup a raw socket address into hostnames.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_reverse_lookup_raw(
    _context: &RuntimeCallContext,
    _out: *mut NativeArray<NativeStringRef>,
    _address: SocketAddress,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.reverseLookup")).boxed())
}

/// Bind a UDP socket to a raw local address.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_udp_bind_raw(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let fd = socket_descriptor(context, handle)?;

    // bind using the provided raw sockaddr
    with_socket_address_raw(address, |sockaddr, length| {
        let result = unsafe { libc::bind(fd, sockaddr, length) };
        if result != 0 {
            return Err(RuntimeError::from(PlatformError::io("bind failed".to_string())).boxed());
        }

        Ok(())
    })
}

/// Bind a UDP socket to a raw local address.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_udp_bind_raw(
    _context: &RuntimeCallContext,
    _handle: SocketHandle,
    _address: SocketAddress,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpBind")).boxed())
}

/// Connect a UDP socket to a raw remote address.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_udp_connect_raw(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let fd = socket_descriptor(context, handle)?;

    // connect using the provided raw sockaddr
    with_socket_address_raw(address, |sockaddr, length| {
        let result = unsafe { libc::connect(fd, sockaddr, length) };
        if result != 0 {
            return Err(
                RuntimeError::from(PlatformError::io("connect failed".to_string())).boxed(),
            );
        }

        Ok(())
    })
}

/// Connect a UDP socket to a raw remote address.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_udp_connect_raw(
    _context: &RuntimeCallContext,
    _handle: SocketHandle,
    _address: SocketAddress,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpConnect")).boxed())
}

/// Receive a UDP datagram with raw sender metadata.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_udp_recv_from_raw(
    context: &RuntimeCallContext,
    out: *mut UdpReceive,
    handle: SocketHandle,
    buffer: crate::platform::NativeSlice<u8>,
    recv_flags: UdpMessageFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve runtime values
    let fd = socket_descriptor(context, handle)?;
    let buffer = unsafe { buffer.as_mut_slice()? };
    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    let mut length = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;

    // receive the datagram
    let bytes = unsafe {
        libc::recvfrom(
            fd,
            buffer.as_mut_ptr() as *mut libc::c_void,
            buffer.len(),
            recv_flags.0 as libc::c_int,
            &mut storage as *mut _ as *mut libc::sockaddr,
            &mut length,
        )
    };
    if bytes < 0 {
        return Err(RuntimeError::from(PlatformError::io("recvfrom failed".to_string())).boxed());
    }

    // encode the sender address and payload
    let address = socket_address_raw_from_storage(context, &storage, length)?;
    unsafe {
        *out = UdpReceive {
            address,
            bytes: bytes as u64,
            recv_flags: UdpMessageFlags(0),
        };
    }

    Ok(())
}

/// Receive a UDP datagram with raw sender metadata.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_udp_recv_from_raw(
    _context: &RuntimeCallContext,
    _out: *mut UdpReceive,
    _handle: SocketHandle,
    _buffer: crate::platform::NativeSlice<u8>,
    _recv_flags: UdpMessageFlags,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpRecvFrom")).boxed())
}

/// Send a UDP datagram to a raw destination address.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_udp_send_to_raw(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: SocketHandle,
    address: SocketAddress,
    buffer: crate::platform::NativeSlice<u8>,
    send_flags: UdpMessageFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve runtime values
    let fd = socket_descriptor(context, handle)?;
    let buffer = unsafe { buffer.as_slice()? };

    // send the datagram
    with_socket_address_raw(address, |sockaddr, length| {
        let bytes = unsafe {
            libc::sendto(
                fd,
                buffer.as_ptr() as *const libc::c_void,
                buffer.len(),
                send_flags.0 as libc::c_int,
                sockaddr,
                length,
            )
        };
        if bytes < 0 {
            return Err(RuntimeError::from(PlatformError::io("sendto failed".to_string())).boxed());
        }

        unsafe {
            *out = bytes as u64;
        }

        Ok(())
    })
}

/// Send a UDP datagram to a raw destination address.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_udp_send_to_raw(
    _context: &RuntimeCallContext,
    _out: *mut u64,
    _handle: SocketHandle,
    _address: SocketAddress,
    _buffer: crate::platform::NativeSlice<u8>,
    _send_flags: UdpMessageFlags,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udpSendTo")).boxed())
}

/// Create a unix domain socket pair.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_uds_socket_pair(
    context: &RuntimeCallContext,
    out: *mut SocketPair,
    socket_type: SocketType,
) -> RuntimeResult<()> {
    unsafe {
        destack_net_socket_pair(
            context,
            out,
            SocketFamily::Unspecified,
            socket_type,
            SocketProtocol(0),
        )
    }
}

/// Create a unix domain socket pair.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_uds_socket_pair(
    _context: &RuntimeCallContext,
    _out: *mut SocketPair,
    _socket_type: SocketType,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.udsSocketPair")).boxed())
}

/// Set or clear the IPv6-only socket mode.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_set_only_v6(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let fd = socket_descriptor(context, handle)?;

    // apply IPV6_V6ONLY
    let value: libc::c_int = if enabled { 1 } else { 0 };
    let result = unsafe {
        libc::setsockopt(
            fd,
            libc::IPPROTO_IPV6,
            libc::IPV6_V6ONLY,
            &value as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        )
    };
    if result != 0 {
        return Err(RuntimeError::from(PlatformError::io("setsockopt failed".to_string())).boxed());
    }

    Ok(())
}

/// Set or clear the IPv6-only socket mode.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_set_only_v6(
    _context: &RuntimeCallContext,
    _handle: SocketHandle,
    _enabled: bool,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setOnlyV6")).boxed())
}

/// Read TCP keepalive settings.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_keep_alive(
    context: &RuntimeCallContext,
    out: *mut KeepAliveConfig,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read keepalive state
    let fd = socket_descriptor(context, handle)?;
    let enabled = get_socket_bool(fd, libc::SOL_SOCKET, libc::SO_KEEPALIVE, "getsockopt")?;

    // select the platform idle option
    #[cfg(any(target_os = "linux", target_os = "android"))]
    let idle_option = libc::TCP_KEEPIDLE;
    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    let idle_option = libc::TCP_KEEPALIVE;

    // read timing values
    let idle_seconds = get_socket_u32(fd, libc::IPPROTO_TCP, idle_option, "getsockopt")?;
    let interval_seconds =
        get_socket_u32(fd, libc::IPPROTO_TCP, libc::TCP_KEEPINTVL, "getsockopt")?;
    let probe_count = get_socket_u32(fd, libc::IPPROTO_TCP, libc::TCP_KEEPCNT, "getsockopt")?;
    unsafe {
        *out = KeepAliveConfig {
            enabled,
            idle_seconds,
            interval_seconds,
            probe_count,
        };
    }

    Ok(())
}

/// Read TCP keepalive settings.
#[cfg(windows)]
pub(crate) unsafe fn destack_net_get_keep_alive(
    context: &RuntimeCallContext,
    out: *mut KeepAliveConfig,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    unsafe { super::os::destack_net_get_keep_alive(context, out, handle) }
}

/// Read TCP keepalive settings.
#[cfg(not(any(unix, windows)))]
pub(crate) unsafe fn destack_net_get_keep_alive(
    _context: &RuntimeCallContext,
    _out: *mut KeepAliveConfig,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getKeepAlive")).boxed())
}

/// Read TCP_NODELAY.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_no_delay(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(context, handle)?;
    let value = get_socket_bool(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, "getsockopt")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read TCP_NODELAY.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_no_delay(
    _context: &RuntimeCallContext,
    _out: *mut bool,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getNoDelay")).boxed())
}

/// Read SO_REUSEADDR.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_reuse_addr(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(context, handle)?;
    let value = get_socket_bool(fd, libc::SOL_SOCKET, libc::SO_REUSEADDR, "getsockopt")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read SO_REUSEADDR.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_reuse_addr(
    _context: &RuntimeCallContext,
    _out: *mut bool,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getReuseAddr")).boxed())
}

/// Read SO_REUSEPORT.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_reuse_port(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(context, handle)?;
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    ))]
    {
        let value = get_socket_bool(fd, libc::SOL_SOCKET, libc::SO_REUSEPORT, "getsockopt")?;
        unsafe {
            *out = value;
        }
        Ok(())
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    )))]
    {
        let _ = fd;
        Err(RuntimeError::from(PlatformError::not_supported("destack.net.getReusePort")).boxed())
    }
}

/// Read SO_REUSEPORT.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_reuse_port(
    _context: &RuntimeCallContext,
    _out: *mut bool,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getReusePort")).boxed())
}

/// Read socket linger settings.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_linger(
    context: &RuntimeCallContext,
    out: *mut Linger,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(context, handle)?;
    let mut value = libc::linger {
        l_onoff: 0,
        l_linger: 0,
    };
    let mut length = std::mem::size_of::<libc::linger>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_LINGER,
            &mut value as *mut _ as *mut libc::c_void,
            &mut length,
        )
    };
    if result != 0 {
        return Err(RuntimeError::from(PlatformError::io("getsockopt failed".to_string())).boxed());
    }

    unsafe {
        *out = Linger {
            enabled: value.l_onoff != 0,
            seconds: value.l_linger as u32,
        };
    }

    Ok(())
}

/// Read socket linger settings.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_linger(
    _context: &RuntimeCallContext,
    _out: *mut Linger,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getLinger")).boxed())
}

/// Read receive buffer size.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_recv_buffer(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(context, handle)?;
    let value = get_socket_u32(fd, libc::SOL_SOCKET, libc::SO_RCVBUF, "getsockopt")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read receive buffer size.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_recv_buffer(
    _context: &RuntimeCallContext,
    _out: *mut u32,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getRecvBuffer")).boxed())
}

/// Read send buffer size.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_send_buffer(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(context, handle)?;
    let value = get_socket_u32(fd, libc::SOL_SOCKET, libc::SO_SNDBUF, "getsockopt")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read send buffer size.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_send_buffer(
    _context: &RuntimeCallContext,
    _out: *mut u32,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getSendBuffer")).boxed())
}

/// Read broadcast mode.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_broadcast(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(context, handle)?;
    let value = get_socket_bool(fd, libc::SOL_SOCKET, libc::SO_BROADCAST, "getsockopt")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read broadcast mode.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_broadcast(
    _context: &RuntimeCallContext,
    _out: *mut bool,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getBroadcast")).boxed())
}

/// Read IP time to live.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_ttl(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(context, handle)?;
    let value = get_socket_u32(fd, libc::IPPROTO_IP, libc::IP_TTL, "getsockopt")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read IP time to live.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_ttl(
    _context: &RuntimeCallContext,
    _out: *mut u32,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getTtl")).boxed())
}

/// Read IP type of service.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_tos(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(context, handle)?;
    let value = get_socket_u32(fd, libc::IPPROTO_IP, libc::IP_TOS, "getsockopt")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read IP type of service.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_tos(
    _context: &RuntimeCallContext,
    _out: *mut u32,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getTos")).boxed())
}

/// Read read timeout in milliseconds.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_read_timeout(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(context, handle)?;
    let mut timeout = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut length = std::mem::size_of::<libc::timeval>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            &mut timeout as *mut _ as *mut libc::c_void,
            &mut length,
        )
    };
    if result != 0 {
        return Err(RuntimeError::from(PlatformError::io("getsockopt failed".to_string())).boxed());
    }

    let seconds = (timeout.tv_sec as u64).saturating_mul(1_000);
    let millis = (timeout.tv_usec as u64) / 1_000;
    unsafe {
        *out = (seconds.saturating_add(millis)).min(u32::MAX as u64) as u32;
    }

    Ok(())
}

/// Read read timeout in milliseconds.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_read_timeout(
    _context: &RuntimeCallContext,
    _out: *mut u32,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getReadTimeout")).boxed())
}

/// Read write timeout in milliseconds.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_write_timeout(
    context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(context, handle)?;
    let mut timeout = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut length = std::mem::size_of::<libc::timeval>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_SNDTIMEO,
            &mut timeout as *mut _ as *mut libc::c_void,
            &mut length,
        )
    };
    if result != 0 {
        return Err(RuntimeError::from(PlatformError::io("getsockopt failed".to_string())).boxed());
    }

    let seconds = (timeout.tv_sec as u64).saturating_mul(1_000);
    let millis = (timeout.tv_usec as u64) / 1_000;
    unsafe {
        *out = (seconds.saturating_add(millis)).min(u32::MAX as u64) as u32;
    }

    Ok(())
}

/// Read write timeout in milliseconds.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_write_timeout(
    _context: &RuntimeCallContext,
    _out: *mut u32,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getWriteTimeout")).boxed())
}

/// Read IPv6-only mode.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_only_v6(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(context, handle)?;
    let value = get_socket_bool(fd, libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, "getsockopt")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read IPv6-only mode.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_only_v6(
    _context: &RuntimeCallContext,
    _out: *mut bool,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getOnlyV6")).boxed())
}

/// Resolve a `OsPath` into byte path data on unix platforms.
#[cfg(unix)]
pub(crate) fn unix_path_bytes(path: OsPath, label: &str) -> RuntimeResult<Vec<u8>> {
    // decode byte paths directly
    if path.encoding == PathEncoding::Bytes {
        let bytes = unsafe { path.data.0.as_slice()? };

        return Ok(bytes.to_vec());
    }

    // decode utf16 paths and transcode to bytes
    core_fs::utf16_path_to_utf8_bytes(path.data, label)
}

/// Core networking interface exposed to bindings.
/// This provides a stable entrypoint that selects the active OS backend.
pub(crate) use super::os::*;
