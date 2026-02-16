#![cfg_attr(windows, allow(dead_code, unused_imports))]
use std::net::ToSocketAddrs;

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeErrorId, RuntimeResult, RuntimeStatus};
use crate::platform::abi::{NativeAbi, VmAbi};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    OsPath, OsPathVm, PathBytesAbi, PathEncoding, PathUtf16Abi, core as core_fs,
};
use crate::platform::resource::{ListenerHandle, ResourceId, SocketHandle};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, PlatformError, VmArray, VmSlice, VmValueCodec,
    net as platform_net,
};
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;
use platform_net::{
    AcceptFlags, KeepAliveConfig, Linger, LingerVm, ResolveFlags, ResolveQuery, ReverseLookupFlags,
    ReverseLookupName, SocketAddress, SocketAddressVm, SocketCredentials, SocketCredentialsVm,
    SocketFamily, SocketMessageFlags, SocketProtocol, SocketRecvBatchRequest, SocketRecvMessage,
    SocketSendBatchEntry, SocketSendMessage, SocketSendMessageVm, SocketShutdown, SocketType,
    UdpMessageFlags, UdpReceive, UdpReceiveVm, UdsAddress, UdsAddressKind, vm as platform_vm,
};

#[path = "harness.generated.rs"]
mod harness;

/// Selects the backing harness kind for network tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetHarnessKind {
    /// Native bindings backed by the host ABI.
    Native,
    /// VM bindings backed by VM ABI values.
    Vm,
}

/// Network harness context used by tests.
pub(crate) struct NetHarnessContext<'call> {
    /// Runtime backing this harness.
    runtime: &'call TestRuntime,
    /// Runtime call context active for this operation.
    call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    vm_context: Option<*mut ()>,
}

/// Native network harness backed by native bindings.
pub(crate) struct NativeNetHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeNetHarness {
    /// Create a new native network harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM network harness backed by VM bindings.
#[allow(dead_code)]
pub(crate) struct VmNetHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

#[allow(dead_code)]
impl VmNetHarness {
    /// Create a new VM network harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum NetHarnessHandle {
    /// Native network harness.
    Native(NativeNetHarness),
    /// VM network harness.
    Vm(VmNetHarness),
}

impl NetHarnessHandle {
    /// Run a native or VM call context around the callback.
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(NetHarnessContext<'call>) -> R,
    {
        match self {
            NetHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(NetHarnessContext {
                        runtime: &harness.runtime,
                        call_context,
                        vm_context: None,
                    })
                })
            }
            NetHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(NetHarnessContext {
                            runtime: &harness.runtime,
                            call_context,
                            vm_context: Some(vm_context),
                        })
                    })
            }
        }
    }

    /// Run a test callback that returns a runtime result.
    pub(crate) fn run<F>(&self, callback: F)
    where
        F: for<'call> FnOnce(NetHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("network harness call should succeed");
    }
}

/// Run a test against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&NetHarnessHandle),
{
    let native = NetHarnessHandle::Native(NativeNetHarness::new());
    callback(&native);
    let vm = NetHarnessHandle::Vm(VmNetHarness::new());
    callback(&vm);
}

/// Run a test callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(NetHarnessContext<'call>) -> RuntimeResult<()>,
{
    // run the callback for each harness
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

#[cfg(windows)]
impl<'call> NetHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut_manual(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }

    /// Create one connected socket pair.
    pub(crate) fn socket_pair(
        &mut self,
        family: SocketFamily,
        socket_type: SocketType,
        protocol: SocketProtocol,
    ) -> RuntimeResult<(SocketHandle, SocketHandle)> {
        match self.vm_context_mut_manual() {
            // dispatch through VM bindings
            Some(context) => {
                let pair = platform_vm::destack_net_socket_pair(
                    self.call_context,
                    context,
                    family,
                    socket_type,
                    protocol,
                )?;
                Ok((pair.first, pair.second))
            }

            // dispatch through native bindings
            None => {
                let mut pair = platform_net::SocketPair {
                    first: SocketHandle(ResourceId(0)),
                    second: SocketHandle(ResourceId(0)),
                };
                let status = unsafe {
                    platform_net::destack_net_socket_pair(&mut pair, family, socket_type, protocol)
                };
                self.status_ok(status, "socketPair")?;

                Ok((pair.first, pair.second))
            }
        }
    }

    /// Create one connected unix-domain socket pair.
    pub(crate) fn uds_socket_pair(
        &mut self,
        socket_type: SocketType,
    ) -> RuntimeResult<(SocketHandle, SocketHandle)> {
        match self.vm_context_mut_manual() {
            // dispatch through VM bindings
            Some(context) => {
                let pair = platform_vm::destack_net_uds_socket_pair(
                    self.call_context,
                    context,
                    socket_type,
                )?;
                Ok((pair.first, pair.second))
            }

            // dispatch through native bindings
            None => {
                let mut pair = platform_net::SocketPair {
                    first: SocketHandle(ResourceId(0)),
                    second: SocketHandle(ResourceId(0)),
                };
                let status =
                    unsafe { platform_net::destack_net_uds_socket_pair(&mut pair, socket_type) };
                self.status_ok(status, "udsSocketPair")?;

                Ok((pair.first, pair.second))
            }
        }
    }
}

