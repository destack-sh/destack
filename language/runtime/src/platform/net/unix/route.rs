#[cfg(any(target_os = "linux", target_os = "macos"))]
use super::core::*;

use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::platform::core as core_platform;
use crate::platform::net::*;
use crate::platform::{NativeArray, PlatformError};
use crate::runtime::BindingCallContext;

#[cfg(target_os = "linux")]
use std::ffi::{CStr, CString};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::net::{Ipv4Addr, Ipv6Addr};
#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicI32, Ordering};

/// Linux procfs IPv4 route-table snapshot path.
#[cfg(target_os = "linux")]
const PROC_ROUTE_IPV4_PATH: &str = "/proc/net/route";
/// Linux procfs IPv6 route-table snapshot path.
#[cfg(target_os = "linux")]
const PROC_ROUTE_IPV6_PATH: &str = "/proc/net/ipv6_route";
/// macOS route-socket alignment width.
#[cfg(target_os = "macos")]
const MACOS_ROUTE_ALIGNMENT: usize = std::mem::size_of::<libc::c_long>();
/// macOS IPv4 route sysctl family selector.
#[cfg(target_os = "macos")]
const MACOS_ROUTE_FAMILY_IPV4: libc::c_int = libc::AF_INET;
/// macOS IPv6 route sysctl family selector.
#[cfg(target_os = "macos")]
const MACOS_ROUTE_FAMILY_IPV6: libc::c_int = libc::AF_INET6;
/// macOS route mutation reply buffer size.
#[cfg(target_os = "macos")]
const MACOS_ROUTE_REPLY_BUFFER_SIZE: usize = 4096;

/// Runtime-owned mutable state for macOS route sockets.
#[cfg(target_os = "macos")]
#[derive(Debug, Default)]
pub(crate) struct MacosRouteRuntimeState {
    /// Monotonic route message sequence for route sockets.
    sequence: AtomicI32,
}

/// Return runtime-owned macOS route state.
#[cfg(target_os = "macos")]
fn macos_route_runtime_state(binding: &BindingCallContext) -> Arc<MacosRouteRuntimeState> {
    binding
        .worker()
        .platform_state
        .net
        .macos_route_runtime_state(|| MacosRouteRuntimeState {
            sequence: AtomicI32::new(1),
        })
}

/// Return one route binding not-supported error.
fn route_not_supported(operation: &'static str) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Round one route-socket sockaddr size to kernel alignment.
#[cfg(target_os = "macos")]
fn macos_route_roundup(length: usize) -> usize {
    if length == 0 {
        return MACOS_ROUTE_ALIGNMENT;
    }

    1 + ((length - 1) | (MACOS_ROUTE_ALIGNMENT - 1))
}

/// Count one prefix length from contiguous netmask bits.
#[cfg(target_os = "macos")]
fn macos_prefix_length_from_mask(mask: &[u8]) -> u8 {
    let mut prefix = 0u8;
    let mut saw_zero = false;
    for byte in mask {
        for bit in 0..8 {
            let is_set = (byte & (0x80 >> bit)) != 0;
            if is_set && saw_zero {
                return prefix;
            }
            if is_set {
                prefix = prefix.saturating_add(1);
            } else {
                saw_zero = true;
            }
        }
    }

    prefix
}

/// Build one sequence value for one macOS route message exchange.
#[cfg(target_os = "macos")]
fn macos_route_sequence_for_context(binding: &BindingCallContext) -> i32 {
    let runtime_state = macos_route_runtime_state(binding);
    runtime_state.sequence.fetch_add(1, Ordering::Relaxed)
}

/// Encode one sockaddr value into one route message vector with alignment.
#[cfg(target_os = "macos")]
fn macos_route_append_sockaddr<T>(message: &mut Vec<u8>, address: &T) {
    // append one sockaddr payload
    let length = std::mem::size_of::<T>();
    let bytes = unsafe { std::slice::from_raw_parts(address as *const _ as *const u8, length) };
    message.extend_from_slice(bytes);

    // append route-socket padding bytes
    let padded_length = macos_route_roundup(length);
    if padded_length > length {
        message.resize(message.len() + (padded_length - length), 0);
    }
}

/// Build one IPv4 route sockaddr payload for one address.
#[cfg(target_os = "macos")]
fn macos_sockaddr_from_ipv4(address: Ipv4Addr) -> libc::sockaddr_in {
    libc::sockaddr_in {
        sin_len: std::mem::size_of::<libc::sockaddr_in>() as u8,
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr {
            s_addr: u32::from(address).to_be(),
        },
        sin_zero: [0; 8],
    }
}

/// Build one IPv6 route sockaddr payload for one address.
#[cfg(target_os = "macos")]
fn macos_sockaddr_from_ipv6(address: Ipv6Addr) -> libc::sockaddr_in6 {
    libc::sockaddr_in6 {
        sin6_len: std::mem::size_of::<libc::sockaddr_in6>() as u8,
        sin6_family: libc::AF_INET6 as libc::sa_family_t,
        sin6_port: 0,
        sin6_flowinfo: 0,
        sin6_addr: libc::in6_addr {
            s6_addr: address.octets(),
        },
        sin6_scope_id: 0,
    }
}

/// Build one IPv4 route-netmask sockaddr payload from one prefix length.
#[cfg(target_os = "macos")]
fn macos_ipv4_netmask(prefix_length: u8) -> libc::sockaddr_in {
    let mask_bits = if prefix_length == 0 {
        0
    } else {
        u32::MAX << (32 - u32::from(prefix_length))
    };
    let mask = Ipv4Addr::from(mask_bits);

    macos_sockaddr_from_ipv4(mask)
}

/// Build one IPv6 route-netmask sockaddr payload from one prefix length.
#[cfg(target_os = "macos")]
fn macos_ipv6_netmask(prefix_length: u8) -> libc::sockaddr_in6 {
    let mut mask = [0u8; 16];
    for bit_index in 0..usize::from(prefix_length) {
        let byte_index = bit_index / 8;
        let bit = 7 - (bit_index % 8);
        mask[byte_index] |= 1 << bit;
    }

    macos_sockaddr_from_ipv6(Ipv6Addr::from(mask))
}

/// Linux IPv6 route mutation payload for route ioctls.
#[cfg(target_os = "linux")]
#[repr(C)]
struct LinuxIpv6RouteMessage {
    /// Destination network address.
    rtmsg_dst: libc::in6_addr,
    /// Source network address.
    rtmsg_src: libc::in6_addr,
    /// Gateway network address.
    rtmsg_gateway: libc::in6_addr,
    /// Route type value.
    rtmsg_type: u32,
    /// Destination prefix length in bits.
    rtmsg_dst_len: u16,
    /// Source prefix length in bits.
    rtmsg_src_len: u16,
    /// Route metric value.
    rtmsg_metric: u32,
    /// Route table metadata field.
    rtmsg_info: libc::c_ulong,
    /// Host route flags.
    rtmsg_flags: u32,
    /// Interface index.
    rtmsg_ifindex: libc::c_int,
}

/// Build one raw sockaddr payload for one IPv4 address.
#[cfg(target_os = "linux")]
fn sockaddr_from_ipv4(address: Ipv4Addr) -> libc::sockaddr {
    // encode one sockaddr_in payload
    let socket_address = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr {
            s_addr: u32::from(address).to_be(),
        },
        sin_zero: [0; 8],
    };

    // copy sockaddr_in bytes into one sockaddr field
    let mut raw = unsafe { std::mem::zeroed::<libc::sockaddr>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &socket_address as *const _ as *const u8,
            &mut raw as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr>(),
        );
    }

    raw
}

