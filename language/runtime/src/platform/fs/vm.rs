use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{core as core_fs, host as host_fs};

use crate::platform::abi::{NativeAbi, VmAbi};
use crate::platform::fs::{
    AccessMode, AllocFlags, AtFlags, CopyFlags, DirectoryHandle, Dirent, DirentNext,
    DirentNextEndVm, DirentNextEntryVm, DirentNextVm, DirentVm, FdFlags, FileAdvice, FileHandle,
    FileLockFlags, FileMode, FileOffset, FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags,
    NodeDevice, OpenFlags, OpenOptions, OpenOptionsVm, OsPath, OsPathBytesVm, OsPathUtf16Vm,
    OsPathVm, PathBytes, PathBytesAbi, PathBytesVm, PathUtf16, PathUtf16Abi, PathUtf16Vm,
    ReadWriteFlags, RenameFlags, SeekWhence, SpliceCursor, SpliceFlags, Stat, StatFs, StatusFlags,
    Statx, StatxFlags, StatxMask, SymlinkType, SyncFlags, WatchBatch, WatchBatchVm,
    WatchCreateEventVm, WatchEvent, WatchEventMetadataVm, WatchEventVm, WatchMetadataEventVm,
    WatchModifyEventVm, WatchOptions, WatchOptionsVm, WatchOverflowEventVm, WatchRemoveEventVm,
    WatchRenameEventVm, XattrFlags,
};
use crate::platform::resource::{PipeHandle, ResourceId, SocketHandle, WatchHandle};
use crate::platform::{NativeArray, PlatformError, VmArray, VmSlice};
use crate::runtime::{BindingCallContext, NativeSlice, NativeStringRef};

fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    let mut out = std::mem::MaybeUninit::<T>::uninit();
    call(out.as_mut_ptr())?;
    Ok(unsafe { out.assume_init() })
}

/// Check file access permissions.
///
/// Check file access permissions via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses access(2) on Unix and GetFileAttributesW plus ACL checks on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_access(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_access(runtime, path, mode) }
}

/// Check file access permissions relative to a directory handle.
///
/// Check file access permissions relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses faccessat(2) on Unix and relative path checks via native handles on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_accessat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: AccessMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_accessat(runtime, dir, path, mode, flags) }
}

/// Change file permissions.
///
/// Change file permissions via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses chmod(2) on Unix and file attribute/security updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chmod`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_chmod(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_chmod(runtime, path, mode) }
}

/// Change file permissions relative to a directory handle.
///
/// Change file permissions relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fchmodat(2) on Unix and handle-relative mode updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chmod`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fchmodat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_fchmodat(runtime, dir, path, mode, flags) }
}

/// Change file owner and group.
///
/// Change file owner and group via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses chown(2) on Unix and token/owner updates where supported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chown`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_chown(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_chown(runtime, path, uid, gid) }
}

/// Change file owner and group relative to a directory handle.
///
/// Change file owner and group relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fchownat(2) on Unix and handle-relative owner updates where supported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chown`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fchownat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_fchownat(runtime, dir, path, uid, gid, flags) }
}

/// Update access and modification times.
///
/// Update access and modification times via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses utimensat/utimes on Unix and SetFileTime on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_utimes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_utimes(runtime, path, atime_ns, mtime_ns) }
}

/// Update access and modification times without following symlinks.
///
/// Update access and modification times without following symlinks via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lutimes/utimensat with nofollow on Unix and reparse-point time updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_lutimes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_lutimes(runtime, path, atime_ns, mtime_ns) }
}

/// Update access and modification times relative to a directory handle.
///
/// Update access and modification times relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses utimensat(2) on Unix and handle-relative SetFileTime on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_utimensat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    atime_ns: u64,
    mtime_ns: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_utimensat(runtime, dir, path, atime_ns, mtime_ns, flags) }
}

/// Create a directory.
///
/// Create a single directory entry at the provided path with the supplied mode bits.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mkdir(2) on Unix and CreateDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_mkdir(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_mkdir(runtime, path, mode) }
}

/// Create a directory relative to a directory handle.
///
/// Create a single directory entry relative to an existing directory descriptor.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mkdirat(2) on Unix and handle-relative directory create on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_mkdirat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_mkdirat(runtime, dir, path, mode) }
}

/// Remove a directory.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses rmdir(2) on Unix and RemoveDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_rmdir(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_rmdir(runtime, path) }
}

/// Open a directory and return a handle.
///
/// Open the target resource with the requested flags and return the host handle exposed by the kernel.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses opendir/readdir on Unix and FindFirstFileW directory enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_opendir(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<DirectoryHandle> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_opendir(runtime, out, path) })
}

/// Create a temporary directory.
///
/// Create a unique temporary directory from the template in the platform temp directory.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mkdtemp(3) on Unix and GetTempPathW plus CreateDirectoryW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.temp`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_mkdtemp(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    template: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let template = path_ref_from_vm(runtime, context, template)?;
    let path = call_out(|out| unsafe { host_fs::destack_fs_mkdtemp(runtime, out, template) })?;
    path_ref_to_vm(context, path)
}

/// Open a file and return a handle.
///
/// Open one filesystem entry by path and return a host-backed file handle.
/// Flag interpretation, creation behavior, and inheritance defaults follow host open semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses open(2) on Unix and CreateFileW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_open(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<FileHandle> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_open(runtime, out, path, flags, mode) })
}

/// Open a file relative to a directory handle.
///
/// Open one filesystem entry resolved relative to an explicit directory handle.
/// This avoids ambient current-working-directory resolution and keeps caller-controlled base directory scope.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat(2) on Unix and NtCreateFile relative opens on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_openat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<FileHandle> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_openat(runtime, out, dir, path, flags, mode) })
}

/// Open a file relative to a directory handle with openat2 semantics.
///
/// Open one filesystem entry relative to an explicit directory handle with resolve policy flags.
/// Resolve behavior is passed through to supported hosts and rejected when the host backend cannot honor requested guarantees.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat2(2) on Linux and Android, falls back to openat semantics on other Unix targets when resolve flags are empty, and maps to openat semantics on Windows with resolve flags rejected.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_openat2(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    how: OpenOptionsVm,
) -> RuntimeResult<FileHandle> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let how = OpenOptions {
        flags: how.flags,
        mode: how.mode,
        resolve: how.resolve,
    };
    call_out(|out| unsafe { host_fs::destack_fs_openat2(runtime, out, dir, path, how) })
}

/// Close an open file handle.
///
/// Close the target handle by forwarding the descriptor teardown to the host kernel.
/// The descriptor becomes invalid immediately for subsequent read or write operations.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_close(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_close(runtime, handle) }
}

/// Close a directory handle.
///
/// Close the target handle by forwarding the descriptor teardown to the host kernel.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses closedir(3) on Unix and FindClose/CloseHandle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_closedir(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_closedir(runtime, handle) }
}

/// Read directory entries from an open directory handle.
///
/// Read the full directory stream from the current cursor until the host reports end-of-directory.
/// Entry ordering and type classification follow host directory iteration semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readdir(3) loop on Unix and FindNextFileW loop on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_readdir(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<VmArray<DirentVm>> {
    let entries = call_out(|out| unsafe { host_fs::destack_fs_readdir(runtime, out, handle) })?;
    dirent_array_to_vm(context, entries)
}

