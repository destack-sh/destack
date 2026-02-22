#![allow(unused_imports)]

use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::OsPath;
use crate::platform::net::{core as core_net, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

use std::ffi::{CStr, CString};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::unix::io::RawFd;

/// Host IPv6 multicast interface selector type.
#[cfg(target_os = "android")]
type Ipv6MulticastInterface = libc::c_int;
/// Host IPv6 multicast interface selector type.
#[cfg(not(target_os = "android"))]
type Ipv6MulticastInterface = libc::c_uint;

/// Convert one logical interface index into one host socket-option interface selector.
fn ipv6_multicast_interface(interface_index: u32) -> RuntimeResult<Ipv6MulticastInterface> {
    #[cfg(target_os = "android")]
    {
        libc::c_int::try_from(interface_index).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "interfaceIndex",
                "interface index must fit one platform integer",
            ))
            .boxed()
        })
    }

    #[cfg(not(target_os = "android"))]
    {
        Ok(interface_index)
    }
}

/// Enable or disable nonblocking mode on a socket.
///
/// Enable or disable nonblocking mode on a socket descriptor.
/// Subsequent I/O blocking behavior follows host kernel descriptor state.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses fcntl/ioctl on Unix and ioctlsocket on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_nonblocking(
    context: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // update nonblocking mode on unix platforms
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

/// Enable or disable TCP_NODELAY.
///
/// Set TCP_NODELAY on the target TCP socket.
/// Nagle aggregation behavior changes immediately according to host TCP stack semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(TCP_NODELAY) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_no_delay(
    context: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    set_socket_bool(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, enabled)
}

/// Write full TCP keepalive parameters.
///
/// Set host keepalive configuration fields for this TCP socket.
/// Unsupported subfields are returned as `notSupported` rather than silently ignored.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_KEEPALIVE and TCP_KEEP*) on Unix and WSAIoctl on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_keep_alive(
    context: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
    idle_seconds: u32,
    interval_seconds: u32,
    probe_count: u32,
) -> RuntimeResult<()> {
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

/// Enable or disable SO_REUSEADDR.
///
/// Enable or disable SO_REUSEADDR via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_REUSEADDR) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_reuse_addr(
    context: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    set_socket_bool(fd, libc::SOL_SOCKET, libc::SO_REUSEADDR, enabled)
}

/// Enable or disable SO_REUSEPORT.
///
/// Enable or disable SO_REUSEPORT via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix only. This operation returns `notSupported` on Windows.
/// Uses setsockopt(SO_REUSEPORT) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_reuse_port(
    context: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    os::set_reuse_port(fd, enabled)
}

/// Set socket linger settings.
///
/// Set SO_LINGER parameters on the target socket.
/// The host kernel validates and applies linger policy exactly once for this call.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_LINGER) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_linger(
    context: &BindingCallContext,
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
///
/// Set SO_RCVBUF on the target socket.
/// The host kernel clamps requested values according to platform limits.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_RCVBUF) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_recv_buffer(
    context: &BindingCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    set_socket_u32(fd, libc::SOL_SOCKET, libc::SO_RCVBUF, size)
}

/// Set the send buffer size.
///
/// Set SO_SNDBUF on the target socket.
/// The host kernel clamps requested values according to platform limits.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_SNDBUF) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_send_buffer(
    context: &BindingCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    set_socket_u32(fd, libc::SOL_SOCKET, libc::SO_SNDBUF, size)
}

/// Enable or disable broadcast.
///
/// Enable or disable broadcast via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_BROADCAST) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_broadcast(
    context: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    set_socket_bool(fd, libc::SOL_SOCKET, libc::SO_BROADCAST, enabled)
}

