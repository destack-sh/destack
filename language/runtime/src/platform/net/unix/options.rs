use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::net::*;
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

use std::net::{Ipv4Addr, Ipv6Addr};

/// Host IPv6 multicast interface selector type.
#[cfg(target_os = "android")]
type Ipv6MulticastInterface = libc::c_int;
/// Host IPv6 multicast interface selector type.
#[cfg(not(target_os = "android"))]
type Ipv6MulticastInterface = libc::c_uint;

/// Linux `group_source_req` payload for IPv6 source-specific multicast control.
#[cfg(target_os = "linux")]
#[repr(C)]
struct LinuxGroupSourceRequest {
    /// Interface index for the membership operation.
    gsr_interface: libc::c_uint,
    /// Multicast group sockaddr payload.
    gsr_group: libc::sockaddr_storage,
    /// Source sockaddr payload.
    gsr_source: libc::sockaddr_storage,
}

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

/// Convert one host timeval timeout payload into milliseconds.
fn timeout_millis_from_timeval(timeout: libc::timeval) -> u32 {
    // convert seconds and micros with signed-host safety
    let seconds = u64::try_from(timeout.tv_sec)
        .unwrap_or(0)
        .saturating_mul(1_000);
    let millis = u64::try_from(timeout.tv_usec).unwrap_or(0) / 1_000;

    // clamp to binding width
    seconds.saturating_add(millis).min(u32::MAX as u64) as u32
}