/// Build one route socket-address payload for one IPv4 address.
#[cfg(target_os = "linux")]
fn socket_address_ipv4(
    binding: &BindingCallContext,
    address: Ipv4Addr,
) -> RuntimeResult<SocketAddress> {
    // encode sockaddr storage bytes for the IPv4 address
    let raw = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr {
            s_addr: u32::from(address).to_be(),
        },
        sin_zero: [0; 8],
    };
    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &raw as *const _ as *const u8,
            &mut storage as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in>(),
        );
    }

    // map storage bytes into the runtime socket-address ABI
    socket_address_from_storage(
        binding,
        &storage,
        std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
    )
}

/// Build one route socket-address payload for one IPv6 address.
#[cfg(target_os = "linux")]
fn socket_address_ipv6(
    binding: &BindingCallContext,
    address: Ipv6Addr,
) -> RuntimeResult<SocketAddress> {
    // encode sockaddr storage bytes for the IPv6 address
    let raw = libc::sockaddr_in6 {
        sin6_family: libc::AF_INET6 as libc::sa_family_t,
        sin6_port: 0,
        sin6_flowinfo: 0,
        sin6_addr: libc::in6_addr {
            s6_addr: address.octets(),
        },
        sin6_scope_id: 0,
    };
    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &raw as *const _ as *const u8,
            &mut storage as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in6>(),
        );
    }

    // map storage bytes into the runtime socket-address ABI
    socket_address_from_storage(
        binding,
        &storage,
        std::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
    )
}

/// Decode one IPv4 address from one runtime socket-address payload.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn route_ipv4_from_address(
    address: SocketAddress,
    label: &'static str,
    allow_empty: bool,
) -> RuntimeResult<Ipv4Addr> {
    // decode raw socket-address bytes
    let bytes = unsafe { address.bytes.as_slice()? };
    if bytes.is_empty() {
        if allow_empty {
            return Ok(Ipv4Addr::UNSPECIFIED);
        }
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "socket address payload is empty",
        ))
        .boxed());
    }
    if bytes.len() < std::mem::size_of::<libc::sockaddr_in>() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "socket address payload is too short for IPv4",
        ))
        .boxed());
    }

    // copy one sockaddr_in payload from raw bytes
    let mut raw = unsafe { std::mem::zeroed::<libc::sockaddr_in>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            &mut raw as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in>(),
        );
    }

    // validate the address family and decode the IPv4 address
    if raw.sin_family != libc::AF_INET as libc::sa_family_t {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "route entry requires one IPv4 socket address",
        ))
        .boxed());
    }

    Ok(Ipv4Addr::from(u32::from_be(raw.sin_addr.s_addr)))
}

/// Decode one IPv6 address from one runtime socket-address payload.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn route_ipv6_from_address(
    address: SocketAddress,
    label: &'static str,
    allow_empty: bool,
) -> RuntimeResult<Ipv6Addr> {
    // decode raw socket-address bytes
    let bytes = unsafe { address.bytes.as_slice()? };
    if bytes.is_empty() {
        if allow_empty {
            return Ok(Ipv6Addr::UNSPECIFIED);
        }
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "socket address payload is empty",
        ))
        .boxed());
    }
    if bytes.len() < std::mem::size_of::<libc::sockaddr_in6>() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "socket address payload is too short for IPv6",
        ))
        .boxed());
    }

    // copy one sockaddr_in6 payload from raw bytes
    let mut raw = unsafe { std::mem::zeroed::<libc::sockaddr_in6>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            &mut raw as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in6>(),
        );
    }

    // validate the address family and decode the IPv6 address
    if raw.sin6_family != libc::AF_INET6 as libc::sa_family_t {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "route entry requires one IPv6 socket address",
        ))
        .boxed());
    }

    Ok(Ipv6Addr::from(raw.sin6_addr.s6_addr))
}

/// Convert one IPv6 address into one host in6_addr value.
#[cfg(target_os = "linux")]
fn in6_addr_from_ipv6(address: Ipv6Addr) -> libc::in6_addr {
    libc::in6_addr {
        s6_addr: address.octets(),
    }
}

