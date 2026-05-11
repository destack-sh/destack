use std::mem;
use std::net::{Ipv4Addr, Ipv6Addr};

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::NetworkManagement::IpHelper::{
    CreateIpForwardEntry2, DeleteIpForwardEntry2, FreeMibTable, GetIpForwardTable2,
    InitializeIpForwardEntry, MIB_IPFORWARD_ROW2, MIB_IPFORWARD_TABLE2,
};
use windows_sys::Win32::Networking::WinSock::{
    ADDRESS_FAMILY, AF_INET, AF_INET6, AF_UNSPEC, IN_ADDR, IN_ADDR_0, IN6_ADDR, IN6_ADDR_0,
    MIB_IPPROTO_NETMGMT, NlroManual, SOCKADDR_IN, SOCKADDR_IN6, SOCKADDR_INET, SOCKADDR_STORAGE,
};

use super::util::socket_address_raw_from_storage;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{RouteEntry, RouteKind, SocketAddress, SocketFamily};
use crate::platform::{NativeArray, PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

/// Guard one route-table pointer returned by `GetIpForwardTable2`.
struct RouteTableGuard {
    /// Route-table pointer owned by this guard.
    table: *mut MIB_IPFORWARD_TABLE2,
}

impl Drop for RouteTableGuard {
    fn drop(&mut self) {
        if self.table.is_null() {
            return;
        }

        unsafe {
            FreeMibTable(self.table as *const _);
        }
    }
}

/// Build one socket address payload for the given family and unspecified address bytes.
fn unspecified_socket_address(binding: &BindingCallContext, family: SocketFamily) -> SocketAddress {
    match family {
        SocketFamily::IPv4 => {
            let raw = SOCKADDR_IN {
                sin_family: AF_INET as ADDRESS_FAMILY,
                sin_port: 0,
                sin_addr: IN_ADDR {
                    S_un: IN_ADDR_0 { S_addr: 0 },
                },
                sin_zero: [0; 8],
            };
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    &raw as *const _ as *const u8,
                    mem::size_of::<SOCKADDR_IN>(),
                )
            };

            SocketAddress {
                family: AF_INET,
                length: mem::size_of::<SOCKADDR_IN>() as u32,
                bytes: binding.store_array_copy(bytes),
            }
        }
        SocketFamily::IPv6 => {
            let raw = SOCKADDR_IN6 {
                sin6_family: AF_INET6 as ADDRESS_FAMILY,
                sin6_port: 0,
                sin6_flowinfo: 0,
                sin6_addr: IN6_ADDR {
                    u: IN6_ADDR_0 { Byte: [0; 16] },
                },
                Anonymous: unsafe { mem::zeroed() },
            };
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    &raw as *const _ as *const u8,
                    mem::size_of::<SOCKADDR_IN6>(),
                )
            };

            SocketAddress {
                family: AF_INET6,
                length: mem::size_of::<SOCKADDR_IN6>() as u32,
                bytes: binding.store_array_copy(bytes),
            }
        }
        SocketFamily::Unspecified => SocketAddress {
            family: AF_UNSPEC,
            length: 0,
            bytes: binding.store_array(Vec::new()),
        },
    }
}

/// Decode one runtime socket address as IPv4.
fn decode_ipv4_address(
    address: SocketAddress,
    label: &'static str,
    allow_empty: bool,
) -> RuntimeResult<Ipv4Addr> {
    // decode raw payload bytes
    let bytes = unsafe { address.bytes.as_slice()? };

    // allow empty payloads only when explicitly permitted
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

    // enforce one full sockaddr payload
    if bytes.len() < mem::size_of::<SOCKADDR_IN>() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "socket address payload is too short for IPv4",
        ))
        .boxed());
    }

    // copy one sockaddr payload into local storage
    let mut raw = unsafe { mem::zeroed::<SOCKADDR_IN>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            &mut raw as *mut _ as *mut u8,
            mem::size_of::<SOCKADDR_IN>(),
        );
    }

    // reject mismatched address families
    if raw.sin_family != AF_INET as ADDRESS_FAMILY {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "route entry requires one IPv4 socket address",
        ))
        .boxed());
    }

    // decode the IPv4 address
    let raw_address = unsafe { raw.sin_addr.S_un.S_addr };
    Ok(Ipv4Addr::from(u32::from_be(raw_address)))
}

