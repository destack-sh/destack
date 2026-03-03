#![allow(unused_imports)]

use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::OsPath;
use crate::platform::net::{core as core_net, *};
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::{BindingCallContext, NativeStringRef};

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::unix::io::RawFd;

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

/// Convert one runtime reverse-lookup bitmask into libc flags.
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

    // map runtime bits to libc NI_* flags
    let mut native_flags = 0;
    if (flags.0 & REVERSE_LOOKUP_FLAG_NUMERIC_HOST) != 0 {
        native_flags |= libc::NI_NUMERICHOST;
    }
    if (flags.0 & REVERSE_LOOKUP_FLAG_NUMERIC_SERVICE) != 0 {
        native_flags |= libc::NI_NUMERICSERV;
    }
    if (flags.0 & REVERSE_LOOKUP_FLAG_NAME_REQUIRED) != 0 {
        native_flags |= libc::NI_NAMEREQD;
    }
    if (flags.0 & REVERSE_LOOKUP_FLAG_DGRAM) != 0 {
        native_flags |= libc::NI_DGRAM;
    }
    if (flags.0 & REVERSE_LOOKUP_FLAG_NO_FQDN) != 0 {
        native_flags |= libc::NI_NOFQDN;
    }

    Ok(native_flags)
}

/// Decode one raw socket-address payload into sockaddr storage.
fn reverse_lookup_storage(
    address: SocketAddress,
) -> RuntimeResult<(libc::sockaddr_storage, libc::socklen_t)> {
    // decode and validate the raw socket address
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

    // copy bytes into sockaddr storage
    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            &mut storage as *mut _ as *mut u8,
            bytes.len(),
        );
    }
    let length = bytes.len() as libc::socklen_t;

    Ok((storage, length))
}

/// Interface accumulator while collecting host interface rows.
struct InterfaceRow {
    /// Interface name.
    name: String,
    /// Interface index.
    index: u32,
    /// Interface flags.
    flags: u64,
    /// Interface mtu.
    mtu: u32,
    /// Interface hardware address bytes.
    mac_address: Vec<u8>,
    /// Interface address rows.
    addresses: Vec<SocketAddress>,
}

/// RAII guard for one `ifaddrs` snapshot.
struct IfAddrsGuard(*mut libc::ifaddrs);

impl Drop for IfAddrsGuard {
    fn drop(&mut self) {
        unsafe {
            libc::freeifaddrs(self.0);
        }
    }
}

/// Build one socket-address row from one `ifaddrs` address pointer.
fn interface_address_from_ifaddrs(
    context: &BindingCallContext,
    pointer: *const libc::sockaddr,
) -> RuntimeResult<Option<SocketAddress>> {
    // skip missing address pointers
    if pointer.is_null() {
        return Ok(None);
    }

    // read the socket family and limit to address families we can encode
    let family = unsafe { (*pointer).sa_family as libc::c_int };
    let length = if family == libc::AF_INET {
        std::mem::size_of::<libc::sockaddr_in>()
    } else if family == libc::AF_INET6 {
        std::mem::size_of::<libc::sockaddr_in6>()
    } else {
        return Ok(None);
    };

    // copy sockaddr bytes into fixed storage
    let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            pointer as *const u8,
            &mut storage as *mut _ as *mut u8,
            length,
        );
    }

    // encode one socket-address row
    let address = socket_address_from_storage(context, &storage, length as libc::socklen_t)?;
    Ok(Some(address))
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
    context: &BindingCallContext,
    out: *mut u32,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
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

    // resolve the interface index
    let index = unsafe { libc::if_nametoindex(name.as_ptr()) };
    if index == 0 {
        return Err(core_platform::net_error("if_nametoindex"));
    }

    // write the output
    unsafe {
        *out = index;
    }

    let _ = context;
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

    // query one ifaddrs snapshot
    let mut head = std::ptr::null_mut();
    let rc = unsafe { libc::getifaddrs(&mut head) };
    if rc != 0 {
        return Err(core_platform::net_error("getifaddrs"));
    }

    // release the ifaddrs snapshot on scope exit
    let _guard = IfAddrsGuard(head);

    // collect interface rows from snapshot entries
    let mut rows = Vec::<InterfaceRow>::new();
    let mut row_by_name = HashMap::<String, usize>::new();
    let mut cursor = head;
    while !cursor.is_null() {
        let entry = unsafe { &*cursor };

        // decode the interface name
        if entry.ifa_name.is_null() {
            cursor = entry.ifa_next;
            continue;
        }
        let name = unsafe { CStr::from_ptr(entry.ifa_name) }
            .to_string_lossy()
            .to_string();

        // create one row on first sight of the interface name
        let row_index = if let Some(row_index) = row_by_name.get(&name) {
            *row_index
        } else {
            let index = unsafe { libc::if_nametoindex(entry.ifa_name) };
            let row_index = rows.len();
            rows.push(InterfaceRow {
                name: name.clone(),
                index,
                flags: 0,
                mtu: 0,
                mac_address: Vec::new(),
                addresses: Vec::new(),
            });
            row_by_name.insert(name.clone(), row_index);
            row_index
        };

        // fold host interface flags
        let row = &mut rows[row_index];
        row.flags |= entry.ifa_flags as u64;

        // append one address row when the address family is supported
        if let Some(address) = interface_address_from_ifaddrs(context, entry.ifa_addr)? {
            row.addresses.push(address);
        }

        cursor = entry.ifa_next;
    }

    // encode rows into ABI interface values
    let mut interfaces = Vec::with_capacity(rows.len());
    for row in rows {
        interfaces.push(NetInterface {
            name: context.store_string(&row.name),
            index: row.index,
            flags: NetInterfaceFlags(row.flags),
            mtu: row.mtu,
            mac_address: context.store_array(row.mac_address),
            addresses: context.store_array(row.addresses),
        });
    }

    // write output array
    unsafe {
        *out = context.store_array(interfaces);
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
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // allocate interface name storage
    let mut buffer = vec![0 as libc::c_char; libc::IF_NAMESIZE];

    // resolve the interface name
    let pointer = unsafe { libc::if_indextoname(index, buffer.as_mut_ptr()) };
    if pointer.is_null() {
        return Err(core_platform::net_error("if_indextoname"));
    }

    // decode and store the string output
    let name = unsafe { CStr::from_ptr(pointer) };
    let name = name.to_string_lossy().to_string();
    let name = context.store_string(&name);
    unsafe {
        *out = name;
    }

    Ok(())
}

