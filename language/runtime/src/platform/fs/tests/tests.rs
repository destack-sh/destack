#![cfg_attr(windows, allow(dead_code, unused_imports))]

#[path = "harness.rs"]
mod harness;

use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};

use destack_vm as vm;

#[cfg(windows)]
use crate::diagnostic::RuntimeStatus;
use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(windows)]
use crate::platform::abi::NativeAbi;
use crate::platform::abi::{NativeSlice, NativeStringRef};
#[cfg(windows)]
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{
    SocketAddress, SocketAddressVm, SocketFamily, SocketProtocol, SocketType,
};
use crate::platform::resource::{ListenerHandle, ResourceKind};
use crate::platform::{
    NativeArray, PlatformError, VmAggregateCodec, VmArray, VmSlice, fs as platform_fs,
};
use crate::runtime::{Worker, BindingCallContext};
pub(crate) use crate::tests::platform::assert_platform_error_codes_with_privileged_policy;
use crate::tests::runtime::TestRuntime;
use platform_fs::{Dirent, DirentVm, OsPath, OsPathVm, WatchEvent, WatchEventVm};

/// Path reference payload used by filesystem test helpers.
pub(crate) type FsPathRef = harness::HarnessValue<OsPath, OsPathVm>;

/// Directory entry payload for filesystem tests.
#[derive(Clone, Copy)]
pub(crate) enum FsDirent {
    /// Native directory entry.
    Native(Dirent),
    /// VM directory entry.
    Vm(DirentVm),
}

/// Memory mapping payload used by filesystem tests.
#[derive(Clone, Copy)]
pub(crate) enum FsMapping {
    /// Native mapping.
    Native(NativeSlice<u8>),
    /// VM mapping.
    Vm(VmSlice<u8>),
}

/// Watch event payload used by filesystem tests.
#[derive(Clone, Copy)]
pub(crate) enum FsWatchEvent {
    /// Native watch event payload.
    Native(WatchEvent),
    /// VM watch event payload.
    Vm(WatchEventVm),
}

/// Filesystem harness context used by tests.
pub(crate) struct FsHarnessContext<'call> {
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