/// Read a single directory entry from an open directory handle.
///
/// Read at most one entry from the current directory cursor and advance the host iterator.
/// Callers can iterate deterministically by repeatedly invoking this operation until `entry` is void.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readdir(3) step on Unix and FindNextFileW step on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_readdir_next(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<DirentNextVm> {
    let entry = call_out(|out| unsafe { host_fs::destack_fs_readdir_next(runtime, out, handle) })?;
    dirent_next_to_vm(context, entry)
}

/// Reset an open directory handle to the first entry.
///
/// Reset the directory iteration cursor to the beginning of the stream.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses rewinddir(3) on Unix and enumeration reset on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_rewinddir(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_rewinddir(runtime, handle) }
}

/// Resolve the directory descriptor for an open directory handle.
///
/// Extract the underlying file descriptor or handle value from an open directory stream.
/// The returned handle is valid only while the source directory handle remains open.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses dirfd(3) on Unix and directory handle extraction on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_dirfd(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<FileHandle> {
    call_out(|out| unsafe { host_fs::destack_fs_dirfd(runtime, out, handle) })
}

/// Read from a file into the provided slice.
///
/// Read bytes into one contiguous caller-provided buffer from the current file position.
/// The file position advances by the exact byte count returned by the host.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses read(2) on Unix and ReadFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let native = allocate_read_buffer(runtime, buffer);
    let count = call_out(|out| unsafe { host_fs::destack_fs_read(runtime, out, handle, native) })?;
    write_read_buffer(context, buffer, native)?;
    Ok(count)
}

/// Read from a file at the given file offset.
///
/// Read bytes into one contiguous caller-provided buffer at an explicit file offset.
/// The descriptor's current file position is not changed by positioned reads.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses pread(2) on Unix and positioned ReadFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_pread(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let native = allocate_read_buffer(runtime, buffer);
    let count =
        call_out(|out| unsafe { host_fs::destack_fs_pread(runtime, out, handle, native, offset) })?;
    write_read_buffer(context, buffer, native)?;
    Ok(count)
}

/// Write to a file from the provided slice.
///
/// Write bytes from one contiguous caller-provided buffer at the current file position.
/// The file position advances by the exact byte count accepted by the host.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses write(2) on Unix and WriteFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_write(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let native = buffer_from_vm(runtime, context, buffer)?;
    call_out(|out| unsafe { host_fs::destack_fs_write(runtime, out, handle, native) })
}

/// Write to a file at the given file offset.
///
/// Write bytes from one contiguous caller-provided buffer at an explicit file offset.
/// The descriptor's current file position is not changed by positioned writes.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses pwrite(2) on Unix and positioned WriteFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_pwrite(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let native = buffer_from_vm(runtime, context, buffer)?;
    call_out(|out| unsafe { host_fs::destack_fs_pwrite(runtime, out, handle, native, offset) })
}

/// Read into multiple buffers.
///
/// Read bytes into a scatter buffer list from the current file position.
/// Buffer fill order follows host iovec semantics and advances the file position by bytes read.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readv(2) on Unix and vectored file I/O loop on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_readv(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let (native_buffers, vm_buffers) = allocate_read_buffers(runtime, context, buffers)?;
    let count =
        call_out(|out| unsafe { host_fs::destack_fs_readv(runtime, out, handle, native_buffers) })?;
    write_read_buffers(context, vm_buffers, native_buffers)?;
    Ok(count)
}

/// Read into multiple buffers at the given file offset.
///
/// Read bytes into a scatter buffer list at an explicit file offset.
/// The descriptor's current file position is not changed by positioned vectored reads.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses preadv(2) on Unix and vectored positioned file I/O loop on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_preadv(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let (native_buffers, vm_buffers) = allocate_read_buffers(runtime, context, buffers)?;
    let count = call_out(|out| unsafe {
        host_fs::destack_fs_preadv(runtime, out, handle, native_buffers, offset)
    })?;
    write_read_buffers(context, vm_buffers, native_buffers)?;
    Ok(count)
}

/// Read into multiple buffers at the given file offset with explicit read flags.
///
/// Read bytes into a scatter buffer list at an explicit file offset and apply host read flags.
/// Flag bits are passed through directly and may enable nowait or high-priority reads on supported kernels.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses preadv2(2) on Linux and runtime fallback to preadv on other targets when flags are zero.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_preadv2(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<u64> {
    let (native_buffers, vm_buffers) = allocate_read_buffers(runtime, context, buffers)?;
    let count = call_out(|out| unsafe {
        host_fs::destack_fs_preadv2(runtime, out, handle, native_buffers, offset, flags)
    })?;
    write_read_buffers(context, vm_buffers, native_buffers)?;
    Ok(count)
}

/// Write from multiple buffers.
///
/// Write bytes from a gather buffer list at the current file position.
/// Buffer consumption order follows host iovec semantics and advances the file position by bytes written.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses writev(2) on Unix and vectored file I/O loop on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_writev(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let native_buffers = buffers_from_vm(runtime, context, buffers)?;
    call_out(|out| unsafe { host_fs::destack_fs_writev(runtime, out, handle, native_buffers) })
}

/// Write from multiple buffers at the given file offset.
///
/// Write bytes from a gather buffer list at an explicit file offset.
/// The descriptor's current file position is not changed by positioned vectored writes.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses pwritev(2) on Unix and vectored positioned file I/O loop on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_pwritev(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let native_buffers = buffers_from_vm(runtime, context, buffers)?;
    call_out(|out| unsafe {
        host_fs::destack_fs_pwritev(runtime, out, handle, native_buffers, offset)
    })
}

/// Write from multiple buffers at the given file offset with explicit write flags.
///
/// Write bytes from a gather buffer list at an explicit file offset and apply host write flags.
/// Flag bits are passed through directly and may enable append, sync, or nowait behavior on supported kernels.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses pwritev2(2) on Linux and runtime fallback to pwritev on other targets when flags are zero.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_pwritev2(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<u64> {
    let native_buffers = buffers_from_vm(runtime, context, buffers)?;
    call_out(|out| unsafe {
        host_fs::destack_fs_pwritev2(runtime, out, handle, native_buffers, offset, flags)
    })
}

/// Change file permissions by handle.
///
/// Change file permissions by handle via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fchmod(2) on Unix and handle-based mode updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chmod`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fchmod(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fchmod(runtime, handle, mode) }
}

/// Change file owner and group by handle.
///
/// Change file owner and group by handle via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fchown(2) on Unix and handle owner updates where supported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chown`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fchown(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fchown(runtime, handle, uid, gid) }
}

/// Synchronize a file's in-core state with storage.
///
/// Synchronize buffered file state to storage for the target file descriptor.
/// Completion guarantees and writeback scope follow host kernel fsync semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fsync(2) on Unix and FlushFileBuffers on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.sync`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fsync(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fsync(runtime, handle) }
}

/// Synchronize file data only.
///
/// Flush file data pages for the target descriptor without requiring full metadata durability.
/// Metadata needed for data reachability may still be persisted per host kernel rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fdatasync(2) on Unix and FlushFileBuffers on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.sync`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fdatasync(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fdatasync(runtime, handle) }
}

/// Truncate a file by handle.
///
/// Truncate the target file to the requested size using host file-size control APIs.
/// Growth behavior for sparse expansion and zero-fill follows host filesystem policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses ftruncate(2) on Unix and SetEndOfFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_ftruncate(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_ftruncate(runtime, handle, size) }
}