/// Read the local socket address as raw bytes.
pub(crate) unsafe fn destack_net_local_address_raw(
    context: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the local address on unix platforms
    // resolve the socket descriptor
    let fd = socket_descriptor(context, handle)?;

    // query the socket address
    let address = socket_address_raw_from_fd(context, fd, "getsockname", libc::getsockname)?;

    // write the output
    unsafe {
        *out = address;
    }

    Ok(())
}

/// Read the remote socket address as raw bytes.
pub(crate) unsafe fn destack_net_peer_address_raw(
    context: &BindingCallContext,
    out: *mut SocketAddress,
    handle: SocketHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read the peer address on unix platforms
    // resolve the socket descriptor
    let fd = socket_descriptor(context, handle)?;

    // query the socket address
    let address = socket_address_raw_from_fd(context, fd, "getpeername", libc::getpeername)?;

    // write the output
    unsafe {
        *out = address;
    }

    Ok(())
}

/// Resolve a host and service query into raw socket addresses.
///
/// Resolve the requested host and service to one or more socket addresses.
/// Name-service order, search domains, and canonicalization follow host resolver policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getaddrinfo(3) on Unix and GetAddrInfoW on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.dns`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_resolve(
    context: &BindingCallContext,
    out: *mut NativeArray<SocketAddress>,
    host: NativeStringRef,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the host string
    let host = unsafe { host.as_str()? };
    if host.contains('\0') {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "host contains nul byte",
        ))
        .boxed());
    }

    // enforce numeric-only resolution when requested
    if flags.0 & 0x4 != 0 && host.parse::<IpAddr>().is_err() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "numeric host required",
        ))
        .boxed());
    }

    // build addrinfo hints
    let mut hints: libc::addrinfo = unsafe { std::mem::zeroed() };
    hints.ai_family = match family {
        SocketFamily::IPv4 => libc::AF_INET,
        SocketFamily::IPv6 => libc::AF_INET6,
        SocketFamily::Unspecified => libc::AF_UNSPEC,
    };
    hints.ai_socktype = libc::SOCK_STREAM;
    hints.ai_protocol = libc::IPPROTO_TCP;
    hints.ai_flags = 0;
    if flags.0 & 0x1 != 0 {
        hints.ai_flags |= libc::AI_PASSIVE;
    }
    if flags.0 & 0x2 != 0 {
        hints.ai_flags |= libc::AI_CANONNAME;
    }
    if flags.0 & 0x4 != 0 {
        hints.ai_flags |= libc::AI_NUMERICHOST;
    }
    if flags.0 & 0x8 != 0 {
        hints.ai_flags |= libc::AI_NUMERICSERV;
    }
    if flags.0 & 0x10 != 0 {
        hints.ai_flags |= libc::AI_V4MAPPED;
    }
    if flags.0 & 0x20 != 0 {
        hints.ai_flags |= libc::AI_ALL;
    }
    if flags.0 & 0x40 != 0 {
        hints.ai_flags |= libc::AI_ADDRCONFIG;
    }

    // resolve addresses
    let host_c = CString::new(host).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "host",
            "host contains nul byte",
        ))
        .boxed()
    })?;
    let service = CString::new(port.to_string()).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "port",
            "port conversion failed",
        ))
        .boxed()
    })?;
    let mut result: *mut libc::addrinfo = std::ptr::null_mut();
    let rc = unsafe { libc::getaddrinfo(host_c.as_ptr(), service.as_ptr(), &hints, &mut result) };
    if rc != 0 {
        let error = unsafe { CStr::from_ptr(libc::gai_strerror(rc)) }
            .to_string_lossy()
            .to_string();
        return Err(
            RuntimeError::from(PlatformError::io(format!("getaddrinfo failed: {error}"))).boxed(),
        );
    }

    struct AddrInfoGuard(*mut libc::addrinfo);
    impl Drop for AddrInfoGuard {
        fn drop(&mut self) {
            unsafe {
                libc::freeaddrinfo(self.0);
            }
        }
    }
    let _guard = AddrInfoGuard(result);

    // collect addresses
    let mut addresses = Vec::new();
    let mut current = result;
    while !current.is_null() {
        let info = unsafe { &*current };
        let storage = unsafe { &*(info.ai_addr as *const libc::sockaddr_storage) };
        let address = socket_address_from_storage(context, storage, info.ai_addrlen)?;
        addresses.push(address);
        current = info.ai_next;
    }

    unsafe {
        *out = context.store_array(addresses);
    }

    Ok(())
}

