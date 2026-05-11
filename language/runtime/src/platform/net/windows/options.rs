use std::mem;
use std::net::{Ipv4Addr, Ipv6Addr};

use windows_sys::Win32::Foundation::{
    ERROR_CALL_NOT_IMPLEMENTED, ERROR_FILE_NOT_FOUND, ERROR_INVALID_NAME, ERROR_INVALID_PARAMETER,
    ERROR_NOT_FOUND, ERROR_NOT_SUPPORTED, ERROR_SUCCESS,
};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    ConvertInterfaceAliasToLuid, ConvertInterfaceLuidToIndex,
};
use windows_sys::Win32::NetworkManagement::Ndis::NET_LUID_LH;
use windows_sys::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, FIONBIO, GROUP_SOURCE_REQ, IN_ADDR, IN_ADDR_0, IN6_ADDR, IN6_ADDR_0,
    IP_ADD_MEMBERSHIP, IP_ADD_SOURCE_MEMBERSHIP, IP_DROP_MEMBERSHIP, IP_DROP_SOURCE_MEMBERSHIP,
    IP_HDRINCL, IP_MREQ, IP_MREQ_SOURCE, IP_MULTICAST_IF, IP_MULTICAST_LOOP, IP_MULTICAST_TTL,
    IP_TOS, IP_TTL, IPPROTO_IP, IPPROTO_IPV6, IPPROTO_TCP, IPV6_JOIN_GROUP, IPV6_LEAVE_GROUP,
    IPV6_MREQ, IPV6_MULTICAST_HOPS, IPV6_MULTICAST_IF, IPV6_MULTICAST_LOOP, IPV6_V6ONLY, LINGER,
    MCAST_JOIN_SOURCE_GROUP, MCAST_LEAVE_SOURCE_GROUP, SIO_KEEPALIVE_VALS, SO_BROADCAST,
    SO_KEEPALIVE, SO_LINGER, SO_RCVBUF, SO_RCVTIMEO, SO_REUSE_UNICASTPORT, SO_REUSEADDR, SO_SNDBUF,
    SO_SNDTIMEO, SO_TIMESTAMP, SOCK_RAW, SOCKADDR, SOCKADDR_IN6, SOCKADDR_IN6_0, SOCKADDR_STORAGE,
    SOCKET, SOL_SOCKET, TCP_KEEPCNT, TCP_KEEPIDLE, TCP_KEEPINTVL, TCP_NODELAY, WSAEINVAL,
    WSAENOPROTOOPT, WSAEOPNOTSUPP, WSAIoctl, getsockname, getsockopt, ioctlsocket, setsockopt,
    socket, tcp_keepalive,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::net::{
    KeepAliveConfig, Linger, SocketFamily, SocketHandle, SocketOptionLevel, SocketOptionName,
    SocketTimestampingMode, UdpSourceMembershipV4, UdpSourceMembershipV6,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

/// Operation tag for interface-index lookup.
const INTERFACE_INDEX_OPERATION: &str = "destack.net.interface.interfaceIndex";

/// Return whether one ip-helper status code reports not-supported behavior.
fn is_ip_helper_not_supported(status: u32) -> bool {
    status == ERROR_NOT_SUPPORTED || status == ERROR_CALL_NOT_IMPLEMENTED
}

fn interface_lookup_error(
    field: &'static str,
    operation: &'static str,
    status: u32,
) -> Box<RuntimeError> {
    // map unavailable interface conversion APIs explicitly
    if is_ip_helper_not_supported(status) {
        return core_platform::not_supported(operation);
    }

    // reject missing or invalid interface references explicitly
    if matches!(
        status,
        ERROR_NOT_FOUND | ERROR_FILE_NOT_FOUND | ERROR_INVALID_NAME | ERROR_INVALID_PARAMETER
    ) {
        return RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "interface does not exist",
        ))
        .boxed();
    }

    // preserve remaining host status values
    core_platform::net_error_with_code(operation, status as i32)
}

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

    // resolve the interface alias to one LUID
    let name = core_platform::wide_with_nul(value);
    let mut luid = NET_LUID_LH { Value: 0 };
    let status = unsafe { ConvertInterfaceAliasToLuid(name.as_ptr(), &mut luid) };
    if status != ERROR_SUCCESS {
        return Err(interface_lookup_error(
            "interfaceAddress",
            INTERFACE_INDEX_OPERATION,
            status,
        ));
    }

    // resolve the LUID to one interface index
    let mut index = 0u32;
    let status = unsafe { ConvertInterfaceLuidToIndex(&luid, &mut index) };
    if status != ERROR_SUCCESS {
        return Err(interface_lookup_error(
            "interfaceAddress",
            INTERFACE_INDEX_OPERATION,
            status,
        ));
    }

    Ok(index)
}