/// Resolve one interface index from one interface name.
#[cfg(target_os = "linux")]
fn interface_index_from_name(name: &str) -> u32 {
    // map interface name to the host interface index
    let name = match CString::new(name) {
        Ok(name) => name,
        Err(_) => return 0,
    };
    unsafe { libc::if_nametoindex(name.as_ptr()) }
}

/// Decode one Linux procfs IPv6 hex address into one IPv6 value.
#[cfg(target_os = "linux")]
fn parse_proc_ipv6_address(value: &str) -> RuntimeResult<Ipv6Addr> {
    // validate encoded hexadecimal width
    if value.len() != 32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "route",
            "invalid IPv6 route address width",
        ))
        .boxed());
    }

    // decode hexadecimal bytes into one IPv6 octet array
    let mut octets = [0u8; 16];
    for (index, byte) in octets.iter_mut().enumerate() {
        let offset = index * 2;
        let hex = &value[offset..offset + 2];
        let parsed = u8::from_str_radix(hex, 16).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "route",
                "invalid IPv6 route hex byte",
            ))
            .boxed()
        })?;
        *byte = parsed;
    }

    Ok(Ipv6Addr::from(octets))
}

/// Derive one route kind from one IPv4 route tuple.
#[cfg(target_os = "linux")]
fn ipv4_route_kind(destination: Ipv4Addr, flags: u32) -> RouteKind {
    // classify reject routes as blackholes
    if (flags & (libc::RTF_REJECT as u32)) != 0 {
        return RouteKind::Blackhole;
    }

    // classify broadcast and multicast destinations
    if destination.is_broadcast() {
        return RouteKind::Broadcast;
    }
    if destination.is_multicast() {
        return RouteKind::Multicast;
    }

    // classify loopback destinations as local routes
    if destination.is_loopback() {
        return RouteKind::Local;
    }

    RouteKind::Unicast
}

/// Derive one route kind from one IPv6 route tuple.
#[cfg(target_os = "linux")]
fn ipv6_route_kind(destination: Ipv6Addr, flags: u32) -> RouteKind {
    // classify reject routes as blackholes
    if (flags & (libc::RTF_REJECT as u32)) != 0 {
        return RouteKind::Blackhole;
    }

    // classify multicast destinations explicitly
    if destination.is_multicast() {
        return RouteKind::Multicast;
    }

    // classify loopback destinations as local routes
    if destination.is_loopback() {
        return RouteKind::Local;
    }

    RouteKind::Unicast
}

/// Derive one macOS IPv4 route kind from one route tuple.
#[cfg(target_os = "macos")]
fn macos_ipv4_route_kind(destination: Ipv4Addr, flags: i32) -> RouteKind {
    // classify reject and blackhole routes first
    if (flags & libc::RTF_REJECT) != 0 || (flags & libc::RTF_BLACKHOLE) != 0 {
        return RouteKind::Blackhole;
    }

    // classify broadcast and multicast routes
    if (flags & libc::RTF_BROADCAST) != 0 || destination.is_broadcast() {
        return RouteKind::Broadcast;
    }
    if (flags & libc::RTF_MULTICAST) != 0 || destination.is_multicast() {
        return RouteKind::Multicast;
    }

    // classify loopback and local routes
    if (flags & libc::RTF_LOCAL) != 0 || destination.is_loopback() {
        return RouteKind::Local;
    }

    RouteKind::Unicast
}

/// Derive one macOS IPv6 route kind from one route tuple.
#[cfg(target_os = "macos")]
fn macos_ipv6_route_kind(destination: Ipv6Addr, flags: i32) -> RouteKind {
    // classify reject and blackhole routes first
    if (flags & libc::RTF_REJECT) != 0 || (flags & libc::RTF_BLACKHOLE) != 0 {
        return RouteKind::Blackhole;
    }

    // classify multicast routes
    if (flags & libc::RTF_MULTICAST) != 0 || destination.is_multicast() {
        return RouteKind::Multicast;
    }

    // classify loopback and local routes
    if (flags & libc::RTF_LOCAL) != 0 || destination.is_loopback() {
        return RouteKind::Local;
    }

    RouteKind::Unicast
}

/// Decode one socket-address family from one raw sockaddr payload.
#[cfg(target_os = "macos")]
fn macos_sockaddr_family(raw: &[u8]) -> Option<i32> {
    if raw.len() < 2 {
        return None;
    }

    Some(raw[1] as i32)
}

/// Decode one IPv4 value from one raw sockaddr payload.
#[cfg(target_os = "macos")]
fn macos_sockaddr_ipv4(raw: &[u8]) -> Option<Ipv4Addr> {
    if raw.len() < std::mem::size_of::<libc::sockaddr_in>() {
        return None;
    }

    let mut address = unsafe { std::mem::zeroed::<libc::sockaddr_in>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            raw.as_ptr(),
            &mut address as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in>(),
        );
    }
    if address.sin_family != libc::AF_INET as libc::sa_family_t {
        return None;
    }

    Some(Ipv4Addr::from(address.sin_addr.s_addr.to_ne_bytes()))
}

/// Decode one IPv6 value from one raw sockaddr payload.
#[cfg(target_os = "macos")]
fn macos_sockaddr_ipv6(raw: &[u8]) -> Option<Ipv6Addr> {
    if raw.len() < std::mem::size_of::<libc::sockaddr_in6>() {
        return None;
    }

    let mut address = unsafe { std::mem::zeroed::<libc::sockaddr_in6>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            raw.as_ptr(),
            &mut address as *mut _ as *mut u8,
            std::mem::size_of::<libc::sockaddr_in6>(),
        );
    }
    if address.sin6_family != libc::AF_INET6 as libc::sa_family_t {
        return None;
    }

    Some(Ipv6Addr::from(address.sin6_addr.s6_addr))
}