/// Assert one result failed with one exact platform error code.
pub(crate) fn assert_platform_error_code<T>(
    result: RuntimeResult<T>,
    expected: PlatformErrorCode,
) -> RuntimeResult<()> {
    let error = match result {
        Ok(_) => panic!("operation should fail"),
        Err(error) => error,
    };
    let platform = error
        .platform_error()
        .expect("error should contain one platform error");
    assert_no_permission_denied_in_privileged_mode(platform.code, &[expected]);
    assert_eq!(platform.code, expected);

    Ok(())
}

/// Assert one result failed with one of the expected platform error codes.
pub(crate) fn assert_platform_error_codes<T>(
    result: RuntimeResult<T>,
    expected: &[PlatformErrorCode],
) -> RuntimeResult<()> {
    let error = match result {
        Ok(_) => panic!("operation should fail"),
        Err(error) => error,
    };
    let platform = error
        .platform_error()
        .expect("error should contain one platform error");
    assert_no_permission_denied_in_privileged_mode(platform.code, expected);
    assert!(
        expected.contains(&platform.code),
        "unexpected platform error code: {:?}, expected one of {:?}",
        platform.code,
        expected,
    );

    Ok(())
}

/// Return true when privileged test mode is enabled.
fn is_privileged_test_mode() -> bool {
    let value = std::env::var("DESTACK_TEST_PRIVILEGED").unwrap_or_default();
    matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES")
}

/// Return true when one platform code represents a permission denial.
fn is_permission_denied_code(code: PlatformErrorCode) -> bool {
    matches!(
        code,
        PlatformErrorCode::IoPermissionDenied
            | PlatformErrorCode::ProcessPermissionDenied
            | PlatformErrorCode::SecurityDenied
    )
}

/// Fail privileged runs when assertions observe permission-denied errors.
fn assert_no_permission_denied_in_privileged_mode(
    observed: PlatformErrorCode,
    expected: &[PlatformErrorCode],
) {
    if !is_privileged_test_mode() {
        return;
    }

    if is_permission_denied_code(observed) {
        panic!(
            "permission-denied error {:?} is not allowed when DESTACK_TEST_PRIVILEGED=1 (expected one of {:?})",
            observed, expected,
        );
    }
}

/// Build a NativeSlice from a mutable byte buffer.
pub(crate) fn native_slice_mut(buffer: &mut [u8]) -> NativeSlice<u8> {
    NativeSlice {
        data: buffer.as_mut_ptr(),
        len: buffer.len() as u32,
    }
}

/// Build a NativeSlice from an immutable byte buffer.
pub(crate) fn native_slice(buffer: &[u8]) -> NativeSlice<u8> {
    NativeSlice {
        data: buffer.as_ptr() as *mut u8,
        len: buffer.len() as u32,
    }
}

/// Build a nested NativeSlice from immutable buffer slices.
pub(crate) fn native_slice_slices(buffers: &[NativeSlice<u8>]) -> NativeSlice<NativeSlice<u8>> {
    NativeSlice {
        data: buffers.as_ptr() as *mut NativeSlice<u8>,
        len: buffers.len() as u32,
    }
}

