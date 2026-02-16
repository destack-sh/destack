#![cfg_attr(windows, allow(dead_code, unused_imports))]
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeErrorId, RuntimeResult, RuntimeStatus};
use crate::platform::abi::NativeAbi;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{
    self as core_net, AcceptFlags, SocketAddress, SocketAddressVm, SocketFamily, SocketProtocol,
    SocketType, vm as platform_net_vm,
};
use crate::platform::resource::{DirectoryHandle, ListenerHandle, ResourceId, SocketHandle};
use crate::platform::{
    NativeArray, NativeSlice, NativeStringRef, PlatformError, VmArray, VmSlice, fs as platform_fs,
};
use crate::runtime::RuntimeCallContext;
use crate::tests::runtime::TestRuntime;
use platform_fs::{
    AllocFlags, AtFlags, CopyFlags, Dirent, DirentKind, DirentVm, FileAdvice, FileHandle,
    FileLockFlags, FileMode, FileOffset, FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags,
    OpenFlags, OpenOptions, OsPath, OsPathVm, PathBytesAbi, PathEncoding, PathUtf16Abi,
    RenameFlags, SeekWhence, Stat, StatFs, SymlinkType, SyncFlags, XattrFlags, core as core_fs,
    vm as platform_vm,
};

#[path = "harness.generated.rs"]
mod harness;

/// Selects the backing harness kind for filesystem tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FsHarnessKind {
    /// Native bindings backed by the host ABI.
    Native,
    /// VM bindings backed by VM ABI values.
    Vm,
}

/// Path reference payload used by filesystem test helpers.
#[derive(Clone)]
pub(crate) enum FsPathRef {
    /// Native path with owned storage (keeps buffers alive).
    Native {
        /// The owned byte buffer, if the path was byte-encoded.
        _bytes: Option<Vec<u8>>,
        /// The owned utf16 buffer, if the path was utf16-encoded.
        _utf16: Option<Vec<u16>>,
        /// The native path reference.
        path: OsPath,
    },
    /// VM path reference.
    Vm {
        /// The VM path reference.
        path: OsPathVm,
    },
}

impl FsPathRef {
    /// Return the native path reference if available.
    pub(crate) fn native(&self) -> Option<OsPath> {
        match self {
            FsPathRef::Native { path, .. } => Some(*path),
            FsPathRef::Vm { .. } => None,
        }
    }
}

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