fn socket_address_storage_ipv6(address: Ipv6Addr) -> SOCKADDR_STORAGE {
    // build one ipv6 sockaddr payload
    let source = SOCKADDR_IN6 {
        sin6_family: AF_INET6,
        sin6_port: 0,
        sin6_flowinfo: 0,
        sin6_addr: IN6_ADDR {
            u: IN6_ADDR_0 {
                Byte: address.octets(),
            },
        },
        Anonymous: SOCKADDR_IN6_0 { sin6_scope_id: 0 },
    };

    // copy the sockaddr bytes into generic storage
    let mut storage = unsafe { mem::zeroed::<SOCKADDR_STORAGE>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &source as *const _ as *const u8,
            &mut storage as *mut _ as *mut u8,
            mem::size_of::<SOCKADDR_IN6>(),
        );
    }

    storage
}

fn socket_family(socket: SOCKET) -> RuntimeResult<SocketFamily> {
    // allocate address storage
    let mut storage = unsafe { mem::zeroed::<SOCKADDR_STORAGE>() };
    let mut length = mem::size_of::<SOCKADDR_STORAGE>() as i32;

    // query the socket address
    let rc = unsafe { getsockname(socket, &mut storage as *mut _ as *mut SOCKADDR, &mut length) };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "getsockname",
            core_platform::last_wsa_error_code(),
        ));
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
        return Err(core_platform::net_error_with_code(
            syscall,
            core_platform::last_wsa_error_code(),
        ));
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
        return Err(core_platform::net_error_with_code(
            syscall,
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(value)
}

fn windows_option_not_supported_error(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

fn is_windows_option_not_supported(code: i32) -> bool {
    code == WSAENOPROTOOPT || code == WSAEOPNOTSUPP || code == WSAEINVAL
}

/// Enable or disable nonblocking mode on a socket.
pub(crate) unsafe fn destack_net_set_nonblocking(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

    // set the nonblocking flag
    let mut value: u32 = if enabled { 1 } else { 0 };
    let rc = unsafe { ioctlsocket(socket, FIONBIO, &mut value) };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "ioctlsocket",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Enable or disable TCP_NODELAY.
pub(crate) unsafe fn destack_net_set_no_delay(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Set socket linger settings.
pub(crate) unsafe fn destack_net_set_linger(
    binding: &BindingCallContext,
    handle: SocketHandle,
    linger: Linger,
) -> RuntimeResult<()> {
    let socket = socket_descriptor(binding, handle)?;
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
        return Err(core_platform::net_error_with_code(
            "setsockopt(SO_LINGER)",
            core_platform::last_wsa_error_code(),
        ));
    }
    Ok(())
}

/// Write full TCP keepalive parameters.
pub(crate) unsafe fn destack_net_set_keep_alive(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
    idle_seconds: u32,
    interval_seconds: u32,
    probe_count: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
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
            return Err(core_platform::net_error_with_code(
                "WSAIoctl",
                core_platform::last_wsa_error_code(),
            ));
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
                return Err(core_platform::net_error_with_code(
                    "setsockopt(TCP_KEEPINTVL)",
                    core_platform::last_wsa_error_code(),
                ));
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
                return Err(core_platform::net_error_with_code(
                    "setsockopt(TCP_KEEPCNT)",
                    core_platform::last_wsa_error_code(),
                ));
            }
        }
    }

    Ok(())
}