/// Decode one route prefix length from one raw netmask sockaddr payload.
#[cfg(target_os = "macos")]
fn macos_route_prefix_length(netmask: Option<&[u8]>, family: SocketFamily, flags: i32) -> u8 {
    if let Some(raw) = netmask {
        if family == SocketFamily::IPv4 {
            let mut mask = [0u8; 4];
            if raw.len() >= std::mem::size_of::<libc::sockaddr_in>()
                && raw[1] == libc::AF_INET as u8
            {
                mask.copy_from_slice(&raw[4..8]);
            } else if raw.len() > 2 {
                let mask_length = (raw.len() - 2).min(4);
                mask[..mask_length].copy_from_slice(&raw[2..2 + mask_length]);
            }
            return macos_prefix_length_from_mask(&mask);
        }

        if family == SocketFamily::IPv6 {
            let mut mask = [0u8; 16];
            if raw.len() >= std::mem::size_of::<libc::sockaddr_in6>()
                && raw[1] == libc::AF_INET6 as u8
            {
                mask.copy_from_slice(&raw[8..24]);
            } else if raw.len() > 2 {
                let mask_length = (raw.len() - 2).min(16);
                mask[..mask_length].copy_from_slice(&raw[2..2 + mask_length]);
            }
            return macos_prefix_length_from_mask(&mask);
        }
    }

    if (flags & libc::RTF_HOST) != 0 {
        if family == SocketFamily::IPv4 {
            return 32;
        }
        if family == SocketFamily::IPv6 {
            return 128;
        }
    }

    0
}

/// Parse one route-socket sockaddr vector from one macOS route message.
#[cfg(target_os = "macos")]
fn macos_route_sockaddrs(message: &[u8], addrs_mask: i32) -> RuntimeResult<[Option<Vec<u8>>; 8]> {
    // initialize the parsed sockaddr vector
    let mut sockaddrs: [Option<Vec<u8>>; 8] = Default::default();
    let mut cursor = std::mem::size_of::<libc::rt_msghdr>();
    if message.len() < cursor {
        return Ok(sockaddrs);
    }

    // decode every sockaddr entry requested by the route message bitmask
    for (index, slot) in sockaddrs
        .iter_mut()
        .enumerate()
        .take(libc::RTAX_MAX as usize)
    {
        if (addrs_mask & (1 << index)) == 0 {
            continue;
        }
        if cursor >= message.len() {
            break;
        }

        let length = message[cursor] as usize;
        let entry_length = if length == 0 {
            0
        } else if cursor + length > message.len() {
            break;
        } else {
            length
        };
        if entry_length > 0 {
            *slot = Some(message[cursor..cursor + entry_length].to_vec());
        }

        let step = macos_route_roundup(entry_length);
        cursor = cursor.saturating_add(step);
    }

    Ok(sockaddrs)
}

/// Convert one raw sockaddr payload into one runtime socket address.
#[cfg(target_os = "macos")]
fn macos_socket_address(binding: &BindingCallContext, raw: &[u8]) -> RuntimeResult<SocketAddress> {
    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    let copy_length = raw.len().min(std::mem::size_of::<libc::sockaddr_storage>());
    unsafe {
        std::ptr::copy_nonoverlapping(raw.as_ptr(), &mut storage as *mut _ as *mut u8, copy_length);
    }

    socket_address_from_storage(binding, &storage, copy_length as libc::socklen_t)
}

/// Build one unspecified socket-address payload for macOS route rows.
#[cfg(target_os = "macos")]
fn macos_unspecified_socket_address(
    binding: &BindingCallContext,
    family: SocketFamily,
) -> RuntimeResult<SocketAddress> {
    if family == SocketFamily::IPv4 {
        let raw = libc::sockaddr_in {
            sin_len: std::mem::size_of::<libc::sockaddr_in>() as u8,
            sin_family: libc::AF_INET as libc::sa_family_t,
            sin_port: 0,
            sin_addr: libc::in_addr { s_addr: 0 },
            sin_zero: [0; 8],
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &raw as *const _ as *const u8,
                std::mem::size_of::<libc::sockaddr_in>(),
            )
        };
        return macos_socket_address(binding, bytes);
    }

    if family == SocketFamily::IPv6 {
        let raw = libc::sockaddr_in6 {
            sin6_len: std::mem::size_of::<libc::sockaddr_in6>() as u8,
            sin6_family: libc::AF_INET6 as libc::sa_family_t,
            sin6_port: 0,
            sin6_flowinfo: 0,
            sin6_addr: libc::in6_addr { s6_addr: [0; 16] },
            sin6_scope_id: 0,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &raw as *const _ as *const u8,
                std::mem::size_of::<libc::sockaddr_in6>(),
            )
        };
        return macos_socket_address(binding, bytes);
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "family",
        "unspecified family is not valid for route addresses",
    ))
    .boxed())
}