/// Update access and modification times by handle.
///
/// Update access and modification times by handle via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses futimens/futimes on Unix and SetFileTime on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_futimes(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_futimes(runtime, handle, atime_ns, mtime_ns) }
}

/// Stat a file by handle.
///
/// Stat a file by handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fstat(2) on Unix and GetFileInformationByHandleEx on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fstat(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<Stat> {
    call_out(|out| unsafe { host_fs::destack_fs_fstat(runtime, out, handle) })
}

/// Stat a filesystem by handle.
///
/// Stat a filesystem by handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fstatfs/statvfs by handle on Unix and volume information by handle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fstatfs(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<StatFs> {
    call_out(|out| unsafe { host_fs::destack_fs_fstatfs(runtime, out, handle) })
}

/// Apply file locks to a file handle.
///
/// Apply, release, or test advisory locking state for one file descriptor.
/// Lock scope and conflict behavior follow host flock/fcntl locking semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses flock/fcntl on Unix and LockFileEx/UnlockFileEx on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.lock`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_lock(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_lock(runtime, handle, flags) }
}

/// Read file descriptor flags.
///
/// Read descriptor flags such as close-on-exec from the target file descriptor.
/// Returned bits reflect current host descriptor state at call time.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fcntl(F_GETFD) on Unix and runtime handle metadata on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_get_fd_flags(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<FdFlags> {
    call_out(|out| unsafe { host_fs::destack_fs_get_fd_flags(runtime, out, handle) })
}

/// Read file status flags.
///
/// Read status flags such as append and nonblocking from the target file descriptor.
/// Returned bits reflect current host descriptor state at call time.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fcntl(F_GETFL) on Unix and runtime handle metadata on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_get_status_flags(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<StatusFlags> {
    call_out(|out| unsafe { host_fs::destack_fs_get_status_flags(runtime, out, handle) })
}

/// Write file descriptor flags.
///
/// Write descriptor flags such as close-on-exec to the target file descriptor.
/// Unsupported flag bits are rejected according to host descriptor control rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fcntl(F_SETFD) on Unix and runtime handle metadata on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_set_fd_flags(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    flags: FdFlags,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_set_fd_flags(runtime, handle, flags) }
}

/// Write file status flags.
///
/// Write status flags such as append and nonblocking to the target file descriptor.
/// Unsupported or immutable status bits are rejected by host fcntl validation.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fcntl(F_SETFL) on Unix and runtime handle metadata on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_set_status_flags(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    flags: StatusFlags,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_set_status_flags(runtime, handle, flags) }
}

/// Truncate a file.
///
/// Truncate the target file to the requested size using host file-size control APIs.
/// Growth behavior for sparse expansion and zero-fill follows host filesystem policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses truncate(2) on Unix and SetEndOfFile via path handle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_truncate(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    size: FileOffset,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_truncate(runtime, path, size) }
}

/// Rename or move a file.
///
/// Rename one path entry to a new absolute or relative path in the current process namespace.
/// The operation targets plain path names and does not expose directory-handle scoping.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses rename(2) on Unix and MoveFileExW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_rename(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    from: OsPathVm,
    to: OsPathVm,
) -> RuntimeResult<()> {
    let from = path_ref_from_vm(runtime, context, from)?;
    let to = path_ref_from_vm(runtime, context, to)?;
    unsafe { host_fs::destack_fs_rename(runtime, from, to) }
}

/// Rename or move a file relative to directory handles.
///
/// Rename one path entry where both source and destination are resolved relative to explicit directory handles.
/// This avoids ambient current-working-directory resolution for both sides of the rename.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses renameat(2) on Unix and handle-relative rename on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_renameat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    from_dir: DirectoryHandle,
    from: OsPathVm,
    to_dir: DirectoryHandle,
    to: OsPathVm,
) -> RuntimeResult<()> {
    let from = path_ref_from_vm(runtime, context, from)?;
    let to = path_ref_from_vm(runtime, context, to)?;
    unsafe { host_fs::destack_fs_renameat(runtime, from_dir, from, to_dir, to) }
}

/// Rename or move a file relative to directory handles with renameat2 semantics.
///
/// Rename one path entry with explicit rename flags controlling replace and exchange behavior.
/// Flag handling follows host support levels and returns notSupported when the requested mode is unavailable.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses renameat2(2) on linux and runtime emulation/fallback on other targets.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_renameat2(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    from_dir: DirectoryHandle,
    from: OsPathVm,
    to_dir: DirectoryHandle,
    to: OsPathVm,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    let from = path_ref_from_vm(runtime, context, from)?;
    let to = path_ref_from_vm(runtime, context, to)?;
    unsafe { host_fs::destack_fs_renameat2(runtime, from_dir, from, to_dir, to, flags) }
}

/// Unlink a file.
///
/// Remove one directory entry that names a non-directory filesystem object.
/// Data blocks are reclaimed by the host once link count and open-handle rules allow.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses unlink(2) on Unix and DeleteFileW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_unlink(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_unlink(runtime, path) }
}

/// Unlink a file relative to a directory handle.
///
/// Remove one directory entry resolved from `dir` for a non-directory filesystem object.
/// Relative unlink avoids ambient cwd traversal and keeps deletion scope explicit.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses unlinkat(2) on Unix and handle-relative delete on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_unlinkat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_unlinkat(runtime, dir, path, flags) }
}

/// Create a hard link.
///
/// Create a hard-link entry that points to an existing inode without copying file contents.
/// Source and destination remain independent path entries with shared storage identity.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses link(2) on Unix and CreateHardLinkW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_link(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    existing_path: OsPathVm,
    new_path: OsPathVm,
) -> RuntimeResult<()> {
    let existing_path = path_ref_from_vm(runtime, context, existing_path)?;
    let new_path = path_ref_from_vm(runtime, context, new_path)?;
    unsafe { host_fs::destack_fs_link(runtime, existing_path, new_path) }
}

/// Create a hard link relative to directory handles.
///
/// Create a hard-link entry using directory-relative paths for both source and destination.
/// Relative resolution keeps both lookup roots explicit and avoids ambient cwd lookup.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses linkat(2) on Unix and handle-relative hard-link creation on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_linkat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    existing_dir: DirectoryHandle,
    existing_path: OsPathVm,
    new_dir: DirectoryHandle,
    new_path: OsPathVm,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let existing_path = path_ref_from_vm(runtime, context, existing_path)?;
    let new_path = path_ref_from_vm(runtime, context, new_path)?;
    unsafe {
        host_fs::destack_fs_linkat(
            runtime,
            existing_dir,
            existing_path,
            new_dir,
            new_path,
            flags,
        )
    }
}

/// Create a symbolic link.
///
/// Create a symbolic-link entry that stores the provided target path payload.
/// Target bytes are persisted as link data and are not resolved during creation.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses symlink(2) on Unix and CreateSymbolicLinkW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_symlink(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    target: OsPathVm,
    path: OsPathVm,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let target = path_ref_from_vm(runtime, context, target)?;
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_symlink(runtime, target, path, kind) }
}

/// Create a symbolic link relative to a directory handle.
///
/// Create a symbolic-link entry using a directory-relative destination path.
/// Destination lookup uses `dir` while `target` bytes are stored verbatim by the host.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses symlinkat(2) on Unix and handle-relative symlink creation on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.link`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_symlinkat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    target: OsPathVm,
    dir: DirectoryHandle,
    path: OsPathVm,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let target = path_ref_from_vm(runtime, context, target)?;
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_symlinkat(runtime, target, dir, path, kind) }
}

