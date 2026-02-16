use std::ffi::CString;
use std::mem;
use std::net::{Ipv4Addr, Ipv6Addr};

use windows_sys::Win32::NetworkManagement::IpHelper::if_nametoindex;
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, FIONBIO, IN_ADDR, IN_ADDR_0, IN6_ADDR, IN6_ADDR_0, IP_ADD_MEMBERSHIP,
    IP_DROP_MEMBERSHIP, IP_MREQ, IP_MULTICAST_LOOP, IP_MULTICAST_TTL, IP_TOS, IP_TTL, IPPROTO_IP,
    IPPROTO_IPV6, IPPROTO_TCP, IPV6_JOIN_GROUP, IPV6_LEAVE_GROUP, IPV6_MREQ, IPV6_MULTICAST_HOPS,
    IPV6_MULTICAST_LOOP, IPV6_V6ONLY, LINGER, SIO_KEEPALIVE_VALS, SO_BROADCAST, SO_KEEPALIVE,
    SO_LINGER, SO_RCVBUF, SO_RCVTIMEO, SO_REUSEADDR, SO_SNDBUF, SO_SNDTIMEO, SOCKADDR,
    SOCKADDR_STORAGE, SOCKET, SOL_SOCKET, TCP_KEEPCNT, TCP_KEEPIDLE, TCP_KEEPINTVL, TCP_NODELAY,
    WSAIoctl, getsockname, getsockopt, ioctlsocket, setsockopt, tcp_keepalive,
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

fn get_socket_bool(socket: SOCKET, level: i32, option: i32, syscall: &str) -> RuntimeResult<bool> {
    let mut value: u32 = 0;
    let mut length = mem::size_of::<u32>() as i32;
    let rc = unsafe {
        getsockopt(
            socket,
            level,
            option,
            &mut value as *mut _ as *mut u8,
            &mut length,
        )
    };
    if rc != 0 {
        return Err(last_net_error(syscall));
    }

    Ok(value != 0)
}

fn get_socket_u32(socket: SOCKET, level: i32, option: i32, syscall: &str) -> RuntimeResult<u32> {
    let mut value: u32 = 0;
    let mut length = mem::size_of::<u32>() as i32;
    let rc = unsafe {
        getsockopt(
            socket,
            level,
            option,
            &mut value as *mut _ as *mut u8,
            &mut length,
        )
    };
    if rc != 0 {
        return Err(last_net_error(syscall));
    }

    Ok(value)
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
    _context: &RuntimeCallContext,
    _handle: SocketHandle,
    _enabled: bool,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setReusePort")).boxed())
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
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { destack_net_join_multicast(context, handle, group, interface_address) }
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
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_index: u32,
) -> RuntimeResult<()> {
    let interface_index_text = context.store_string(&interface_index.to_string());
    unsafe { destack_net_join_multicast(context, handle, group, interface_index_text) }
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
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { destack_net_leave_multicast(context, handle, group, interface_address) }
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
    context: &RuntimeCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_index: u32,
) -> RuntimeResult<()> {
    let interface_index_text = context.store_string(&interface_index.to_string());
    unsafe { destack_net_leave_multicast(context, handle, group, interface_index_text) }
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_set_only_v6(
    _context: &RuntimeCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(_context, handle)?;

    // apply IPV6_V6ONLY
    let value: u32 = if enabled { 1 } else { 0 };
    let rc = unsafe {
        setsockopt(
            socket,
            IPPROTO_IPV6,
            IPV6_V6ONLY,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_net_error("setsockopt(IPV6_V6ONLY)"));
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_no_delay(
    _context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(_context, handle)?;
    let value = get_socket_bool(socket, IPPROTO_TCP, TCP_NODELAY, "getsockopt(TCP_NODELAY)")?;
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_reuse_addr(
    _context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(_context, handle)?;
    let value = get_socket_bool(socket, SOL_SOCKET, SO_REUSEADDR, "getsockopt(SO_REUSEADDR)")?;
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_reuse_port(
    _context: &RuntimeCallContext,
    _out: *mut bool,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getReusePort")).boxed())
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_linger(
    _context: &RuntimeCallContext,
    out: *mut Linger,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(_context, handle)?;
    let mut value = LINGER {
        l_onoff: 0,
        l_linger: 0,
    };
    let mut length = mem::size_of::<LINGER>() as i32;
    let rc = unsafe {
        getsockopt(
            socket,
            SOL_SOCKET,
            SO_LINGER,
            &mut value as *mut _ as *mut u8,
            &mut length,
        )
    };
    if rc != 0 {
        return Err(last_net_error("getsockopt(SO_LINGER)"));
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_recv_buffer(
    _context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(_context, handle)?;
    let value = get_socket_u32(socket, SOL_SOCKET, SO_RCVBUF, "getsockopt(SO_RCVBUF)")?;
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_send_buffer(
    _context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(_context, handle)?;
    let value = get_socket_u32(socket, SOL_SOCKET, SO_SNDBUF, "getsockopt(SO_SNDBUF)")?;
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_broadcast(
    _context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(_context, handle)?;
    let value = get_socket_bool(socket, SOL_SOCKET, SO_BROADCAST, "getsockopt(SO_BROADCAST)")?;
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_ttl(
    _context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(_context, handle)?;
    let value = get_socket_u32(socket, IPPROTO_IP, IP_TTL, "getsockopt(IP_TTL)")?;
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_tos(
    _context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(_context, handle)?;
    let value = get_socket_u32(socket, IPPROTO_IP, IP_TOS, "getsockopt(IP_TOS)")?;
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_read_timeout(
    _context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(_context, handle)?;
    let value = get_socket_u32(socket, SOL_SOCKET, SO_RCVTIMEO, "getsockopt(SO_RCVTIMEO)")?;
    unsafe {
        *out = value;
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_write_timeout(
    _context: &RuntimeCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(_context, handle)?;
    let value = get_socket_u32(socket, SOL_SOCKET, SO_SNDTIMEO, "getsockopt(SO_SNDTIMEO)")?;
    unsafe {
        *out = value;
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
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_get_only_v6(
    _context: &RuntimeCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(_context, handle)?;
    let value = get_socket_bool(socket, IPPROTO_IPV6, IPV6_V6ONLY, "getsockopt(IPV6_V6ONLY)")?;
    unsafe {
        *out = value;
    }

    Ok(())
}