/// Build a nested NativeSlice from mutable buffer slices.
pub(crate) fn native_slice_slices_mut(
    buffers: &mut [NativeSlice<u8>],
) -> NativeSlice<NativeSlice<u8>> {
    NativeSlice {
        data: buffers.as_mut_ptr(),
        len: buffers.len() as u32,
    }
}

/// Build a VM slice that contains VM byte-slices.
fn vm_slice_of_slices(
    context: &mut vm::ExternalCallContext<'_>,
    slices: &[VmSlice<u8>],
) -> VmSlice<VmSlice<u8>> {
    let values = slices
        .iter()
        .map(|slice| slice.to_value(context))
        .collect::<Vec<_>>();
    let data = context.allocate_raw_values(values);
    VmSlice {
        data,
        len: slices.len() as u32,
        _marker: std::marker::PhantomData::<VmSlice<u8>>,
    }
}

fn host_from_vm(context: &mut vm::ExternalCallContext<'_>, host: &str) -> vm::StringHandle {
    let value = context.intern_string(host);
    vm::StringHandle::new(value)
}

/// Decode a native raw socket address for assertions.
fn socket_address_raw_native(address: SocketAddress) -> RuntimeResult<(u16, Vec<u8>)> {
    let bytes = unsafe { address.bytes.as_slice()? };
    Ok((address.family, bytes.to_vec()))
}

/// Decode a VM raw socket address for assertions.
fn socket_address_raw_vm(
    context: &mut vm::ExternalCallContext<'_>,
    address: SocketAddressVm,
) -> RuntimeResult<(u16, Vec<u8>)> {
    let bytes = address.bytes.read_values(context)?;
    Ok((address.family, bytes))
}

fn socket_addresses_native(
    addresses: NativeArray<SocketAddress>,
) -> RuntimeResult<Vec<(String, u16, SocketFamily)>> {
    let addresses = unsafe { addresses.as_slice()? };
    let mut decoded = Vec::with_capacity(addresses.len());
    for address in addresses {
        let bytes = unsafe { address.bytes.as_slice()? };
        decoded.push(socket_address_from_raw(address.family, bytes)?);
    }
    Ok(decoded)
}

fn socket_addresses_vm(
    context: &mut vm::ExternalCallContext<'_>,
    addresses: VmArray<SocketAddressVm>,
) -> RuntimeResult<Vec<(String, u16, SocketFamily)>> {
    let values = addresses.raw_values(context)?;
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let address = socket_address_vm_from_value(context, value)?;
        let bytes = address.bytes.read_values(context)?;
        decoded.push(socket_address_from_raw(address.family, &bytes)?);
    }
    Ok(decoded)
}

/// Decode reverse lookup names from native values.
fn reverse_lookup_names_native(
    values: NativeArray<ReverseLookupName>,
) -> RuntimeResult<Vec<String>> {
    let values = unsafe { values.as_slice()? };
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let host = unsafe { value.host.as_str()? };
        decoded.push(host.to_string());
    }

    Ok(decoded)
}

/// Decode reverse lookup names from VM values.
fn reverse_lookup_names_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: VmArray<platform_net::ReverseLookupNameVm>,
) -> RuntimeResult<Vec<String>> {
    let values = values.raw_values(context)?;
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let slots = context
            .aggregate_slots(value)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        if slots.len() != 2 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "names",
                "expected reverse lookup name aggregate with 2 fields",
            ))
            .boxed());
        }
        let host = vm::StringHandle::new(slots[0]);
        let host = context
            .string_ref(host)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        decoded.push(host.as_str().to_string());
    }

    Ok(decoded)
}

fn udp_receive_native(receive: UdpReceive) -> RuntimeResult<(String, u16, SocketFamily, u64)> {
    let bytes = unsafe { receive.address.bytes.as_slice()? };
    let (host, port, family) = socket_address_from_raw(receive.address.family, bytes)?;
    Ok((host, port, family, receive.bytes))
}

fn udp_receive_vm(
    context: &mut vm::ExternalCallContext<'_>,
    receive: UdpReceiveVm,
) -> RuntimeResult<(String, u16, SocketFamily, u64)> {
    let bytes = receive.address.bytes.read_values(context)?;
    let (host, port, family) = socket_address_from_raw(receive.address.family, &bytes)?;
    Ok((host, port, family, receive.bytes))
}

#[cfg(unix)]
fn tcp_stream_socket_type() -> SocketType {
    SocketType(libc::SOCK_STREAM as u32)
}