/// Read a symbolic link.
///
/// Read the link payload stored at the target path and return it as an `OsPath`.
/// The returned path is link data and is not canonicalized or dereferenced.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readlink(2) on Unix and reparse-point target query on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_readlink(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let target = call_out(|out| unsafe { host_fs::destack_fs_readlink(runtime, out, path) })?;
    path_ref_to_vm(context, target)
}

/// Read a symbolic link relative to a directory handle.
///
/// Read the link payload stored at a directory-relative target path.
/// The returned path is raw link data and is not dereferenced during the read.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses readlinkat(2) on Unix and handle-relative target query on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_readlinkat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let target =
        call_out(|out| unsafe { host_fs::destack_fs_readlinkat(runtime, out, dir, path) })?;
    path_ref_to_vm(context, target)
}

/// Resolve a path to its canonical form.
///
/// Resolve the input path to a canonical absolute form using host path-resolution rules.
/// Canonicalization follows host symlink, mount, and case-normalization behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses realpath(3) on Unix and GetFinalPathNameByHandleW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_realpath(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let resolved = call_out(|out| unsafe { host_fs::destack_fs_realpath(runtime, out, path) })?;
    path_ref_to_vm(context, resolved)
}

/// Copy a file.
///
/// Copy file contents and requested metadata behavior from source path to destination path.
/// Copy flags control overwrite behavior and host fast-copy strategies.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses copy_file_range/copy fallback on Unix and CopyFileW/CopyFile2 on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_copyfile(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    from: OsPathVm,
    to: OsPathVm,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    let from = path_ref_from_vm(runtime, context, from)?;
    let to = path_ref_from_vm(runtime, context, to)?;
    unsafe { host_fs::destack_fs_copyfile(runtime, from, to, flags) }
}

/// Create a FIFO special file.
///
/// Create a FIFO special file node at the target path.
/// The created node participates in host pipe semantics when opened for I/O.
///
/// # Platform
/// Unix only. This operation returns `notSupported` on Windows.
/// Uses mkfifo(2) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.special`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_mkfifo(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_mkfifo(runtime, path, mode) }
}

/// Create a FIFO special file relative to a directory handle.
///
/// Create a FIFO special file node at a directory-relative path.
/// Relative node creation keeps lookup scope anchored to `dir`.
///
/// # Platform
/// Unix only. This operation returns `notSupported` on Windows.
/// Uses mkfifoat(2) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.special`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_mkfifoat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_mkfifoat(runtime, dir, path, mode) }
}

/// Create a filesystem node.
///
/// Create a filesystem node with the requested mode and device number.
/// Node interpretation follows host mknod rules for file type and device payload.
///
/// # Platform
/// Unix only. This operation returns `notSupported` on Windows.
/// Uses mknod(2) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.special`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_mknod(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_mknod(runtime, path, mode, device) }
}

/// Create a filesystem node relative to a directory handle.
///
/// Create a filesystem node at a directory-relative path with the requested mode and device number.
/// Relative creation keeps lookup scope anchored to `dir`.
///
/// # Platform
/// Unix only. This operation returns `notSupported` on Windows.
/// Uses mknodat(2) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.special`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_mknodat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { host_fs::destack_fs_mknodat(runtime, dir, path, mode, device) }
}

/// Stat a file.
///
/// Stat a file via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses stat(2) on Unix and GetFileInformationByHandleEx on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_stat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<Stat> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_stat(runtime, out, path) })
}

/// Stat a file relative to a directory handle.
///
/// Stat a file relative to a directory handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fstatat(2) on Unix and handle-relative stat on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_statat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    flags: AtFlags,
) -> RuntimeResult<Stat> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_statat(runtime, out, dir, path, flags) })
}

/// Stat a file without following symlinks.
///
/// Stat a file without following symlinks via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lstat(2) on Unix and reparse-point aware metadata query on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_lstat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<Stat> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_lstat(runtime, out, path) })
}

/// Stat a filesystem.
///
/// Stat a filesystem via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses statfs/statvfs on Unix and GetDiskFreeSpaceExW/GetVolumeInformationW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_statfs(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<StatFs> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_statfs(runtime, out, path) })
}

/// Stat a path with statx semantics.
///
/// Stat a path with statx semantics via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses statx(2) on linux and runtime fallback on other targets.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_statx(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    flags: StatxFlags,
    mask: StatxMask,
) -> RuntimeResult<Statx> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_statx(runtime, out, dir, path, flags, mask) })
}

/// Synchronize a filesystem by file handle.
///
/// Flush pending filesystem writeback for the mount that contains this handle.
/// Scope and ordering guarantees follow host mount-level sync semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses syncfs(2) on Unix and volume flush APIs on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.sync`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_syncfs(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_syncfs(runtime, handle) }
}

/// Start watching a path and return a watch handle.
///
/// Registers the path with the native watch backend and starts event delivery for the selected mask.
/// Event ordering and coalescing behavior are backend defined.
///
/// # Platform
/// Unix and Windows.
/// Uses inotify on Linux, kqueue on BSD, FSEvents on macOS, and ReadDirectoryChangesW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.watch`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_watch(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    options: WatchOptionsVm,
) -> RuntimeResult<WatchHandle> {
    // decode vm path and options
    let path = path_ref_from_vm(runtime, context, path)?;
    let options = WatchOptions {
        mask: options.mask,
        recursive: options.recursive,
        follow_symlinks: options.follow_symlinks,
    };

    // open one host watch handle
    call_out(|out| unsafe { host_fs::destack_fs_watch(runtime, out, path, options) })
}

/// Close a watch handle.
///
/// Unregisters the watch from the backend and releases associated runtime resources.
/// No further events are delivered after close succeeds.
///
/// # Platform
/// Unix and Windows.
/// Uses backend specific handle close and unregister operations.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.watch`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_watch_close(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: WatchHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_watch_close(runtime, handle) }
}

/// Read a batch of events from a watch handle.
///
/// Reads available watch records from the backend queue and reports overflow explicitly when events were dropped.
/// Callers should treat `overflowed` as a signal to resynchronize state.
///
/// # Platform
/// Unix and Windows.
/// Uses inotify event reads on Linux, kevent on BSD, FSEvents stream reads on macOS, and ReadDirectoryChangesW reads on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.watch`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_watch_read(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: WatchHandle,
) -> RuntimeResult<WatchBatchVm> {
    // read one watch batch from the host runtime
    let batch = call_out(|out| unsafe { host_fs::destack_fs_watch_read(runtime, out, handle) })?;
    watch_batch_to_vm(context, batch)
}

/// Start watching a path relative to a directory handle.
///
/// Resolves the path relative to the supplied directory and registers the resulting entry with the backend watcher.
/// Event ordering and coalescing behavior are backend defined.
///
/// # Platform
/// Unix and Windows.
/// Uses inotify on Linux, kqueue on BSD, FSEvents on macOS, and ReadDirectoryChangesW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.watch`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_watchat(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    directory: DirectoryHandle,
    path: OsPathVm,
    options: WatchOptionsVm,
) -> RuntimeResult<WatchHandle> {
    // decode vm path and options
    let path = path_ref_from_vm(runtime, context, path)?;
    let options = WatchOptions {
        mask: options.mask,
        recursive: options.recursive,
        follow_symlinks: options.follow_symlinks,
    };

    // open one host watch handle relative to a directory
    call_out(|out| unsafe { host_fs::destack_fs_watchat(runtime, out, directory, path, options) })
}