/// Read full TCP keepalive parameters.
pub(crate) unsafe fn destack_net_get_keep_alive(
    binding: &BindingCallContext,
    out: *mut KeepAliveConfig,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "getsockopt(SO_KEEPALIVE)",
            core_platform::last_wsa_error_code(),
        ));
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
        return Err(core_platform::net_error_with_code(
            "getsockopt(TCP_KEEPIDLE)",
            core_platform::last_wsa_error_code(),
        ));
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
        return Err(core_platform::net_error_with_code(
            "getsockopt(TCP_KEEPINTVL)",
            core_platform::last_wsa_error_code(),
        ));
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
        return Err(core_platform::net_error_with_code(
            "getsockopt(TCP_KEEPCNT)",
            core_platform::last_wsa_error_code(),
        ));
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Enable or disable SO_REUSEPORT.
pub(crate) unsafe fn destack_net_set_reuse_port(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

    // apply SO_REUSE_UNICASTPORT as the Windows reuse-port lane
    let value: u32 = if enabled { 1 } else { 0 };
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_REUSE_UNICASTPORT,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        let code = core_platform::last_wsa_error_code();
        if is_windows_option_not_supported(code) {
            return Err(windows_option_not_supported_error(
                "destack.net.setReusePort",
            ));
        }

        return Err(core_platform::net_error_with_code(
            "setsockopt(SO_REUSE_UNICASTPORT)",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Set the receive buffer size.
pub(crate) unsafe fn destack_net_set_recv_buffer(
    binding: &BindingCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }
    Ok(())
}

/// Set the send buffer size.
pub(crate) unsafe fn destack_net_set_send_buffer(
    binding: &BindingCallContext,
    handle: SocketHandle,
    size: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }
    Ok(())
}

/// Enable or disable broadcast.
pub(crate) unsafe fn destack_net_set_broadcast(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }
    Ok(())
}

/// Set the IP time-to-live.
pub(crate) unsafe fn destack_net_set_ttl(
    binding: &BindingCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }
    Ok(())
}

/// Set the IP type-of-service field.
pub(crate) unsafe fn destack_net_set_tos(
    binding: &BindingCallContext,
    handle: SocketHandle,
    tos: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }
    Ok(())
}

/// Set the read timeout in milliseconds.
pub(crate) unsafe fn destack_net_set_read_timeout(
    binding: &BindingCallContext,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }
    Ok(())
}

/// Set the write timeout in milliseconds.
pub(crate) unsafe fn destack_net_set_write_timeout(
    binding: &BindingCallContext,
    handle: SocketHandle,
    timeout_ms: u32,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }
    Ok(())
}

/// Join a multicast group.
pub(crate) unsafe fn destack_net_join_multicast(
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve socket metadata
    let socket = socket_descriptor(binding, handle)?;
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
                return Err(core_platform::net_error_with_code(
                    "setsockopt(IP_ADD_MEMBERSHIP)",
                    core_platform::last_wsa_error_code(),
                ));
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
                return Err(core_platform::net_error_with_code(
                    "setsockopt(IPV6_JOIN_GROUP)",
                    core_platform::last_wsa_error_code(),
                ));
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve socket metadata
    let socket = socket_descriptor(binding, handle)?;
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
                return Err(core_platform::net_error_with_code(
                    "setsockopt(IP_DROP_MEMBERSHIP)",
                    core_platform::last_wsa_error_code(),
                ));
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
                return Err(core_platform::net_error_with_code(
                    "setsockopt(IPV6_LEAVE_GROUP)",
                    core_platform::last_wsa_error_code(),
                ));
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
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { destack_net_join_multicast(binding, handle, group, interface_address) }
}

/// Join an IPv6 multicast group.
pub(crate) unsafe fn destack_net_join_multicast_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_index: u32,
) -> RuntimeResult<()> {
    let interface_index_text = binding.store_string(&interface_index.to_string());
    unsafe { destack_net_join_multicast(binding, handle, group, interface_index_text) }
}

/// Leave an IPv4 multicast group.
pub(crate) unsafe fn destack_net_leave_multicast_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    unsafe { destack_net_leave_multicast(binding, handle, group, interface_address) }
}

/// Leave an IPv6 multicast group.
pub(crate) unsafe fn destack_net_leave_multicast_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    group: NativeStringRef,
    interface_index: u32,
) -> RuntimeResult<()> {
    let interface_index_text = binding.store_string(&interface_index.to_string());
    unsafe { destack_net_leave_multicast(binding, handle, group, interface_index_text) }
}

/// Enable or disable multicast loopback.
pub(crate) unsafe fn destack_net_set_multicast_loop(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve socket metadata
    let socket = socket_descriptor(binding, handle)?;
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
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }
    Ok(())
}

/// Set multicast TTL.
pub(crate) unsafe fn destack_net_set_multicast_ttl(
    binding: &BindingCallContext,
    handle: SocketHandle,
    ttl: u32,
) -> RuntimeResult<()> {
    // resolve socket metadata
    let socket = socket_descriptor(binding, handle)?;
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
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }
    Ok(())
}