/// Filesystem harness context used by tests.
pub(crate) struct FsHarnessContext<'call> {
    /// Runtime backing this harness.
    runtime: &'call TestRuntime,
    /// Runtime call context active for this operation.
    call_context: &'call RuntimeCallContext,
    /// VM context when running VM bindings.
    vm_context: Option<*mut ()>,
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
    pub(crate) fn with_context<F, R>(&self, callback: F) -> R
    where
        F: for<'call> FnOnce(FsHarnessContext<'call>) -> R,
    {
        match self {
            FsHarnessHandle::Native(harness) => {
                harness.runtime.with_native_call_context(|call_context| {
                    callback(FsHarnessContext {
                        runtime: &harness.runtime,
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
        F: for<'call> FnOnce(FsHarnessContext<'call>) -> RuntimeResult<()>,
    {
        self.with_context(callback)
            .expect("filesystem harness call should succeed");
    }
}

/// Run a test against both harnesses.
pub(crate) fn with_harnesses<F>(mut callback: F)
where
    F: FnMut(&FsHarnessHandle),
{
    let native = FsHarnessHandle::Native(NativeFsHarness::new());
    callback(&native);
    let vm = FsHarnessHandle::Vm(VmFsHarness::new());
    callback(&vm);
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

/// Build a NativeArray from a byte buffer.
pub(crate) fn native_array(buffer: &mut [u8]) -> NativeArray<u8> {
    NativeArray {
        data: buffer.as_mut_ptr(),
        len: buffer.len() as u32,
        capacity: buffer.len() as u32,
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

/// Build a native byte path reference.
#[cfg(unix)]
fn path_bytes_native(path: &Path) -> (Vec<u8>, OsPath) {
    use std::os::unix::ffi::OsStrExt;

    let mut bytes = path.as_os_str().as_bytes().to_vec();
    let path = PathBytesAbi::<NativeAbi>(native_array(&mut bytes));
    (bytes, core_fs::path_ref_from_bytes(path))
}

/// Build a native byte path reference.
#[cfg(windows)]
fn path_bytes_native(path: &Path) -> (Vec<u8>, OsPath) {
    let value = path.to_str().expect("path must be utf8 for windows tests");
    let mut bytes = value.as_bytes().to_vec();
    let path = PathBytesAbi::<NativeAbi>(native_array(&mut bytes));
    (bytes, core_fs::path_ref_from_bytes(path))
}

/// Build a native byte path reference.
#[cfg(not(any(unix, windows)))]
fn path_bytes_native(_path: &Path) -> (Vec<u8>, OsPath) {
    unreachable!("unsupported platform for fs tests");
}

/// Build a native UTF-16 path reference.
#[cfg(unix)]
fn path_utf16_native(path: &Path) -> (Vec<u16>, OsPath) {
    use std::os::unix::ffi::OsStrExt;

    let bytes = path.as_os_str().as_bytes();
    let text = std::str::from_utf8(bytes).expect("path must be utf8 for unix utf16 tests");
    let mut utf16_units: Vec<u16> = text.encode_utf16().collect();
    let path = PathUtf16Abi::<NativeAbi>(NativeArray {
        data: utf16_units.as_mut_ptr(),
        len: utf16_units.len() as u32,
        capacity: utf16_units.len() as u32,
    });

    (utf16_units, core_fs::path_ref_from_utf16(path))
}

/// Build a native UTF-16 path reference.
#[cfg(windows)]
fn path_utf16_native(path: &Path) -> (Vec<u16>, OsPath) {
    use std::os::windows::ffi::OsStrExt;

    let mut utf16_units: Vec<u16> = path.as_os_str().encode_wide().collect();
    let path = PathUtf16Abi::<NativeAbi>(NativeArray {
        data: utf16_units.as_mut_ptr(),
        len: utf16_units.len() as u32,
        capacity: utf16_units.len() as u32,
    });

    (utf16_units, core_fs::path_ref_from_utf16(path))
}

/// Build a native UTF-16 path reference.
#[cfg(not(any(unix, windows)))]
#[allow(dead_code)]
fn path_utf16_native(_path: &Path) -> (Vec<u16>, OsPath) {
    unreachable!("unsupported platform for utf16 tests");
}

/// Read raw bytes from a native path reference.
fn path_ref_bytes_native(path: OsPath) -> Vec<u8> {
    match path.encoding {
        PathEncoding::Bytes => unsafe { path.bytes.0.as_slice() }
            .expect("path bytes should be valid")
            .to_vec(),
        PathEncoding::Utf16 => panic!("expected byte path"),
    }
}

/// Build an owned native path reference from a raw path reference.
fn own_path_ref_native(path: OsPath) -> FsPathRef {
    match path.encoding {
        PathEncoding::Bytes => {
            let mut bytes = path_ref_bytes_native(path);
            let path = PathBytesAbi::<NativeAbi>(native_array(&mut bytes));
            let path = core_fs::path_ref_from_bytes(path);
            FsPathRef::Native {
                _bytes: Some(bytes),
                _utf16: None,
                path,
            }
        }
        PathEncoding::Utf16 => {
            let mut utf16_units = path_ref_utf16_native(path);
            let path = PathUtf16Abi::<NativeAbi>(NativeArray {
                data: utf16_units.as_mut_ptr(),
                len: utf16_units.len() as u32,
                capacity: utf16_units.len() as u32,
            });
            let path = core_fs::path_ref_from_utf16(path);

            FsPathRef::Native {
                _bytes: None,
                _utf16: Some(utf16_units),
                path,
            }
        }
    }
}

/// Read UTF-16 units from a native path reference.
fn path_ref_utf16_native(path: OsPath) -> Vec<u16> {
    match path.encoding {
        PathEncoding::Utf16 => unsafe { path.utf16.0.as_slice() }
            .expect("path utf16 should be valid")
            .to_vec(),
        PathEncoding::Bytes => panic!("expected utf16 path"),
    }
}

/// Render a native path reference into a displayable string.
fn path_ref_string_native(path: OsPath) -> String {
    match path.encoding {
        PathEncoding::Bytes => {
            let bytes = path_ref_bytes_native(path);
            String::from_utf8_lossy(&bytes).to_string()
        }
        PathEncoding::Utf16 => {
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
    context: &vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<Vec<u8>> {
    match path.encoding {
        PathEncoding::Bytes => path.bytes.0.read_bytes(context),
        PathEncoding::Utf16 => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "expected byte path",
        ))
        .boxed()),
    }
}

/// Decode a VM path reference into a string.
fn path_ref_string_vm(
    context: &vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<String> {
    match path.encoding {
        PathEncoding::Bytes => {
            let bytes = path.bytes.0.read_bytes(context)?;
            Ok(String::from_utf8_lossy(&bytes).to_string())
        }
        PathEncoding::Utf16 => {
            let units = path.utf16.0.read_values(context)?;
            Ok(String::from_utf16_lossy(&units))
        }
    }
}

/// Decode a VM path reference from an aggregate value.
fn decode_path_ref_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::Value,
) -> RuntimeResult<OsPathVm> {
    let slots = context
        .aggregate_slots(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    if slots.len() != 3 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "path",
            "expected OsPath aggregate with 3 fields",
        ))
        .boxed());
    }

    let (encoding, width) = slots[0].as_uint_with_width().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_type("path", "OsPath")).boxed()
    })?;
    if width != 8 {
        return Err(
            RuntimeError::from(PlatformError::invalid_argument_type("path", "OsPath")).boxed(),
        );
    }
    let encoding = match encoding as u8 {
        value if value == PathEncoding::Bytes as u8 => PathEncoding::Bytes,
        value if value == PathEncoding::Utf16 as u8 => PathEncoding::Utf16,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "unknown path encoding",
            ))
            .boxed());
        }
    };

    let bytes = VmArray::from_value(context, slots[1], "path.bytes", "PathBytes")?;
    let utf16 = VmArray::from_value(context, slots[2], "path.utf16", "PathUtf16")?;
    Ok(OsPathVm {
        encoding,
        bytes: PathBytesAbi(bytes),
        utf16: PathUtf16Abi(utf16),
    })
}