/// Duplicate a file handle.
///
/// Duplicate one descriptor and return a new descriptor that references the same open file description.
/// Both descriptors share file-offset and status-flag state per host dup semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses dup(2) on Unix and DuplicateHandle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_dup(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<FileHandle> {
    call_out(|out| unsafe { host_fs::destack_fs_dup(runtime, out, handle) })
}

/// Duplicate a file handle to a specific target.
///
/// Duplicate one descriptor onto a caller-provided target descriptor number.
/// Existing target descriptor state is replaced according to host dup2 semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses dup2(2) on Unix and DuplicateHandle target replacement on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_dup2(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    target: FileHandle,
) -> RuntimeResult<FileHandle> {
    call_out(|out| unsafe { host_fs::destack_fs_dup2(runtime, out, handle, target) })
}

/// Duplicate a file handle to a specific target with flags.
///
/// Duplicate one descriptor onto a target descriptor while applying explicit duplication flags.
/// Flag support and close-on-exec semantics follow host dup3 behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses dup3(2) on linux and runtime emulation on other targets.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_dup3(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    target: FileHandle,
    flags: OpenFlags,
) -> RuntimeResult<FileHandle> {
    call_out(|out| unsafe { host_fs::destack_fs_dup3(runtime, out, handle, target, flags) })
}

/// Copy a range between file descriptors.
///
/// Copy bytes from one file descriptor range into another descriptor range.
/// Source and destination offsets are applied exactly as provided to the host operation.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses copy_file_range(2) on linux and runtime copy fallback on other targets.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_copy_file_range(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    src: FileHandle,
    src_offset: FileOffset,
    dst: FileHandle,
    dst_offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe {
        host_fs::destack_fs_copy_file_range(runtime, out, src, src_offset, dst, dst_offset, length)
    })
}

/// Send file data to a socket.
///
/// Transfer file bytes directly from storage-backed pages to a socket endpoint.
/// Host fast-path behavior may bypass user-space copies when supported.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses sendfile(2) on Unix variants and TransmitFile or copy fallback on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_sendfile(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    socket: SocketHandle,
    file: FileHandle,
    offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe {
        host_fs::destack_fs_sendfile(runtime, out, socket, file, offset, length)
    })
}

/// Transfer bytes between descriptors using kernel splice pipelines.
///
/// Move bytes between descriptor endpoints and optionally update explicit cursors for each side.
/// This operation is intended for zero-copy file, pipe, and socket data paths where the host supports splice semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses splice(2) on Linux and runtime fallback on targets without splice support.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.zero.copy`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_splice(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    source: ResourceId,
    sourcecursor: SpliceCursor,
    target: ResourceId,
    targetcursor: SpliceCursor,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<u64> {
    // forward splice to the host lane
    call_out(|out| unsafe {
        host_fs::destack_fs_splice(
            runtime,
            out,
            source,
            sourcecursor,
            target,
            targetcursor,
            length,
            flags,
        )
    })
}

/// Duplicate bytes from one pipe to another without consuming source bytes.
///
/// Clone bytes between two pipe descriptors while preserving source pipe contents.
/// This operation is useful for fanout pipelines where consumers share the same byte stream.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses tee(2) on Linux and runtime fallback on targets without tee support.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.zero.copy`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_tee(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    sourcepipe: PipeHandle,
    targetpipe: PipeHandle,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<u64> {
    // forward tee to the host lane
    call_out(|out| unsafe {
        host_fs::destack_fs_tee(runtime, out, sourcepipe, targetpipe, length, flags)
    })
}

/// Map user memory pages into a pipe as queued pipe buffers.
///
/// Publish one set of user buffers into a pipe endpoint for downstream splice pipelines.
/// Host kernels may pin pages or copy data depending on flags and memory state.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses vmsplice(2) on Linux and runtime fallback on targets without vmsplice support.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.zero.copy`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_vmsplice(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    pipe: PipeHandle,
    buffers: VmSlice<VmSlice<u8>>,
    flags: SpliceFlags,
) -> RuntimeResult<u64> {
    // decode VM buffer slices into native host buffers
    let native_buffers = buffers_from_vm(runtime, context, buffers)?;

    // forward vmsplice to the host lane
    call_out(|out| unsafe {
        host_fs::destack_fs_vmsplice(runtime, out, pipe, native_buffers, flags)
    })
}

/// Seek within a file and return the new offset.
///
/// Reposition the descriptor file offset using the supplied origin and delta.
/// Returned offset is the new descriptor position after host seek processing.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lseek(2) on Unix and SetFilePointerEx on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_seek(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    whence: SeekWhence,
) -> RuntimeResult<FileOffset> {
    call_out(|out| unsafe { host_fs::destack_fs_seek(runtime, out, handle, offset, whence) })
}

/// Advise the kernel about access patterns.
///
/// Provide expected access pattern hints for one descriptor range.
/// Advice is best effort and does not change correctness or visibility semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses posix_fadvise(2) on Unix and notSupported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fadvise(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    advice: FileAdvice,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fadvise(runtime, handle, offset, length, advice) }
}

/// Allocate or punch file space.
///
/// Reserve, deallocate, or punch one byte range using host allocation controls.
/// Flag combinations define keep-size and hole-punch behavior where supported.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fallocate(2) or posix_fallocate on Unix and allocation/truncate APIs on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fallocate(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: AllocFlags,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fallocate(runtime, handle, offset, length, flags) }
}

/// Synchronize a file range.
///
/// Request writeback of one byte range for the target descriptor.
/// Range ordering, blocking behavior, and fallback support follow host kernel policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses sync_file_range(2) on linux and runtime fallback on other targets.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.sync`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_sync_file_range(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: SyncFlags,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_sync_file_range(runtime, handle, offset, length, flags) }
}

/// Read an extended attribute by path.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses getxattr(2) on Unix and extended-attribute APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_getxattr(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    let values = call_out(|out| unsafe { host_fs::destack_fs_getxattr(runtime, out, path, name) })?;
    array_u8_to_vm(context, values)
}

/// Read an extended attribute by path with a raw name payload.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses getxattr(2) on Unix and extended-attribute APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_getxattr_bytes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    // decode path and raw name inputs
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = buffer_from_vm(runtime, context, name)?;

    // dispatch by path encoding
    let values = call_out(|out| match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_getxattr_bytes(runtime, out, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_getxattr_utf16(runtime, out, path_utf16.utf16, name)
        },
    })?;

    array_u8_to_vm(context, values)
}

/// Read an extended attribute without following symlinks.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lgetxattr(2) on Unix and reparse-aware xattr query where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_lgetxattr(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    let values =
        call_out(|out| unsafe { host_fs::destack_fs_lgetxattr(runtime, out, path, name) })?;
    array_u8_to_vm(context, values)
}

/// Read an extended attribute without following symlinks, using a raw name payload.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lgetxattr(2) on Unix and reparse-aware xattr query where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_lgetxattr_bytes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    // decode path and raw name inputs
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = buffer_from_vm(runtime, context, name)?;

    // dispatch by path encoding
    let values = call_out(|out| match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_lgetxattr_bytes(runtime, out, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_lgetxattr_utf16(runtime, out, path_utf16.utf16, name)
        },
    })?;

    array_u8_to_vm(context, values)
}

