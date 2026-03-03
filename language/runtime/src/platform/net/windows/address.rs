use std::ffi::{CStr, CString};
use std::mem;
use std::net::IpAddr;
use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_PREFIX, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH, if_indextoname,
    if_nametoindex,
};
use windows_sys::Win32::Networking::WinSock::{
    ADDRINFOW, AF_INET, AF_INET6, AF_UNSPEC, AI_ADDRCONFIG, AI_ALL, AI_CANONNAME, AI_NUMERICHOST,
    AI_NUMERICSERV, AI_PASSIVE, AI_V4MAPPED, FreeAddrInfoW, GetAddrInfoW, GetNameInfoW, NI_DGRAM,
    NI_MAXHOST, NI_MAXSERV, NI_NAMEREQD, NI_NOFQDN, NI_NUMERICHOST, NI_NUMERICSERV, SOCKADDR,
    SOCKADDR_STORAGE, getpeername, getsockname,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::net::{
    NetInterface, NetInterfaceFlags, ResolveFlags, ReverseLookupFlags, ReverseLookupName,
    SocketAddress, SocketFamily, SocketHandle,
};
use crate::platform::{NativeArray, NativeStringRef, PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

/// Reverse-lookup flag bit for numeric host output.
const REVERSE_LOOKUP_FLAG_NUMERIC_HOST: u32 = 0x1;
/// Reverse-lookup flag bit for numeric service output.
const REVERSE_LOOKUP_FLAG_NUMERIC_SERVICE: u32 = 0x2;
/// Reverse-lookup flag bit to require a resolvable host name.
const REVERSE_LOOKUP_FLAG_NAME_REQUIRED: u32 = 0x4;
/// Reverse-lookup flag bit for datagram service names.
const REVERSE_LOOKUP_FLAG_DGRAM: u32 = 0x8;
/// Reverse-lookup flag bit to drop the default domain where supported.
const REVERSE_LOOKUP_FLAG_NO_FQDN: u32 = 0x10;
/// Reverse-lookup bitmask of all supported flags.
const REVERSE_LOOKUP_SUPPORTED_FLAGS: u32 = REVERSE_LOOKUP_FLAG_NUMERIC_HOST
    | REVERSE_LOOKUP_FLAG_NUMERIC_SERVICE
    | REVERSE_LOOKUP_FLAG_NAME_REQUIRED
    | REVERSE_LOOKUP_FLAG_DGRAM
    | REVERSE_LOOKUP_FLAG_NO_FQDN;

/// Convert one runtime reverse-lookup bitmask into WinSock flags.
fn reverse_lookup_native_flags(flags: ReverseLookupFlags) -> RuntimeResult<i32> {
    // reject unknown runtime flags eagerly
    let unsupported_flags = flags.0 & !REVERSE_LOOKUP_SUPPORTED_FLAGS;
    if unsupported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            format!("unsupported reverse-lookup flags: {unsupported_flags:#x}"),
        ))
        .boxed());
    }

    // map runtime bits to WinSock NI_* flags
    let mut native_flags = 0;
    if (flags.0 & REVERSE_LOOKUP_FLAG_NUMERIC_HOST) != 0 {
        native_flags |= NI_NUMERICHOST as i32;
    }
    if (flags.0 & REVERSE_LOOKUP_FLAG_NUMERIC_SERVICE) != 0 {
        native_flags |= NI_NUMERICSERV as i32;
    }
    if (flags.0 & REVERSE_LOOKUP_FLAG_NAME_REQUIRED) != 0 {
        native_flags |= NI_NAMEREQD as i32;
    }
    if (flags.0 & REVERSE_LOOKUP_FLAG_DGRAM) != 0 {
        native_flags |= NI_DGRAM as i32;
    }
    if (flags.0 & REVERSE_LOOKUP_FLAG_NO_FQDN) != 0 {
        native_flags |= NI_NOFQDN as i32;
    }

    Ok(native_flags)
}

/// Free one address-info chain on drop.
struct AddrInfoGuard {
    /// The address-info chain pointer.
    result: *mut ADDRINFOW,
}

impl Drop for AddrInfoGuard {
    /// Free the native address-info chain.
    fn drop(&mut self) {
        if !self.result.is_null() {
            unsafe {
                FreeAddrInfoW(self.result);
            }
        }
    }
}