/// Restrict an IPv6 socket to IPv6 traffic only.
pub(crate) unsafe fn destack_net_set_only_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

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
        return Err(core_platform::net_error_with_code(
            "setsockopt(IPV6_V6ONLY)",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Read TCP_NODELAY.
pub(crate) unsafe fn destack_net_get_no_delay(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(binding, handle)?;
    let value = get_socket_bool(socket, IPPROTO_TCP, TCP_NODELAY, "getsockopt(TCP_NODELAY)")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read SO_REUSEADDR.
pub(crate) unsafe fn destack_net_get_reuse_addr(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(binding, handle)?;
    let value = get_socket_bool(socket, SOL_SOCKET, SO_REUSEADDR, "getsockopt(SO_REUSEADDR)")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read SO_REUSEPORT.
pub(crate) unsafe fn destack_net_get_reuse_port(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read SO_REUSE_UNICASTPORT
    let socket = socket_descriptor(binding, handle)?;
    let mut value: u32 = 0;
    let mut length = mem::size_of::<u32>() as i32;
    let rc = unsafe {
        getsockopt(
            socket,
            SOL_SOCKET,
            SO_REUSE_UNICASTPORT,
            &mut value as *mut _ as *mut u8,
            &mut length,
        )
    };
    if rc != 0 {
        let code = core_platform::last_wsa_error_code();
        if is_windows_option_not_supported(code) {
            return Err(windows_option_not_supported_error(
                "destack.net.getReusePort",
            ));
        }

        return Err(core_platform::net_error_with_code(
            "getsockopt(SO_REUSE_UNICASTPORT)",
            core_platform::last_wsa_error_code(),
        ));
    }

    unsafe {
        *out = value != 0;
    }

    Ok(())
}

/// Read socket linger settings.
pub(crate) unsafe fn destack_net_get_linger(
    binding: &BindingCallContext,
    out: *mut Linger,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(binding, handle)?;
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
        return Err(core_platform::net_error_with_code(
            "getsockopt(SO_LINGER)",
            core_platform::last_wsa_error_code(),
        ));
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
pub(crate) unsafe fn destack_net_get_recv_buffer(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(binding, handle)?;
    let value = get_socket_u32(socket, SOL_SOCKET, SO_RCVBUF, "getsockopt(SO_RCVBUF)")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read the send buffer size.
pub(crate) unsafe fn destack_net_get_send_buffer(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(binding, handle)?;
    let value = get_socket_u32(socket, SOL_SOCKET, SO_SNDBUF, "getsockopt(SO_SNDBUF)")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read broadcast mode.
pub(crate) unsafe fn destack_net_get_broadcast(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(binding, handle)?;
    let value = get_socket_bool(socket, SOL_SOCKET, SO_BROADCAST, "getsockopt(SO_BROADCAST)")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read the IP time-to-live.
pub(crate) unsafe fn destack_net_get_ttl(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(binding, handle)?;
    let value = get_socket_u32(socket, IPPROTO_IP, IP_TTL, "getsockopt(IP_TTL)")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read the IP type-of-service field.
pub(crate) unsafe fn destack_net_get_tos(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(binding, handle)?;
    let value = get_socket_u32(socket, IPPROTO_IP, IP_TOS, "getsockopt(IP_TOS)")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read the read timeout in milliseconds.
pub(crate) unsafe fn destack_net_get_read_timeout(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(binding, handle)?;
    let value = get_socket_u32(socket, SOL_SOCKET, SO_RCVTIMEO, "getsockopt(SO_RCVTIMEO)")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read the write timeout in milliseconds.
pub(crate) unsafe fn destack_net_get_write_timeout(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(binding, handle)?;
    let value = get_socket_u32(socket, SOL_SOCKET, SO_SNDTIMEO, "getsockopt(SO_SNDTIMEO)")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read IPv6-only mode.
pub(crate) unsafe fn destack_net_get_only_v6(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and read the socket option
    let socket = socket_descriptor(binding, handle)?;
    let value = get_socket_bool(socket, IPPROTO_IPV6, IPV6_V6ONLY, "getsockopt(IPV6_V6ONLY)")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Set one raw socket option payload.
pub(crate) unsafe fn destack_net_set_sock_opt_raw(
    binding: &BindingCallContext,
    handle: SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    value: NativeSlice<u8>,
) -> RuntimeResult<()> {
    // resolve socket descriptor and value payload
    let socket = socket_descriptor(binding, handle)?;
    let value = unsafe { value.as_slice()? };

    // apply host socket option bytes
    let rc = unsafe {
        setsockopt(
            socket,
            level.0 as i32,
            name.0 as i32,
            value.as_ptr(),
            value.len() as i32,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "setsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Read one raw socket option payload.
pub(crate) unsafe fn destack_net_get_sock_opt_raw(
    binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: SocketHandle,
    level: SocketOptionLevel,
    name: SocketOptionName,
    maxbytes: u32,
) -> RuntimeResult<()> {
    // validate output pointer and output cap
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

    // query host socket option bytes
    let socket = socket_descriptor(binding, handle)?;
    let mut value = vec![0u8; maxbytes as usize];
    let mut length = value.len() as i32;
    let rc = unsafe {
        getsockopt(
            socket,
            level.0 as i32,
            name.0 as i32,
            value.as_mut_ptr(),
            &mut length,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "getsockopt",
            core_platform::last_wsa_error_code(),
        ));
    }
    value.truncate(length.max(0) as usize);

    // write output payload
    unsafe {
        *out = binding.store_array(value);
    }

    Ok(())
}

/// Set packet timestamping mode.
pub(crate) unsafe fn destack_net_set_timestamping(
    binding: &BindingCallContext,
    handle: SocketHandle,
    mode: SocketTimestampingMode,
) -> RuntimeResult<()> {
    // reject hardware mode where winsock exposes only software timestamps
    if mode == SocketTimestampingMode::Hardware {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.net.setTimestamping",
        ))
        .boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(binding, handle)?;

    // map runtime mode to SO_TIMESTAMP values
    let value: u32 = if mode == SocketTimestampingMode::Off {
        0
    } else {
        1
    };
    let rc = unsafe {
        setsockopt(
            socket,
            SOL_SOCKET,
            SO_TIMESTAMP as i32,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "setsockopt(SO_TIMESTAMP)",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Read packet timestamping mode.
pub(crate) unsafe fn destack_net_get_timestamping(
    binding: &BindingCallContext,
    out: *mut SocketTimestampingMode,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor and read SO_TIMESTAMP mode
    let socket = socket_descriptor(binding, handle)?;
    let value = get_socket_u32(
        socket,
        SOL_SOCKET,
        SO_TIMESTAMP as i32,
        "getsockopt(SO_TIMESTAMP)",
    )?;

    // map host value into runtime timestamp mode
    let mode = if value == 0 {
        SocketTimestampingMode::Off
    } else {
        SocketTimestampingMode::Software
    };
    unsafe {
        *out = mode;
    }

    Ok(())
}

/// Set socket packet mark.
pub(crate) unsafe fn destack_net_set_packet_mark(
    _binding: &BindingCallContext,
    _handle: SocketHandle,
    _mark: u32,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.setPacketMark")).boxed())
}

/// Read socket packet mark.
pub(crate) unsafe fn destack_net_get_packet_mark(
    _binding: &BindingCallContext,
    _out: *mut u32,
    _handle: SocketHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.net.getPacketMark")).boxed())
}

/// Select the default IPv4 multicast interface for one socket.
pub(crate) unsafe fn destack_net_set_multicast_interface_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    interface_address: NativeStringRef,
) -> RuntimeResult<()> {
    // resolve socket and parse interface address
    let socket = socket_descriptor(binding, handle)?;
    let interface_address = unsafe { interface_address.as_str()? };
    let interface_address = parse_ipv4_interface(interface_address)?;
    let value = IN_ADDR {
        S_un: IN_ADDR_0 {
            S_addr: u32::from(interface_address).to_be(),
        },
    };

    // apply IP_MULTICAST_IF
    let rc = unsafe {
        setsockopt(
            socket,
            IPPROTO_IP,
            IP_MULTICAST_IF,
            &value as *const _ as *const u8,
            mem::size_of::<IN_ADDR>() as i32,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "setsockopt(IP_MULTICAST_IF)",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Read the default IPv4 multicast interface for one socket.
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
    let socket = socket_descriptor(binding, handle)?;
    let mut value = IN_ADDR {
        S_un: IN_ADDR_0 { S_addr: 0 },
    };
    let mut length = mem::size_of::<IN_ADDR>() as i32;
    let rc = unsafe {
        getsockopt(
            socket,
            IPPROTO_IP,
            IP_MULTICAST_IF,
            &mut value as *mut _ as *mut u8,
            &mut length,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "getsockopt(IP_MULTICAST_IF)",
            core_platform::last_wsa_error_code(),
        ));
    }

    // encode and write string output
    let address = Ipv4Addr::from(u32::from_be(unsafe { value.S_un.S_addr }));
    let address = binding.store_string(&address.to_string());
    unsafe {
        *out = address;
    }

    Ok(())
}

/// Select the default IPv6 multicast interface for one socket.
pub(crate) unsafe fn destack_net_set_multicast_interface_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    interface_index: u32,
) -> RuntimeResult<()> {
    // resolve socket and apply IPV6_MULTICAST_IF
    let socket = socket_descriptor(binding, handle)?;
    let value = interface_index;
    let rc = unsafe {
        setsockopt(
            socket,
            IPPROTO_IPV6,
            IPV6_MULTICAST_IF,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "setsockopt(IPV6_MULTICAST_IF)",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Read the default IPv6 multicast interface for one socket.
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
    let socket = socket_descriptor(binding, handle)?;
    let mut value: u32 = 0;
    let mut length = mem::size_of::<u32>() as i32;
    let rc = unsafe {
        getsockopt(
            socket,
            IPPROTO_IPV6,
            IPV6_MULTICAST_IF,
            &mut value as *mut _ as *mut u8,
            &mut length,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "getsockopt(IPV6_MULTICAST_IF)",
            core_platform::last_wsa_error_code(),
        ));
    }

    // write output value
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Read multicast loopback mode.
pub(crate) unsafe fn destack_net_get_multicast_loop(
    binding: &BindingCallContext,
    out: *mut bool,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // select option by socket family
    let socket = socket_descriptor(binding, handle)?;
    let family = socket_family(socket)?;
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

    // read host loopback mode
    let value = get_socket_u32(socket, level, option, "getsockopt(multicast_loop)")?;
    unsafe {
        *out = value != 0;
    }

    Ok(())
}

/// Read multicast TTL or hop-limit.
pub(crate) unsafe fn destack_net_get_multicast_ttl(
    binding: &BindingCallContext,
    out: *mut u32,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // select option by socket family
    let socket = socket_descriptor(binding, handle)?;
    let family = socket_family(socket)?;
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

    // read host ttl value
    let value = get_socket_u32(socket, level, option, "getsockopt(multicast_ttl)")?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Join one IPv4 source-specific multicast membership.
pub(crate) unsafe fn destack_net_join_multicast_source_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV4,
) -> RuntimeResult<()> {
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
    let interface_address = parse_ipv4_interface(interface_address)?;

    // apply host source-specific membership
    let socket = socket_descriptor(binding, handle)?;
    let request = IP_MREQ_SOURCE {
        imr_multiaddr: IN_ADDR {
            S_un: IN_ADDR_0 {
                S_addr: u32::from(group).to_be(),
            },
        },
        imr_sourceaddr: IN_ADDR {
            S_un: IN_ADDR_0 {
                S_addr: u32::from(source).to_be(),
            },
        },
        imr_interface: IN_ADDR {
            S_un: IN_ADDR_0 {
                S_addr: u32::from(interface_address).to_be(),
            },
        },
    };
    let rc = unsafe {
        setsockopt(
            socket,
            IPPROTO_IP,
            IP_ADD_SOURCE_MEMBERSHIP,
            &request as *const _ as *const u8,
            mem::size_of::<IP_MREQ_SOURCE>() as i32,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "setsockopt(IP_ADD_SOURCE_MEMBERSHIP)",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Join one IPv6 source-specific multicast membership.
pub(crate) unsafe fn destack_net_join_multicast_source_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV6,
) -> RuntimeResult<()> {
    // decode source membership fields
    let group = unsafe { membership.group.as_str()? };
    let source = unsafe { membership.source.as_str()? };
    let interface_index = membership.interface_index;

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

    // build one group-source request and apply host membership
    let socket = socket_descriptor(binding, handle)?;
    let request = GROUP_SOURCE_REQ {
        gsr_interface: interface_index,
        gsr_group: socket_address_storage_ipv6(group),
        gsr_source: socket_address_storage_ipv6(source),
    };
    let rc = unsafe {
        setsockopt(
            socket,
            IPPROTO_IPV6,
            MCAST_JOIN_SOURCE_GROUP as i32,
            &request as *const _ as *const u8,
            mem::size_of::<GROUP_SOURCE_REQ>() as i32,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "setsockopt(MCAST_JOIN_SOURCE_GROUP)",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Leave one IPv4 source-specific multicast membership.
pub(crate) unsafe fn destack_net_leave_multicast_source_v4(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV4,
) -> RuntimeResult<()> {
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
    let interface_address = parse_ipv4_interface(interface_address)?;

    // apply host source-specific membership removal
    let socket = socket_descriptor(binding, handle)?;
    let request = IP_MREQ_SOURCE {
        imr_multiaddr: IN_ADDR {
            S_un: IN_ADDR_0 {
                S_addr: u32::from(group).to_be(),
            },
        },
        imr_sourceaddr: IN_ADDR {
            S_un: IN_ADDR_0 {
                S_addr: u32::from(source).to_be(),
            },
        },
        imr_interface: IN_ADDR {
            S_un: IN_ADDR_0 {
                S_addr: u32::from(interface_address).to_be(),
            },
        },
    };
    let rc = unsafe {
        setsockopt(
            socket,
            IPPROTO_IP,
            IP_DROP_SOURCE_MEMBERSHIP,
            &request as *const _ as *const u8,
            mem::size_of::<IP_MREQ_SOURCE>() as i32,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "setsockopt(IP_DROP_SOURCE_MEMBERSHIP)",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Leave one IPv6 source-specific multicast membership.
pub(crate) unsafe fn destack_net_leave_multicast_source_v6(
    binding: &BindingCallContext,
    handle: SocketHandle,
    membership: UdpSourceMembershipV6,
) -> RuntimeResult<()> {
    // decode source membership fields
    let group = unsafe { membership.group.as_str()? };
    let source = unsafe { membership.source.as_str()? };
    let interface_index = membership.interface_index;

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

    // build one group-source request and apply host membership removal
    let socket = socket_descriptor(binding, handle)?;
    let request = GROUP_SOURCE_REQ {
        gsr_interface: interface_index,
        gsr_group: socket_address_storage_ipv6(group),
        gsr_source: socket_address_storage_ipv6(source),
    };
    let rc = unsafe {
        setsockopt(
            socket,
            IPPROTO_IPV6,
            MCAST_LEAVE_SOURCE_GROUP as i32,
            &request as *const _ as *const u8,
            mem::size_of::<GROUP_SOURCE_REQ>() as i32,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "setsockopt(MCAST_LEAVE_SOURCE_GROUP)",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}

/// Open a raw IP socket.
pub(crate) unsafe fn destack_net_raw_socket(
    binding: &BindingCallContext,
    out: *mut SocketHandle,
    family: SocketFamily,
    protocol: i32,
) -> RuntimeResult<()> {
    // validate output pointer and family
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    if family == SocketFamily::Unspecified {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "family",
            "unspecified family is not valid for raw sockets",
        ))
        .boxed());
    }

    // ensure winsock and create raw socket
    core_platform::ensure_winsock()?;
    let socket = unsafe { socket(socket_family_to_raw(family), SOCK_RAW, protocol) };
    if socket == windows_sys::Win32::Networking::WinSock::INVALID_SOCKET {
        return Err(core_platform::net_error_with_code(
            "socket",
            core_platform::last_wsa_error_code(),
        ));
    }

    // register socket resource
    let entry = ResourceEntry::new(ResourceKind::Socket)
        .with_socket(socket as _)
        .with_finalizer(SocketFinalizer::new(socket));
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
pub(crate) unsafe fn destack_net_raw_set_header_included(
    binding: &BindingCallContext,
    handle: SocketHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    // resolve socket descriptor
    let socket = socket_descriptor(binding, handle)?;
    let value: u32 = if enabled { 1 } else { 0 };

    // apply IP_HDRINCL
    let rc = unsafe {
        setsockopt(
            socket,
            IPPROTO_IP,
            IP_HDRINCL,
            &value as *const _ as *const u8,
            mem::size_of::<u32>() as i32,
        )
    };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "setsockopt(IP_HDRINCL)",
            core_platform::last_wsa_error_code(),
        ));
    }

    Ok(())
}