/// Build one IPv6 sockaddr-storage payload for multicast source filtering.
#[cfg(target_os = "linux")]
fn ipv6_sockaddr_storage(address: Ipv6Addr) -> libc::sockaddr_storage {
    // encode one sockaddr_in6 payload first
    let socket_address = libc::sockaddr_in6 {
        sin6_family: libc::AF_INET6 as libc::sa_family_t,
        sin6_port: 0,
        sin6_flowinfo: 0,
        sin6_addr: libc::in6_addr {
            s6_addr: address.octets(),
        },
        sin6_scope_id: 0,
    };

    // copy sockaddr bytes into one generic storage payload
    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &socket_address as *const _ as *const u8,
            &mut storage as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in6>(),
        );
    }

    storage
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // update nonblocking mode on unix platforms
    // resolve the socket descriptor
    let fd = socket_descriptor(binding, handle)?;

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
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
    idle_seconds: u32,
    interval_seconds: u32,
    probe_count: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(binding, handle)?;

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
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    linger: Linger,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve socket descriptor
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve socket descriptor
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_index: u32,
) -> RuntimeResult<()> {
    // resolve socket descriptor
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_index: u32,
) -> RuntimeResult<()> {
    // resolve socket descriptor
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve socket metadata
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    // resolve socket metadata
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(binding, handle)?;
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    let fd = socket_descriptor(binding, handle)?;
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
pub(crate) unsafe fn destack_net_set_only_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let fd = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error("setsockopt"));
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
pub(crate) unsafe fn destack_net_get_keep_alive(
    binding: &BindingCallContext,
    out: *mut KeepAliveConfig,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read keepalive state
    let fd = socket_descriptor(binding, handle)?;
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
pub(crate) unsafe fn destack_net_get_no_delay(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(binding, handle)?;
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
pub(crate) unsafe fn destack_net_get_reuse_addr(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(binding, handle)?;
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
pub(crate) unsafe fn destack_net_get_reuse_port(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(binding, handle)?;
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
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
        target_os = "ios",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
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
pub(crate) unsafe fn destack_net_get_linger(
    binding: &BindingCallContext,
    out: *mut Linger,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(binding, handle)?;
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
        return Err(core_platform::net_error("getsockopt"));
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
pub(crate) unsafe fn destack_net_get_recv_buffer(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(binding, handle)?;
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
pub(crate) unsafe fn destack_net_get_send_buffer(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(binding, handle)?;
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
pub(crate) unsafe fn destack_net_get_broadcast(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(binding, handle)?;
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
pub(crate) unsafe fn destack_net_get_ttl(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(binding, handle)?;
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
pub(crate) unsafe fn destack_net_get_tos(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(binding, handle)?;
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
pub(crate) unsafe fn destack_net_get_read_timeout(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(binding, handle)?;
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
        return Err(core_platform::net_error("getsockopt"));
    }

    unsafe {
        *out = timeout_millis_from_timeval(timeout);
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
pub(crate) unsafe fn destack_net_get_write_timeout(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(binding, handle)?;
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
        return Err(core_platform::net_error("getsockopt"));
    }

    unsafe {
        *out = timeout_millis_from_timeval(timeout);
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
pub(crate) unsafe fn destack_net_get_only_v6(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let fd = socket_descriptor(binding, handle)?;
    let value = get_socket_bool(fd, libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, "getsockopt")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Set one raw socket option payload.
///
/// Set one host socket option using raw level, name, and byte payload.
/// This escape hatch covers options that do not yet have typed bindings.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_sock_opt_raw(
    binding: &BindingCallContext,
    handle: SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // resolve socket descriptor and value payload
    let fd = socket_descriptor(binding, handle)?;
    let value = unsafe { value.as_slice()? };

    // apply the host socket option payload
    let result = unsafe {
        libc::setsockopt(
            fd,
            level.0 as libc::c_int,
            name.0 as libc::c_int,
            value.as_ptr() as *const libc::c_void,
            value.len() as libc::socklen_t,
        )
    };
    if result != 0 {
        return Err(core_platform::net_error("setsockopt"));
    }

    Ok(())
}

/// Read one raw socket option payload.
///
/// Read one host socket option using raw level and name.
/// The returned byte payload is host-defined and must be decoded by the caller.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_sock_opt_raw(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    maxbytes: u32,
) -> RuntimeResult<()> {
    // validate output pointers and requested output size
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    if maxbytes == 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "maxBytes",
            "max bytes must be greater than zero",
        ))
        .boxed());
    }

    // allocate one buffer and read the host socket option payload
    let fd = socket_descriptor(binding, handle)?;
    let mut value = vec![0u8; maxbytes as usize];
    let mut length = value.len() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            fd,
            level.0 as libc::c_int,
            name.0 as libc::c_int,
            value.as_mut_ptr() as *mut libc::c_void,
            &mut length,
        )
    };
    if result != 0 {
        return Err(core_platform::net_error("getsockopt"));
    }
    value.truncate(length as usize);

    // write the output payload
    unsafe {
        *out = binding.store_array(value);
    }

    Ok(())
}

/// Set packet timestamping mode.
///
/// Configure timestamping controls on one socket endpoint.
/// Timestamp delivery channel and precision follow host kernel capabilities.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses SO_TIMESTAMP families on Unix and host timestamping controls on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_timestamping(
    binding: &BindingCallContext,
    handle: SocketHandle,
    mode: SocketTimestampingMode,
) -> RuntimeResult<()> {
    // reject non-linux targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        // reject hardware mode on unix targets without SO_TIMESTAMPING support
        if mode == SocketTimestampingMode::Hardware {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.net.setTimestamping",
            ))
            .boxed());
        }

        // resolve the descriptor and map mode into SO_TIMESTAMP
        let fd = socket_descriptor(binding, handle)?;
        let enabled = mode == SocketTimestampingMode::Software;
        set_socket_bool(fd, libc::SOL_SOCKET, libc::SO_TIMESTAMP, enabled)
    }

    #[cfg(target_os = "linux")]
    {
        // resolve socket descriptor
        let fd = socket_descriptor(binding, handle)?;

        // map runtime mode into linux timestamp flags
        let flags = match mode {
            SocketTimestampingMode::Off => 0u32,
            SocketTimestampingMode::Software => {
                libc::SOF_TIMESTAMPING_SOFTWARE | libc::SOF_TIMESTAMPING_RX_SOFTWARE
            }
            SocketTimestampingMode::Hardware => {
                libc::SOF_TIMESTAMPING_RX_HARDWARE
                    | libc::SOF_TIMESTAMPING_SYS_HARDWARE
                    | libc::SOF_TIMESTAMPING_RAW_HARDWARE
            }
        };

        // apply host timestamping mode
        set_socket_u32(fd, libc::SOL_SOCKET, libc::SO_TIMESTAMPING, flags)
    }
}