#[cfg(windows)]
fn tcp_stream_socket_type() -> SocketType {
    SocketType(windows_sys::Win32::Networking::WinSock::SOCK_STREAM as u32)
}

#[cfg(unix)]
fn tcp_protocol() -> SocketProtocol {
    SocketProtocol(libc::IPPROTO_TCP)
}

#[cfg(windows)]
fn tcp_protocol() -> SocketProtocol {
    SocketProtocol(windows_sys::Win32::Networking::WinSock::IPPROTO_TCP)
}

#[cfg(unix)]
fn socket_address_from_raw(
    family: u16,
    bytes: &[u8],
) -> RuntimeResult<(String, u16, SocketFamily)> {
    if family == libc::AF_INET as u16 {
        if bytes.len() < std::mem::size_of::<libc::sockaddr_in>() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "address.bytes",
                "invalid ipv4 socket address bytes",
            ))
            .boxed());
        }

        let mut storage = std::mem::MaybeUninit::<libc::sockaddr_in>::zeroed();
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                storage.as_mut_ptr() as *mut u8,
                std::mem::size_of::<libc::sockaddr_in>(),
            );
            let address = storage.assume_init();
            let ip = std::net::Ipv4Addr::from(address.sin_addr.s_addr.to_ne_bytes());
            let port = u16::from_be(address.sin_port);
            return Ok((ip.to_string(), port, SocketFamily::IPv4));
        }
    }

    if family == libc::AF_INET6 as u16 {
        if bytes.len() < std::mem::size_of::<libc::sockaddr_in6>() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "address.bytes",
                "invalid ipv6 socket address bytes",
            ))
            .boxed());
        }

        let mut storage = std::mem::MaybeUninit::<libc::sockaddr_in6>::zeroed();
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                storage.as_mut_ptr() as *mut u8,
                std::mem::size_of::<libc::sockaddr_in6>(),
            );
            let address = storage.assume_init();
            let ip = std::net::Ipv6Addr::from(address.sin6_addr.s6_addr);
            let port = u16::from_be(address.sin6_port);
            return Ok((ip.to_string(), port, SocketFamily::IPv6));
        }
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "address.family",
        "unsupported address family",
    ))
    .boxed())
}

#[cfg(windows)]
fn socket_address_from_raw(
    family: u16,
    bytes: &[u8],
) -> RuntimeResult<(String, u16, SocketFamily)> {
    // decode ipv4
    if family == windows_sys::Win32::Networking::WinSock::AF_INET as u16 {
        if bytes.len() < 16 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "address.bytes",
                "invalid ipv4 socket address bytes",
            ))
            .boxed());
        }

        let port = u16::from_be_bytes([bytes[2], bytes[3]]);
        let ip = std::net::Ipv4Addr::new(bytes[4], bytes[5], bytes[6], bytes[7]);
        return Ok((ip.to_string(), port, SocketFamily::IPv4));
    }

    // decode ipv6
    if family == windows_sys::Win32::Networking::WinSock::AF_INET6 as u16 {
        if bytes.len() < 28 {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "address.bytes",
                "invalid ipv6 socket address bytes",
            ))
            .boxed());
        }

        let port = u16::from_be_bytes([bytes[2], bytes[3]]);
        let mut octets = [0u8; 16];
        octets.copy_from_slice(&bytes[8..24]);
        let ip = std::net::Ipv6Addr::from(octets);
        return Ok((ip.to_string(), port, SocketFamily::IPv6));
    }

    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        "address.family",
        "unsupported address family",
    ))
    .boxed())
}