/// Decode one nul-terminated wide pointer into one runtime string.
fn string_from_wide_pointer(pointer: *const u16, label: &str) -> RuntimeResult<String> {
    // reject null pointers
    if pointer.is_null() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "wide string pointer is null",
        ))
        .boxed());
    }

    // scan one nul-terminated string length
    let mut length = 0usize;
    loop {
        let value = unsafe { *pointer.add(length) };
        if value == 0 {
            break;
        }
        length = length.saturating_add(1);
    }

    // decode the wide string payload
    let units = unsafe { std::slice::from_raw_parts(pointer, length) };
    core_platform::string_from_wide(label, units)
}

/// Resolve an interface name to an index.
///
/// Maps a host interface name to its numeric index for route and multicast operations.
/// The mapping follows host network namespace rules.
///
/// # Platform
/// Unix and Windows.
/// Uses if_nametoindex on Unix and ConvertInterfaceAliasToLuid plus ConvertInterfaceLuidToIndex on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.interface`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_interface_index(
    _context: &BindingCallContext,
    out: *mut u32,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode and validate the interface name
    let name = unsafe { name.as_str()? };
    let name = CString::new(name).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "name",
            "interface name contains nul byte",
        ))
        .boxed()
    })?;

    // resolve interface index
    let index = unsafe { if_nametoindex(name.as_ptr() as *const u8) };
    if index == 0 {
        return Err(core_platform::net_error_with_code(
            "if_nametoindex",
            core_platform::last_wsa_error_code(),
        ));
    }

    // write output value
    unsafe {
        *out = index;
    }

    Ok(())
}

/// Resolve an interface index to a name.
///
/// Maps a numeric host interface index to its canonical interface name.
/// The mapping follows host network namespace rules.
///
/// # Platform
/// Unix and Windows.
/// Uses if_indextoname on Unix and ConvertInterfaceIndexToLuid plus ConvertInterfaceLuidToAlias on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.interface`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_interface_name(
    context: &BindingCallContext,
    out: *mut NativeStringRef,
    index: u32,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the interface name
    let mut buffer = vec![0u8; 256];
    let pointer = unsafe { if_indextoname(index, buffer.as_mut_ptr()) };
    if pointer.is_null() {
        return Err(core_platform::net_error_with_code(
            "if_indextoname",
            core_platform::last_wsa_error_code(),
        ));
    }

    // decode and store output string
    let name = unsafe { CStr::from_ptr(pointer as *const i8) };
    let name = name.to_string_lossy().to_string();
    let name = context.store_string(&name);
    unsafe {
        *out = name;
    }

    Ok(())
}

/// List network interfaces with addresses and flags.
///
/// Enumerates host interfaces and returns their current address records.
/// Results are snapshots and may become stale immediately after the call.
///
/// # Platform
/// Unix and Windows.
/// Uses getifaddrs on Unix and iphlpapi adapter enumeration on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netTimedOut, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `net.interface`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_list_interfaces(
    context: &BindingCallContext,
    out: *mut NativeArray<NetInterface>,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // request adapter rows with one growable buffer
    let mut size = 15_000u32;
    let mut buffer = vec![0u8; size as usize];
    let result = loop {
        let status = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                GAA_FLAG_INCLUDE_PREFIX,
                std::ptr::null(),
                buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
                &mut size,
            )
        };

        if status == ERROR_BUFFER_OVERFLOW {
            buffer.resize(size as usize, 0u8);
            continue;
        }

        break status;
    };
    if result != ERROR_SUCCESS {
        return Err(core_platform::net_error_with_code(
            "GetAdaptersAddresses",
            result as i32,
        ));
    }

    // collect interface rows from adapter output
    let mut interfaces = Vec::new();
    let mut adapter = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
    while !adapter.is_null() {
        let entry = unsafe { &*adapter };

        // resolve interface name from friendly or adapter strings
        let name = if !entry.FriendlyName.is_null() {
            string_from_wide_pointer(entry.FriendlyName, "name")?
        } else if !entry.AdapterName.is_null() {
            unsafe { CStr::from_ptr(entry.AdapterName as *const i8) }
                .to_string_lossy()
                .to_string()
        } else {
            String::new()
        };

        // resolve interface index and flag payload
        let mut index = unsafe { entry.Anonymous1.Anonymous.IfIndex };
        if index == 0 {
            index = entry.Ipv6IfIndex;
        }
        let flags = unsafe { entry.Anonymous2.Flags as u64 };

        // collect adapter hardware address bytes
        let mac_length = (entry.PhysicalAddressLength as usize).min(entry.PhysicalAddress.len());
        let mac_address = entry.PhysicalAddress[..mac_length].to_vec();

        // collect adapter unicast address rows
        let mut addresses = Vec::new();
        let mut unicast = entry.FirstUnicastAddress;
        while !unicast.is_null() {
            let unicast_entry = unsafe { &*unicast };
            let socket_address = unicast_entry.Address;

            if !socket_address.lpSockaddr.is_null() && socket_address.iSockaddrLength > 0 {
                let length = socket_address.iSockaddrLength as usize;
                let bytes = unsafe {
                    std::slice::from_raw_parts(socket_address.lpSockaddr as *const u8, length)
                };
                let family = unsafe { (*socket_address.lpSockaddr).sa_family };
                addresses.push(SocketAddress {
                    family,
                    length: socket_address.iSockaddrLength as u32,
                    bytes: context.store_array(bytes.to_vec()),
                });
            }

            unicast = unicast_entry.Next;
        }

        // encode and append one interface row
        interfaces.push(NetInterface {
            name: context.store_string(&name),
            index,
            flags: NetInterfaceFlags(flags),
            mtu: entry.Mtu,
            mac_address: context.store_array(mac_address),
            addresses: context.store_array(addresses),
        });

        adapter = entry.Next;
    }

    // write output array
    unsafe {
        *out = context.store_array(interfaces);
    }

    Ok(())
}