/// Reverse lookup a raw socket address into host and service names.
pub(crate) unsafe fn destack_net_reverse_lookup_names(
    context: &BindingCallContext,
    out: *mut NativeArray<ReverseLookupName>,
    address: SocketAddress,
    flags: ReverseLookupFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the socket address and reverse-lookup flags
    let (storage, length) = reverse_lookup_storage(address)?;
    let native_flags = reverse_lookup_native_flags(flags)?;

    // allocate host and service output buffers
    #[cfg(target_os = "android")]
    let mut host = [0 as libc::c_char; libc::NI_MAXHOST];
    #[cfg(not(target_os = "android"))]
    let mut host = [0 as libc::c_char; libc::NI_MAXHOST as usize];
    #[cfg(target_os = "android")]
    let mut service = [0 as libc::c_char; libc::NI_MAXSERV];
    #[cfg(not(target_os = "android"))]
    let mut service = [0 as libc::c_char; libc::NI_MAXSERV as usize];

    // resolve host and service for the socket address
    #[cfg(target_os = "android")]
    let rc = unsafe {
        libc::getnameinfo(
            &storage as *const _ as *const libc::sockaddr,
            length,
            host.as_mut_ptr(),
            host.len(),
            service.as_mut_ptr(),
            service.len(),
            native_flags,
        )
    };
    #[cfg(not(target_os = "android"))]
    let rc = unsafe {
        libc::getnameinfo(
            &storage as *const _ as *const libc::sockaddr,
            length,
            host.as_mut_ptr(),
            host.len() as libc::socklen_t,
            service.as_mut_ptr(),
            service.len() as libc::socklen_t,
            native_flags,
        )
    };
    if rc != 0 {
        let error = unsafe { CStr::from_ptr(libc::gai_strerror(rc)) }
            .to_string_lossy()
            .to_string();
        return Err(RuntimeError::from(PlatformError::io(format!(
            "reverse lookup failed: {error}"
        )))
        .boxed());
    }

    // decode resolved host and service strings
    let host = unsafe { CStr::from_ptr(host.as_ptr()) }
        .to_string_lossy()
        .to_string();
    let service = unsafe { CStr::from_ptr(service.as_ptr()) }
        .to_string_lossy()
        .to_string();

    // encode one reverse-lookup output record
    let names = vec![ReverseLookupName {
        host: context.store_string(&host),
        service: context.store_string(&service),
    }];
    unsafe {
        *out = context.store_array(names);
    }

    Ok(())
}

/// Reverse lookup a raw socket address into host and service names.
///
/// Resolve a socket address back to host and service names.
/// Reverse lookup policy and name formatting follow host resolver behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the socket feature is unavailable.
/// Uses getnameinfo(3) on Unix and GetNameInfoW on Windows.
///
/// # Errors
/// Returns netAddressNotAvailable, netConnectionRefused, netTimedOut, netConnectionReset, netBrokenPipe, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `net.dns`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_net_reverse_lookup(
    context: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    address: SocketAddress,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve host and service names with default reverse-lookup policy
    let mut names = std::mem::MaybeUninit::<NativeArray<ReverseLookupName>>::uninit();
    unsafe {
        destack_net_reverse_lookup_names(
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

/// Resolve host and port into raw socket addresses.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_resolve_raw(
    context: &BindingCallContext,
    out: *mut NativeArray<SocketAddress>,
    host: NativeStringRef,
    port: u16,
    family: SocketFamily,
    flags: ResolveFlags,
) -> RuntimeResult<()> {
    unsafe { super::destack_net_resolve(context, out, host, port, family, flags) }
}

/// Reverse lookup a raw socket address into hostnames.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_reverse_lookup_raw(
    context: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    address: SocketAddress,
) -> RuntimeResult<()> {
    unsafe { super::destack_net_reverse_lookup(context, out, address) }
}

/// Reverse lookup a raw socket address into host and service names.
#[cfg(unix)]
pub(crate) unsafe fn destack_net_reverse_lookup_names_raw(
    context: &BindingCallContext,
    out: *mut NativeArray<ReverseLookupName>,
    address: SocketAddress,
    flags: ReverseLookupFlags,
) -> RuntimeResult<()> {
    unsafe { super::destack_net_reverse_lookup_names(context, out, address, flags) }
}