/// Read packet timestamping mode.
///
/// Read timestamping controls from one socket endpoint.
/// Returned mode is normalized across host option variants.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses SO_TIMESTAMP families on Unix and host timestamping controls on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_timestamping(
    binding: &BindingCallContext,
    out: *mut SocketTimestampingMode,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate output pointers
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // reject non-linux targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        // resolve descriptor and read SO_TIMESTAMP state
        let fd = socket_descriptor(binding, handle)?;
        let enabled = get_socket_bool(fd, libc::SOL_SOCKET, libc::SO_TIMESTAMP, "getsockopt")?;
        let mode = if enabled {
            SocketTimestampingMode::Software
        } else {
            SocketTimestampingMode::Off
        };

        // write output mode for unix software timestamp state
        unsafe {
            *out = mode;
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        // resolve socket descriptor and read linux timestamp flags
        let fd = socket_descriptor(binding, handle)?;
        let flags = get_socket_u32(fd, libc::SOL_SOCKET, libc::SO_TIMESTAMPING, "getsockopt")?;
        let has_hardware_flags = (flags
            & (libc::SOF_TIMESTAMPING_RX_HARDWARE
                | libc::SOF_TIMESTAMPING_SYS_HARDWARE
                | libc::SOF_TIMESTAMPING_RAW_HARDWARE))
            != 0;
        let has_software_flags =
            (flags & (libc::SOF_TIMESTAMPING_SOFTWARE | libc::SOF_TIMESTAMPING_RX_SOFTWARE)) != 0;

        // map host flags into one runtime mode
        let mode = if has_hardware_flags {
            SocketTimestampingMode::Hardware
        } else if has_software_flags {
            SocketTimestampingMode::Software
        } else {
            SocketTimestampingMode::Off
        };

        // write the output mode
        unsafe {
            *out = mode;
        }

        Ok(())
    }
}

/// Set socket packet mark.
///
/// Set packet mark metadata used by host routing and firewall policy.
/// Mark interpretation is host-network-stack specific.
///
/// # Platform
/// Unix only.
/// Uses SO_MARK on Linux and returns `notSupported` on Unix targets without socket-mark support.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_packet_mark(
    binding: &BindingCallContext,
    handle: SocketHandle,
    mark: u32,
) -> RuntimeResult<()> {
    // reject non-linux targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, mark);
        Err(RuntimeError::from(PlatformError::not_supported("destack.net.setPacketMark")).boxed())
    }

    #[cfg(target_os = "linux")]
    {
        // resolve socket descriptor and apply packet mark
        let fd = socket_descriptor(binding, handle)?;
        set_socket_u32(fd, libc::SOL_SOCKET, libc::SO_MARK, mark)
    }
}

/// Read socket packet mark.
///
/// Read packet mark metadata from one socket endpoint.
/// Mark value interpretation is host-network-stack specific.
///
/// # Platform
/// Unix only.
/// Uses SO_MARK on Linux and returns `notSupported` on Unix targets without socket-mark support.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_packet_mark(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate output pointers
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // reject non-linux targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, out, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.net.getPacketMark")).boxed())
    }

    #[cfg(target_os = "linux")]
    {
        // resolve socket descriptor and read packet mark
        let fd = socket_descriptor(binding, handle)?;
        let mark = get_socket_u32(fd, libc::SOL_SOCKET, libc::SO_MARK, "getsockopt")?;

        // write the output
        unsafe {
            *out = mark;
        }

        Ok(())
    }
}