#[cfg(unix)]
fn socket_address_native_from_host_port(
    _context: &RuntimeCallContext,
    host: &str,
    port: u16,
    family: SocketFamily,
) -> RuntimeResult<NativeSocketAddressArg> {
    let (family, mut bytes) = match family {
        SocketFamily::IPv4 => {
            let ip = if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
                ip
            } else {
                let mut addresses = (host, port).to_socket_addrs().map_err(|_| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "host",
                        "failed to resolve ipv4 host",
                    ))
                    .boxed()
                })?;
                let Some(ip) = addresses.find_map(|address| match address {
                    std::net::SocketAddr::V4(address) => Some(*address.ip()),
                    std::net::SocketAddr::V6(_) => None,
                }) else {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "host",
                        "failed to resolve ipv4 host",
                    ))
                    .boxed());
                };
                ip
            };

            let mut socket_address = unsafe { std::mem::zeroed::<libc::sockaddr_in>() };
            #[cfg(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            ))]
            {
                socket_address.sin_len = std::mem::size_of::<libc::sockaddr_in>() as u8;
            }
            socket_address.sin_family = libc::AF_INET as libc::sa_family_t;
            socket_address.sin_port = port.to_be();
            socket_address.sin_addr = libc::in_addr {
                s_addr: u32::from_ne_bytes(ip.octets()),
            };

            let bytes = unsafe {
                std::slice::from_raw_parts(
                    &socket_address as *const _ as *const u8,
                    std::mem::size_of::<libc::sockaddr_in>(),
                )
            };
            (libc::AF_INET as u16, bytes.to_vec())
        }
        SocketFamily::IPv6 => {
            let ip = if let Ok(ip) = host.parse::<std::net::Ipv6Addr>() {
                ip
            } else {
                let mut addresses = (host, port).to_socket_addrs().map_err(|_| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "host",
                        "failed to resolve ipv6 host",
                    ))
                    .boxed()
                })?;
                let Some(ip) = addresses.find_map(|address| match address {
                    std::net::SocketAddr::V4(_) => None,
                    std::net::SocketAddr::V6(address) => Some(*address.ip()),
                }) else {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "host",
                        "failed to resolve ipv6 host",
                    ))
                    .boxed());
                };
                ip
            };

            let mut socket_address = unsafe { std::mem::zeroed::<libc::sockaddr_in6>() };
            #[cfg(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            ))]
            {
                socket_address.sin6_len = std::mem::size_of::<libc::sockaddr_in6>() as u8;
            }
            socket_address.sin6_family = libc::AF_INET6 as libc::sa_family_t;
            socket_address.sin6_port = port.to_be();
            socket_address.sin6_addr = libc::in6_addr {
                s6_addr: ip.octets(),
            };

            let bytes = unsafe {
                std::slice::from_raw_parts(
                    &socket_address as *const _ as *const u8,
                    std::mem::size_of::<libc::sockaddr_in6>(),
                )
            };
            (libc::AF_INET6 as u16, bytes.to_vec())
        }
        SocketFamily::Unspecified => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "family",
                "unspecified family is not supported for literal address encoding",
            ))
            .boxed());
        }
    };

    let address = SocketAddress {
        family,
        length: bytes.len() as u32,
        bytes: NativeArray {
            data: bytes.as_mut_ptr(),
            len: bytes.len() as u32,
            capacity: bytes.len() as u32,
        },
    };

    Ok(NativeSocketAddressArg {
        _bytes: bytes,
        address,
    })
}