/// Decode one runtime socket address as IPv6.
fn decode_ipv6_address(
    address: SocketAddress,
    label: &'static str,
    allow_empty: bool,
) -> RuntimeResult<Ipv6Addr> {
    // decode raw payload bytes
    let bytes = unsafe { address.bytes.as_slice()? };

    // allow empty payloads only when explicitly permitted
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

    // enforce one full sockaddr payload
    if bytes.len() < mem::size_of::<SOCKADDR_IN6>() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "socket address payload is too short for IPv6",
        ))
        .boxed());
    }

    // copy one sockaddr payload into local storage
    let mut raw = unsafe { mem::zeroed::<SOCKADDR_IN6>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            &mut raw as *mut _ as *mut u8,
            mem::size_of::<SOCKADDR_IN6>(),
        );
    }

    // reject mismatched address families
    if raw.sin6_family != AF_INET6 as ADDRESS_FAMILY {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "route entry requires one IPv6 socket address",
        ))
        .boxed());
    }

    // decode the IPv6 address
    let raw_address = unsafe { raw.sin6_addr.u.Byte };
    Ok(Ipv6Addr::from(raw_address))
}

/// Build one `SOCKADDR_INET` from one IPv4 address.
fn sockaddr_inet_from_ipv4(address: Ipv4Addr) -> SOCKADDR_INET {
    SOCKADDR_INET {
        Ipv4: SOCKADDR_IN {
            sin_family: AF_INET as ADDRESS_FAMILY,
            sin_port: 0,
            sin_addr: IN_ADDR {
                S_un: IN_ADDR_0 {
                    S_addr: u32::from(address).to_be(),
                },
            },
            sin_zero: [0; 8],
        },
    }
}

/// Build one `SOCKADDR_INET` from one IPv6 address.
fn sockaddr_inet_from_ipv6(address: Ipv6Addr) -> SOCKADDR_INET {
    SOCKADDR_INET {
        Ipv6: SOCKADDR_IN6 {
            sin6_family: AF_INET6 as ADDRESS_FAMILY,
            sin6_port: 0,
            sin6_flowinfo: 0,
            sin6_addr: IN6_ADDR {
                u: IN6_ADDR_0 {
                    Byte: address.octets(),
                },
            },
            Anonymous: unsafe { mem::zeroed() },
        },
    }
}

/// Convert one host `SOCKADDR_INET` into runtime socket-address bytes.
fn socket_address_from_sockaddr_inet(
    binding: &BindingCallContext,
    address: SOCKADDR_INET,
) -> RuntimeResult<SocketAddress> {
    // dispatch conversion by native family
    let family = unsafe { address.si_family as i32 };
    if family == AF_INET as i32 {
        let ipv4 = unsafe { address.Ipv4 };
        let mut storage = unsafe { mem::zeroed::<SOCKADDR_STORAGE>() };
        unsafe {
            std::ptr::copy_nonoverlapping(
                &ipv4 as *const _ as *const u8,
                &mut storage as *mut _ as *mut u8,
                mem::size_of::<SOCKADDR_IN>(),
            );
        }
        return socket_address_raw_from_storage(
            binding,
            &storage,
            mem::size_of::<SOCKADDR_IN>() as i32,
        );
    }

    // dispatch conversion for ipv6 addresses
    if family == AF_INET6 as i32 {
        let ipv6 = unsafe { address.Ipv6 };
        let mut storage = unsafe { mem::zeroed::<SOCKADDR_STORAGE>() };
        unsafe {
            std::ptr::copy_nonoverlapping(
                &ipv6 as *const _ as *const u8,
                &mut storage as *mut _ as *mut u8,
                mem::size_of::<SOCKADDR_IN6>(),
            );
        }
        return socket_address_raw_from_storage(
            binding,
            &storage,
            mem::size_of::<SOCKADDR_IN6>() as i32,
        );
    }

    // reject unsupported route families
    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "route.family",
        "unsupported route address family",
    ))
    .boxed())
}