/// Select the default IPv4 multicast interface for one socket.
///
/// Set the local interface used for outgoing IPv4 multicast datagrams.
/// Interface selection follows host route and socket option semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_MULTICAST_IF) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_multicast_interface_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    // decode the interface address text
    let interface_address = unsafe { interface_address.as_str()? };
    let interface_address = if interface_address.is_empty() {
        Ipv4Addr::UNSPECIFIED
    } else {
        interface_address.parse::<Ipv4Addr>().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "interfaceAddress",
                "invalid IPv4 interface address",
            ))
            .boxed()
        })?
    };

    // encode and apply IP_MULTICAST_IF
    let fd = socket_descriptor(binding, handle)?;
    let value = libc::in_addr {
        s_addr: u32::from(interface_address).to_be(),
    };
    let result = unsafe {
        libc::setsockopt(
            fd,
            libc::IPPROTO_IP,
            libc::IP_MULTICAST_IF,
            &value as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::in_addr>() as libc::socklen_t,
        )
    };
    if result != 0 {
        return Err(core_platform::net_error("setsockopt(IP_MULTICAST_IF)"));
    }

    Ok(())
}

/// Read the default IPv4 multicast interface for one socket.
///
/// Read the local interface address used for outgoing IPv4 multicast datagrams.
/// Returned address follows host socket option encoding rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IP_MULTICAST_IF) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_multicast_interface_v4(
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // query IP_MULTICAST_IF
    let fd = socket_descriptor(binding, handle)?;
    let mut value = libc::in_addr { s_addr: 0 };
    let mut length = std::mem::size_of::<libc::in_addr>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            fd,
            libc::IPPROTO_IP,
            libc::IP_MULTICAST_IF,
            &mut value as *mut _ as *mut libc::c_void,
            &mut length,
        )
    };
    if result != 0 {
        return Err(core_platform::net_error("getsockopt(IP_MULTICAST_IF)"));
    }

    // encode and store output string
    let interface_address = Ipv4Addr::from(u32::from_be(value.s_addr));
    let interface_address = binding.store_string(&interface_address.to_string());
    unsafe {
        *out = interface_address;
    }

    Ok(())
}

/// Select the default IPv6 multicast interface for one socket.
///
/// Set the local interface index used for outgoing IPv6 multicast datagrams.
/// Interface selection follows host route and socket option semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IPV6_MULTICAST_IF) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_set_multicast_interface_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    interface_index: u32,
) -> RuntimeResult<()> {
    // resolve socket descriptor and encode interface selector
    let fd = socket_descriptor(binding, handle)?;
    let interface_index = ipv6_multicast_interface(interface_index)?;
    let result = unsafe {
        libc::setsockopt(
            fd,
            libc::IPPROTO_IPV6,
            libc::IPV6_MULTICAST_IF,
            &interface_index as *const _ as *const libc::c_void,
            std::mem::size_of_val(&interface_index) as libc::socklen_t,
        )
    };
    if result != 0 {
        return Err(core_platform::net_error("setsockopt(IPV6_MULTICAST_IF)"));
    }

    Ok(())
}

/// Read the default IPv6 multicast interface for one socket.
///
/// Read the local interface index used for outgoing IPv6 multicast datagrams.
/// Returned index follows host socket option encoding rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IPV6_MULTICAST_IF) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_multicast_interface_v6(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // query IPV6_MULTICAST_IF
    let fd = socket_descriptor(binding, handle)?;
    let mut value: Ipv6MulticastInterface = 0;
    let mut length = std::mem::size_of::<Ipv6MulticastInterface>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            fd,
            libc::IPPROTO_IPV6,
            libc::IPV6_MULTICAST_IF,
            &mut value as *mut _ as *mut libc::c_void,
            &mut length,
        )
    };
    if result != 0 {
        return Err(core_platform::net_error("getsockopt(IPV6_MULTICAST_IF)"));
    }

    // decode and write interface index
    #[cfg(target_os = "android")]
    let interface_index = u32::try_from(value).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "negative interface index returned by host",
        ))
        .boxed()
    })?;

    #[cfg(not(target_os = "android"))]
    let interface_index = value;

    unsafe {
        *out = interface_index;
    }

    Ok(())
}

