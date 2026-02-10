use std::ffi::CString;
use std::mem;
use std::net::{Ipv4Addr, Ipv6Addr};

use windows_sys::Win32::NetworkManagement::IpHelper::if_nametoindex;
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, FIONBIO, IN_ADDR, IN_ADDR_0, IN6_ADDR, IN6_ADDR_0, IP_ADD_MEMBERSHIP,
    IP_DROP_MEMBERSHIP, IP_MREQ, IP_MULTICAST_LOOP, IP_MULTICAST_TTL, IP_TOS, IP_TTL, IPPROTO_IP,
    IPPROTO_IPV6, IPPROTO_TCP, IPV6_JOIN_GROUP, IPV6_LEAVE_GROUP, IPV6_MREQ, IPV6_MULTICAST_HOPS,
    IPV6_MULTICAST_LOOP, LINGER, SIO_KEEPALIVE_VALS, SO_BROADCAST, SO_KEEPALIVE, SO_LINGER,
    SO_RCVBUF, SO_RCVTIMEO, SO_REUSEADDR, SO_SNDBUF, SO_SNDTIMEO, SOCKADDR, SOCKADDR_STORAGE,
    SOCKET, SOL_SOCKET, TCP_KEEPCNT, TCP_KEEPIDLE, TCP_KEEPINTVL, TCP_NODELAY, WSAIoctl,
    getsockname, getsockopt, ioctlsocket, setsockopt, tcp_keepalive,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{KeepAliveConfig, Linger, SocketFamily, SocketHandle};
use crate::platform::{NativeStringRef, PlatformError};
use crate::runtime::RuntimeCallContext;

fn parse_ipv4_interface(value: &str) -> RuntimeResult<Ipv4Addr> {
    // treat empty values as INADDR_ANY
    if value.is_empty() {
        return Ok(Ipv4Addr::UNSPECIFIED);
    }

    // parse the interface address
    value.parse::<Ipv4Addr>().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "interfaceAddress",
            "invalid IPv4 interface address",
        ))
        .boxed()
    })
}

fn resolve_interface_index(value: &str) -> RuntimeResult<u32> {
    // treat empty values as the default interface
    if value.is_empty() {
        return Ok(0);
    }

    // accept numeric indexes
    if let Ok(index) = value.parse::<u32>() {
        return Ok(index);
    }

    // map interface names to indices
    let cstr = CString::new(value).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "interfaceAddress",
            "interface name contains nul",
        ))
        .boxed()
    })?;

    let index = unsafe { if_nametoindex(cstr.as_ptr() as *const u8) };
    if index == 0 {
        return Err(last_net_error("if_nametoindex"));
    }

    Ok(index)
}

fn socket_family(socket: SOCKET) -> RuntimeResult<SocketFamily> {
    // allocate address storage
    let mut storage = unsafe { mem::zeroed::<SOCKADDR_STORAGE>() };
    let mut length = mem::size_of::<SOCKADDR_STORAGE>() as i32;

    // query the socket address
    let rc = unsafe { getsockname(socket, &mut storage as *mut _ as *mut SOCKADDR, &mut length) };
    if rc != 0 {
        return Err(last_net_error("getsockname"));
    }

    // map the socket family
    let family = storage.ss_family as i32;
    if family == AF_INET as i32 {
        return Ok(SocketFamily::IPv4);
    }
    if family == AF_INET6 as i32 {
        return Ok(SocketFamily::IPv6);
    }
    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "handle",
        "unsupported socket family",
    ))
    .boxed())
}

fn milliseconds_from_seconds(value: u32, label: &str) -> RuntimeResult<u32> {
    let milliseconds = value.checked_mul(1000).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "duration is too large",
        ))
        .boxed()
    })?;

    Ok(milliseconds)
}

/// Enable or disable non-blocking mode.
pub(crate) unsafe fn destack_net_set_nonblocking(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // set the nonblocking flag
    let mut value: u32 = if enabled { 1 } else { 0 };
    let rc = unsafe { ioctlsocket(socket, FIONBIO, &mut value) };
    if rc != 0 {
        return Err(last_net_error("ioctlsocket"));
    }

    Ok(())
}