/// Read an extended attribute by handle.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Partial transfers are preserved exactly as reported by the host, and callers must loop when full completion is required.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fgetxattr(2) on Unix and handle-based xattr query where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fgetxattr(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    name: vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let name = string_ref_from_vm(runtime, context, name)?;
    let values =
        call_out(|out| unsafe { host_fs::destack_fs_fgetxattr(runtime, out, handle, name) })?;
    array_u8_to_vm(context, values)
}

/// Read an extended attribute by handle with a raw name payload.
///
/// Transfer bytes directly between caller buffers and host descriptors using short I/O semantics.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fgetxattr(2) on Unix and handle-based xattr query where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fgetxattr_bytes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    // decode the raw name input
    let name = buffer_from_vm(runtime, context, name)?;

    // dispatch through the raw handle lane
    let values = call_out(|out| unsafe {
        host_fs::destack_fs_fgetxattr_handle(runtime, out, handle, name)
    })?;

    array_u8_to_vm(context, values)
}

/// Set an extended attribute by path.
///
/// Set the requested control value on the descriptor through the native option interface.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses setxattr(2) on Unix and extended-attribute APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_setxattr(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    let value = buffer_from_vm(runtime, context, value)?;
    unsafe { host_fs::destack_fs_setxattr(runtime, path, name, value, flags) }
}

/// Set an extended attribute by path with a raw name payload.
///
/// Set the requested control value on the descriptor through the native option interface.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses setxattr(2) on Unix and extended-attribute APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_setxattr_bytes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // decode path, raw name, and value inputs
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = buffer_from_vm(runtime, context, name)?;
    let value = buffer_from_vm(runtime, context, value)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_setxattr_bytes(runtime, path_bytes.bytes, name, value, flags)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_setxattr_utf16(runtime, path_utf16.utf16, name, value, flags)
        },
    }
}

/// Set an extended attribute without following symlinks.
///
/// Set the requested control value on the descriptor through the native option interface.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lsetxattr(2) on Unix and reparse-aware xattr write where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_lsetxattr(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    let value = buffer_from_vm(runtime, context, value)?;
    unsafe { host_fs::destack_fs_lsetxattr(runtime, path, name, value, flags) }
}

/// Set an extended attribute without following symlinks, using a raw name payload.
///
/// Set the requested control value on the descriptor through the native option interface.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lsetxattr(2) on Unix and reparse-aware xattr write where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_lsetxattr_bytes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // decode path, raw name, and value inputs
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = buffer_from_vm(runtime, context, name)?;
    let value = buffer_from_vm(runtime, context, value)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_lsetxattr_bytes(runtime, path_bytes.bytes, name, value, flags)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_lsetxattr_utf16(runtime, path_utf16.utf16, name, value, flags)
        },
    }
}

/// Set an extended attribute by handle.
///
/// Set the requested control value on the descriptor through the native option interface.
/// The binding performs one control transaction and returns the exact host outcome without policy retries.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fsetxattr(2) on Unix and handle-based xattr write where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fsetxattr(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    name: vm::StringHandle,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let name = string_ref_from_vm(runtime, context, name)?;
    let value = buffer_from_vm(runtime, context, value)?;
    unsafe { host_fs::destack_fs_fsetxattr(runtime, handle, name, value, flags) }
}

/// Set an extended attribute by handle with a raw name payload.
///
/// Set the requested control value on the descriptor through the native option interface.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fsetxattr(2) on Unix and handle-based xattr write where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fsetxattr_bytes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    name: VmSlice<u8>,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // decode raw name and value inputs
    let name = buffer_from_vm(runtime, context, name)?;
    let value = buffer_from_vm(runtime, context, value)?;

    // dispatch through the raw handle lane
    unsafe { host_fs::destack_fs_fsetxattr_handle(runtime, handle, name, value, flags) }
}

/// List extended attribute names by path.
///
/// List extended attribute names by path via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses listxattr(2) on Unix and xattr enumeration APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_listxattr(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let names = call_out(|out| unsafe { host_fs::destack_fs_listxattr(runtime, out, path) })?;
    string_array_to_vm(context, names)
}

/// List extended attribute names by path as raw byte payloads.
///
/// List extended attribute names by path via host kernel APIs.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses listxattr(2) on Unix and xattr enumeration APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_listxattr_bytes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<VmArray<VmArray<u8>>> {
    // decode the path input
    let path = path_ref_from_vm(runtime, context, path)?;

    // query host names and encode them as vm byte arrays
    let names = call_out(|out| match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_listxattr_bytes(runtime, out, path_bytes.bytes)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_listxattr_utf16(runtime, out, path_utf16.utf16)
        },
    })?;

    array_array_u8_to_vm(context, names)
}

/// List extended attribute names without following symlinks.
///
/// List extended attribute names without following symlinks via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses llistxattr(2) on Unix and reparse-aware xattr enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_llistxattr(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let names = call_out(|out| unsafe { host_fs::destack_fs_llistxattr(runtime, out, path) })?;
    string_array_to_vm(context, names)
}

/// List extended attribute names without following symlinks as raw byte payloads.
///
/// List extended attribute names without following symlinks via host kernel APIs.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses llistxattr(2) on Unix and reparse-aware xattr enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_llistxattr_bytes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<VmArray<VmArray<u8>>> {
    // decode the path input
    let path = path_ref_from_vm(runtime, context, path)?;

    // query host names and encode them as vm byte arrays
    let names = call_out(|out| match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_llistxattr_bytes(runtime, out, path_bytes.bytes)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_llistxattr_utf16(runtime, out, path_utf16.utf16)
        },
    })?;

    array_array_u8_to_vm(context, names)
}

/// List extended attribute names by handle.
///
/// List extended attribute names by handle via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses flistxattr(2) on Unix and handle-based xattr enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_flistxattr(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let names = call_out(|out| unsafe { host_fs::destack_fs_flistxattr(runtime, out, handle) })?;
    string_array_to_vm(context, names)
}

/// List extended attribute names by handle as raw byte payloads.
///
/// List extended attribute names by handle via host kernel APIs.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses flistxattr(2) on Unix and handle-based xattr enumeration on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_flistxattr_bytes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<VmArray<VmArray<u8>>> {
    // query host names and encode them as vm byte arrays
    let names =
        call_out(|out| unsafe { host_fs::destack_fs_flistxattr_handle(runtime, out, handle) })?;

    array_array_u8_to_vm(context, names)
}

/// Remove an extended attribute by path.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses removexattr(2) on Unix and xattr delete APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_removexattr(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    unsafe { host_fs::destack_fs_removexattr(runtime, path, name) }
}

/// Remove an extended attribute by path with a raw name payload.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses removexattr(2) on Unix and xattr delete APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_removexattr_bytes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    // decode path and raw name inputs
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = buffer_from_vm(runtime, context, name)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_removexattr_bytes(runtime, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_removexattr_utf16(runtime, path_utf16.utf16, name)
        },
    }
}

/// Remove an extended attribute without following symlinks.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lremovexattr(2) on Unix and reparse-aware xattr delete on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_lremovexattr(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    unsafe { host_fs::destack_fs_lremovexattr(runtime, path, name) }
}