/// Read multicast loopback mode.
///
/// Read whether outgoing multicast packets are looped back to local receivers.
/// Returned state reflects host socket-option state at call time.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IP_MULTICAST_LOOP/IPV6_MULTICAST_LOOP) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_multicast_loop(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve socket metadata
    let fd = socket_descriptor(binding, handle)?;
    let family = socket_family_from_fd(fd)?;

    // read loopback mode by socket family
    let value = match family {
        SocketFamily::IPv4 => {
            get_socket_u32(fd, libc::IPPROTO_IP, libc::IP_MULTICAST_LOOP, "getsockopt")?
        }
        SocketFamily::IPv6 => get_socket_u32(
            fd,
            libc::IPPROTO_IPV6,
            libc::IPV6_MULTICAST_LOOP,
            "getsockopt",
        )?,
        SocketFamily::Unspecified => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                "unsupported socket family",
            ))
            .boxed());
        }
    };

    // write the output boolean
    unsafe {
        *out = value != 0;
    }

    Ok(())
}

/// Read multicast TTL or hop-limit.
///
/// Read the active multicast TTL or IPv6 hop-limit used for outgoing datagrams.
/// Returned value follows host socket-option interpretation for the active family.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getsockopt(IP_MULTICAST_TTL/IPV6_MULTICAST_HOPS) on Unix and getsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_get_multicast_ttl(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve socket metadata
    let fd = socket_descriptor(binding, handle)?;
    let family = socket_family_from_fd(fd)?;

    // read ttl value by socket family
    let ttl = match family {
        SocketFamily::IPv4 => {
            get_socket_u32(fd, libc::IPPROTO_IP, libc::IP_MULTICAST_TTL, "getsockopt")?
        }
        SocketFamily::IPv6 => get_socket_u32(
            fd,
            libc::IPPROTO_IPV6,
            libc::IPV6_MULTICAST_HOPS,
            "getsockopt",
        )?,
        SocketFamily::Unspecified => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                "unsupported socket family",
            ))
            .boxed());
        }
    };

    // write output value
    unsafe {
        *out = ttl;
    }

    Ok(())
}

/// Join one IPv4 source-specific multicast membership.
///
/// Join one IGMPv3 source-specific membership for the given group and source.
/// Membership installation is host scoped and can be rejected by kernel policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_ADD_SOURCE_MEMBERSHIP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_join_multicast_source_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV4,
) -> RuntimeResult<()> {
    // reject non-linux targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, membership);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.net.joinMulticastSourceV4",
        ))
        .boxed())
    }

    #[cfg(target_os = "linux")]
    {
        // decode source membership fields
        let group = unsafe { membership.group.as_str()? };
        let source = unsafe { membership.source.as_str()? };
        let interface_address = unsafe { membership.interface_address.as_str()? };

        let group = group.parse::<Ipv4Addr>().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "membership.group",
                "invalid IPv4 multicast group address",
            ))
            .boxed()
        })?;
        let source = source.parse::<Ipv4Addr>().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "membership.source",
                "invalid IPv4 source address",
            ))
            .boxed()
        })?;
        let interface_address = if interface_address.is_empty() {
            Ipv4Addr::UNSPECIFIED
        } else {
            interface_address.parse::<Ipv4Addr>().map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "membership.interfaceAddress",
                    "invalid IPv4 interface address",
                ))
                .boxed()
            })?
        };

        // build and apply host membership request
        let fd = socket_descriptor(binding, handle)?;
        let request = libc::ip_mreq_source {
            imr_multiaddr: libc::in_addr {
                s_addr: u32::from(group).to_be(),
            },
            imr_sourceaddr: libc::in_addr {
                s_addr: u32::from(source).to_be(),
            },
            imr_interface: libc::in_addr {
                s_addr: u32::from(interface_address).to_be(),
            },
        };
        let result = unsafe {
            libc::setsockopt(
                fd,
                libc::IPPROTO_IP,
                libc::IP_ADD_SOURCE_MEMBERSHIP,
                &request as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::ip_mreq_source>() as libc::socklen_t,
            )
        };
        if result != 0 {
            return Err(core_platform::net_error(
                "setsockopt(IP_ADD_SOURCE_MEMBERSHIP)",
            ));
        }

        Ok(())
    }
}