/// Return the local address bytes for a socket.
pub(crate) unsafe fn destack_net_local_address_raw(
    context: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(context, handle)?;

    // query the local address
    let mut storage = mem::MaybeUninit::<SOCKADDR_STORAGE>::uninit();
    let mut length = mem::size_of::<SOCKADDR_STORAGE>() as i32;
    let rc = unsafe { getsockname(socket, storage.as_mut_ptr() as *mut SOCKADDR, &mut length) };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "getsockname",
            core_platform::last_wsa_error_code(),
        ));
    }

    // decode and write the output
    let storage = unsafe { storage.assume_init() };
    let address = socket_address_raw_from_storage(context, &storage, length)?;
    unsafe {
        *out = address;
    }

    Ok(())
}

/// Return the peer address bytes for a socket.
pub(crate) unsafe fn destack_net_peer_address_raw(
    context: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the socket descriptor
    let socket = socket_descriptor(context, handle)?;

    // query the peer address
    let mut storage = mem::MaybeUninit::<SOCKADDR_STORAGE>::uninit();
    let mut length = mem::size_of::<SOCKADDR_STORAGE>() as i32;
    let rc = unsafe { getpeername(socket, storage.as_mut_ptr() as *mut SOCKADDR, &mut length) };
    if rc != 0 {
        return Err(core_platform::net_error_with_code(
            "getpeername",
            core_platform::last_wsa_error_code(),
        ));
    }

    // decode and write the output
    let storage = unsafe { storage.assume_init() };
    let address = socket_address_raw_from_storage(context, &storage, length)?;
    unsafe {
        *out = address;
    }

    Ok(())
}