/// Read one macOS route snapshot through NET_RT_DUMP.
#[cfg(target_os = "macos")]
fn list_macos_routes(
    binding: &BindingCallContext,
    family: SocketFamily,
) -> RuntimeResult<Vec<RouteEntry>> {
    // map runtime family to one sysctl family selector
    let route_family = match family {
        SocketFamily::IPv4 => MACOS_ROUTE_FAMILY_IPV4,
        SocketFamily::IPv6 => MACOS_ROUTE_FAMILY_IPV6,
        SocketFamily::Unspecified => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "family",
                "unspecified family is not valid for routeList",
            ))
            .boxed());
        }
    };

    // query one route-table dump byte size
    let mut mib = [
        libc::CTL_NET,
        libc::PF_ROUTE,
        0,
        route_family,
        libc::NET_RT_DUMP,
        0,
    ];
    let mut length = 0usize;
    let first_status = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as libc::c_uint,
            std::ptr::null_mut(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    if first_status != 0 {
        return Err(core_platform::net_error("sysctl(NET_RT_DUMP)"));
    }

    // read one route-table dump snapshot
    let mut buffer = vec![0u8; length];
    let second_status = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as libc::c_uint,
            buffer.as_mut_ptr() as *mut libc::c_void,
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    if second_status != 0 {
        return Err(core_platform::net_error("sysctl(NET_RT_DUMP)"));
    }
    buffer.truncate(length);

    // decode route messages from the sysctl snapshot
    let mut routes = Vec::new();
    let mut offset = 0usize;
    while offset + std::mem::size_of::<libc::rt_msghdr>() <= buffer.len() {
        let header = unsafe {
            let pointer = buffer.as_ptr().add(offset) as *const libc::rt_msghdr;
            std::ptr::read_unaligned(pointer)
        };
        let message_length = header.rtm_msglen as usize;
        if message_length == 0 || offset + message_length > buffer.len() {
            break;
        }
        let message = &buffer[offset..offset + message_length];
        offset += message_length;

        // parse route sockaddr vector and decode route destination
        let sockaddrs = macos_route_sockaddrs(message, header.rtm_addrs)?;
        let Some(destination_raw) = sockaddrs[libc::RTAX_DST as usize].as_ref() else {
            continue;
        };
        let Some(destination_family_raw) = macos_sockaddr_family(destination_raw) else {
            continue;
        };
        let destination_family = if destination_family_raw == libc::AF_INET {
            SocketFamily::IPv4
        } else if destination_family_raw == libc::AF_INET6 {
            SocketFamily::IPv6
        } else {
            continue;
        };

        // decode destination and gateway socket-address payloads
        let destination = macos_socket_address(binding, destination_raw)?;
        let gateway = if let Some(gateway_raw) = sockaddrs[libc::RTAX_GATEWAY as usize].as_ref() {
            macos_socket_address(binding, gateway_raw)?
        } else {
            macos_unspecified_socket_address(binding, destination_family)?
        };

        // decode destination IP and derive route kind
        let kind = if destination_family == SocketFamily::IPv4 {
            let Some(address) = macos_sockaddr_ipv4(destination_raw) else {
                continue;
            };
            macos_ipv4_route_kind(address, header.rtm_flags)
        } else {
            let Some(address) = macos_sockaddr_ipv6(destination_raw) else {
                continue;
            };
            macos_ipv6_route_kind(address, header.rtm_flags)
        };

        // decode prefix length from the netmask sockaddr when present
        let netmask = sockaddrs[libc::RTAX_NETMASK as usize].as_deref();
        let prefix_length =
            macos_route_prefix_length(netmask, destination_family, header.rtm_flags);

        routes.push(RouteEntry {
            family: destination_family,
            destination,
            prefix_length,
            gateway,
            interface_index: header.rtm_index as u32,
            metric: header.rtm_rmx.rmx_hopcount,
            kind,
        });
    }

    Ok(routes)
}

/// Read one IPv4 route snapshot from procfs.
#[cfg(target_os = "linux")]
fn list_ipv4_routes(binding: &BindingCallContext) -> RuntimeResult<Vec<RouteEntry>> {
    // load one procfs IPv4 route table snapshot
    let rows = std::fs::read_to_string(PROC_ROUTE_IPV4_PATH).map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "failed to read {PROC_ROUTE_IPV4_PATH}: {error}",
        )))
        .boxed()
    })?;

    // decode every route-table data row
    let mut routes = Vec::new();
    for (line_index, line) in rows.lines().enumerate() {
        // skip the procfs header row
        if line_index == 0 || line.trim().is_empty() {
            continue;
        }

        // parse one tokenized route-table row
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.len() < 11 {
            continue;
        }

        let destination_raw = match u32::from_str_radix(fields[1], 16) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let gateway_raw = match u32::from_str_radix(fields[2], 16) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let flags_raw = match u32::from_str_radix(fields[3], 16) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let metric = match fields[6].parse::<u32>() {
            Ok(value) => value,
            Err(_) => continue,
        };
        let mask_raw = match u32::from_str_radix(fields[7], 16) {
            Ok(value) => value,
            Err(_) => continue,
        };

        // map host-encoded route columns into runtime fields
        let destination = Ipv4Addr::from(u32::from_le(destination_raw));
        let gateway = Ipv4Addr::from(u32::from_le(gateway_raw));
        let mask = u32::from_le(mask_raw);
        let prefix_length = mask.count_ones() as u8;
        let interface_index = interface_index_from_name(fields[0]);
        let kind = ipv4_route_kind(destination, flags_raw);

        // encode runtime route-entry socket addresses
        let destination_address = socket_address_ipv4(binding, destination)?;
        let gateway_address = socket_address_ipv4(binding, gateway)?;

        routes.push(RouteEntry {
            family: SocketFamily::IPv4,
            destination: destination_address,
            prefix_length,
            gateway: gateway_address,
            interface_index,
            metric,
            kind,
        });
    }

    Ok(routes)
}