#[cfg(windows)]
fn socket_address_native_from_host_port(
    _context: &RuntimeCallContext,
    host: &str,
    port: u16,
    family: SocketFamily,
) -> RuntimeResult<NativeSocketAddressArg> {
    let (family, bytes): (u16, Vec<u8>) = match family {
        // encode ipv4 sockaddr bytes
        SocketFamily::IPv4 => {
            let ip = if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
                ip
            } else {
                let mut addresses = (host, port).to_socket_addrs().map_err(|_| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "host",
                        "failed to resolve ipv4 host",
                    ))
                    .boxed()
                })?;
                let Some(ip) = addresses.find_map(|address| match address {
                    std::net::SocketAddr::V4(address) => Some(*address.ip()),
                    std::net::SocketAddr::V6(_) => None,
                }) else {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "host",
                        "failed to resolve ipv4 host",
                    ))
                    .boxed());
                };
                ip
            };

            let mut bytes = vec![0u8; 16];
            let family_bytes =
                (windows_sys::Win32::Networking::WinSock::AF_INET as u16).to_ne_bytes();
            bytes[0] = family_bytes[0];
            bytes[1] = family_bytes[1];
            let port_bytes = port.to_be_bytes();
            bytes[2] = port_bytes[0];
            bytes[3] = port_bytes[1];
            bytes[4..8].copy_from_slice(&ip.octets());

            (
                windows_sys::Win32::Networking::WinSock::AF_INET as u16,
                bytes,
            )
        }
        // encode ipv6 sockaddr bytes
        SocketFamily::IPv6 => {
            let ip = if let Ok(ip) = host.parse::<std::net::Ipv6Addr>() {
                ip
            } else {
                let mut addresses = (host, port).to_socket_addrs().map_err(|_| {
                    RuntimeError::from(PlatformError::invalid_argument_value(
                        "host",
                        "failed to resolve ipv6 host",
                    ))
                    .boxed()
                })?;
                let Some(ip) = addresses.find_map(|address| match address {
                    std::net::SocketAddr::V4(_) => None,
                    std::net::SocketAddr::V6(address) => Some(*address.ip()),
                }) else {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "host",
                        "failed to resolve ipv6 host",
                    ))
                    .boxed());
                };
                ip
            };

            let mut bytes = vec![0u8; 28];
            let family_bytes =
                (windows_sys::Win32::Networking::WinSock::AF_INET6 as u16).to_ne_bytes();
            bytes[0] = family_bytes[0];
            bytes[1] = family_bytes[1];
            let port_bytes = port.to_be_bytes();
            bytes[2] = port_bytes[0];
            bytes[3] = port_bytes[1];
            bytes[8..24].copy_from_slice(&ip.octets());

            (
                windows_sys::Win32::Networking::WinSock::AF_INET6 as u16,
                bytes,
            )
        }
        SocketFamily::Unspecified => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "family",
                "unspecified family is not supported for literal address encoding",
            ))
            .boxed());
        }
    };

    let mut bytes = bytes;
    let address = SocketAddress {
        family,
        length: bytes.len() as u32,
        bytes: NativeArray {
            data: bytes.as_mut_ptr(),
            len: bytes.len() as u32,
            capacity: bytes.len() as u32,
        },
    };

    Ok(NativeSocketAddressArg {
        _bytes: bytes,
        address,
    })
}

fn socket_address_vm_from_host_port(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    host: &str,
    port: u16,
    family: SocketFamily,
) -> RuntimeResult<SocketAddressVm> {
    let address = socket_address_native_from_host_port(runtime, host, port, family)?;
    let bytes = VmArray::from_values(context, address.bytes()).map_err(|error| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "address.bytes",
            format!("failed to encode vm byte array: {error}"),
        ))
        .boxed()
    })?;

    Ok(SocketAddressVm {
        family: address.address().family,
        length: address.address().length,
        bytes,
    })
}

/// Owns the raw bytes backing a native socket-address binding value.
struct NativeSocketAddressArg {
    /// The owned raw bytes storage.
    _bytes: Vec<u8>,
    /// The socket-address ABI payload pointing into `_bytes`.
    address: SocketAddress,
}

impl NativeSocketAddressArg {
    /// Return the socket-address payload.
    fn address(&self) -> SocketAddress {
        self.address
    }

    /// Return the socket-address raw bytes.
    fn bytes(&self) -> &[u8] {
        &self._bytes
    }
}

/// Decode a VM socket address aggregate value.
fn socket_address_vm_from_value(
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::Value,
) -> RuntimeResult<SocketAddressVm> {
    // decode aggregate fields
    if value.tag() != vm::ValueTag::Aggregate {
        return Err(RuntimeError::from(PlatformError::invalid_argument_type(
            "address",
            "SocketAddress",
        ))
        .boxed());
    }
    let slots = context
        .aggregate_slots(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    if slots.len() != 2 && slots.len() != 3 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "address",
            "expected SocketAddress aggregate with 2 or 3 fields",
        ))
        .boxed());
    }

    // decode family and bytes with backward compatible shape support
    let family = <u16 as VmValueCodec>::decode(slots[0])?;
    let (length, bytes) = if slots.len() == 3 {
        let length = <u32 as VmValueCodec>::decode(slots[1])?;
        let bytes = VmArray::<u8>::from_value(context, slots[2], "address.bytes", "VmArray<u8>")?;

        (length, bytes)
    } else {
        let bytes = VmArray::<u8>::from_value(context, slots[1], "address.bytes", "VmArray<u8>")?;
        let length = bytes.len;

        (length, bytes)
    };

    Ok(SocketAddressVm {
        family,
        length,
        bytes,
    })
}