/// Leave one IPv4 source-specific multicast membership.
///
/// Leave one IGMPv3 source-specific membership for the given group and source.
/// Membership removal is host scoped and can be rejected by kernel policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses setsockopt(IP_DROP_SOURCE_MEMBERSHIP) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_leave_multicast_source_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV4,
) -> RuntimeResult<()> {
    // reject non-linux targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, membership);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.net.leaveMulticastSourceV4",
        ))
        .boxed())
    }

    #[cfg(target_os = "linux")]
    {
        // decode source membership fields
        let group = unsafe { membership.group.as_str()? };
        let source = unsafe { membership.source.as_str()? };
        let interface_address = unsafe { membership.interface_address.as_str()? };

        let group = group.parse::<Ipv4Addr>().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "membership.group",
                "invalid IPv4 multicast group address",
            ))
            .boxed()
        })?;
        let source = source.parse::<Ipv4Addr>().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "membership.source",
                "invalid IPv4 source address",
            ))
            .boxed()
        })?;
        let interface_address = if interface_address.is_empty() {
            Ipv4Addr::UNSPECIFIED
        } else {
            interface_address.parse::<Ipv4Addr>().map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "membership.interfaceAddress",
                    "invalid IPv4 interface address",
                ))
                .boxed()
            })?
        };

        // build and apply host membership request
        let fd = socket_descriptor(binding, handle)?;
        let request = libc::ip_mreq_source {
            imr_multiaddr: libc::in_addr {
                s_addr: u32::from(group).to_be(),
            },
            imr_sourceaddr: libc::in_addr {
                s_addr: u32::from(source).to_be(),
            },
            imr_interface: libc::in_addr {
                s_addr: u32::from(interface_address).to_be(),
            },
        };
        let result = unsafe {
            libc::setsockopt(
                fd,
                libc::IPPROTO_IP,
                libc::IP_DROP_SOURCE_MEMBERSHIP,
                &request as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::ip_mreq_source>() as libc::socklen_t,
            )
        };
        if result != 0 {
            return Err(core_platform::net_error(
                "setsockopt(IP_DROP_SOURCE_MEMBERSHIP)",
            ));
        }

        Ok(())
    }
}

/// Join one IPv6 source-specific multicast membership.
///
/// Join one source-filtered IPv6 multicast membership for the given group and source.
/// Membership installation is host scoped and can be rejected by kernel policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses MCAST_JOIN_SOURCE_GROUP family socket options on Unix and equivalent host APIs on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_join_multicast_source_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV6,
) -> RuntimeResult<()> {
    // reject non-linux targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, membership);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.net.joinMulticastSourceV6",
        ))
        .boxed())
    }

    #[cfg(target_os = "linux")]
    {
        // decode source membership fields
        let group = unsafe { membership.group.as_str()? };
        let source = unsafe { membership.source.as_str()? };
        let group = group.parse::<Ipv6Addr>().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "membership.group",
                "invalid IPv6 multicast group address",
            ))
            .boxed()
        })?;
        let source = source.parse::<Ipv6Addr>().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "membership.source",
                "invalid IPv6 source address",
            ))
            .boxed()
        })?;

        // build and apply host membership request
        let fd = socket_descriptor(binding, handle)?;
        let request = LinuxGroupSourceRequest {
            gsr_interface: membership.interface_index,
            gsr_group: ipv6_sockaddr_storage(group),
            gsr_source: ipv6_sockaddr_storage(source),
        };
        let result = unsafe {
            libc::setsockopt(
                fd,
                libc::IPPROTO_IPV6,
                libc::MCAST_JOIN_SOURCE_GROUP,
                &request as *const _ as *const libc::c_void,
                std::mem::size_of::<LinuxGroupSourceRequest>() as libc::socklen_t,
            )
        };
        if result != 0 {
            return Err(core_platform::net_error(
                "setsockopt(MCAST_JOIN_SOURCE_GROUP)",
            ));
        }

        Ok(())
    }
}