/// Enable or disable TCP_NODELAY.
pub(crate) unsafe fn destack_net_set_no_delay(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // set TCP_NODELAY
    let value: u32 = if enabled { 1 } else { 0 };
    let rc = unsafe {
        setsockopt(
            socket,
            IPPROTO_TCP,
            TCP_NODELAY,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }

    Ok(())
}

/// Set socket linger settings.
pub(crate) unsafe fn destack_net_set_linger(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    linger: Linger,
) -> RuntimeResult<()> {
    let socket = socket_descriptor(context, handle)?;
    let value = LINGER {
        l_onoff: if linger.enabled { 1 } else { 0 },
        l_linger: linger.seconds as u16,
    };
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_LINGER,
            &value as *const _ as *const _,
            mem::size_of::<LINGER>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt(SO_LINGER)"));
    }
    Ok(())
}

/// Enable or disable TCP keepalive.
pub(crate) unsafe fn destack_net_set_keep_alive(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
    idle_seconds: u32,
    interval_seconds: u32,
    probe_count: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // set SO_KEEPALIVE
    let value: u32 = if enabled { 1 } else { 0 };
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_KEEPALIVE,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }

    // update keepalive timing
    if enabled {
        let idle_milliseconds = milliseconds_from_seconds(idle_seconds, "idleSeconds")?;
        let interval_milliseconds = if interval_seconds > 0 {
            milliseconds_from_seconds(interval_seconds, "intervalSeconds")?
        } else {
            1000
        };

        let mut settings = tcp_keepalive {
            onoff: 1,
            keepalivetime: idle_milliseconds,
            keepaliveinterval: interval_milliseconds,
        };
        let mut returned = 0u32;
        let rc = unsafe {
            WSAIoctl(
                socket,
                SIO_KEEPALIVE_VALS,
                &mut settings as *mut _ as *mut _,
                mem::size_of::<tcp_keepalive>() as u32,
                std::ptr::null_mut(),
                0,
                &mut returned,
                std::ptr::null_mut(),
                None,
            )
        };
        if rc != 0 {
            return Err(last_net_error("WSAIoctl"));
        }

        // apply per-probe interval when supported
        if interval_seconds > 0 {
            let rc = unsafe {
                setsockopt(
                    socket,
                    IPPROTO_TCP,
                    TCP_KEEPINTVL,
                    &interval_seconds as *const _ as *const u8,
                    mem::size_of::<u32>() as i32,
                )
            };
            if rc != 0 {
                return Err(last_net_error("setsockopt(TCP_KEEPINTVL)"));
            }
        }

        // apply probe count when supported
        if probe_count > 0 {
            let rc = unsafe {
                setsockopt(
                    socket,
                    IPPROTO_TCP,
                    TCP_KEEPCNT,
                    &probe_count as *const _ as *const u8,
                    mem::size_of::<u32>() as i32,
                )
            };
            if rc != 0 {
                return Err(last_net_error("setsockopt(TCP_KEEPCNT)"));
            }
        }
    }

    Ok(())
}