/// Remove an extended attribute without following symlinks, using a raw name payload.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lremovexattr(2) on Unix and reparse-aware xattr delete on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_lremovexattr_bytes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    // decode path and raw name inputs
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = buffer_from_vm(runtime, context, name)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_lremovexattr_bytes(runtime, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_lremovexattr_utf16(runtime, path_utf16.utf16, name)
        },
    }
}

/// Remove an extended attribute by handle.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fremovexattr(2) on Unix and handle-based xattr delete on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fremovexattr(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let name = string_ref_from_vm(runtime, context, name)?;
    unsafe { host_fs::destack_fs_fremovexattr(runtime, handle, name) }
}

/// Remove an extended attribute by handle with a raw name payload.
///
/// Remove the target resource through a single host namespace operation with no runtime fallback path.
/// Raw name bytes preserve host namespace data without UTF transcoding.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fremovexattr(2) on Unix and handle-based xattr delete on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.xattr`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_fremovexattr_bytes(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: FileHandle,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    // decode the raw name input
    let name = buffer_from_vm(runtime, context, name)?;

    // dispatch through the raw handle lane
    unsafe { host_fs::destack_fs_fremovexattr_handle(runtime, handle, name) }
}

/// Create a file-backed memory mapping.
///
/// Map a file-backed region into virtual memory using the requested offset, length, and protection.
/// Page alignment, commit behavior, and memory fault semantics follow host virtual-memory rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mmap(2) MAP_SHARED/MAP_PRIVATE on Unix and CreateFileMapping/MapViewOfFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.mmap`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_mmap_file(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _handle: FileHandle,
    _offset: FileOffset,
    _length: FileSize,
    _prot: MmapProt,
    _flags: MmapFlags,
) -> RuntimeResult<VmSlice<u8>> {
    // NOTE #Incomplete: implement VM-safe mmap by mapping into shared/foreign memory
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmapFile")).boxed())
}

/// Create an anonymous memory mapping.
///
/// Map an anonymous zero-initialized region into virtual memory using host allocation primitives.
/// Page alignment, commit behavior, and memory fault semantics follow host virtual-memory rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mmap(2) MAP_ANONYMOUS on Unix and VirtualAlloc/MapViewOfFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.mmap`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_mmap_anonymous(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _length: FileSize,
    _prot: MmapProt,
    _flags: MmapFlags,
) -> RuntimeResult<VmSlice<u8>> {
    // NOTE #Incomplete: implement VM-safe mmap by mapping into shared/foreign memory
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmapAnonymous")).boxed())
}

/// Unmap a memory region.
///
/// Unmap the specified virtual-memory range and release its mapping resources.
/// Page alignment, commit behavior, and memory fault semantics follow host virtual-memory rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses munmap(2) on Unix and UnmapViewOfFile/VirtualFree on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.mmap`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_munmap(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _mapping: VmSlice<u8>,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement VM-safe mmap by mapping into shared/foreign memory
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.munmap")).boxed())
}

/// Change memory protection for a mapping.
///
/// Change memory protection for a mapping via host kernel APIs.
/// Page alignment, commit behavior, and memory fault semantics follow host virtual-memory rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses mprotect(2) on Unix and VirtualProtect on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.mmap`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_mprotect(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _mapping: VmSlice<u8>,
    _prot: MmapProt,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement VM-safe mmap by mapping into shared/foreign memory
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mprotect")).boxed())
}

/// Flush a mapping to storage.
///
/// Flush a mapping to storage via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses msync(2) on Unix and FlushViewOfFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.mmap`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_msync(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _mapping: VmSlice<u8>,
    _flags: MmapSyncFlags,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement VM-safe mmap by mapping into shared/foreign memory
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.msync")).boxed())
}

/// Advise the kernel about access patterns.
///
/// Advise the kernel about access patterns via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses madvise(2) on Unix and advisory memory APIs where available on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.mmap`.
///
/// # Replay
/// External, recordable.
pub fn destack_fs_madvise(
    _runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    _mapping: VmSlice<u8>,
    _advice: MmapAdvice,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement VM-safe mmap by mapping into shared/foreign memory
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.madvise")).boxed())
}

fn path_bytes_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<PathBytes> {
    let bytes = path.0.read_bytes(context)?;
    Ok(PathBytesAbi::<NativeAbi>(runtime.store_array(bytes)))
}

fn path_utf16_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: PathUtf16Vm,
) -> RuntimeResult<PathUtf16> {
    let units = path.0.read_values(context)?;
    Ok(PathUtf16Abi::<NativeAbi>(runtime.store_array(units)))
}

fn path_ref_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<OsPath> {
    match path {
        OsPathVm::OsPathBytes(path_bytes) => {
            let bytes = path_bytes_from_vm(runtime, context, path_bytes.bytes)?;
            Ok(core_fs::path_ref_from_bytes(bytes))
        }
        OsPathVm::OsPathUtf16(path_utf16) => {
            let utf16 = path_utf16_from_vm(runtime, context, path_utf16.utf16)?;
            Ok(core_fs::path_ref_from_utf16(utf16))
        }
    }
}

fn path_bytes_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    path: PathBytes,
) -> RuntimeResult<PathBytesVm> {
    let bytes = unsafe { path.0.as_slice()? };
    let array = VmArray::from_bytes(context, bytes);
    Ok(PathBytesAbi::<VmAbi>(array))
}

fn path_utf16_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    path: PathUtf16,
) -> RuntimeResult<PathUtf16Vm> {
    let units = unsafe { path.0.as_slice()? };
    let array = VmArray::from_values(context, units)?;
    Ok(PathUtf16Abi::<VmAbi>(array))
}

fn path_ref_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    path: OsPath,
) -> RuntimeResult<OsPathVm> {
    match path {
        OsPath::OsPathBytes(path_bytes) => {
            let bytes = path_bytes_to_vm(context, path_bytes.bytes)?;
            Ok(OsPathVm::OsPathBytes(OsPathBytesVm {
                kind: vm::StringHandle::new(context.intern_string("bytes")),
                bytes,
            }))
        }
        OsPath::OsPathUtf16(path_utf16) => {
            let utf16 = path_utf16_to_vm(context, path_utf16.utf16)?;
            Ok(OsPathVm::OsPathUtf16(OsPathUtf16Vm {
                kind: vm::StringHandle::new(context.intern_string("utf16")),
                utf16,
            }))
        }
    }
}

fn string_ref_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    let string_ref = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    Ok(runtime.store_string(string_ref.as_str()))
}

fn string_ref_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeStringRef,
) -> RuntimeResult<vm::StringHandle> {
    let value = unsafe { value.as_str()? };
    Ok(vm::StringHandle::new(context.intern_string(value)))
}

fn array_u8_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    array: NativeArray<u8>,
) -> RuntimeResult<VmArray<u8>> {
    let bytes = unsafe { array.as_slice()? };
    Ok(VmArray::from_bytes(context, bytes))
}

fn string_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    array: NativeArray<NativeStringRef>,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let names = unsafe { array.as_slice()? };
    let mut handles = Vec::with_capacity(names.len());
    for name in names {
        let value = unsafe { name.as_str()? };
        let handle = vm::StringHandle::new(context.intern_string(value));
        handles.push(handle);
    }
    VmArray::from_values(context, &handles)
}

