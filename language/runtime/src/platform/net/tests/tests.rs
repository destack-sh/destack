#![cfg_attr(windows, allow(dead_code, unused_imports))]

#[path = "harness.rs"]
mod harness;
pub(super) use harness::HarnessValue;

use std::net::ToSocketAddrs;

use destack_vm as vm;
#[cfg(windows)]
use destack_workspace::RuntimeOptions;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::fs::{
    OsPath, OsPathBytesVm, OsPathUtf16Vm, OsPathVm, PathBytesAbi, PathUtf16Abi,
};
use crate::platform::resource::{ListenerHandle, ResourceKind, SocketHandle};
use crate::platform::{
    NativeArray, PlatformError, VmAggregateCodec, VmArray, VmSlice, net as platform_net,
};
use crate::runtime::{BindingCallContext, Worker};
#[cfg(windows)]
pub(crate) use crate::tests::platform::assert_not_supported_result;
pub(crate) use crate::tests::platform::{
    assert_platform_error_code_with_privileged_policy,
    assert_platform_error_codes_with_privileged_policy, vm_test_string, vm_test_values,
};
use crate::tests::runtime::TestRuntime;
use platform_net::{
    ReverseLookupName, SocketAddress, SocketAddressVm, SocketFamily, SocketProtocol, SocketType,
    UdpReceive, UdpReceiveVm, UdsAddress, UdsPathAddress,
};

/// Network harness context used by tests.
pub(crate) struct NetHarnessContext<'call> {
    /// Runtime call context active for this operation.
    pub(super) call_context: &'call BindingCallContext,
    /// VM context when running VM bindings.
    pub(super) vm_context: Option<*mut ()>,
}

/// Read the port assigned to one listener handle.
#[cfg(unix)]
pub(super) fn listener_port(worker: &Worker, handle: ListenerHandle) -> u16 {
    let fd = worker
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Listener {
                return None;
            }
            entry.fd()
        })
        .flatten()
        .expect("listener handle must be valid");

    let mut storage = std::mem::MaybeUninit::<libc::sockaddr_storage>::uninit();
    let mut length = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
    let result = unsafe { libc::getsockname(fd, storage.as_mut_ptr() as *mut _, &mut length) };
    assert_eq!(result, 0, "getsockname failed");
    let storage = unsafe { storage.assume_init() };

    match storage.ss_family as libc::c_int {
        libc::AF_INET => {
            let address = unsafe { &*(std::ptr::addr_of!(storage) as *const libc::sockaddr_in) };
            u16::from_be(address.sin_port)
        }
        libc::AF_INET6 => {
            let address = unsafe { &*(std::ptr::addr_of!(storage) as *const libc::sockaddr_in6) };
            u16::from_be(address.sin6_port)
        }
        _ => panic!("unsupported listener address family"),
    }
}

/// Read the port assigned to one listener handle.
#[cfg(windows)]
pub(super) fn listener_port(worker: &Worker, handle: ListenerHandle) -> u16 {
    use windows_sys::Win32::Networking::WinSock::{
        AF_INET, AF_INET6, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6, SOCKADDR_STORAGE, SOCKET,
        getsockname,
    };

    let socket = worker
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Listener {
                return None;
            }
            entry.socket()
        })
        .flatten()
        .expect("listener handle must be valid") as SOCKET;

    let mut storage = std::mem::MaybeUninit::<SOCKADDR_STORAGE>::uninit();
    let mut length = std::mem::size_of::<SOCKADDR_STORAGE>() as i32;
    let result = unsafe { getsockname(socket, storage.as_mut_ptr() as *mut SOCKADDR, &mut length) };
    assert_eq!(result, 0, "getsockname failed");
    let storage = unsafe { storage.assume_init() };

    match storage.ss_family as i32 {
        value if value == AF_INET as i32 => {
            let address = unsafe { &*(std::ptr::addr_of!(storage) as *const SOCKADDR_IN) };
            u16::from_be(address.sin_port)
        }
        value if value == AF_INET6 as i32 => {
            let address = unsafe { &*(std::ptr::addr_of!(storage) as *const SOCKADDR_IN6) };
            u16::from_be(address.sin6_port)
        }
        _ => panic!("unsupported listener address family"),
    }
}

/// Return whether one socket handle is in nonblocking mode.
#[cfg(unix)]
pub(super) fn socket_is_nonblocking(worker: &Worker, handle: SocketHandle) -> bool {
    let fd = worker
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Socket {
                return None;
            }
            entry.fd()
        })
        .flatten()
        .expect("socket handle must be valid");

    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    assert!(flags >= 0, "fcntl(F_GETFL) failed");

    (flags & libc::O_NONBLOCK) != 0
}