/// Read full TCP keepalive settings.
pub(crate) unsafe fn destack_net_get_keep_alive(
    _context: &RuntimeCallContext,
    out: *mut KeepAliveConfig,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // read SO_KEEPALIVE
    let mut enabled: u32 = 0;
    let mut enabled_length = mem::size_of::<u32>() as i32;
    let rc = unsafe {
        getsockopt(
            socket,
            SOL_SOCKET,
            SO_KEEPALIVE,
            &mut enabled as *mut _ as *mut u8,
            &mut enabled_length,
        )
    };
    if rc != 0 {
        return Err(last_net_error("getsockopt(SO_KEEPALIVE)"));
    }

    // read keepalive idle time
    let mut idle_seconds: u32 = 0;
    let mut idle_length = mem::size_of::<u32>() as i32;
    let rc = unsafe {
        getsockopt(
            socket,
            IPPROTO_TCP,
            TCP_KEEPIDLE,
            &mut idle_seconds as *mut _ as *mut u8,
            &mut idle_length,
        )
    };
    if rc != 0 {
        return Err(last_net_error("getsockopt(TCP_KEEPIDLE)"));
    }

    // read keepalive interval
    let mut interval_seconds: u32 = 0;
    let mut interval_length = mem::size_of::<u32>() as i32;
    let rc = unsafe {
        getsockopt(
            socket,
            IPPROTO_TCP,
            TCP_KEEPINTVL,
            &mut interval_seconds as *mut _ as *mut u8,
            &mut interval_length,
        )
    };
    if rc != 0 {
        return Err(last_net_error("getsockopt(TCP_KEEPINTVL)"));
    }

    // read keepalive probe count
    let mut probe_count: u32 = 0;
    let mut probe_length = mem::size_of::<u32>() as i32;
    let rc = unsafe {
        getsockopt(
            socket,
            IPPROTO_TCP,
            TCP_KEEPCNT,
            &mut probe_count as *mut _ as *mut u8,
            &mut probe_length,
        )
    };
    if rc != 0 {
        return Err(last_net_error("getsockopt(TCP_KEEPCNT)"));
    }

    unsafe {
        *out = KeepAliveConfig {
            enabled: enabled != 0,
            idle_seconds,
            interval_seconds,
            probe_count,
        };
    }

    Ok(())
}

/// Enable or disable SO_REUSEADDR.
pub(crate) unsafe fn destack_net_set_reuse_addr(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // set SO_REUSEADDR
    let value: u32 = if enabled { 1 } else { 0 };
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_REUSEADDR,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }

    Ok(())
}

/// Enable or disable SO_REUSEPORT.
pub(crate) unsafe fn destack_net_set_reuse_port(
    _context: &RuntimeCallContext,
    _handle: SocketHandle,
    _enabled: bool,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReusePort")).boxed())
}

/// Set the receive buffer size.
pub(crate) unsafe fn destack_net_set_recv_buffer(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // apply the socket option
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_RCVBUF,
            &size as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }
    Ok(())
}

/// Set the send buffer size.
pub(crate) unsafe fn destack_net_set_send_buffer(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // apply the socket option
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_SNDBUF,
            &size as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }
    Ok(())
}

/// Enable or disable broadcast.
pub(crate) unsafe fn destack_net_set_broadcast(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // apply the socket option
    let value: u32 = if enabled { 1 } else { 0 };
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_BROADCAST,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }
    Ok(())
}

/// Set the IP time-to-live.
pub(crate) unsafe fn destack_net_set_ttl(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // apply the socket option
    let rc = unsafe {
        setsockopt(
            socket,
            IPPROTO_IP,
            IP_TTL,
            &ttl as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }
    Ok(())
}

/// Set the IP type-of-service field.
pub(crate) unsafe fn destack_net_set_tos(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // apply the socket option
    let rc = unsafe {
        setsockopt(
            socket,
            IPPROTO_IP,
            IP_TOS,
            &tos as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }
    Ok(())
}

/// Set the read timeout in milliseconds.
pub(crate) unsafe fn destack_net_set_read_timeout(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // apply the socket option
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_RCVTIMEO,
            &timeout_ms as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }
    Ok(())
}

/// Set the write timeout in milliseconds.
pub(crate) unsafe fn destack_net_set_write_timeout(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // apply the socket option
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_SNDTIMEO,
            &timeout_ms as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }
    Ok(())
}