fn array_array_u8_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    array: NativeArray<NativeArray<u8>>,
) -> RuntimeResult<VmArray<VmArray<u8>>> {
    // decode the native byte arrays
    let arrays = unsafe { array.as_slice()? };
    let mut encoded = Vec::with_capacity(arrays.len());

    // encode each byte array as vm bytes
    for array in arrays {
        let bytes = unsafe { array.as_slice()? };
        encoded.push(VmArray::from_bytes(context, bytes));
    }

    VmArray::from_values(context, &encoded)
}

fn buffer_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<NativeSlice<u8>> {
    let bytes = buffer.read_bytes(context)?;
    Ok(runtime.store_slice(bytes))
}

fn allocate_read_buffer(runtime: &BindingCallContext, buffer: VmSlice<u8>) -> NativeSlice<u8> {
    let length = buffer.len as usize;
    runtime.store_slice(vec![0u8; length])
}

fn write_read_buffer(
    context: &mut vm::ExternalCallContext<'_>,
    buffer: VmSlice<u8>,
    native: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let bytes = unsafe { native.as_slice()? };
    buffer.write_bytes(context, bytes)
}

fn decode_buffer_slices(
    context: &mut vm::ExternalCallContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<Vec<VmSlice<u8>>> {
    let values = buffers.raw_values(context)?;
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        decoded.push(VmSlice::from_value(
            context,
            value,
            "buffers",
            "Slice<uint8>",
        )?);
    }
    Ok(decoded)
}

/// Allocate native read buffers for a set of VM slices.
#[allow(clippy::type_complexity)]
fn allocate_read_buffers(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<(NativeSlice<NativeSlice<u8>>, Vec<VmSlice<u8>>)> {
    let vm_buffers = decode_buffer_slices(context, buffers)?;
    let mut native_buffers = Vec::with_capacity(vm_buffers.len());
    for buffer in vm_buffers.iter() {
        let length = buffer.len as usize;
        native_buffers.push(runtime.store_slice(vec![0u8; length]));
    }
    let native_slice = runtime.store_slice(native_buffers);

    Ok((native_slice, vm_buffers))
}

/// Copy native buffer data back into VM slices.
fn write_read_buffers(
    context: &mut vm::ExternalCallContext<'_>,
    vm_buffers: Vec<VmSlice<u8>>,
    native_buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let native_buffers = unsafe { native_buffers.as_slice()? };
    if native_buffers.len() != vm_buffers.len() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "buffers",
            "buffer length mismatch",
        ))
        .boxed());
    }

    for (vm_buffer, native_buffer) in vm_buffers.into_iter().zip(native_buffers.iter()) {
        let bytes = unsafe { native_buffer.as_slice()? };
        vm_buffer.write_bytes(context, bytes)?;
    }

    Ok(())
}

/// Convert VM slice buffers into native slices.
fn buffers_from_vm(
    runtime: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<NativeSlice<NativeSlice<u8>>> {
    let vm_buffers = decode_buffer_slices(context, buffers)?;
    let mut native_buffers = Vec::with_capacity(vm_buffers.len());
    for buffer in vm_buffers {
        let bytes = buffer.read_bytes(context)?;
        native_buffers.push(runtime.store_slice(bytes));
    }

    Ok(runtime.store_slice(native_buffers))
}

fn watch_batch_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    batch: WatchBatch,
) -> RuntimeResult<WatchBatchVm> {
    let events = unsafe { batch.events.as_slice()? };
    let mut values = Vec::with_capacity(events.len());
    for event in events {
        let value = watch_event_to_vm(context, *event)?;
        values.push(value);
    }

    let events = VmArray::from_values(context, &values)?;

    Ok(WatchBatchVm {
        events,
        overflowed: batch.overflowed,
    })
}

fn watch_event_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    event: WatchEvent,
) -> RuntimeResult<WatchEventVm> {
    match event {
        WatchEvent::WatchCreateEvent(event_create) => {
            let kind = string_ref_to_vm(context, event_create.kind)?;
            let path = path_ref_to_vm(context, event_create.path)?;
            Ok(WatchEventVm::WatchCreateEvent(WatchCreateEventVm {
                kind,
                metadata: WatchEventMetadataVm {
                    cookie: event_create.metadata.cookie,
                },
                path,
            }))
        }
        WatchEvent::WatchMetadataEvent(event_metadata) => {
            let kind = string_ref_to_vm(context, event_metadata.kind)?;
            let path = path_ref_to_vm(context, event_metadata.path)?;
            Ok(WatchEventVm::WatchMetadataEvent(WatchMetadataEventVm {
                kind,
                metadata: WatchEventMetadataVm {
                    cookie: event_metadata.metadata.cookie,
                },
                path,
            }))
        }
        WatchEvent::WatchModifyEvent(event_modify) => {
            let kind = string_ref_to_vm(context, event_modify.kind)?;
            let path = path_ref_to_vm(context, event_modify.path)?;
            Ok(WatchEventVm::WatchModifyEvent(WatchModifyEventVm {
                kind,
                metadata: WatchEventMetadataVm {
                    cookie: event_modify.metadata.cookie,
                },
                path,
            }))
        }
        WatchEvent::WatchOverflowEvent(event_overflow) => {
            let kind = string_ref_to_vm(context, event_overflow.kind)?;
            Ok(WatchEventVm::WatchOverflowEvent(WatchOverflowEventVm {
                kind,
                metadata: WatchEventMetadataVm {
                    cookie: event_overflow.metadata.cookie,
                },
            }))
        }
        WatchEvent::WatchRemoveEvent(event_remove) => {
            let kind = string_ref_to_vm(context, event_remove.kind)?;
            let path = path_ref_to_vm(context, event_remove.path)?;
            Ok(WatchEventVm::WatchRemoveEvent(WatchRemoveEventVm {
                kind,
                metadata: WatchEventMetadataVm {
                    cookie: event_remove.metadata.cookie,
                },
                path,
            }))
        }
        WatchEvent::WatchRenameEvent(event_rename) => {
            let kind = string_ref_to_vm(context, event_rename.kind)?;
            let path = path_ref_to_vm(context, event_rename.path)?;
            let related_path = path_ref_to_vm(context, event_rename.related_path)?;
            Ok(WatchEventVm::WatchRenameEvent(WatchRenameEventVm {
                kind,
                metadata: WatchEventMetadataVm {
                    cookie: event_rename.metadata.cookie,
                },
                path,
                related_path,
            }))
        }
    }
}

fn dirent_array_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    entries: NativeArray<Dirent>,
) -> RuntimeResult<VmArray<DirentVm>> {
    let entries = unsafe { entries.as_slice()? };
    let mut values = Vec::with_capacity(entries.len());
    for entry in entries {
        let name = path_ref_to_vm(context, entry.name)?;
        values.push(DirentVm {
            name,
            kind: entry.kind,
        });
    }

    VmArray::from_values(context, &values)
}

fn dirent_next_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: DirentNext,
) -> RuntimeResult<DirentNextVm> {
    match value {
        DirentNext::DirentNextEnd(end_value) => {
            let kind = string_ref_to_vm(context, end_value.kind)?;
            Ok(DirentNextVm::DirentNextEnd(DirentNextEndVm { kind }))
        }
        DirentNext::DirentNextEntry(entry_value) => {
            let kind = string_ref_to_vm(context, entry_value.kind)?;
            let name = path_ref_to_vm(context, entry_value.entry.name)?;
            let entry = DirentVm {
                name,
                kind: entry_value.entry.kind,
            };
            Ok(DirentNextVm::DirentNextEntry(DirentNextEntryVm {
                kind,
                entry,
            }))
        }
    }
}