/// Decode a VM directory entry from an aggregate value.
fn decode_dirent_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::Value,
) -> RuntimeResult<DirentVm> {
    let slots = context
        .aggregate_slots(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    if slots.len() != 2 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "dirent",
            "expected Dirent aggregate with 2 fields",
        ))
        .boxed());
    }

    let name = decode_path_ref_vm(context, slots[0])?;
    let (kind, width) = slots[1].as_uint_with_width().ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_type("dirent", "Dirent")).boxed()
    })?;
    if width != 8 {
        return Err(
            RuntimeError::from(PlatformError::invalid_argument_type("dirent", "Dirent")).boxed(),
        );
    }
    let kind = match kind as u8 {
        value if value == DirentKind::Unknown as u8 => DirentKind::Unknown,
        value if value == DirentKind::File as u8 => DirentKind::File,
        value if value == DirentKind::Directory as u8 => DirentKind::Directory,
        value if value == DirentKind::Symlink as u8 => DirentKind::Symlink,
        _ => DirentKind::Unknown,
    };

    Ok(DirentVm { name, kind })
}

fn decode_string_value(
    _context: &vm::ExternalCallContext<'_>,
    value: vm::Value,
) -> RuntimeResult<vm::StringHandle> {
    if value.tag() != vm::ValueTag::String {
        return Err(
            RuntimeError::from(PlatformError::invalid_argument_type("string", "string")).boxed(),
        );
    }
    Ok(vm::StringHandle::new(value))
}

fn vm_string(context: &mut vm::ExternalCallContext<'_>, value: &str) -> vm::StringHandle {
    let value = context.intern_string(value);
    vm::StringHandle::new(value)
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
    _runtime: &RuntimeCallContext,
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
    _runtime: &RuntimeCallContext,
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

#[cfg(any(unix, windows))]
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

fn array_u8_native(values: NativeArray<u8>) -> RuntimeResult<Vec<u8>> {
    let values = unsafe { values.as_slice()? };
    Ok(values.to_vec())
}

fn array_u8_vm(
    context: &mut vm::ExternalCallContext<'_>,
    values: VmArray<u8>,
) -> RuntimeResult<Vec<u8>> {
    values.read_bytes(context)
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
    let values = values.raw_values(context)?;
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