/// Return whether one socket handle is close-on-exec or non-inheritable.
#[cfg(unix)]
pub(super) fn socket_is_close_on_exec(worker: &Worker, handle: SocketHandle) -> bool {
    let fd = worker
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Socket {
                return None;
            }
            entry.fd()
        })
        .flatten()
        .expect("socket handle must be valid");

    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    assert!(flags >= 0, "fcntl(F_GETFD) failed");

    (flags & libc::FD_CLOEXEC) != 0
}

/// Return whether one socket handle is close-on-exec or non-inheritable.
#[cfg(windows)]
pub(super) fn socket_is_close_on_exec(worker: &Worker, handle: SocketHandle) -> bool {
    use windows_sys::Win32::Foundation::{GetHandleInformation, HANDLE_FLAG_INHERIT};
    use windows_sys::Win32::Networking::WinSock::SOCKET;

    let socket = worker
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != ResourceKind::Socket {
                return None;
            }
            entry.socket()
        })
        .flatten()
        .expect("socket handle must be valid") as SOCKET;

    let mut flags = 0u32;
    let result = unsafe { GetHandleInformation(socket as isize, &mut flags) };
    assert_ne!(result, 0, "GetHandleInformation failed");

    (flags & HANDLE_FLAG_INHERIT) == 0
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

    /// Create a new native network harness with explicit runtime options.
    #[cfg(windows)]
    pub(crate) fn new_with_runtime_options(configure: &impl Fn(&mut RuntimeOptions)) -> Self {
        Self {
            runtime: TestRuntime::deterministic_random_with_options(|options| configure(options)),
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

    /// Create a new VM network harness with explicit runtime options.
    #[cfg(windows)]
    pub(crate) fn new_with_runtime_options(configure: &impl Fn(&mut RuntimeOptions)) -> Self {
        Self {
            runtime: TestRuntime::deterministic_random_with_options(|options| configure(options)),
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
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(NetHarnessContext<'call>) -> R,
    {
        match self {
            NetHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(NetHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            NetHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::BindingContext<'_> as *mut ();
                        callback(NetHarnessContext {
                            call_context,
                            vm_context: Some(vm_context),
                        })
                    })
            }
        }
    }

    /// Run a test callback that returns a runtime result.
    pub(crate) fn run<F>(&mut self, callback: F)
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
    F: FnMut(&mut NetHarnessHandle),
{
    let mut native = NetHarnessHandle::Native(NativeNetHarness::new());
    callback(&mut native);
    let mut vm = NetHarnessHandle::Vm(VmNetHarness::new());
    callback(&mut vm);
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

/// Run a test callback against both harness contexts with explicit runtime options.
#[cfg(windows)]
pub(crate) fn with_harness_context_with_runtime_options<F>(
    configure: impl Fn(&mut RuntimeOptions),
    mut callback: F,
) where
    F: for<'call> FnMut(NetHarnessContext<'call>) -> RuntimeResult<()>,
{
    // run the callback for each configured harness
    let native = NetHarnessHandle::Native(NativeNetHarness::new_with_runtime_options(&configure));
    native.run(&mut callback);

    let vm = NetHarnessHandle::Vm(VmNetHarness::new_with_runtime_options(&configure));
    vm.run(&mut callback);
}

impl<'call> NetHarnessContext<'call> {
    /// Create a connected socket pair.
    pub(crate) fn socket_pair(
        &mut self,
        family: SocketFamily,
        socket_type: SocketType,
        protocol: SocketProtocol,
    ) -> RuntimeResult<(SocketHandle, SocketHandle)> {
        match self.destack_net_socket_pair(family, socket_type, protocol)? {
            HarnessValue::Native(pair) => Ok((pair.first, pair.second)),
            HarnessValue::Vm(pair) => Ok((pair.first, pair.second)),
        }
    }

    /// Create a connected UDS socket pair.
    pub(crate) fn uds_socket_pair(
        &mut self,
        socket_type: SocketType,
    ) -> RuntimeResult<(SocketHandle, SocketHandle)> {
        match self.destack_net_uds_socket_pair(socket_type)? {
            HarnessValue::Native(pair) => Ok((pair.first, pair.second)),
            HarnessValue::Vm(pair) => Ok((pair.first, pair.second)),
        }
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

/// Build a VM slice that contains VM byte-slices.
fn vm_slice_of_slices(
    context: &mut vm::BindingContext<'_>,
    slices: &[VmSlice<u8>],
) -> VmSlice<VmSlice<u8>> {
    let values = slices
        .iter()
        .map(|slice| slice.to_value(&mut context.write()))
        .collect::<RuntimeResult<Vec<_>>>()
        .expect("vm test slice values should encode");
    let data = vm_test_values(context, values);
    VmSlice {
        data,
        len: slices.len() as u32,
        _marker: std::marker::PhantomData::<VmSlice<u8>>,
    }
}

fn host_from_vm(context: &mut vm::BindingContext<'_>, host: &str) -> vm::StringHandle {
    vm_test_string(context, host)
}

/// Decode a native raw socket address for assertions.
fn socket_address_raw_native(address: SocketAddress) -> RuntimeResult<(u16, Vec<u8>)> {
    let bytes = unsafe { address.bytes.as_slice()? };
    Ok((address.family, bytes.to_vec()))
}

/// Decode a VM raw socket address for assertions.
fn socket_address_raw_vm(
    context: &mut vm::BindingContext<'_>,
    address: SocketAddressVm,
) -> RuntimeResult<(u16, Vec<u8>)> {
    let bytes = address.bytes.read_bytes(&context.read())?;
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
    context: &mut vm::BindingContext<'_>,
    addresses: VmArray<SocketAddressVm>,
) -> RuntimeResult<Vec<(String, u16, SocketFamily)>> {
    let values = addresses.values(&context.read())?;
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let address = socket_address_vm_from_value(context, value)?;
        let bytes = address.bytes.read_bytes(&context.read())?;
        decoded.push(socket_address_from_raw(address.family, &bytes)?);
    }
    Ok(decoded)
}

/// Decode reverse lookup names from native values.
fn reverse_lookup_names_native(
    values: NativeArray<ReverseLookupName>,
) -> RuntimeResult<Vec<String>> {
    let values = reverse_lookup_records_native(values)?;
    let mut decoded = Vec::with_capacity(values.len());
    for (host, _service) in values {
        decoded.push(host);
    }

    Ok(decoded)
}

/// Decode reverse lookup names from VM values.
fn reverse_lookup_names_vm(
    context: &mut vm::BindingContext<'_>,
    values: VmArray<platform_net::ReverseLookupNameVm>,
) -> RuntimeResult<Vec<String>> {
    let values = reverse_lookup_records_vm(context, values)?;
    let mut decoded = Vec::with_capacity(values.len());
    for (host, _service) in values {
        decoded.push(host);
    }

    Ok(decoded)
}

/// Decode reverse lookup records from native values.
fn reverse_lookup_records_native(
    values: NativeArray<ReverseLookupName>,
) -> RuntimeResult<Vec<(String, String)>> {
    let values = unsafe { values.as_slice()? };
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let host = unsafe { value.host.as_str()? };
        let service = unsafe { value.service.as_str()? };
        decoded.push((host.to_string(), service.to_string()));
    }

    Ok(decoded)
}

/// Decode reverse lookup records from VM values.
fn reverse_lookup_records_vm(
    context: &mut vm::BindingContext<'_>,
    values: VmArray<platform_net::ReverseLookupNameVm>,
) -> RuntimeResult<Vec<(String, String)>> {
    let values = values.read_values(&context.read())?;
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let host = context
            .string_ref(value.host)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        let service = context
            .string_ref(value.service)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        decoded.push((host.as_str().to_string(), service.as_str().to_string()));
    }

    Ok(decoded)
}

fn udp_receive_native(receive: UdpReceive) -> RuntimeResult<(String, u16, SocketFamily, u64, u32)> {
    let bytes = unsafe { receive.address.bytes.as_slice()? };
    let (host, port, family) = socket_address_from_raw(receive.address.family, bytes)?;
    Ok((host, port, family, receive.bytes, receive.recv_flags.0))
}

fn udp_receive_vm(
    context: &mut vm::BindingContext<'_>,
    receive: UdpReceiveVm,
) -> RuntimeResult<(String, u16, SocketFamily, u64, u32)> {
    let bytes = receive.address.bytes.read_bytes(&context.read())?;
    let (host, port, family) = socket_address_from_raw(receive.address.family, &bytes)?;
    Ok((host, port, family, receive.bytes, receive.recv_flags.0))
}

#[cfg(unix)]
pub(super) fn tcp_stream_socket_type() -> SocketType {
    SocketType(libc::SOCK_STREAM as u32)
}

#[cfg(windows)]
pub(super) fn tcp_stream_socket_type() -> SocketType {
    SocketType(windows_sys::Win32::Networking::WinSock::SOCK_STREAM as u32)
}

#[cfg(unix)]
pub(super) fn tcp_protocol() -> SocketProtocol {
    SocketProtocol(libc::IPPROTO_TCP)
}

#[cfg(windows)]
pub(super) fn tcp_protocol() -> SocketProtocol {
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
    if family == windows_sys::Win32::Networking::WinSock::AF_INET {
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
    if family == windows_sys::Win32::Networking::WinSock::AF_INET6 {
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
    _binding: &BindingCallContext,
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
    _binding: &BindingCallContext,
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
            let family_bytes = windows_sys::Win32::Networking::WinSock::AF_INET.to_ne_bytes();
            bytes[0] = family_bytes[0];
            bytes[1] = family_bytes[1];
            let port_bytes = port.to_be_bytes();
            bytes[2] = port_bytes[0];
            bytes[3] = port_bytes[1];
            bytes[4..8].copy_from_slice(&ip.octets());

            (windows_sys::Win32::Networking::WinSock::AF_INET, bytes)
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
            let family_bytes = windows_sys::Win32::Networking::WinSock::AF_INET6.to_ne_bytes();
            bytes[0] = family_bytes[0];
            bytes[1] = family_bytes[1];
            let port_bytes = port.to_be_bytes();
            bytes[2] = port_bytes[0];
            bytes[3] = port_bytes[1];
            bytes[8..24].copy_from_slice(&ip.octets());

            (windows_sys::Win32::Networking::WinSock::AF_INET6, bytes)
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
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    host: &str,
    port: u16,
    family: SocketFamily,
) -> RuntimeResult<SocketAddressVm> {
    let address = socket_address_native_from_host_port(binding, host, port, family)?;
    let bytes = VmArray::from_bytes(&mut context.write(), address.bytes()).map_err(|error| {
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
    context: &mut vm::BindingContext<'_>,
    value: vm::Word,
) -> RuntimeResult<SocketAddressVm> {
    SocketAddressVm::decode_with_context(&context.read(), value)
}

fn path_ref_vm(context: &mut vm::BindingContext<'_>, path: &std::path::Path) -> OsPathVm {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let bytes = path.as_os_str().as_bytes();
        let array = VmArray::from_bytes(&mut context.write(), bytes)
            .expect("vm test byte array should allocate");
        let kind = vm::StringHandle::new(
            context
                .intern_string("bytes")
                .expect("vm test string should intern"),
        );
        let path = OsPathBytesVm {
            kind,
            bytes: PathBytesAbi(array),
        };

        OsPathVm::OsPathBytes(path)
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let units: Vec<u16> = path.as_os_str().encode_wide().collect();
        let array = VmArray::from_values(&mut context.write(), &units)
            .expect("vm utf16 path should encode");
        let kind = vm::StringHandle::new(
            context
                .intern_string("utf16")
                .expect("vm test string should intern"),
        );
        let path = OsPathUtf16Vm {
            kind,
            utf16: PathUtf16Abi(array),
        };

        OsPathVm::OsPathUtf16(path)
    }
}

#[cfg(unix)]
fn path_ref_vm_utf16(context: &mut vm::BindingContext<'_>, path: &std::path::Path) -> OsPathVm {
    use std::os::unix::ffi::OsStrExt;

    // decode bytes as utf8 for deterministic utf16 test paths
    let bytes = path.as_os_str().as_bytes();
    let text = std::str::from_utf8(bytes).expect("test path should be valid utf8");
    let units: Vec<u16> = text.encode_utf16().collect();
    let array =
        VmArray::from_values(&mut context.write(), &units).expect("vm utf16 path should encode");
    let kind = vm::StringHandle::new(
        context
            .intern_string("utf16")
            .expect("vm test string should intern"),
    );
    let path = OsPathUtf16Vm {
        kind,
        utf16: PathUtf16Abi(array),
    };

    OsPathVm::OsPathUtf16(path)
}

/// Build a unix domain socket path address for native calls.
fn uds_path_address_native(path: OsPath) -> UdsAddress {
    UdsAddress::UdsPathAddress(UdsPathAddress {
        kind: "path".into(),
        path,
    })
}

/// Build a unix domain socket path address for VM calls.
fn uds_path_address_vm(
    context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
) -> platform_net::UdsAddressVm {
    platform_net::UdsAddressVm::UdsPathAddress(platform_net::UdsPathAddressVm {
        kind: vm::StringHandle::new(
            context
                .intern_string("path")
                .expect("vm test string should intern"),
        ),
        path,
    })
}