/// Native filesystem harness backed by native bindings.
pub(crate) struct NativeFsHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl NativeFsHarness {
    /// Create a new native filesystem harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// VM filesystem harness backed by VM bindings.
pub(crate) struct VmFsHarness {
    /// Runtime that powers the harness.
    runtime: TestRuntime,
}

impl VmFsHarness {
    /// Create a new VM filesystem harness.
    pub(crate) fn new() -> Self {
        Self {
            runtime: TestRuntime::deterministic_random(),
        }
    }
}

/// Harness handle that dispatches to native or VM implementations.
pub(crate) enum FsHarnessHandle {
    /// Native filesystem harness.
    Native(NativeFsHarness),
    /// VM filesystem harness.
    Vm(VmFsHarness),
}

impl FsHarnessHandle {
    /// Run a native or VM call context around the callback.
    pub(crate) fn with_context<F, R>(&mut self, callback: F) -> R
    where
        F: for<'call> FnOnce(FsHarnessContext<'call>) -> R,
    {
        match self {
            FsHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(FsHarnessContext {
                        call_context,
                        vm_context: None,
                    })
                })
            }
            FsHarnessHandle::Vm(harness) => {
                harness
                    .runtime
                    .with_vm_call_context(|call_context, vm_context| {
                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();
                        callback(FsHarnessContext {
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
        F: for<'call> FnOnce(FsHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("filesystem harness call should succeed");
    }
}

/// Run a test against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&mut FsHarnessHandle),
{
    let mut native = FsHarnessHandle::Native(NativeFsHarness::new());
    callback(&mut native);
    let mut vm = FsHarnessHandle::Vm(VmFsHarness::new());
    callback(&mut vm);
}

/// Run a test callback against both harness contexts.
pub(crate) fn with_harness_context<F>(mut callback: F)
where
    F: for<'call> FnMut(FsHarnessContext<'call>) -> RuntimeResult<()>,
{
    // run the callback for each harness
    with_harnesses(|harness| {
        harness.run(&mut callback);
    });
}

/// Create a unique temp directory for a test.
pub(crate) fn temp_dir(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();

    std::env::temp_dir().join(format!("destack_runtime_{label}_{nonce}"))
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

/// Read raw bytes from a native path reference.
fn path_ref_bytes_native(path: OsPath) -> Vec<u8> {
    match path {
        OsPath::OsPathBytes(path) => unsafe { path.bytes.0.as_slice() }
            .expect("path bytes should be valid")
            .to_vec(),
        OsPath::OsPathUtf16(_) => panic!("expected byte path"),
    }
}

/// Read UTF-16 units from a native path reference.
fn path_ref_utf16_native(path: OsPath) -> Vec<u16> {
    match path {
        OsPath::OsPathUtf16(path) => unsafe { path.utf16.0.as_slice() }
            .expect("path utf16 should be valid")
            .to_vec(),
        OsPath::OsPathBytes(_) => panic!("expected utf16 path"),
    }
}

/// Render a native path reference into a displayable string.
fn path_ref_string_native(path: OsPath) -> String {
    match path {
        OsPath::OsPathBytes(_) => {
            let bytes = path_ref_bytes_native(path);
            String::from_utf8_lossy(&bytes).to_string()
        }
        OsPath::OsPathUtf16(_) => {
            let units = path_ref_utf16_native(path);
            String::from_utf16_lossy(&units)
        }
    }
}

/// Build a byte vector from a path.
#[cfg(unix)]
fn path_bytes_vec(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;

    path.as_os_str().as_bytes().to_vec()
}

/// Build a byte vector from a path.
#[cfg(windows)]
fn path_bytes_vec(path: &Path) -> Vec<u8> {
    path.to_str()
        .expect("path must be utf8 for windows tests")
        .as_bytes()
        .to_vec()
}

/// Build a byte vector from a path.
#[cfg(not(any(unix, windows)))]
fn path_bytes_vec(_path: &Path) -> Vec<u8> {
    unreachable!("unsupported platform for fs tests");
}

/// Build a utf16 vector from a path.
#[cfg(unix)]
fn path_utf16_vec(path: &Path) -> Vec<u16> {
    use std::os::unix::ffi::OsStrExt;

    let bytes = path.as_os_str().as_bytes();
    let text = std::str::from_utf8(bytes).expect("path must be utf8 for unix utf16 tests");

    text.encode_utf16().collect()
}

/// Build a utf16 vector from a path.
#[cfg(windows)]
fn path_utf16_vec(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    path.as_os_str().encode_wide().collect()
}

/// Build a utf16 vector from a path.
#[cfg(not(any(unix, windows)))]
#[allow(dead_code)]
fn path_utf16_vec(_path: &Path) -> Vec<u16> {
    unreachable!("unsupported platform for utf16 tests");
}

/// Decode raw bytes from a VM path reference.
fn path_ref_bytes_vm(
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<Vec<u8>> {
    match path {
        OsPathVm::OsPathBytes(path) => path.bytes.0.read_bytes(&context.read()),
        OsPathVm::OsPathUtf16(_) => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "expected byte path",
        ))
        .boxed()),
    }
}

/// Decode a VM path reference into a string.
fn path_ref_string_vm(
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<String> {
    match path {
        OsPathVm::OsPathBytes(path) => {
            let bytes = path.bytes.0.read_bytes(&context.read())?;
            Ok(String::from_utf8_lossy(&bytes).to_string())
        }
        OsPathVm::OsPathUtf16(path) => {
            let units = path.utf16.0.read_values(&context.read())?;
            Ok(String::from_utf16_lossy(&units))
        }
    }
}

/// Decode a VM directory entry from an aggregate value.
fn decode_dirent_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::Value,
) -> RuntimeResult<DirentVm> {
    DirentVm::decode_with_context(&context.read(), value)
}

/// Decode a VM watch event from an aggregate value.
fn decode_watch_event_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::Value,
) -> RuntimeResult<WatchEventVm> {
    WatchEventVm::decode_with_context(&context.read(), value)
}

fn decode_string_value(
    _context: &vm::ExternalCallContext<'_>,
    value: vm::Value,
) -> RuntimeResult<vm::StringHandle> {
    if value.tag() != vm::ValueTag::ManagedReference {
        return Err(
            RuntimeError::from(PlatformError::invalid_argument_type("string", "string")).boxed(),
        );
    }
    Ok(vm::StringHandle::new(value))
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

#[cfg(any(unix, windows))]
fn socket_address_vm_from_host_port(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
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

fn array_u8_native(values: NativeArray<u8>) -> RuntimeResult<Vec<u8>> {
    let values = unsafe { values.as_slice()? };
    Ok(values.to_vec())
}

fn array_u8_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: VmArray<u8>,
) -> RuntimeResult<Vec<u8>> {
    values.read_bytes(&context.read())
}

fn array_string_native(values: NativeArray<NativeStringRef>) -> RuntimeResult<Vec<String>> {
    let values = unsafe { values.as_slice()? };
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        decoded.push(unsafe { value.as_str()? }.to_string());
    }
    Ok(decoded)
}

fn array_string_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: VmArray<vm::StringHandle>,
) -> RuntimeResult<Vec<String>> {
    let values = values.raw_values(&context.read())?;
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        let handle = decode_string_value(context, value)?;
        let string_ref = context
            .string_ref(handle)
            .map_err(|error| RuntimeError::from(error).boxed())?;
        decoded.push(string_ref.as_str().to_string());
    }
    Ok(decoded)
}