/// Read one IPv6 route snapshot from procfs.
#[cfg(target_os = "linux")]
fn list_ipv6_routes(binding: &BindingCallContext) -> RuntimeResult<Vec<RouteEntry>> {
    // load one procfs IPv6 route table snapshot
    let rows = std::fs::read_to_string(PROC_ROUTE_IPV6_PATH).map_err(|error| {
        RuntimeError::from(PlatformError::io(format!(
            "failed to read {PROC_ROUTE_IPV6_PATH}: {error}",
        )))
        .boxed()
    })?;

    // decode every route-table data row
    let mut routes = Vec::new();
    for line in rows.lines() {
        // skip empty rows
        if line.trim().is_empty() {
            continue;
        }

        // parse one tokenized route-table row
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.len() < 10 {
            continue;
        }

        let destination = match parse_proc_ipv6_address(fields[0]) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let prefix_length = match u8::from_str_radix(fields[1], 16) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let gateway = match parse_proc_ipv6_address(fields[4]) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let metric = match u32::from_str_radix(fields[5], 16) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let flags_raw = match u32::from_str_radix(fields[8], 16) {
            Ok(value) => value,
            Err(_) => continue,
        };

        // map host-encoded route columns into runtime fields
        let interface_index = interface_index_from_name(fields[9]);
        let kind = ipv6_route_kind(destination, flags_raw);

        // encode runtime route-entry socket addresses
        let destination_address = socket_address_ipv6(binding, destination)?;
        let gateway_address = socket_address_ipv6(binding, gateway)?;

        routes.push(RouteEntry {
            family: SocketFamily::IPv6,
            destination: destination_address,
            prefix_length,
            gateway: gateway_address,
            interface_index,
            metric,
            kind,
        });
    }

    Ok(routes)
}

/// Apply one macOS route-table mutation through route sockets.
#[cfg(target_os = "macos")]
fn mutate_macos_route(
    binding: &BindingCallContext,
    route: RouteEntry,
    message_type: libc::c_int,
    operation: &'static str,
) -> RuntimeResult<()> {
    // validate family upfront
    if route.family != SocketFamily::IPv4 && route.family != SocketFamily::IPv6 {
        return route_not_supported(operation);
    }

    // validate prefix width for each route family
    if route.family == SocketFamily::IPv4 && route.prefix_length > 32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "route.prefixLength",
            "prefix length must be between 0 and 32 for IPv4",
        ))
        .boxed());
    }
    if route.family == SocketFamily::IPv6 && route.prefix_length > 128 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "route.prefixLength",
            "prefix length must be between 0 and 128 for IPv6",
        ))
        .boxed());
    }

    // encode one route message payload with family-specific sockaddr segments
    let header_length = std::mem::size_of::<libc::rt_msghdr>();
    let mut message = vec![0u8; header_length];
    let mut addrs = libc::RTA_DST | libc::RTA_NETMASK;
    let family = if route.family == SocketFamily::IPv4 {
        let destination = route_ipv4_from_address(route.destination, "route.destination", false)?;
        let gateway = route_ipv4_from_address(route.gateway, "route.gateway", true)?;
        let destination = macos_sockaddr_from_ipv4(destination);
        let netmask = macos_ipv4_netmask(route.prefix_length);
        let has_gateway = gateway != Ipv4Addr::UNSPECIFIED;
        let gateway = macos_sockaddr_from_ipv4(gateway);

        macos_route_append_sockaddr(&mut message, &destination);
        if has_gateway {
            addrs |= libc::RTA_GATEWAY;
            macos_route_append_sockaddr(&mut message, &gateway);
        }
        macos_route_append_sockaddr(&mut message, &netmask);

        libc::AF_INET as libc::sa_family_t
    } else {
        let destination = route_ipv6_from_address(route.destination, "route.destination", false)?;
        let gateway = route_ipv6_from_address(route.gateway, "route.gateway", true)?;
        let destination = macos_sockaddr_from_ipv6(destination);
        let netmask = macos_ipv6_netmask(route.prefix_length);
        let has_gateway = gateway != Ipv6Addr::UNSPECIFIED;
        let gateway = macos_sockaddr_from_ipv6(gateway);

        macos_route_append_sockaddr(&mut message, &destination);
        if has_gateway {
            addrs |= libc::RTA_GATEWAY;
            macos_route_append_sockaddr(&mut message, &gateway);
        }
        macos_route_append_sockaddr(&mut message, &netmask);

        libc::AF_INET6 as libc::sa_family_t
    };

    // map route metadata to host route message flags
    let mut route_flags = libc::RTF_UP;
    if (addrs & libc::RTA_GATEWAY) != 0 {
        route_flags |= libc::RTF_GATEWAY;
    }
    if (route.family == SocketFamily::IPv4 && route.prefix_length == 32)
        || (route.family == SocketFamily::IPv6 && route.prefix_length == 128)
    {
        route_flags |= libc::RTF_HOST;
    }
    if route.kind == RouteKind::Blackhole {
        route_flags |= libc::RTF_BLACKHOLE;
    }

    // open one route-control socket
    let socket_fd = unsafe { libc::socket(libc::PF_ROUTE, libc::SOCK_RAW, family as libc::c_int) };
    if socket_fd < 0 {
        return Err(core_platform::net_error("socket(PF_ROUTE)"));
    }

    // populate one route header
    let sequence = macos_route_sequence_for_context(binding);
    let message_length = u16::try_from(message.len()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "route",
            "route message length is out of range",
        ))
        .boxed()
    })?;
    let interface_index = u16::try_from(route.interface_index).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "route.interfaceIndex",
            "route interface index is out of range",
        ))
        .boxed()
    })?;
    let mut header = unsafe { std::mem::zeroed::<libc::rt_msghdr>() };
    header.rtm_msglen = message_length;
    header.rtm_version = libc::RTM_VERSION as libc::c_uchar;
    header.rtm_type = message_type as libc::c_uchar;
    header.rtm_index = interface_index;
    header.rtm_flags = route_flags;
    header.rtm_addrs = addrs;
    header.rtm_pid = unsafe { libc::getpid() };
    header.rtm_seq = sequence;
    if route.metric > 0 {
        header.rtm_inits = libc::RTV_HOPCOUNT as u32;
        header.rtm_rmx.rmx_hopcount = route.metric;
    }

    // write the route header into the message prefix
    let header_bytes = unsafe {
        std::slice::from_raw_parts(
            &header as *const _ as *const u8,
            std::mem::size_of::<libc::rt_msghdr>(),
        )
    };
    message[..header_length].copy_from_slice(header_bytes);

    // write one route mutation message
    let written = unsafe {
        libc::write(
            socket_fd,
            message.as_ptr() as *const libc::c_void,
            message.len(),
        )
    };
    if written < 0 {
        let _ = unsafe { libc::close(socket_fd) };
        return Err(core_platform::net_error(operation));
    }
    if usize::try_from(written).ok() != Some(message.len()) {
        let _ = unsafe { libc::close(socket_fd) };
        return Err(RuntimeError::from(PlatformError::io(
            "route socket write returned partial message",
        ))
        .boxed());
    }

    // read one matching kernel response for this route sequence
    loop {
        let mut response = [0u8; MACOS_ROUTE_REPLY_BUFFER_SIZE];
        let read = unsafe {
            libc::read(
                socket_fd,
                response.as_mut_ptr() as *mut libc::c_void,
                response.len(),
            )
        };
        if read < 0 {
            let _ = unsafe { libc::close(socket_fd) };
            return Err(core_platform::net_error("read(PF_ROUTE)"));
        }
        if (read as usize) < std::mem::size_of::<libc::rt_msghdr>() {
            continue;
        }

        let response_header = unsafe {
            let pointer = response.as_ptr() as *const libc::rt_msghdr;
            std::ptr::read_unaligned(pointer)
        };
        if response_header.rtm_version != libc::RTM_VERSION as libc::c_uchar {
            continue;
        }
        if response_header.rtm_pid != header.rtm_pid || response_header.rtm_seq != sequence {
            continue;
        }
        if response_header.rtm_errno != 0 {
            core_platform::set_errno(response_header.rtm_errno);
            let _ = unsafe { libc::close(socket_fd) };
            return Err(core_platform::net_error(operation));
        }

        break;
    }

    // close one route-control socket
    let close_rc = unsafe { libc::close(socket_fd) };
    if close_rc != 0 {
        return Err(core_platform::net_error("close(PF_ROUTE)"));
    }

    Ok(())
}