/// Join an IPv4 multicast group.
///
/// Join an IPv4 multicast membership on the selected interface.
/// Group membership tracking and validation follow host IGMP implementation rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_ADD_MEMBERSHIP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_join_multicast_v4(
    context: &BindingCallContext,
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
///
/// Leave an IPv4 multicast membership on the selected interface.
/// Group membership tracking and validation follow host IGMP implementation rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_DROP_MEMBERSHIP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_leave_multicast_v4(
    context: &BindingCallContext,
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
///
/// Join an IPv6 multicast membership on the selected interface index.
/// Group membership tracking and validation follow host MLD implementation rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IPV6_JOIN_GROUP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_join_multicast_v6(
    context: &BindingCallContext,
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
    let interface_index = ipv6_multicast_interface(interface_index)?;
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
///
/// Leave an IPv6 multicast membership on the selected interface index.
/// Group membership tracking and validation follow host MLD implementation rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IPV6_LEAVE_GROUP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_leave_multicast_v6(
    context: &BindingCallContext,
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
    let interface_index = ipv6_multicast_interface(interface_index)?;
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
///
/// Enable or disable local loopback delivery for outgoing multicast packets.
/// Loopback behavior follows host multicast socket-option semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_MULTICAST_LOOP/IPV6_MULTICAST_LOOP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_multicast_loop(
    context: &BindingCallContext,
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
///
/// Set multicast TTL or hop-limit for outgoing datagrams.
/// Hop-limit interpretation follows host IP stack option semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_MULTICAST_TTL/IPV6_MULTICAST_HOPS) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_multicast_ttl(
    context: &BindingCallContext,
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
///
/// Set the requested control value on the descriptor through the native option interface.
/// The binding performs one control transaction and returns the exact host outcome without policy retries.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_TTL/IPV6_UNICAST_HOPS) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_ttl(
    context: &BindingCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    set_socket_u32(fd, libc::IPPROTO_IP, libc::IP_TTL, ttl)
}

/// Set the IP type-of-service field.
///
/// Set the requested control value on the descriptor through the native option interface.
/// The binding performs one control transaction and returns the exact host outcome without policy retries.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_TOS/IPV6_TCLASS) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_tos(
    context: &BindingCallContext,
    handle: SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(context, handle)?;
    set_socket_u32(fd, libc::IPPROTO_IP, libc::IP_TOS, tos)
}

/// Set the read timeout in milliseconds.
///
/// Set SO_RCVTIMEO on the target socket.
/// Timeout interpretation and rounding follow host kernel socket-timeout semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_RCVTIMEO) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_read_timeout(
    context: &BindingCallContext,
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
///
/// Set SO_SNDTIMEO on the target socket.
/// Timeout interpretation and rounding follow host kernel socket-timeout semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(SO_SNDTIMEO) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_write_timeout(
    context: &BindingCallContext,
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

/// Restrict an IPv6 socket to IPv6 traffic only.
///
/// Set IPV6_V6ONLY on the target socket.
/// Dual-stack behavior follows host kernel policy after this option is applied.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IPV6_V6ONLY) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_set_only_v6(
    context: &BindingCallContext,
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

/// Read full TCP keepalive parameters.
///
/// Read host keepalive configuration fields for this TCP socket.
/// Field units and defaults follow platform TCP stack semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_KEEPALIVE and TCP_KEEP*) on Unix and WSAIoctl on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_keep_alive(
    context: &BindingCallContext,
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

/// Read TCP_NODELAY.
///
/// Read the current TCP_NODELAY setting from the host TCP option layer.
/// The returned boolean reflects whether Nagle aggregation is disabled for this socket.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(TCP_NODELAY) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_no_delay(
    context: &BindingCallContext,
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

/// Read SO_REUSEADDR.
///
/// Read the current SO_REUSEADDR value from the host socket option layer.
/// The returned boolean reflects host option state at the instant of the call.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_REUSEADDR) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_reuse_addr(
    context: &BindingCallContext,
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

/// Read SO_REUSEPORT.
///
/// Read the current SO_REUSEPORT value from the host socket option layer.
/// The returned boolean reflects host option state at the instant of the call.
///
/// # Platform
/// Unix only. This operation returns `notSupported` on Windows.
/// Uses getsockopt(SO_REUSEPORT) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_reuse_port(
    context: &BindingCallContext,
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

/// Read socket linger settings.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_LINGER) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_linger(
    context: &BindingCallContext,
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

/// Read the receive buffer size.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_RCVBUF) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_recv_buffer(
    context: &BindingCallContext,
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

/// Read the send buffer size.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_SNDBUF) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_send_buffer(
    context: &BindingCallContext,
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

/// Read broadcast mode.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_BROADCAST) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_broadcast(
    context: &BindingCallContext,
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

/// Read the IP time-to-live.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IP_TTL/IPV6_UNICAST_HOPS) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_ttl(
    context: &BindingCallContext,
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

/// Read the IP type-of-service field.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IP_TOS/IPV6_TCLASS) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_tos(
    context: &BindingCallContext,
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

/// Read the read timeout in milliseconds.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_RCVTIMEO) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_read_timeout(
    context: &BindingCallContext,
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

/// Read the write timeout in milliseconds.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(SO_SNDTIMEO) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_write_timeout(
    context: &BindingCallContext,
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

/// Read IPv6-only mode.
///
/// Read the current option value from the host socket option layer.
/// Returned units and ranges follow host option semantics for the active platform.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IPV6_V6ONLY) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_get_only_v6(
    context: &BindingCallContext,
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