/// Resolve host and port into raw socket addresses.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_resolve_raw(
    context: &BindingCallContext,
    out: *mut NativeArray<SocketAddress>,
    host: NativeStringRef,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    core_platform::ensure_winsock()?;

    // decode and validate the host
    let host = unsafe { host.as_str()? };
    if host.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "host contains nul byte",
        ))
        .boxed());
    }
    if flags.0 & 0x4 != 0 && host.parse::<IpAddr>().is_err() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "numeric host required",
        ))
        .boxed());
    }

    // build addrinfo hints
    let mut hints: ADDRINFOW = unsafe { std::mem::zeroed() };
    hints.ai_family = match family {
        SocketFamily::IPv4 => AF_INET as i32,
        SocketFamily::IPv6 => AF_INET6 as i32,
        SocketFamily::Unspecified => AF_UNSPEC as i32,
    };
    hints.ai_socktype = windows_sys::Win32::Networking::WinSock::SOCK_STREAM;
    hints.ai_protocol = windows_sys::Win32::Networking::WinSock::IPPROTO_TCP;
    hints.ai_flags = 0;
    if flags.0 & 0x1 != 0 {
        hints.ai_flags |= AI_PASSIVE as i32;
    }
    if flags.0 & 0x2 != 0 {
        hints.ai_flags |= AI_CANONNAME as i32;
    }
    if flags.0 & 0x4 != 0 {
        hints.ai_flags |= AI_NUMERICHOST as i32;
    }
    if flags.0 & 0x8 != 0 {
        hints.ai_flags |= AI_NUMERICSERV as i32;
    }
    if flags.0 & 0x10 != 0 {
        hints.ai_flags |= AI_V4MAPPED as i32;
    }
    if flags.0 & 0x20 != 0 {
        hints.ai_flags |= AI_ALL as i32;
    }
    if flags.0 & 0x40 != 0 {
        hints.ai_flags |= AI_ADDRCONFIG as i32;
    }

    // resolve addresses
    let host = core_platform::wide_with_nul(host);
    let service = core_platform::wide_with_nul(&port.to_string());
    let mut result: *mut ADDRINFOW = std::ptr::null_mut();
    let rc = unsafe { GetAddrInfoW(host.as_ptr(), service.as_ptr(), &hints, &mut result) };
    if rc != 0 {
        return Err(core_platform::net_error_with_code("GetAddrInfoW", rc));
    }
    let _guard = AddrInfoGuard { result };

    // collect each raw sockaddr result
    let mut addresses = Vec::new();
    let mut current = result;
    while !current.is_null() {
        let info = unsafe { &*current };
        if !info.ai_addr.is_null() && info.ai_addrlen > 0 {
            let bytes =
                unsafe { std::slice::from_raw_parts(info.ai_addr as *const u8, info.ai_addrlen) };
            let family = unsafe { (*info.ai_addr).sa_family };
            addresses.push(SocketAddress {
                family,
                length: info.ai_addrlen as u32,
                bytes: context.store_array(bytes.to_vec()),
            });
        }
        current = info.ai_next;
    }

    unsafe {
        *out = context.store_array(addresses);
    }

    Ok(())
}

/// Reverse lookup a raw socket address into host and service names.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_reverse_lookup_names_raw(
    context: &BindingCallContext,
    out: *mut NativeArray<ReverseLookupName>,
    address: SocketAddress,
    flags: ReverseLookupFlags,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // ensure winsock is initialized
    core_platform::ensure_winsock()?;

    // decode reverse-lookup flags
    let native_flags = reverse_lookup_native_flags(flags)?;

    // decode the raw socket address
    with_socket_address_raw(address, |sockaddr, length| {
        // resolve host and service from the socket address
        let mut host = vec![0u16; NI_MAXHOST as usize];
        let mut service = vec![0u16; NI_MAXSERV as usize];
        let rc = unsafe {
            GetNameInfoW(
                sockaddr,
                length,
                host.as_mut_ptr(),
                host.len() as u32,
                service.as_mut_ptr(),
                service.len() as u32,
                native_flags,
            )
        };
        if rc != 0 {
            return Err(core_platform::net_error_with_code("GetNameInfoW", rc));
        }

        // decode resolved host and service strings
        let host_length = host
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(host.len());
        let host = String::from_utf16(&host[..host_length]).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "address",
                "resolved host is not valid utf16",
            ))
            .boxed()
        })?;
        let service_length = service
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(service.len());
        let service = String::from_utf16(&service[..service_length]).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "address",
                "resolved service is not valid utf16",
            ))
            .boxed()
        })?;

        // encode one reverse-lookup output record
        let names = vec![ReverseLookupName {
            host: context.store_string(&host),
            service: context.store_string(&service),
        }];
        unsafe {
            *out = context.store_array(names);
        }

        Ok(())
    })
}

/// Reverse lookup a raw socket address into hostnames.
#[cfg(not(unix))]
pub(crate) unsafe fn destack_net_reverse_lookup_raw(
    context: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve host and service names with default reverse-lookup policy
    let mut names = std::mem::MaybeUninit::<NativeArray<ReverseLookupName>>::uninit();
    unsafe {
        destack_net_reverse_lookup_names_raw(
            context,
            names.as_mut_ptr(),
            address,
            ReverseLookupFlags(0),
        )
    }?;
    let names = unsafe { names.assume_init() };
    let names = unsafe { names.as_slice()? };

    // project lookup records to hostname-only payloads
    let mut hosts = Vec::with_capacity(names.len());
    for name in names {
        hosts.push(name.host);
    }
    unsafe {
        *out = context.store_array(hosts);
    }

    Ok(())
}