/// Apply one Linux route-table mutation through ioctl.
#[cfg(target_os = "linux")]
fn mutate_ipv4_route(
    route: RouteEntry,
    command: libc::c_ulong,
    operation: &'static str,
) -> RuntimeResult<()> {
    // validate route fields and decode route addresses
    if route.prefix_length > 32 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "route.prefixLength",
            "prefix length must be between 0 and 32 for IPv4",
        ))
        .boxed());
    }
    let destination = route_ipv4_from_address(route.destination, "route.destination", false)?;
    let gateway = route_ipv4_from_address(route.gateway, "route.gateway", true)?;

    // compute one IPv4 route mask from the prefix length
    let mask_bits = if route.prefix_length == 0 {
        0
    } else {
        u32::MAX << (32 - u32::from(route.prefix_length))
    };
    let mask = Ipv4Addr::from(mask_bits);

    // map route kind and gateway behavior into host route flags
    let mut flags = libc::RTF_UP as libc::c_ushort;
    if gateway != Ipv4Addr::UNSPECIFIED {
        flags |= libc::RTF_GATEWAY as libc::c_ushort;
    }
    if route.prefix_length == 32 {
        flags |= libc::RTF_HOST as libc::c_ushort;
    }
    if route.kind == RouteKind::Blackhole {
        flags |= libc::RTF_REJECT as libc::c_ushort;
    }

    // open one route-control socket
    let socket_fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if socket_fd < 0 {
        return Err(core_platform::net_error("socket"));
    }

    // prepare one optional interface-name payload
    let interface_name = if route.interface_index == 0 {
        None
    } else {
        let mut name = [0 as libc::c_char; libc::IF_NAMESIZE];
        let pointer = unsafe { libc::if_indextoname(route.interface_index, name.as_mut_ptr()) };
        if pointer.is_null() {
            let _ = unsafe { libc::close(socket_fd) };
            return Err(core_platform::net_error("if_indextoname"));
        }

        let cstr = unsafe { CStr::from_ptr(name.as_ptr()) };
        let owned = cstr.to_owned();
        Some(owned)
    };

    // build one host rtentry mutation payload
    let mut entry = unsafe { std::mem::zeroed::<libc::rtentry>() };
    entry.rt_dst = sockaddr_from_ipv4(destination);
    entry.rt_gateway = sockaddr_from_ipv4(gateway);
    entry.rt_genmask = sockaddr_from_ipv4(mask);
    entry.rt_flags = flags;
    entry.rt_metric = libc::c_short::try_from(route.metric).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "route.metric",
            "route metric is out of host range",
        ))
        .boxed()
    })?;
    if let Some(name) = interface_name.as_ref() {
        entry.rt_dev = name.as_ptr() as *mut libc::c_char;
    }

    // issue the route mutation ioctl
    let rc = unsafe { libc::ioctl(socket_fd, command, &entry) };
    let close_rc = unsafe { libc::close(socket_fd) };
    if close_rc != 0 && rc == 0 {
        return Err(core_platform::net_error("close"));
    }
    if rc != 0 {
        return Err(core_platform::net_error(operation));
    }

    Ok(())
}