/// Map one route destination and loopback bit into runtime route kind.
fn route_kind_ipv4(destination: Ipv4Addr, loopback: bool) -> RouteKind {
    // classify explicit loopback routes
    if loopback || destination.is_loopback() {
        return RouteKind::Local;
    }

    // classify special destination classes
    if destination.is_multicast() {
        return RouteKind::Multicast;
    }
    if destination.is_broadcast() {
        return RouteKind::Broadcast;
    }

    RouteKind::Unicast
}

/// Map one route destination and loopback bit into runtime route kind.
fn route_kind_ipv6(destination: Ipv6Addr, loopback: bool) -> RouteKind {
    // classify explicit loopback routes
    if loopback || destination.is_loopback() {
        return RouteKind::Local;
    }

    // classify multicast destinations
    if destination.is_multicast() {
        return RouteKind::Multicast;
    }

    RouteKind::Unicast
}

/// Decode one route row into one runtime route entry.
fn route_entry_from_row(
    binding: &BindingCallContext,
    row: MIB_IPFORWARD_ROW2,
) -> RuntimeResult<RouteEntry> {
    // resolve runtime family from destination prefix family
    let family = unsafe { row.DestinationPrefix.Prefix.si_family as i32 };
    let route_family = if family == AF_INET as i32 {
        SocketFamily::IPv4
    } else if family == AF_INET6 as i32 {
        SocketFamily::IPv6
    } else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "family",
            "unsupported route family in host table row",
        ))
        .boxed());
    };

    // encode destination socket address
    let destination = socket_address_from_sockaddr_inet(binding, row.DestinationPrefix.Prefix)?;

    // encode gateway socket address or synthesize one unspecified gateway
    let gateway_family = unsafe { row.NextHop.si_family as i32 };
    let gateway = if gateway_family == AF_INET as i32 || gateway_family == AF_INET6 as i32 {
        socket_address_from_sockaddr_inet(binding, row.NextHop)?
    } else {
        unspecified_socket_address(binding, route_family)
    };

    // classify route kind using destination family and host loopback metadata
    let kind = if route_family == SocketFamily::IPv4 {
        let destination = unsafe { row.DestinationPrefix.Prefix.Ipv4 };
        let destination = Ipv4Addr::from(u32::from_be(unsafe { destination.sin_addr.S_un.S_addr }));
        route_kind_ipv4(destination, row.Loopback != 0)
    } else {
        let destination = unsafe { row.DestinationPrefix.Prefix.Ipv6 };
        let destination = Ipv6Addr::from(unsafe { destination.sin6_addr.u.Byte });
        route_kind_ipv6(destination, row.Loopback != 0)
    };

    Ok(RouteEntry {
        family: route_family,
        destination,
        prefix_length: row.DestinationPrefix.PrefixLength,
        gateway,
        interface_index: row.InterfaceIndex,
        metric: row.Metric,
        kind,
    })
}