/// Join a multicast group.
pub(crate) unsafe fn destack_net_join_multicast(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve socket metadata
    let socket = socket_descriptor(_context, handle)?;
    let family = socket_family(socket)?;
    let group = unsafe { group.as_str()? };
    let interface_address = unsafe { interface_address.as_str()? };

    // dispatch by socket family
    match family {
        SocketFamily::IPv4 => {
            // parse the multicast group
            let group_addr = group.parse::<Ipv4Addr>().map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "group",
                    "invalid IPv4 multicast address",
                ))
                .boxed()
            })?;

            // resolve the interface address
            let interface_addr = parse_ipv4_interface(interface_address)?;

            // build the membership request
            let mut multiaddr = IN_ADDR {
                S_un: unsafe { mem::zeroed::<IN_ADDR_0>() },
            };
            let mut interface = IN_ADDR {
                S_un: unsafe { mem::zeroed::<IN_ADDR_0>() },
            };
            multiaddr.S_un.S_addr = u32::from_ne_bytes(group_addr.octets()).to_be();
            interface.S_un.S_addr = u32::from_ne_bytes(interface_addr.octets()).to_be();

            let request = IP_MREQ {
                imr_multiaddr: multiaddr,
                imr_interface: interface,
            };

            // apply the membership request
            let rc = unsafe {
                setsockopt(
                    socket,
                    IPPROTO_IP,
                    IP_ADD_MEMBERSHIP,
                    &request as *const _ as *const u8,
                    mem::size_of::<IP_MREQ>() as i32,
                )
            };
            if rc != 0 {
                return Err(last_net_error("setsockopt(IP_ADD_MEMBERSHIP)"));
            }
            Ok(())
        }
        SocketFamily::IPv6 => {
            // parse the multicast group
            let group_addr = group.parse::<Ipv6Addr>().map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "group",
                    "invalid IPv6 multicast address",
                ))
                .boxed()
            })?;

            // resolve the interface index
            let interface_index = resolve_interface_index(interface_address)?;

            // build the membership request
            let mut addr = IN6_ADDR {
                u: unsafe { mem::zeroed::<IN6_ADDR_0>() },
            };
            addr.u.Byte = group_addr.octets();
            let request = IPV6_MREQ {
                ipv6mr_multiaddr: addr,
                ipv6mr_interface: interface_index,
            };

            // apply the membership request
            let rc = unsafe {
                setsockopt(
                    socket,
                    IPPROTO_IPV6,
                    IPV6_JOIN_GROUP,
                    &request as *const _ as *const u8,
                    mem::size_of::<IPV6_MREQ>() as i32,
                )
            };
            if rc != 0 {
                return Err(last_net_error("setsockopt(IPV6_JOIN_GROUP)"));
            }
            Ok(())
        }
        SocketFamily::Unspecified => Err(RuntimeError::from(
            PlatformError::invalid_argument_value("handle", "unsupported socket family"),
        )
        .boxed()),
    }
}

/// Leave a multicast group.
pub(crate) unsafe fn destack_net_leave_multicast(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve socket metadata
    let socket = socket_descriptor(_context, handle)?;
    let family = socket_family(socket)?;
    let group = unsafe { group.as_str()? };
    let interface_address = unsafe { interface_address.as_str()? };

    // dispatch by socket family
    match family {
        SocketFamily::IPv4 => {
            // parse the multicast group
            let group_addr = group.parse::<Ipv4Addr>().map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "group",
                    "invalid IPv4 multicast address",
                ))
                .boxed()
            })?;

            // resolve the interface address
            let interface_addr = parse_ipv4_interface(interface_address)?;

            // build the membership request
            let mut multiaddr = IN_ADDR {
                S_un: unsafe { mem::zeroed::<IN_ADDR_0>() },
            };
            let mut interface = IN_ADDR {
                S_un: unsafe { mem::zeroed::<IN_ADDR_0>() },
            };
            multiaddr.S_un.S_addr = u32::from_ne_bytes(group_addr.octets()).to_be();
            interface.S_un.S_addr = u32::from_ne_bytes(interface_addr.octets()).to_be();

            let request = IP_MREQ {
                imr_multiaddr: multiaddr,
                imr_interface: interface,
            };

            // apply the membership request
            let rc = unsafe {
                setsockopt(
                    socket,
                    IPPROTO_IP,
                    IP_DROP_MEMBERSHIP,
                    &request as *const _ as *const u8,
                    mem::size_of::<IP_MREQ>() as i32,
                )
            };
            if rc != 0 {
                return Err(last_net_error("setsockopt(IP_DROP_MEMBERSHIP)"));
            }
            Ok(())
        }
        SocketFamily::IPv6 => {
            // parse the multicast group
            let group_addr = group.parse::<Ipv6Addr>().map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "group",
                    "invalid IPv6 multicast address",
                ))
                .boxed()
            })?;

            // resolve the interface index
            let interface_index = resolve_interface_index(interface_address)?;

            // build the membership request
            let mut addr = IN6_ADDR {
                u: unsafe { mem::zeroed::<IN6_ADDR_0>() },
            };
            addr.u.Byte = group_addr.octets();
            let request = IPV6_MREQ {
                ipv6mr_multiaddr: addr,
                ipv6mr_interface: interface_index,
            };

            // apply the membership request
            let rc = unsafe {
                setsockopt(
                    socket,
                    IPPROTO_IPV6,
                    IPV6_LEAVE_GROUP,
                    &request as *const _ as *const u8,
                    mem::size_of::<IPV6_MREQ>() as i32,
                )
            };
            if rc != 0 {
                return Err(last_net_error("setsockopt(IPV6_LEAVE_GROUP)"));
            }
            Ok(())
        }
        SocketFamily::Unspecified => Err(RuntimeError::from(
            PlatformError::invalid_argument_value("handle", "unsupported socket family"),
        )
        .boxed()),
    }
}