/// Leave one IPv6 source-specific multicast membership.
///
/// Leave one source-filtered IPv6 multicast membership for the given group and source.
/// Membership removal is host scoped and can be rejected by kernel policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses MCAST_LEAVE_SOURCE_GROUP family socket options on Unix and equivalent host APIs on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.multicast`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_leave_multicast_source_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV6,
) -> RuntimeResult<()> {
    // reject non-linux targets explicitly
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, handle, membership);
        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.net.leaveMulticastSourceV6",
        ))
        .boxed())
    }

    #[cfg(target_os = "linux")]
    {
        // decode source membership fields
        let group = unsafe { membership.group.as_str()? };
        let source = unsafe { membership.source.as_str()? };
        let group = group.parse::<Ipv6Addr>().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "membership.group",
                "invalid IPv6 multicast group address",
            ))
            .boxed()
        })?;
        let source = source.parse::<Ipv6Addr>().map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "membership.source",
                "invalid IPv6 source address",
            ))
            .boxed()
        })?;

        // build and apply host membership request
        let fd = socket_descriptor(binding, handle)?;
        let request = LinuxGroupSourceRequest {
            gsr_interface: membership.interface_index,
            gsr_group: ipv6_sockaddr_storage(group),
            gsr_source: ipv6_sockaddr_storage(source),
        };
        let result = unsafe {
            libc::setsockopt(
                fd,
                libc::IPPROTO_IPV6,
                libc::MCAST_LEAVE_SOURCE_GROUP,
                &request as *const _ as *const libc::c_void,
                std::mem::size_of::<LinuxGroupSourceRequest>() as libc::socklen_t,
            )
        };
        if result != 0 {
            return Err(core_platform::net_error(
                "setsockopt(MCAST_LEAVE_SOURCE_GROUP)",
            ));
        }

        Ok(())
    }
}

/// Open a raw IP socket.
///
/// Creates a raw socket endpoint for protocol-level packet control.
/// Host privilege checks and protocol restrictions are enforced by the kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses socket(AF_INET/AF_INET6, SOCK_RAW) on Unix and WSASocketW raw mode on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_raw_socket(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
    protocol: i32,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // reject unspecified family values
    if family == SocketFamily::Unspecified {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "family",
            "unspecified family is not valid for raw sockets",
        ))
        .boxed());
    }

    // create the raw socket
    let fd = unsafe { libc::socket(socket_family_to_raw(family), libc::SOCK_RAW, protocol) };
    if fd < 0 {
        return Err(core_platform::net_error("socket"));
    }

    // register socket resource
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

/// Enable or disable IP header inclusion on a raw socket.
///
/// Updates the raw socket header include mode.
/// Caller is responsible for writing valid protocol headers when enabled.
///
/// # Platform
/// Unix and Windows.
/// Uses setsockopt(IP_HDRINCL) on Unix and setsockopt on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.raw`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_raw_set_header_included(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve socket descriptor and apply IP_HDRINCL
    let fd = socket_descriptor(binding, handle)?;
    set_socket_bool(fd, libc::IPPROTO_IP, libc::IP_HDRINCL, enabled)
}