#[cfg(unix)]
fn path_ref_native(path: &std::path::Path) -> (Vec<u8>, OsPath) {
    use std::os::unix::ffi::OsStrExt;
    let bytes = path.as_os_str().as_bytes().to_vec();
    let path_ref = OsPath {
        encoding: PathEncoding::Bytes,
        bytes: PathBytesAbi(NativeArray {
            data: bytes.as_ptr() as *mut u8,
            len: bytes.len() as u32,
            capacity: bytes.len() as u32,
        }),
        utf16: core_fs::empty_path_utf16(),
    };
    (bytes, path_ref)
}

#[cfg(unix)]
fn path_ref_native_utf16(path: &std::path::Path) -> (Vec<u16>, OsPath) {
    use std::os::unix::ffi::OsStrExt;

    // decode bytes as utf8 for deterministic utf16 test paths
    let bytes = path.as_os_str().as_bytes();
    let text = std::str::from_utf8(bytes).expect("test path should be valid utf8");
    let mut utf16_units: Vec<u16> = text.encode_utf16().collect();
    let path_ref = OsPath {
        encoding: PathEncoding::Utf16,
        bytes: core_fs::empty_path_bytes(),
        utf16: PathUtf16Abi(NativeArray {
            data: utf16_units.as_mut_ptr(),
            len: utf16_units.len() as u32,
            capacity: utf16_units.len() as u32,
        }),
    };

    (utf16_units, path_ref)
}

#[cfg(windows)]
fn path_ref_native(path: &std::path::Path) -> (Vec<u16>, OsPath) {
    use std::os::windows::ffi::OsStrExt;
    let mut utf16_units: Vec<u16> = path.as_os_str().encode_wide().collect();
    let path_ref = OsPath {
        encoding: PathEncoding::Utf16,
        bytes: core_fs::empty_path_bytes(),
        utf16: PathUtf16Abi(NativeArray {
            data: utf16_units.as_mut_ptr(),
            len: utf16_units.len() as u32,
            capacity: utf16_units.len() as u32,
        }),
    };
    (utf16_units, path_ref)
}

fn path_ref_vm(context: &mut vm::ExternalCallContext<'_>, path: &std::path::Path) -> OsPathVm {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let bytes = path.as_os_str().as_bytes();
        let array = VmArray::from_bytes(context, bytes);
        OsPathVm {
            encoding: PathEncoding::Bytes,
            bytes: PathBytesAbi(array),
            utf16: PathUtf16Abi(empty_vm_array()),
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let units: Vec<u16> = path.as_os_str().encode_wide().collect();
        let array = VmArray::from_values(context, &units).expect("vm utf16 path should encode");
        OsPathVm {
            encoding: PathEncoding::Utf16,
            bytes: PathBytesAbi(empty_vm_array()),
            utf16: PathUtf16Abi(array),
        }
    }
}

#[cfg(unix)]
fn path_ref_vm_utf16(
    context: &mut vm::ExternalCallContext<'_>,
    path: &std::path::Path,
) -> OsPathVm {
    use std::os::unix::ffi::OsStrExt;

    // decode bytes as utf8 for deterministic utf16 test paths
    let bytes = path.as_os_str().as_bytes();
    let text = std::str::from_utf8(bytes).expect("test path should be valid utf8");
    let units: Vec<u16> = text.encode_utf16().collect();
    let array = VmArray::from_values(context, &units).expect("vm utf16 path should encode");

    OsPathVm {
        encoding: PathEncoding::Utf16,
        bytes: PathBytesAbi(empty_vm_array()),
        utf16: PathUtf16Abi(array),
    }
}

/// Build an empty VM array.
fn empty_vm_array<T>() -> VmArray<T> {
    VmArray {
        data: vm::RawPointer::NULL,
        len: 0,
        capacity: 0,
        _marker: std::marker::PhantomData,
    }
}

/// Build a unix domain socket path address for native calls.
fn uds_path_address_native(path: OsPath) -> UdsAddress {
    UdsAddress {
        kind: UdsAddressKind::Path,
        path,
        abstract_name: NativeArray {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        },
    }
}

/// Build a unix domain socket path address for VM calls.
fn uds_path_address_vm(
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> platform_net::UdsAddressVm {
    platform_net::UdsAddressVm {
        kind: UdsAddressKind::Path,
        path,
        abstract_name: VmArray::from_bytes(context, &[]),
    }
}