/// Join an IPv4 multicast group.
pub(crate) unsafe fn destack_net_join_multicast_v4(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { destack_net_join_multicast(context, handle, group, interface_address) }
}

/// Join an IPv6 multicast group.
pub(crate) unsafe fn destack_net_join_multicast_v6(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_index: u32,
) -> RuntimeResult<()> {
    let interface_index_text = context.store_string(&interface_index.to_string());
    unsafe { destack_net_join_multicast(context, handle, group, interface_index_text) }
}

/// Leave an IPv4 multicast group.
pub(crate) unsafe fn destack_net_leave_multicast_v4(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { destack_net_leave_multicast(context, handle, group, interface_address) }
}

/// Leave an IPv6 multicast group.
pub(crate) unsafe fn destack_net_leave_multicast_v6(
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_index: u32,
) -> RuntimeResult<()> {
    let interface_index_text = context.store_string(&interface_index.to_string());
    unsafe { destack_net_leave_multicast(context, handle, group, interface_index_text) }
}

/// Enable or disable multicast loopback.
pub(crate) unsafe fn destack_net_set_multicast_loop(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve socket metadata
    let socket = socket_descriptor(_context, handle)?;
    let family = socket_family(socket)?;
    let value: u32 = if enabled { 1 } else { 0 };

    // select the socket option
    let (level, option) = match family {
        SocketFamily::IPv4 => (IPPROTO_IP, IP_MULTICAST_LOOP),
        SocketFamily::IPv6 => (IPPROTO_IPV6, IPV6_MULTICAST_LOOP),
        SocketFamily::Unspecified => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                "unsupported socket family",
            ))
            .boxed());
        }
    };

    // apply the socket option
    let rc = unsafe {
        setsockopt(
            socket,
            level,
            option,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }
    Ok(())
}

/// Set multicast TTL.
pub(crate) unsafe fn destack_net_set_multicast_ttl(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    // resolve socket metadata
    let socket = socket_descriptor(_context, handle)?;
    let family = socket_family(socket)?;

    // select the socket option
    let (level, option) = match family {
        SocketFamily::IPv4 => (IPPROTO_IP, IP_MULTICAST_TTL),
        SocketFamily::IPv6 => (IPPROTO_IPV6, IPV6_MULTICAST_HOPS),
        SocketFamily::Unspecified => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                "unsupported socket family",
            ))
            .boxed());
        }
    };

    // apply the socket option
    let rc = unsafe {
        setsockopt(
            socket,
            level,
            option,
            &ttl as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt"));
    }
    Ok(())
}