/// Encode one runtime route entry into one host route row.
fn route_row_from_entry(
    route: RouteEntry,
    operation: &'static str,
) -> RuntimeResult<MIB_IPFORWARD_ROW2> {
    // reject unsupported route kinds for route mutation lanes
    if route.kind == RouteKind::Blackhole {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // initialize one empty route row with host defaults
    let mut row = unsafe { mem::zeroed::<MIB_IPFORWARD_ROW2>() };
    unsafe {
        InitializeIpForwardEntry(&mut row);
    }

    // map route-family-specific fields
    match route.family {
        SocketFamily::IPv4 => {
            // validate prefix length and decode addresses
            if route.prefix_length > 32 {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "route.prefixLength",
                    "prefix length must be between 0 and 32 for IPv4",
                ))
                .boxed());
            }
            let destination = decode_ipv4_address(route.destination, "route.destination", false)?;
            let gateway = decode_ipv4_address(route.gateway, "route.gateway", true)?;

            // write host route row fields
            row.DestinationPrefix.Prefix = sockaddr_inet_from_ipv4(destination);
            row.DestinationPrefix.PrefixLength = route.prefix_length;
            row.NextHop = sockaddr_inet_from_ipv4(gateway);
        }
        SocketFamily::IPv6 => {
            // validate prefix length and decode addresses
            if route.prefix_length > 128 {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "route.prefixLength",
                    "prefix length must be between 0 and 128 for IPv6",
                ))
                .boxed());
            }
            let destination = decode_ipv6_address(route.destination, "route.destination", false)?;
            let gateway = decode_ipv6_address(route.gateway, "route.gateway", true)?;

            // write host route row fields
            row.DestinationPrefix.Prefix = sockaddr_inet_from_ipv6(destination);
            row.DestinationPrefix.PrefixLength = route.prefix_length;
            row.NextHop = sockaddr_inet_from_ipv6(gateway);
        }
        SocketFamily::Unspecified => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "route.family",
                "unspecified family is not valid for route mutations",
            ))
            .boxed());
        }
    }

    // map shared route metadata
    row.InterfaceIndex = route.interface_index;
    row.Metric = route.metric;
    row.Protocol = MIB_IPPROTO_NETMGMT;
    row.Origin = NlroManual;

    Ok(row)
}

/// Add a route table entry.
pub(crate) unsafe fn destack_net_route_add(
    _binding: &BindingCallContext,
    route: RouteEntry,
) -> RuntimeResult<()> {
    // convert one runtime route into host route row fields
    let row = route_row_from_entry(route, "destack.net.routeAdd")?;

    // submit one host route-table insertion
    let status = unsafe { CreateIpForwardEntry2(&row) };
    if status != ERROR_SUCCESS {
        return Err(core_platform::net_error_with_code(
            "CreateIpForwardEntry2",
            status as i32,
        ));
    }

    Ok(())
}

/// Remove a route table entry.
pub(crate) unsafe fn destack_net_route_delete(
    _binding: &BindingCallContext,
    route: RouteEntry,
) -> RuntimeResult<()> {
    // convert one runtime route into host route row fields
    let row = route_row_from_entry(route, "destack.net.routeDelete")?;

    // submit one host route-table deletion
    let status = unsafe { DeleteIpForwardEntry2(&row) };
    if status != ERROR_SUCCESS {
        return Err(core_platform::net_error_with_code(
            "DeleteIpForwardEntry2",
            status as i32,
        ));
    }

    Ok(())
}

/// List route table entries.
pub(crate) unsafe fn destack_net_route_list(
    binding: &BindingCallContext,
    out: *mut NativeArray<RouteEntry>,
    family: SocketFamily,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve requested address family
    let host_family = match family {
        SocketFamily::IPv4 => AF_INET as ADDRESS_FAMILY,
        SocketFamily::IPv6 => AF_INET6 as ADDRESS_FAMILY,
        SocketFamily::Unspecified => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "family",
                "unspecified family is not valid for routeList",
            ))
            .boxed());
        }
    };

    // query the host route table snapshot
    let mut table = std::ptr::null_mut::<MIB_IPFORWARD_TABLE2>();
    let status = unsafe { GetIpForwardTable2(host_family, &mut table) };
    if status != ERROR_SUCCESS {
        return Err(core_platform::net_error_with_code(
            "GetIpForwardTable2",
            status as i32,
        ));
    }
    let _table_guard = RouteTableGuard { table };

    // decode all route rows into runtime entries
    let mut routes = Vec::new();
    let row_count = unsafe { (*table).NumEntries as usize };
    let row_pointer = unsafe { (*table).Table.as_ptr() };
    for index in 0..row_count {
        let row = unsafe { *row_pointer.add(index) };
        let route = route_entry_from_row(binding, row)?;
        routes.push(route);
    }

    // write the route snapshot output
    unsafe {
        *out = binding.store_array(routes);
    }

    Ok(())
}
