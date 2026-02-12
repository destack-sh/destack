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

impl<'call> FsHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        // safety: the harness guarantees the VM context pointer is valid for the callback
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }
    /// Return the harness kind for this context.
    pub(crate) fn kind(&self) -> FsHarnessKind {
        if self.vm_context.is_some() {
            FsHarnessKind::Vm
        } else {
            FsHarnessKind::Native
        }
    }

    /// Build a byte path reference for this context.
    pub(crate) fn path_bytes(&mut self, path: &Path) -> FsPathRef {
        match self.vm_context_mut() {
            Some(context) => {
                let bytes = path_bytes_vec(path);
                let array = VmArray::from_bytes(context, &bytes);
                let path = OsPathVm {
                    encoding: PathEncoding::Bytes,
                    bytes: PathBytesAbi(array),
                    utf16: PathUtf16Abi(empty_vm_array()),
                };
                FsPathRef::Vm { path }
            }
            None => {
                let (bytes, path) = path_bytes_native(path);
                FsPathRef::Native {
                    _bytes: Some(bytes),
                    _utf16: None,
                    path,
                }
            }
        }
    }

    /// Build a UTF-16 path reference for this context.
    #[allow(dead_code)]
    pub(crate) fn path_utf16(&mut self, path: &Path) -> FsPathRef {
        match self.vm_context_mut() {
            Some(context) => {
                let units = path_utf16_vec(path);
                let array =
                    VmArray::from_values(context, &units).expect("vm utf16 path should encode");
                let path = OsPathVm {
                    encoding: PathEncoding::Utf16,
                    bytes: PathBytesAbi(empty_vm_array()),
                    utf16: PathUtf16Abi(array),
                };
                FsPathRef::Vm { path }
            }
            None => {
                let (utf16, path) = path_utf16_native(path);
                FsPathRef::Native {
                    _bytes: None,
                    _utf16: Some(utf16),
                    path,
                }
            }
        }
    }

    /// Render a path reference into a displayable string.
    #[allow(dead_code)]
    pub(crate) fn path_ref_string(&mut self, path: FsPathRef) -> String {
        match path {
            FsPathRef::Native { path, .. } => path_ref_string_native(path),
            FsPathRef::Vm { path } => path_ref_string_vm(
                self.vm_context_mut()
                    .expect("vm context required for vm path ref"),
                path,
            )
            .unwrap_or_else(|error| panic!("failed to decode vm path ref: {}", error.message())),
        }
    }

    /// Read raw bytes from a path reference.
    pub(crate) fn path_ref_bytes(&mut self, path: FsPathRef) -> Vec<u8> {
        match path {
            FsPathRef::Native { path, .. } => path_ref_bytes_native(path),
            FsPathRef::Vm { path } => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm path ref");
                path_ref_bytes_vm(context, path)
                    .unwrap_or_else(|error| panic!("failed to decode vm path bytes: {error:?}"))
            }
        }
    }

    /// Decode directory entries into a vector.
    pub(crate) fn dirents_from_vm(
        &mut self,
        entries: VmArray<DirentVm>,
    ) -> RuntimeResult<Vec<FsDirent>> {
        let context = self
            .vm_context_mut()
            .expect("vm context required for vm dirents");
        let raw = entries.raw_values(context)?;
        let mut decoded = Vec::with_capacity(raw.len());
        for value in raw {
            let dirent = decode_dirent_vm(context, value)?;
            decoded.push(FsDirent::Vm(dirent));
        }
        Ok(decoded)
    }

    /// Convert a directory entry name to a string.
    pub(crate) fn dirent_name(&mut self, entry: FsDirent) -> String {
        match entry {
            FsDirent::Native(entry) => path_ref_string_native(entry.name),
            FsDirent::Vm(entry) => path_ref_string_vm(
                self.vm_context_mut()
                    .expect("vm context required for vm dirent"),
                entry.name,
            )
            .unwrap_or_else(|error| panic!("failed to decode vm dirent name: {}", error.message())),
        }
    }

    /// Convert a native status into a test-friendly result.
    pub(crate) fn status_ok(&self, status: RuntimeStatus, label: &str) -> RuntimeResult<()> {
        self.runtime.assert_status_ok(status, label);
        Ok(())
    }

    /// Convert a native status into a runtime result.
    pub(crate) fn status_result(&self, status: RuntimeStatus, label: &str) -> RuntimeResult<()> {
        // fast path: success
        if status == RuntimeStatus::OK {
            return Ok(());
        }

        // decode status-only errors when no runtime id is attached
        if status.error_id == 0 {
            if status.code == PlatformErrorCode::NotSupported.number().saturating_add(1) {
                return Err(RuntimeError::from(PlatformError::not_supported(label)).boxed());
            }

            return Err(RuntimeError::from(PlatformError::io(format!(
                "{label} failed without runtime error id",
            )))
            .boxed());
        }

        // load the captured runtime error
        let error = self
            .runtime
            .runtime
            .context
            .errors()
            .take(RuntimeErrorId::from_raw(status.error_id))
            .unwrap_or_else(|| {
                RuntimeError::from(PlatformError::io(format!(
                    "{label} failed with missing runtime error",
                )))
                .boxed()
            });

        Err(error)
    }

    /// Convert a native error status into a test-friendly result.
    pub(crate) fn status_err(&self, status: RuntimeStatus, label: &str) -> RuntimeResult<()> {
        self.runtime.assert_status_err(status, label);
        Ok(())
    }

    /// Convert a native status into Ok, or treat allowed platform codes as acceptable failures.
    pub(crate) fn status_ok_or_codes(
        &self,
        status: RuntimeStatus,
        label: &str,
        allowed: &[PlatformErrorCode],
    ) -> RuntimeResult<bool> {
        // return early on success
        if status == RuntimeStatus::OK {
            return Ok(true);
        }

        // resolve the captured error
        let error_id = RuntimeErrorId::from_raw(status.error_id);
        let error = self
            .runtime
            .runtime
            .context
            .errors()
            .take(error_id)
            .ok_or_else(|| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    "status",
                    format!("{label} returned missing error"),
                ))
                .boxed()
            })?;

        // allow specific platform error codes
        if let Some(platform) = error.platform_error() {
            let code = platform.code;
            if allowed.contains(&code) {
                return Ok(false);
            }
        }

        Err(error)
    }

    /// Convert a VM result into Ok, or treat allowed platform codes as acceptable failures.
    pub(crate) fn result_ok_or_codes<T>(
        &self,
        result: RuntimeResult<T>,
        _label: &str,
        allowed: &[PlatformErrorCode],
    ) -> RuntimeResult<Option<T>> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error) => {
                if let Some(platform) = error.platform_error() {
                    let code = platform.code;
                    if allowed.contains(&code) {
                        return Ok(None);
                    }
                }
                Err(error)
            }
        }
    }

    /// Read the port assigned to a listener handle.
    #[cfg(any(unix, windows))]
    pub(crate) fn listener_port(&self, handle: ListenerHandle) -> u16 {
        self.runtime.listener_port(handle)
    }

    /// Start listening on the given address.
    #[cfg(any(unix, windows))]
    pub(crate) fn listen(
        &mut self,
        host: &str,
        port: u16,
        backlog: u32,
    ) -> RuntimeResult<ListenerHandle> {
        let family = if host.contains(':') {
            SocketFamily::IPv6
        } else {
            SocketFamily::IPv4
        };

        match self.vm_context_mut() {
            Some(context) => {
                let address = socket_address_vm_from_host_port(
                    self.call_context,
                    context,
                    host,
                    port,
                    family,
                )?;

                platform_net_vm::destack_net_listen(self.call_context, context, address, backlog)
            }
            None => {
                let address =
                    socket_address_native_from_host_port(self.call_context, host, port, family)?;

                let mut handle = ListenerHandle(ResourceId(0));
                let status = unsafe {
                    core_net::destack_net_listen(&mut handle, address.address(), backlog)
                };
                self.status_ok(status, "listen")?;

                Ok(handle)
            }
        }
    }

    /// Accept a new connection from a listener.
    #[cfg(any(unix, windows))]
    pub(crate) fn accept(&mut self, listener: ListenerHandle) -> RuntimeResult<SocketHandle> {
        let flags = AcceptFlags(0);
        match self.vm_context_mut() {
            Some(context) => {
                platform_net_vm::destack_net_accept(self.call_context, context, listener, flags)
            }
            None => {
                let mut handle = SocketHandle(ResourceId(0));
                let status = unsafe { core_net::destack_net_accept(&mut handle, listener, flags) };
                self.status_ok(status, "accept")?;

                Ok(handle)
            }
        }
    }

    /// Connect to a remote host and return a socket handle.
    #[cfg(any(unix, windows))]
    pub(crate) fn connect(&mut self, host: &str, port: u16) -> RuntimeResult<SocketHandle> {
        let family = if host.contains(':') {
            SocketFamily::IPv6
        } else {
            SocketFamily::IPv4
        };
        let socket_type = tcp_stream_socket_type();
        let protocol = tcp_protocol();

        match self.vm_context_mut() {
            Some(context) => {
                let address = socket_address_vm_from_host_port(
                    self.call_context,
                    context,
                    host,
                    port,
                    family,
                )?;
                let handle = platform_net_vm::destack_net_socket(
                    self.call_context,
                    context,
                    family,
                    socket_type,
                    protocol,
                )?;
                platform_net_vm::destack_net_connect(self.call_context, context, handle, address)?;

                Ok(handle)
            }
            None => {
                let address =
                    socket_address_native_from_host_port(self.call_context, host, port, family)?;
                let mut handle = SocketHandle(ResourceId(0));
                let socket_status = unsafe {
                    core_net::destack_net_socket(&mut handle, family, socket_type, protocol)
                };
                self.status_ok(socket_status, "connect socket")?;

                let connect_status =
                    unsafe { core_net::destack_net_connect(handle, address.address()) };
                self.status_ok(connect_status, "connect")?;

                Ok(handle)
            }
        }
    }

    /// Close a listener handle.
    #[cfg(any(unix, windows))]
    pub(crate) fn close_listener(&mut self, handle: ListenerHandle) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_net_vm::destack_net_close_listener(self.call_context, context, handle)
            }
            None => {
                let status = unsafe { core_net::destack_net_close_listener(handle) };
                self.status_ok(status, "close_listener")
            }
        }
    }

    /// Close a socket handle.
    #[cfg(any(unix, windows))]
    pub(crate) fn close_socket(&mut self, handle: SocketHandle) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_net_vm::destack_net_close(self.call_context, context, handle),
            None => {
                let status = unsafe { core_net::destack_net_close(handle) };
                self.status_ok(status, "close_socket")
            }
        }
    }

    /// Read from a socket into the provided buffer.
    #[cfg(any(unix, windows))]
    pub(crate) fn socket_read(
        &mut self,
        handle: SocketHandle,
        buffer: &mut [u8],
    ) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_slice = VmSlice::from_bytes(context, &vec![0u8; buffer.len()]);
                let bytes = platform_net_vm::destack_net_read(
                    self.call_context,
                    context,
                    handle,
                    vm_slice,
                )?;

                let read_bytes = vm_slice.read_bytes(context)?;
                let count = bytes as usize;
                if count > buffer.len() {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "buffer",
                        "read size exceeded buffer length",
                    ))
                    .boxed());
                }
                buffer[..count].copy_from_slice(&read_bytes[..count]);

                Ok(bytes)
            }
            None => {
                let slice = native_slice_mut(buffer);
                let mut out = 0u64;
                let status = unsafe { core_net::destack_net_read(&mut out, handle, slice) };
                self.status_ok(status, "socket_read")?;

                Ok(out)
            }
        }
    }

    /// Build a connected socket pair.
    #[cfg(any(unix, windows))]
    pub(crate) fn tcp_pair(&mut self) -> RuntimeResult<(SocketHandle, SocketHandle)> {
        let listener = self.listen("127.0.0.1", 0, 1)?;
        let port = self.listener_port(listener);
        let client = self.connect("127.0.0.1", port)?;
        let server = self.accept(listener)?;
        self.close_listener(listener)?;

        Ok((server, client))
    }

    /// Run fs::access for the current harness.
    pub(crate) fn access(
        &mut self,
        path: FsPathRef,
        mode: platform_fs::AccessMode,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_access(self.call_context, context, path, mode)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let status = unsafe { platform_fs::destack_fs_attrs_access(path, mode) };
                self.status_ok(status, "access")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::chmod for the current harness.
    pub(crate) fn chmod(&mut self, path: FsPathRef, mode: FileMode) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_chmod(self.call_context, context, path, mode)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let status = unsafe { platform_fs::destack_fs_attrs_chmod(path, mode) };
                self.status_ok(status, "chmod")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::fchmodat for the current harness.
    pub(crate) fn fchmodat(
        &mut self,
        dir: DirectoryHandle,
        path: FsPathRef,
        mode: FileMode,
        flags: AtFlags,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_fchmodat(self.call_context, context, dir, path, mode, flags)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let status =
                    unsafe { platform_fs::destack_fs_attrs_fchmodat(dir, path, mode, flags) };
                self.status_result(status, "fchmodat")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::chown for the current harness.
    pub(crate) fn chown(&mut self, path: FsPathRef, uid: u32, gid: u32) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_chown(self.call_context, context, path, uid, gid)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let status = unsafe { platform_fs::destack_fs_attrs_chown(path, uid, gid) };
                self.status_ok(status, "chown")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::fchownat for the current harness.
    pub(crate) fn fchownat(
        &mut self,
        dir: DirectoryHandle,
        path: FsPathRef,
        uid: u32,
        gid: u32,
        flags: AtFlags,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => platform_vm::destack_fs_fchownat(
                self.call_context,
                context,
                dir,
                path,
                uid,
                gid,
                flags,
            ),
            (None, FsPathRef::Native { path, .. }) => {
                let status =
                    unsafe { platform_fs::destack_fs_attrs_fchownat(dir, path, uid, gid, flags) };
                self.status_result(status, "fchownat")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::utimes for the current harness.
    pub(crate) fn utimes(
        &mut self,
        path: FsPathRef,
        atime_ns: u64,
        mtime_ns: u64,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_utimes(self.call_context, context, path, atime_ns, mtime_ns)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let status =
                    unsafe { platform_fs::destack_fs_attrs_utimes(path, atime_ns, mtime_ns) };
                self.status_ok(status, "utimes")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::utimensat for the current harness.
    pub(crate) fn utimensat(
        &mut self,
        dir: DirectoryHandle,
        path: FsPathRef,
        atime_ns: u64,
        mtime_ns: u64,
        flags: AtFlags,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => platform_vm::destack_fs_utimensat(
                self.call_context,
                context,
                dir,
                path,
                atime_ns,
                mtime_ns,
                flags,
            ),
            (None, FsPathRef::Native { path, .. }) => {
                let status = unsafe {
                    platform_fs::destack_fs_attrs_utimensat(dir, path, atime_ns, mtime_ns, flags)
                };
                self.status_result(status, "utimensat")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::lutimes for the current harness.
    pub(crate) fn lutimes(
        &mut self,
        path: FsPathRef,
        atime_ns: u64,
        mtime_ns: u64,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => platform_vm::destack_fs_lutimes(
                self.call_context,
                context,
                path,
                atime_ns,
                mtime_ns,
            ),
            (None, FsPathRef::Native { path, .. }) => {
                let status =
                    unsafe { platform_fs::destack_fs_attrs_lutimes(path, atime_ns, mtime_ns) };
                self.status_ok(status, "lutimes")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::mkdir for the current harness.
    pub(crate) fn mkdir(&mut self, path: FsPathRef, mode: FileMode) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_mkdir(self.call_context, context, path, mode)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let status = unsafe { platform_fs::destack_fs_dir_mkdir(path, mode) };
                self.status_ok(status, "mkdir")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::mkdirat for the current harness.
    pub(crate) fn mkdirat(
        &mut self,
        dir: DirectoryHandle,
        path: FsPathRef,
        mode: FileMode,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_mkdirat(self.call_context, context, dir, path, mode)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let status = unsafe { platform_fs::destack_fs_dir_mkdirat(dir, path, mode) };
                self.status_ok(status, "mkdirat")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::rmdir for the current harness.
    pub(crate) fn rmdir(&mut self, path: FsPathRef) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_rmdir(self.call_context, context, path)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let status = unsafe { platform_fs::destack_fs_dir_rmdir(path) };
                self.status_ok(status, "rmdir")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::unlink for the current harness.
    pub(crate) fn unlink(&mut self, path: FsPathRef) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_unlink(self.call_context, context, path)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let status = unsafe { platform_fs::destack_fs_path_unlink(path) };
                self.status_ok(status, "unlink")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::unlinkat for the current harness.
    pub(crate) fn unlinkat(
        &mut self,
        dir: DirectoryHandle,
        path: FsPathRef,
        flags: AtFlags,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_unlinkat(self.call_context, context, dir, path, flags)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let status = unsafe { platform_fs::destack_fs_path_unlinkat(dir, path, flags) };
                self.status_ok(status, "unlinkat")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::open for the current harness.
    pub(crate) fn open(
        &mut self,
        path: FsPathRef,
        flags: OpenFlags,
        mode: FileMode,
    ) -> RuntimeResult<FileHandle> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_open(self.call_context, context, path, flags, mode)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut handle = FileHandle(ResourceId(0));
                let status =
                    unsafe { platform_fs::destack_fs_file_open(&mut handle, path, flags, mode) };
                self.status_ok(status, "open")?;
                Ok(handle)
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::openat for the current harness.
    pub(crate) fn openat(
        &mut self,
        dir: DirectoryHandle,
        path: FsPathRef,
        flags: OpenFlags,
        mode: FileMode,
    ) -> RuntimeResult<FileHandle> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_openat(self.call_context, context, dir, path, flags, mode)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut handle = FileHandle(ResourceId(0));
                let status = unsafe {
                    platform_fs::destack_fs_file_openat(&mut handle, dir, path, flags, mode)
                };
                self.status_ok(status, "openat")?;
                Ok(handle)
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::openat2 for the current harness.
    pub(crate) fn openat2(
        &mut self,
        dir: DirectoryHandle,
        path: FsPathRef,
        how: OpenOptions,
    ) -> RuntimeResult<FileHandle> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_openat2(self.call_context, context, dir, path, how)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut handle = FileHandle(ResourceId(0));
                let status =
                    unsafe { platform_fs::destack_fs_file_openat2(&mut handle, dir, path, how) };
                self.status_result(status, "openat2")?;
                Ok(handle)
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::close for the current harness.
    pub(crate) fn close(&mut self, handle: FileHandle) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_close(self.call_context, context, handle),
            None => {
                let status = unsafe { platform_fs::destack_fs_file_close(handle) };
                self.status_ok(status, "close")
            }
        }
    }

    /// Run fs::opendir for the current harness.
    pub(crate) fn opendir(&mut self, path: FsPathRef) -> RuntimeResult<DirectoryHandle> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_opendir(self.call_context, context, path)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut handle = DirectoryHandle(ResourceId(0));
                let status = unsafe { platform_fs::destack_fs_dir_opendir(&mut handle, path) };
                self.status_ok(status, "opendir")?;
                Ok(handle)
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::closedir for the current harness.
    pub(crate) fn closedir(&mut self, handle: DirectoryHandle) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_closedir(self.call_context, context, handle),
            None => {
                let status = unsafe { platform_fs::destack_fs_dir_closedir(handle) };
                self.status_ok(status, "closedir")
            }
        }
    }

    /// Run fs::readdir for the current harness.
    pub(crate) fn readdir(&mut self, handle: DirectoryHandle) -> RuntimeResult<Vec<FsDirent>> {
        match self.vm_context_mut() {
            Some(context) => {
                let entries = platform_vm::destack_fs_readdir(self.call_context, context, handle)?;
                self.dirents_from_vm(entries)
            }
            None => {
                let mut out = std::mem::MaybeUninit::<NativeArray<Dirent>>::uninit();
                let status =
                    unsafe { platform_fs::destack_fs_dir_readdir(out.as_mut_ptr(), handle) };
                self.status_ok(status, "readdir")?;
                let entries = unsafe { out.assume_init() };
                let entries = unsafe { entries.as_slice()? };
                Ok(entries.iter().copied().map(FsDirent::Native).collect())
            }
        }
    }

    /// Run fs::mkdtemp for the current harness.
    pub(crate) fn mkdtemp(&mut self, template: FsPathRef) -> RuntimeResult<FsPathRef> {
        match (self.vm_context_mut(), template) {
            (Some(context), FsPathRef::Vm { path }) => {
                let path = platform_vm::destack_fs_mkdtemp(self.call_context, context, path)?;
                Ok(FsPathRef::Vm { path })
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut out = std::mem::MaybeUninit::<OsPath>::uninit();
                let status = unsafe { platform_fs::destack_fs_dir_mkdtemp(out.as_mut_ptr(), path) };
                self.status_ok(status, "mkdtemp")?;
                Ok(own_path_ref_native(unsafe { out.assume_init() }))
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::read for the current harness.
    pub(crate) fn read(&mut self, handle: FileHandle, buffer: &mut [u8]) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                let slice = VmSlice::from_bytes(context, buffer);
                let bytes =
                    platform_vm::destack_fs_read(self.call_context, context, handle, slice)?;
                let read_back = slice.read_bytes(context)?;
                buffer[..bytes as usize].copy_from_slice(&read_back[..bytes as usize]);
                Ok(bytes)
            }
            None => {
                let slice = native_slice_mut(buffer);
                let mut out = 0u64;
                let status = unsafe { platform_fs::destack_fs_file_read(&mut out, handle, slice) };
                self.status_ok(status, "read")?;
                Ok(out)
            }
        }
    }

    /// Run fs::pread for the current harness.
    pub(crate) fn pread(
        &mut self,
        handle: FileHandle,
        buffer: &mut [u8],
        offset: FileOffset,
    ) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                let slice = VmSlice::from_bytes(context, buffer);
                let bytes = platform_vm::destack_fs_pread(
                    self.call_context,
                    context,
                    handle,
                    slice,
                    offset,
                )?;
                let read_back = slice.read_bytes(context)?;
                buffer[..bytes as usize].copy_from_slice(&read_back[..bytes as usize]);
                Ok(bytes)
            }
            None => {
                let slice = native_slice_mut(buffer);
                let mut out = 0u64;
                let status =
                    unsafe { platform_fs::destack_fs_file_pread(&mut out, handle, slice, offset) };
                self.status_ok(status, "pread")?;
                Ok(out)
            }
        }
    }

    /// Run fs::readv for the current harness.
    pub(crate) fn readv(
        &mut self,
        handle: FileHandle,
        buffers: &mut [&mut [u8]],
    ) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_buffers = buffers
                    .iter()
                    .map(|buffer| VmSlice::from_bytes(context, buffer))
                    .collect::<Vec<_>>();
                let vm_values = vm_buffers
                    .iter()
                    .copied()
                    .map(|slice| slice.to_value(context))
                    .collect::<Vec<_>>();
                let data = context.allocate_raw_values(vm_values);
                let vm_slice = VmSlice {
                    data,
                    len: vm_buffers.len() as u32,
                    _marker: std::marker::PhantomData::<VmSlice<u8>>,
                };
                let bytes =
                    platform_vm::destack_fs_readv(self.call_context, context, handle, vm_slice)?;
                for (buffer, vm_buffer) in buffers.iter_mut().zip(vm_buffers.iter()) {
                    let read_back = vm_buffer.read_bytes(context)?;
                    let length = buffer.len().min(read_back.len());
                    buffer[..length].copy_from_slice(&read_back[..length]);
                }
                Ok(bytes)
            }
            None => {
                let mut native_buffers = buffers
                    .iter_mut()
                    .map(|buffer| native_slice_mut(buffer))
                    .collect::<Vec<_>>();
                let slice = NativeSlice {
                    data: native_buffers.as_mut_ptr(),
                    len: native_buffers.len() as u32,
                };
                let mut out = 0u64;
                let status = unsafe { platform_fs::destack_fs_file_readv(&mut out, handle, slice) };
                self.status_ok(status, "readv")?;
                Ok(out)
            }
        }
    }

    /// Run fs::preadv for the current harness.
    pub(crate) fn preadv(
        &mut self,
        handle: FileHandle,
        buffers: &mut [&mut [u8]],
        offset: FileOffset,
    ) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_buffers = buffers
                    .iter()
                    .map(|buffer| VmSlice::from_bytes(context, buffer))
                    .collect::<Vec<_>>();
                let vm_values = vm_buffers
                    .iter()
                    .copied()
                    .map(|slice| slice.to_value(context))
                    .collect::<Vec<_>>();
                let data = context.allocate_raw_values(vm_values);
                let vm_slice = VmSlice {
                    data,
                    len: vm_buffers.len() as u32,
                    _marker: std::marker::PhantomData::<VmSlice<u8>>,
                };
                let bytes = platform_vm::destack_fs_preadv(
                    self.call_context,
                    context,
                    handle,
                    vm_slice,
                    offset,
                )?;
                for (buffer, vm_buffer) in buffers.iter_mut().zip(vm_buffers.iter()) {
                    let read_back = vm_buffer.read_bytes(context)?;
                    let length = buffer.len().min(read_back.len());
                    buffer[..length].copy_from_slice(&read_back[..length]);
                }
                Ok(bytes)
            }
            None => {
                let mut native_buffers = buffers
                    .iter_mut()
                    .map(|buffer| native_slice_mut(buffer))
                    .collect::<Vec<_>>();
                let slice = NativeSlice {
                    data: native_buffers.as_mut_ptr(),
                    len: native_buffers.len() as u32,
                };
                let mut out = 0u64;
                let status =
                    unsafe { platform_fs::destack_fs_file_preadv(&mut out, handle, slice, offset) };
                self.status_ok(status, "preadv")?;
                Ok(out)
            }
        }
    }

    /// Run fs::write for the current harness.
    pub(crate) fn write(&mut self, handle: FileHandle, buffer: &[u8]) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                let slice = VmSlice::from_bytes(context, buffer);
                platform_vm::destack_fs_write(self.call_context, context, handle, slice)
            }
            None => {
                let slice = native_slice(buffer);
                let mut out = 0u64;
                let status = unsafe { platform_fs::destack_fs_file_write(&mut out, handle, slice) };
                self.status_ok(status, "write")?;
                Ok(out)
            }
        }
    }

    /// Run fs::writev for the current harness.
    pub(crate) fn writev(&mut self, handle: FileHandle, buffers: &[&[u8]]) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_buffers = buffers
                    .iter()
                    .copied()
                    .map(|buffer| VmSlice::from_bytes(context, buffer))
                    .collect::<Vec<_>>();
                let vm_values = vm_buffers
                    .iter()
                    .copied()
                    .map(|slice| slice.to_value(context))
                    .collect::<Vec<_>>();
                let data = context.allocate_raw_values(vm_values);
                let vm_slice = VmSlice {
                    data,
                    len: vm_buffers.len() as u32,
                    _marker: std::marker::PhantomData::<VmSlice<u8>>,
                };
                platform_vm::destack_fs_writev(self.call_context, context, handle, vm_slice)
            }
            None => {
                let mut native_buffers = buffers
                    .iter()
                    .copied()
                    .map(native_slice)
                    .collect::<Vec<_>>();
                let slice = NativeSlice {
                    data: native_buffers.as_mut_ptr(),
                    len: native_buffers.len() as u32,
                };
                let mut out = 0u64;
                let status =
                    unsafe { platform_fs::destack_fs_file_writev(&mut out, handle, slice) };
                self.status_ok(status, "writev")?;
                Ok(out)
            }
        }
    }

    /// Run fs::pwrite for the current harness.
    pub(crate) fn pwrite(
        &mut self,
        handle: FileHandle,
        buffer: &[u8],
        offset: FileOffset,
    ) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                let slice = VmSlice::from_bytes(context, buffer);
                platform_vm::destack_fs_pwrite(self.call_context, context, handle, slice, offset)
            }
            None => {
                let slice = native_slice(buffer);
                let mut out = 0u64;
                let status =
                    unsafe { platform_fs::destack_fs_file_pwrite(&mut out, handle, slice, offset) };
                self.status_ok(status, "pwrite")?;
                Ok(out)
            }
        }
    }

    /// Run fs::pwritev for the current harness.
    pub(crate) fn pwritev(
        &mut self,
        handle: FileHandle,
        buffers: &[&[u8]],
        offset: FileOffset,
    ) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => {
                let vm_buffers = buffers
                    .iter()
                    .map(|buffer| VmSlice::from_bytes(context, buffer))
                    .collect::<Vec<_>>();
                let vm_values = vm_buffers
                    .iter()
                    .copied()
                    .map(|slice| slice.to_value(context))
                    .collect::<Vec<_>>();
                let data = context.allocate_raw_values(vm_values);
                let vm_slice = VmSlice {
                    data,
                    len: vm_buffers.len() as u32,
                    _marker: std::marker::PhantomData::<VmSlice<u8>>,
                };
                platform_vm::destack_fs_pwritev(
                    self.call_context,
                    context,
                    handle,
                    vm_slice,
                    offset,
                )
            }
            None => {
                let mut native_buffers = buffers
                    .iter()
                    .map(|buffer| native_slice(buffer))
                    .collect::<Vec<_>>();
                let slice = NativeSlice {
                    data: native_buffers.as_mut_ptr(),
                    len: native_buffers.len() as u32,
                };
                let mut out = 0u64;
                let status = unsafe {
                    platform_fs::destack_fs_file_pwritev(&mut out, handle, slice, offset)
                };
                self.status_ok(status, "pwritev")?;
                Ok(out)
            }
        }
    }

    /// Run fs::truncate for the current harness.
    pub(crate) fn truncate(&mut self, path: FsPathRef, size: FileOffset) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_truncate(self.call_context, context, path, size)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let status = unsafe { platform_fs::destack_fs_file_truncate(path, size) };
                self.status_ok(status, "truncate")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::ftruncate for the current harness.
    pub(crate) fn ftruncate(&mut self, handle: FileHandle, size: FileOffset) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_fs_ftruncate(self.call_context, context, handle, size)
            }
            None => {
                let status = unsafe { platform_fs::destack_fs_file_ftruncate(handle, size) };
                self.status_ok(status, "ftruncate")
            }
        }
    }

    /// Run fs::fsync for the current harness.
    pub(crate) fn fsync(&mut self, handle: FileHandle) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_fsync(self.call_context, context, handle),
            None => {
                let status = unsafe { platform_fs::destack_fs_file_fsync(handle) };
                self.status_ok(status, "fsync")
            }
        }
    }

    /// Run fs::fdatasync for the current harness.
    pub(crate) fn fdatasync(&mut self, handle: FileHandle) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_fdatasync(self.call_context, context, handle),
            None => {
                let status = unsafe { platform_fs::destack_fs_file_fdatasync(handle) };
                self.status_ok(status, "fdatasync")
            }
        }
    }

    /// Run fs::syncFileRange for the current harness.
    pub(crate) fn sync_file_range(
        &mut self,
        handle: FileHandle,
        offset: FileOffset,
        length: FileSize,
        flags: SyncFlags,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_sync_file_range(
                self.call_context,
                context,
                handle,
                offset,
                length,
                flags,
            ),
            None => {
                let status = unsafe {
                    platform_fs::destack_fs_file_sync_file_range(handle, offset, length, flags)
                };
                self.status_ok(status, "syncFileRange")
            }
        }
    }

    /// Run fs::fallocate for the current harness.
    pub(crate) fn fallocate(
        &mut self,
        handle: FileHandle,
        offset: FileOffset,
        length: FileSize,
        flags: AllocFlags,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_fallocate(
                self.call_context,
                context,
                handle,
                offset,
                length,
                flags,
            ),
            None => {
                let status = unsafe {
                    platform_fs::destack_fs_file_fallocate(handle, offset, length, flags)
                };
                self.status_ok(status, "fallocate")
            }
        }
    }

    /// Run fs::fadvise for the current harness.
    pub(crate) fn fadvise(
        &mut self,
        handle: FileHandle,
        offset: FileOffset,
        length: FileSize,
        advice: FileAdvice,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_fadvise(
                self.call_context,
                context,
                handle,
                offset,
                length,
                advice,
            ),
            None => {
                let status =
                    unsafe { platform_fs::destack_fs_file_fadvise(handle, offset, length, advice) };
                self.status_ok(status, "fadvise")
            }
        }
    }

    /// Run fs::stat for the current harness.
    pub(crate) fn stat(&mut self, path: FsPathRef) -> RuntimeResult<Stat> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_stat(self.call_context, context, path)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut out = std::mem::MaybeUninit::<Stat>::uninit();
                let status = unsafe { platform_fs::destack_fs_stat_stat(out.as_mut_ptr(), path) };
                self.status_ok(status, "stat")?;
                Ok(unsafe { out.assume_init() })
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::statat for the current harness.
    pub(crate) fn statat(
        &mut self,
        dir: DirectoryHandle,
        path: FsPathRef,
        flags: AtFlags,
    ) -> RuntimeResult<Stat> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_statat(self.call_context, context, dir, path, flags)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut out = std::mem::MaybeUninit::<Stat>::uninit();
                let status = unsafe {
                    platform_fs::destack_fs_stat_statat(out.as_mut_ptr(), dir, path, flags)
                };
                self.status_ok(status, "statat")?;
                Ok(unsafe { out.assume_init() })
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::lstat for the current harness.
    pub(crate) fn lstat(&mut self, path: FsPathRef) -> RuntimeResult<Stat> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_lstat(self.call_context, context, path)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut out = std::mem::MaybeUninit::<Stat>::uninit();
                let status = unsafe { platform_fs::destack_fs_stat_lstat(out.as_mut_ptr(), path) };
                self.status_ok(status, "lstat")?;
                Ok(unsafe { out.assume_init() })
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::fstat for the current harness.
    pub(crate) fn fstat(&mut self, handle: FileHandle) -> RuntimeResult<Stat> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_fstat(self.call_context, context, handle),
            None => {
                let mut out = std::mem::MaybeUninit::<Stat>::uninit();
                let status =
                    unsafe { platform_fs::destack_fs_stat_fstat(out.as_mut_ptr(), handle) };
                self.status_ok(status, "fstat")?;
                Ok(unsafe { out.assume_init() })
            }
        }
    }

    /// Run fs::statfs for the current harness.
    pub(crate) fn statfs(&mut self, path: FsPathRef) -> RuntimeResult<StatFs> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                platform_vm::destack_fs_statfs(self.call_context, context, path)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut out = std::mem::MaybeUninit::<StatFs>::uninit();
                let status = unsafe { platform_fs::destack_fs_stat_statfs(out.as_mut_ptr(), path) };
                self.status_ok(status, "statfs")?;
                Ok(unsafe { out.assume_init() })
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::fstatfs for the current harness.
    pub(crate) fn fstatfs(&mut self, handle: FileHandle) -> RuntimeResult<StatFs> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_fstatfs(self.call_context, context, handle),
            None => {
                let mut out = std::mem::MaybeUninit::<StatFs>::uninit();
                let status =
                    unsafe { platform_fs::destack_fs_stat_fstatfs(out.as_mut_ptr(), handle) };
                self.status_ok(status, "fstatfs")?;
                Ok(unsafe { out.assume_init() })
            }
        }
    }

    /// Run fs::link for the current harness.
    pub(crate) fn link(&mut self, from: FsPathRef, to: FsPathRef) -> RuntimeResult<()> {
        match (self.vm_context_mut(), from, to) {
            (Some(context), FsPathRef::Vm { path: from }, FsPathRef::Vm { path: to }) => {
                platform_vm::destack_fs_link(self.call_context, context, from, to)
            }
            (None, FsPathRef::Native { path: from, .. }, FsPathRef::Native { path: to, .. }) => {
                let status = unsafe { platform_fs::destack_fs_path_link(from, to) };
                self.status_ok(status, "link")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::linkat for the current harness.
    pub(crate) fn linkat(
        &mut self,
        existing_dir: DirectoryHandle,
        existing: FsPathRef,
        new_dir: DirectoryHandle,
        new: FsPathRef,
        flags: AtFlags,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), existing, new) {
            (Some(context), FsPathRef::Vm { path: existing }, FsPathRef::Vm { path: new }) => {
                platform_vm::destack_fs_linkat(
                    self.call_context,
                    context,
                    existing_dir,
                    existing,
                    new_dir,
                    new,
                    flags,
                )
            }
            (
                None,
                FsPathRef::Native { path: existing, .. },
                FsPathRef::Native { path: new, .. },
            ) => {
                let status = unsafe {
                    platform_fs::destack_fs_path_linkat(existing_dir, existing, new_dir, new, flags)
                };
                self.status_ok(status, "linkat")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::symlink for the current harness.
    pub(crate) fn symlink(
        &mut self,
        target: FsPathRef,
        link: FsPathRef,
        kind: SymlinkType,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), target, link) {
            (Some(context), FsPathRef::Vm { path: target }, FsPathRef::Vm { path: link }) => {
                platform_vm::destack_fs_symlink(self.call_context, context, target, link, kind)
            }
            (
                None,
                FsPathRef::Native { path: target, .. },
                FsPathRef::Native { path: link, .. },
            ) => {
                let status = unsafe { platform_fs::destack_fs_path_symlink(target, link, kind) };
                self.status_ok(status, "symlink")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::symlinkat for the current harness.
    pub(crate) fn symlinkat(
        &mut self,
        target: FsPathRef,
        dir: DirectoryHandle,
        link: FsPathRef,
        kind: SymlinkType,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), target, link) {
            (Some(context), FsPathRef::Vm { path: target }, FsPathRef::Vm { path: link }) => {
                platform_vm::destack_fs_symlinkat(
                    self.call_context,
                    context,
                    target,
                    dir,
                    link,
                    kind,
                )
            }
            (
                None,
                FsPathRef::Native { path: target, .. },
                FsPathRef::Native { path: link, .. },
            ) => {
                let status =
                    unsafe { platform_fs::destack_fs_path_symlinkat(target, dir, link, kind) };
                self.status_ok(status, "symlinkat")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::readlink for the current harness.
    pub(crate) fn readlink(&mut self, link: FsPathRef) -> RuntimeResult<FsPathRef> {
        match (self.vm_context_mut(), link) {
            (Some(context), FsPathRef::Vm { path }) => {
                let path = platform_vm::destack_fs_readlink(self.call_context, context, path)?;
                Ok(FsPathRef::Vm { path })
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut out = std::mem::MaybeUninit::<OsPath>::uninit();
                let status =
                    unsafe { platform_fs::destack_fs_path_readlink(out.as_mut_ptr(), path) };
                self.status_ok(status, "readlink")?;
                Ok(own_path_ref_native(unsafe { out.assume_init() }))
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::readlinkat for the current harness.
    pub(crate) fn readlinkat(
        &mut self,
        dir: DirectoryHandle,
        link: FsPathRef,
    ) -> RuntimeResult<FsPathRef> {
        match (self.vm_context_mut(), link) {
            (Some(context), FsPathRef::Vm { path }) => {
                let path =
                    platform_vm::destack_fs_readlinkat(self.call_context, context, dir, path)?;
                Ok(FsPathRef::Vm { path })
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut out = std::mem::MaybeUninit::<OsPath>::uninit();
                let status =
                    unsafe { platform_fs::destack_fs_path_readlinkat(out.as_mut_ptr(), dir, path) };
                self.status_ok(status, "readlinkat")?;
                Ok(own_path_ref_native(unsafe { out.assume_init() }))
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::rename for the current harness.
    pub(crate) fn rename(&mut self, from: FsPathRef, to: FsPathRef) -> RuntimeResult<()> {
        match (self.vm_context_mut(), from, to) {
            (Some(context), FsPathRef::Vm { path: from }, FsPathRef::Vm { path: to }) => {
                platform_vm::destack_fs_rename(self.call_context, context, from, to)
            }
            (None, FsPathRef::Native { path: from, .. }, FsPathRef::Native { path: to, .. }) => {
                let status = unsafe { platform_fs::destack_fs_path_rename(from, to) };
                self.status_ok(status, "rename")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::renameat for the current harness.
    pub(crate) fn renameat(
        &mut self,
        from_dir: DirectoryHandle,
        from: FsPathRef,
        to_dir: DirectoryHandle,
        to: FsPathRef,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), from, to) {
            (Some(context), FsPathRef::Vm { path: from }, FsPathRef::Vm { path: to }) => {
                platform_vm::destack_fs_renameat(
                    self.call_context,
                    context,
                    from_dir,
                    from,
                    to_dir,
                    to,
                )
            }
            (None, FsPathRef::Native { path: from, .. }, FsPathRef::Native { path: to, .. }) => {
                let status =
                    unsafe { platform_fs::destack_fs_path_renameat(from_dir, from, to_dir, to) };
                self.status_ok(status, "renameat")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::renameat2 for the current harness.
    pub(crate) fn renameat2(
        &mut self,
        from_dir: DirectoryHandle,
        from: FsPathRef,
        to_dir: DirectoryHandle,
        to: FsPathRef,
        flags: RenameFlags,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), from, to) {
            (Some(context), FsPathRef::Vm { path: from }, FsPathRef::Vm { path: to }) => {
                platform_vm::destack_fs_renameat2(
                    self.call_context,
                    context,
                    from_dir,
                    from,
                    to_dir,
                    to,
                    flags,
                )
            }
            (None, FsPathRef::Native { path: from, .. }, FsPathRef::Native { path: to, .. }) => {
                let status = unsafe {
                    platform_fs::destack_fs_path_renameat2(from_dir, from, to_dir, to, flags)
                };
                self.status_result(status, "renameat2")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::copyfile for the current harness.
    pub(crate) fn copyfile(
        &mut self,
        from: FsPathRef,
        to: FsPathRef,
        flags: CopyFlags,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), from, to) {
            (Some(context), FsPathRef::Vm { path: from }, FsPathRef::Vm { path: to }) => {
                platform_vm::destack_fs_copyfile(self.call_context, context, from, to, flags)
            }
            (None, FsPathRef::Native { path: from, .. }, FsPathRef::Native { path: to, .. }) => {
                let status = unsafe { platform_fs::destack_fs_path_copyfile(from, to, flags) };
                self.status_ok(status, "copyfile")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::copyFileRange for the current harness.
    pub(crate) fn copy_file_range(
        &mut self,
        src: FileHandle,
        src_offset: FileOffset,
        dst: FileHandle,
        dst_offset: FileOffset,
        length: FileSize,
    ) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_copy_file_range(
                self.call_context,
                context,
                src,
                src_offset,
                dst,
                dst_offset,
                length,
            ),
            None => {
                let mut out = 0u64;
                let status = unsafe {
                    platform_fs::destack_fs_file_copy_file_range(
                        &mut out, src, src_offset, dst, dst_offset, length,
                    )
                };
                self.status_ok(status, "copyFileRange")?;
                Ok(out)
            }
        }
    }

    /// Run fs::sendfile for the current harness.
    pub(crate) fn sendfile(
        &mut self,
        socket: SocketHandle,
        file: FileHandle,
        offset: FileOffset,
        length: FileSize,
    ) -> RuntimeResult<u64> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_sendfile(
                self.call_context,
                context,
                socket,
                file,
                offset,
                length,
            ),
            None => {
                let mut out = 0u64;
                let status = unsafe {
                    platform_fs::destack_fs_file_sendfile(&mut out, socket, file, offset, length)
                };
                self.status_ok(status, "sendfile")?;
                Ok(out)
            }
        }
    }

    /// Run fs::realpath for the current harness.
    pub(crate) fn realpath(&mut self, path: FsPathRef) -> RuntimeResult<FsPathRef> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                let path = platform_vm::destack_fs_realpath(self.call_context, context, path)?;
                Ok(FsPathRef::Vm { path })
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut out = std::mem::MaybeUninit::<OsPath>::uninit();
                let status =
                    unsafe { platform_fs::destack_fs_path_realpath(out.as_mut_ptr(), path) };
                self.status_ok(status, "realpath")?;
                Ok(own_path_ref_native(unsafe { out.assume_init() }))
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::fchmod for the current harness.
    pub(crate) fn fchmod(&mut self, handle: FileHandle, mode: FileMode) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_fs_fchmod(self.call_context, context, handle, mode)
            }
            None => {
                let status = unsafe { platform_fs::destack_fs_attrs_fchmod(handle, mode) };
                self.status_ok(status, "fchmod")
            }
        }
    }

    /// Run fs::fchown for the current harness.
    pub(crate) fn fchown(&mut self, handle: FileHandle, uid: u32, gid: u32) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_fs_fchown(self.call_context, context, handle, uid, gid)
            }
            None => {
                let status = unsafe { platform_fs::destack_fs_attrs_fchown(handle, uid, gid) };
                self.status_ok(status, "fchown")
            }
        }
    }

    /// Run fs::futimes for the current harness.
    pub(crate) fn futimes(
        &mut self,
        handle: FileHandle,
        atime_ns: u64,
        mtime_ns: u64,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_futimes(
                self.call_context,
                context,
                handle,
                atime_ns,
                mtime_ns,
            ),
            None => {
                let status =
                    unsafe { platform_fs::destack_fs_attrs_futimes(handle, atime_ns, mtime_ns) };
                self.status_ok(status, "futimes")
            }
        }
    }

    /// Run fs::seek for the current harness.
    pub(crate) fn seek(
        &mut self,
        handle: FileHandle,
        offset: FileOffset,
        whence: SeekWhence,
    ) -> RuntimeResult<FileOffset> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_fs_seek(self.call_context, context, handle, offset, whence)
            }
            None => {
                let mut out = FileOffset(0);
                let status =
                    unsafe { platform_fs::destack_fs_file_seek(&mut out, handle, offset, whence) };
                self.status_ok(status, "seek")?;
                Ok(out)
            }
        }
    }

    /// Run fs::dup for the current harness.
    pub(crate) fn dup(&mut self, handle: FileHandle) -> RuntimeResult<FileHandle> {
        match self.vm_context_mut() {
            Some(context) => platform_vm::destack_fs_dup(self.call_context, context, handle),
            None => {
                let mut out = FileHandle(ResourceId(0));
                let status = unsafe { platform_fs::destack_fs_file_dup(&mut out, handle) };
                self.status_ok(status, "dup")?;
                Ok(out)
            }
        }
    }

    /// Run fs::dup2 for the current harness.
    pub(crate) fn dup2(
        &mut self,
        handle: FileHandle,
        target: FileHandle,
    ) -> RuntimeResult<FileHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_fs_dup2(self.call_context, context, handle, target)
            }
            None => {
                let mut out = FileHandle(ResourceId(0));
                let status = unsafe { platform_fs::destack_fs_file_dup2(&mut out, handle, target) };
                self.status_ok(status, "dup2")?;
                Ok(out)
            }
        }
    }

    /// Run fs::dup3 for the current harness.
    pub(crate) fn dup3(
        &mut self,
        handle: FileHandle,
        target: FileHandle,
        flags: OpenFlags,
    ) -> RuntimeResult<FileHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_fs_dup3(self.call_context, context, handle, target, flags)
            }
            None => {
                let mut out = FileHandle(ResourceId(0));
                let status =
                    unsafe { platform_fs::destack_fs_file_dup3(&mut out, handle, target, flags) };
                self.status_ok(status, "dup3")?;
                Ok(out)
            }
        }
    }

    /// Run fs::lock for the current harness.
    pub(crate) fn lock(&mut self, handle: FileHandle, flags: FileLockFlags) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                platform_vm::destack_fs_lock(self.call_context, context, handle, flags)
            }
            None => {
                let status = unsafe { platform_fs::destack_fs_file_lock(handle, flags) };
                self.status_ok(status, "lock")
            }
        }
    }

    /// Run fs::getxattr for the current harness.
    pub(crate) fn getxattr(&mut self, path: FsPathRef, name: &str) -> RuntimeResult<Vec<u8>> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                let name = vm_string(context, name);
                let values =
                    platform_vm::destack_fs_getxattr(self.call_context, context, path, name)?;
                array_u8_vm(context, values)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut out = std::mem::MaybeUninit::<NativeArray<u8>>::uninit();
                let name = NativeStringRef::from(name);
                let status =
                    unsafe { platform_fs::destack_fs_xattr_getxattr(out.as_mut_ptr(), path, name) };
                self.status_ok(status, "getxattr")?;
                let values = unsafe { out.assume_init() };
                array_u8_native(values)
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::lgetxattr for the current harness.
    pub(crate) fn lgetxattr(&mut self, path: FsPathRef, name: &str) -> RuntimeResult<Vec<u8>> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                let name = vm_string(context, name);
                let values =
                    platform_vm::destack_fs_lgetxattr(self.call_context, context, path, name)?;
                array_u8_vm(context, values)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut out = std::mem::MaybeUninit::<NativeArray<u8>>::uninit();
                let name = NativeStringRef::from(name);
                let status = unsafe {
                    platform_fs::destack_fs_xattr_lgetxattr(out.as_mut_ptr(), path, name)
                };
                self.status_ok(status, "lgetxattr")?;
                let values = unsafe { out.assume_init() };
                array_u8_native(values)
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::fgetxattr for the current harness.
    pub(crate) fn fgetxattr(&mut self, handle: FileHandle, name: &str) -> RuntimeResult<Vec<u8>> {
        match self.vm_context_mut() {
            Some(context) => {
                let name = vm_string(context, name);
                let values =
                    platform_vm::destack_fs_fgetxattr(self.call_context, context, handle, name)?;
                array_u8_vm(context, values)
            }
            None => {
                let mut out = std::mem::MaybeUninit::<NativeArray<u8>>::uninit();
                let name = NativeStringRef::from(name);
                let status = unsafe {
                    platform_fs::destack_fs_xattr_fgetxattr(out.as_mut_ptr(), handle, name)
                };
                self.status_ok(status, "fgetxattr")?;
                let values = unsafe { out.assume_init() };
                array_u8_native(values)
            }
        }
    }

    /// Run fs::setxattr for the current harness.
    pub(crate) fn setxattr(
        &mut self,
        path: FsPathRef,
        name: &str,
        value: &[u8],
        flags: XattrFlags,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                let name = vm_string(context, name);
                let value = VmSlice::from_bytes(context, value);
                platform_vm::destack_fs_setxattr(
                    self.call_context,
                    context,
                    path,
                    name,
                    value,
                    flags,
                )
            }
            (None, FsPathRef::Native { path, .. }) => {
                let name = NativeStringRef::from(name);
                let value = native_slice(value);
                let status =
                    unsafe { platform_fs::destack_fs_xattr_setxattr(path, name, value, flags) };
                self.status_ok(status, "setxattr")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::lsetxattr for the current harness.
    pub(crate) fn lsetxattr(
        &mut self,
        path: FsPathRef,
        name: &str,
        value: &[u8],
        flags: XattrFlags,
    ) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                let name = vm_string(context, name);
                let value = VmSlice::from_bytes(context, value);
                platform_vm::destack_fs_lsetxattr(
                    self.call_context,
                    context,
                    path,
                    name,
                    value,
                    flags,
                )
            }
            (None, FsPathRef::Native { path, .. }) => {
                let name = NativeStringRef::from(name);
                let value = native_slice(value);
                let status =
                    unsafe { platform_fs::destack_fs_xattr_lsetxattr(path, name, value, flags) };
                self.status_ok(status, "lsetxattr")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::fsetxattr for the current harness.
    pub(crate) fn fsetxattr(
        &mut self,
        handle: FileHandle,
        name: &str,
        value: &[u8],
        flags: XattrFlags,
    ) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                let name = vm_string(context, name);
                let value = VmSlice::from_bytes(context, value);
                platform_vm::destack_fs_fsetxattr(
                    self.call_context,
                    context,
                    handle,
                    name,
                    value,
                    flags,
                )
            }
            None => {
                let name = NativeStringRef::from(name);
                let value = native_slice(value);
                let status =
                    unsafe { platform_fs::destack_fs_xattr_fsetxattr(handle, name, value, flags) };
                self.status_ok(status, "fsetxattr")
            }
        }
    }

    /// Run fs::listxattr for the current harness.
    pub(crate) fn listxattr(&mut self, path: FsPathRef) -> RuntimeResult<Vec<String>> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                let names = platform_vm::destack_fs_listxattr(self.call_context, context, path)?;
                array_string_vm(context, names)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut out = std::mem::MaybeUninit::<NativeArray<NativeStringRef>>::uninit();
                let status =
                    unsafe { platform_fs::destack_fs_xattr_listxattr(out.as_mut_ptr(), path) };
                self.status_ok(status, "listxattr")?;
                let names = unsafe { out.assume_init() };
                array_string_native(names)
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::llistxattr for the current harness.
    pub(crate) fn llistxattr(&mut self, path: FsPathRef) -> RuntimeResult<Vec<String>> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                let names = platform_vm::destack_fs_llistxattr(self.call_context, context, path)?;
                array_string_vm(context, names)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let mut out = std::mem::MaybeUninit::<NativeArray<NativeStringRef>>::uninit();
                let status =
                    unsafe { platform_fs::destack_fs_xattr_llistxattr(out.as_mut_ptr(), path) };
                self.status_ok(status, "llistxattr")?;
                let names = unsafe { out.assume_init() };
                array_string_native(names)
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::flistxattr for the current harness.
    pub(crate) fn flistxattr(&mut self, handle: FileHandle) -> RuntimeResult<Vec<String>> {
        match self.vm_context_mut() {
            Some(context) => {
                let names = platform_vm::destack_fs_flistxattr(self.call_context, context, handle)?;
                array_string_vm(context, names)
            }
            None => {
                let mut out = std::mem::MaybeUninit::<NativeArray<NativeStringRef>>::uninit();
                let status =
                    unsafe { platform_fs::destack_fs_xattr_flistxattr(out.as_mut_ptr(), handle) };
                self.status_ok(status, "flistxattr")?;
                let names = unsafe { out.assume_init() };
                array_string_native(names)
            }
        }
    }

    /// Run fs::removexattr for the current harness.
    pub(crate) fn removexattr(&mut self, path: FsPathRef, name: &str) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                let name = vm_string(context, name);
                platform_vm::destack_fs_removexattr(self.call_context, context, path, name)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let name = NativeStringRef::from(name);
                let status = unsafe { platform_fs::destack_fs_xattr_removexattr(path, name) };
                self.status_ok(status, "removexattr")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::lremovexattr for the current harness.
    pub(crate) fn lremovexattr(&mut self, path: FsPathRef, name: &str) -> RuntimeResult<()> {
        match (self.vm_context_mut(), path) {
            (Some(context), FsPathRef::Vm { path }) => {
                let name = vm_string(context, name);
                platform_vm::destack_fs_lremovexattr(self.call_context, context, path, name)
            }
            (None, FsPathRef::Native { path, .. }) => {
                let name = NativeStringRef::from(name);
                let status = unsafe { platform_fs::destack_fs_xattr_lremovexattr(path, name) };
                self.status_ok(status, "lremovexattr")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "path",
                "path encoding mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::fremovexattr for the current harness.
    pub(crate) fn fremovexattr(&mut self, handle: FileHandle, name: &str) -> RuntimeResult<()> {
        match self.vm_context_mut() {
            Some(context) => {
                let name = vm_string(context, name);
                platform_vm::destack_fs_fremovexattr(self.call_context, context, handle, name)
            }
            None => {
                let name = NativeStringRef::from(name);
                let status = unsafe { platform_fs::destack_fs_xattr_fremovexattr(handle, name) };
                self.status_ok(status, "fremovexattr")
            }
        }
    }

    /// Run fs::mmapFile for the current harness.
    pub(crate) fn mmap_file(
        &mut self,
        handle: FileHandle,
        offset: FileOffset,
        length: FileSize,
        prot: MmapProt,
        flags: MmapFlags,
    ) -> RuntimeResult<Option<FsMapping>> {
        let allowed = [PlatformErrorCode::NotSupported];
        match self.vm_context_mut() {
            Some(context) => {
                let result = platform_vm::destack_fs_mmap_file(
                    self.call_context,
                    context,
                    handle,
                    offset,
                    length,
                    prot,
                    flags,
                );
                let mapping = self.result_ok_or_codes(result, "mmapFile", &allowed)?;
                Ok(mapping.map(FsMapping::Vm))
            }
            None => {
                let mut out = std::mem::MaybeUninit::<NativeSlice<u8>>::uninit();
                let status = unsafe {
                    platform_fs::destack_fs_mmap_file(
                        out.as_mut_ptr(),
                        handle,
                        offset,
                        length,
                        prot,
                        flags,
                    )
                };
                let ok = self.status_ok_or_codes(status, "mmapFile", &allowed)?;
                if !ok {
                    return Ok(None);
                }
                Ok(Some(FsMapping::Native(unsafe { out.assume_init() })))
            }
        }
    }

    /// Run fs::mmapAnonymous for the current harness.
    pub(crate) fn mmap_anonymous(
        &mut self,
        length: FileSize,
        prot: MmapProt,
        flags: MmapFlags,
    ) -> RuntimeResult<Option<FsMapping>> {
        let allowed = [PlatformErrorCode::NotSupported];
        match self.vm_context_mut() {
            Some(context) => {
                let result = platform_vm::destack_fs_mmap_anonymous(
                    self.call_context,
                    context,
                    length,
                    prot,
                    flags,
                );
                let mapping = self.result_ok_or_codes(result, "mmapAnonymous", &allowed)?;
                Ok(mapping.map(FsMapping::Vm))
            }
            None => {
                let mut out = std::mem::MaybeUninit::<NativeSlice<u8>>::uninit();
                let status = unsafe {
                    platform_fs::destack_fs_mmap_anonymous(out.as_mut_ptr(), length, prot, flags)
                };
                let ok = self.status_ok_or_codes(status, "mmapAnonymous", &allowed)?;
                if !ok {
                    return Ok(None);
                }
                Ok(Some(FsMapping::Native(unsafe { out.assume_init() })))
            }
        }
    }

    /// Run fs::munmap for the current harness.
    pub(crate) fn munmap(&mut self, mapping: FsMapping) -> RuntimeResult<()> {
        match (self.vm_context_mut(), mapping) {
            (Some(context), FsMapping::Vm(mapping)) => {
                platform_vm::destack_fs_munmap(self.call_context, context, mapping)
            }
            (None, FsMapping::Native(mapping)) => {
                let status = unsafe { platform_fs::destack_fs_mmap_munmap(mapping) };
                self.status_ok(status, "munmap")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "mapping",
                "mapping kind mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::mprotect for the current harness.
    pub(crate) fn mprotect(&mut self, mapping: FsMapping, prot: MmapProt) -> RuntimeResult<()> {
        match (self.vm_context_mut(), mapping) {
            (Some(context), FsMapping::Vm(mapping)) => {
                platform_vm::destack_fs_mprotect(self.call_context, context, mapping, prot)
            }
            (None, FsMapping::Native(mapping)) => {
                let status = unsafe { platform_fs::destack_fs_mmap_mprotect(mapping, prot) };
                self.status_ok(status, "mprotect")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "mapping",
                "mapping kind mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::msync for the current harness.
    pub(crate) fn msync(&mut self, mapping: FsMapping, flags: MmapSyncFlags) -> RuntimeResult<()> {
        match (self.vm_context_mut(), mapping) {
            (Some(context), FsMapping::Vm(mapping)) => {
                platform_vm::destack_fs_msync(self.call_context, context, mapping, flags)
            }
            (None, FsMapping::Native(mapping)) => {
                let status = unsafe { platform_fs::destack_fs_mmap_msync(mapping, flags) };
                self.status_ok(status, "msync")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "mapping",
                "mapping kind mismatch",
            ))
            .boxed()),
        }
    }

    /// Run fs::madvise for the current harness.
    pub(crate) fn madvise(&mut self, mapping: FsMapping, advice: MmapAdvice) -> RuntimeResult<()> {
        match (self.vm_context_mut(), mapping) {
            (Some(context), FsMapping::Vm(mapping)) => {
                platform_vm::destack_fs_madvise(self.call_context, context, mapping, advice)
            }
            (None, FsMapping::Native(mapping)) => {
                let status = unsafe { platform_fs::destack_fs_mmap_madvise(mapping, advice) };
                self.status_ok(status, "madvise")
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "mapping",
                "mapping kind mismatch",
            ))
            .boxed()),
        }
    }

    /// Write bytes into a mapping for the current harness.
    pub(crate) fn write_mapping(&mut self, mapping: &FsMapping, bytes: &[u8]) -> RuntimeResult<()> {
        match (self.vm_context_mut(), mapping) {
            (Some(context), FsMapping::Vm(mapping)) => {
                if bytes.len() > mapping.len as usize {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "mapping",
                        "write exceeds mapping length",
                    ))
                    .boxed());
                }
                mapping.write_bytes(context, bytes)?;
                Ok(())
            }
            (None, FsMapping::Native(mapping)) => {
                let slice = unsafe { mapping.as_mut_slice()? };
                if bytes.len() > slice.len() {
                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                        "mapping",
                        "write exceeds mapping length",
                    ))
                    .boxed());
                }
                slice[..bytes.len()].copy_from_slice(bytes);
                Ok(())
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "mapping",
                "mapping kind mismatch",
            ))
            .boxed()),
        }
    }

    /// Read bytes from a mapping for the current harness.
    pub(crate) fn read_mapping(
        &mut self,
        mapping: &FsMapping,
        length: usize,
    ) -> RuntimeResult<Vec<u8>> {
        match (self.vm_context_mut(), mapping) {
            (Some(context), FsMapping::Vm(mapping)) => {
                let bytes = mapping.read_bytes(context)?;
                Ok(bytes.into_iter().take(length).collect())
            }
            (None, FsMapping::Native(mapping)) => {
                let slice = unsafe { mapping.as_slice()? };
                Ok(slice.iter().copied().take(length).collect())
            }
            _ => Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "mapping",
                "mapping kind mismatch",
            ))
            .boxed()),
        }
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