/// Apply one Linux IPv6 route-table mutation through ioctl.
#[cfg(target_os = "linux")]
fn mutate_ipv6_route(
    route: RouteEntry,
    command: libc::c_ulong,
    operation: &'static str,
) -> RuntimeResult<()> {
    // validate route fields and decode route addresses
    if route.prefix_length > 128 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "route.prefixLength",
            "prefix length must be between 0 and 128 for IPv6",
        ))
        .boxed());
    }
    let destination = route_ipv6_from_address(route.destination, "route.destination", false)?;
    let gateway = route_ipv6_from_address(route.gateway, "route.gateway", true)?;

    // map route kind and gateway behavior into host route flags
    let mut flags = libc::RTF_UP as u32;
    if gateway != Ipv6Addr::UNSPECIFIED {
        flags |= libc::RTF_GATEWAY as u32;
    }
    if route.prefix_length == 128 {
        flags |= libc::RTF_HOST as u32;
    }
    if route.kind == RouteKind::Blackhole {
        flags |= libc::RTF_REJECT as u32;
    }

    // map interface index to host route payload
    let interface_index = libc::c_int::try_from(route.interface_index).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "route.interfaceIndex",
            "route interface index is out of host range",
        ))
        .boxed()
    })?;

    // open one route-control socket
    let socket_fd = unsafe { libc::socket(libc::AF_INET6, libc::SOCK_DGRAM, 0) };
    if socket_fd < 0 {
        return Err(core_platform::net_error("socket"));
    }

    // build one host in6_rtmsg-compatible mutation payload
    let message = LinuxIpv6RouteMessage {
        rtmsg_dst: in6_addr_from_ipv6(destination),
        rtmsg_src: in6_addr_from_ipv6(Ipv6Addr::UNSPECIFIED),
        rtmsg_gateway: in6_addr_from_ipv6(gateway),
        rtmsg_type: 0,
        rtmsg_dst_len: route.prefix_length as u16,
        rtmsg_src_len: 0,
        rtmsg_metric: route.metric,
        rtmsg_info: 0,
        rtmsg_flags: flags,
        rtmsg_ifindex: interface_index,
    };

    // issue the route mutation ioctl
    let rc = unsafe { libc::ioctl(socket_fd, command, &message) };
    let close_rc = unsafe { libc::close(socket_fd) };
    if close_rc != 0 && rc == 0 {
        return Err(core_platform::net_error("close"));
    }
    if rc != 0 {
        return Err(core_platform::net_error(operation));
    }

    Ok(())
}

/// Add a route table entry.
///
/// Requests host route table insertion for the supplied entry.
/// Host privilege and policy checks are enforced by the kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses netlink route mutation on Linux and route sockets on macOS.
/// Returns `notSupported` on Unix targets without a route backend.
/// Uses iphlpapi route mutation APIs on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.route.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_route_add(
    _binding: &BindingCallContext,
    route: RouteEntry,
) -> RuntimeResult<()> {
    // reject unix targets without route-mutation support
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (_binding, route);
        route_not_supported("destack.net.routeAdd")
    }

    #[cfg(target_os = "linux")]
    {
        // route IPv4 and IPv6 mutations through route ioctls
        if route.family == SocketFamily::IPv4 {
            return mutate_ipv4_route(route, libc::SIOCADDRT as libc::c_ulong, "ioctl(SIOCADDRT)");
        }
        if route.family == SocketFamily::IPv6 {
            return mutate_ipv6_route(route, libc::SIOCADDRT as libc::c_ulong, "ioctl(SIOCADDRT)");
        }

        route_not_supported("destack.net.routeAdd")
    }

    #[cfg(target_os = "macos")]
    {
        mutate_macos_route(_binding, route, libc::RTM_ADD, "write(PF_ROUTE:RTM_ADD)")
    }
}

/// Remove a route table entry.
///
/// Requests host route table deletion for the supplied entry.
/// Host privilege and policy checks are enforced by the kernel.
///
/// # Platform
/// Unix and Windows.
/// Uses netlink route mutation on Linux and route sockets on macOS.
/// Returns `notSupported` on Unix targets without a route backend.
/// Uses iphlpapi route mutation APIs on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.route.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_route_delete(
    _binding: &BindingCallContext,
    route: RouteEntry,
) -> RuntimeResult<()> {
    // reject unix targets without route-mutation support
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (_binding, route);
        route_not_supported("destack.net.routeDelete")
    }

    #[cfg(target_os = "linux")]
    {
        // route IPv4 and IPv6 mutations through route ioctls
        if route.family == SocketFamily::IPv4 {
            return mutate_ipv4_route(route, libc::SIOCDELRT as libc::c_ulong, "ioctl(SIOCDELRT)");
        }
        if route.family == SocketFamily::IPv6 {
            return mutate_ipv6_route(route, libc::SIOCDELRT as libc::c_ulong, "ioctl(SIOCDELRT)");
        }

        route_not_supported("destack.net.routeDelete")
    }

    #[cfg(target_os = "macos")]
    {
        mutate_macos_route(
            _binding,
            route,
            libc::RTM_DELETE,
            "write(PF_ROUTE:RTM_DELETE)",
        )
    }
}

/// List route table entries.
///
/// Reads the host route table and returns route entries for the selected family.
/// Results are snapshots and may become stale immediately after the call.
///
/// # Platform
/// Unix and Windows.
/// Uses netlink route tables on Linux and route sockets on macOS.
/// Returns `notSupported` on Unix targets without a route backend.
/// Uses iphlpapi route tables on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.route.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_route_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<RouteEntry>,
    family: SocketFamily,
) -> RuntimeResult<()> {
    // reject unix targets without one implemented route-list backend
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (binding, out, family);
        route_not_supported("destack.net.routeList")
    }

    #[cfg(target_os = "linux")]
    {
        // validate output pointer
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }

        // read one host route-table snapshot for the selected family
        let entries = match family {
            SocketFamily::IPv4 => list_ipv4_routes(binding)?,
            SocketFamily::IPv6 => list_ipv6_routes(binding)?,
            SocketFamily::Unspecified => {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "family",
                    "unspecified family is not valid for routeList",
                ))
                .boxed());
            }
        };

        // write one runtime route-entry array output
        let entries = binding.store_array(entries);
        unsafe {
            *out = entries;
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // validate output pointer
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }

        // read one host route-table snapshot for the selected family
        let entries = list_macos_routes(binding, family)?;

        // write one runtime route-entry array output
        let entries = binding.store_array(entries);
        unsafe {
            *out = entries;
        }

        Ok(())
    }
}
