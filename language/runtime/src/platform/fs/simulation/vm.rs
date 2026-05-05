#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{
    AccessMode, AllocFlags, AtFlags, CopyFlags, DirentNextVm, DirentVm, FdFlags, FileAdvice,
    FileLockFlags, FileMode, FileOffset, FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags,
    NodeDevice, OpenFlags, OpenOptionsVm, OsPathVm, ReadWriteFlags, RenameFlags, SeekWhence,
    SpliceCursorVm, SpliceFlags, StatFsVm, StatVm, StatusFlags, StatxFlags, StatxMask, StatxVm,
    SymlinkType, SyncFlags, WatchBatchVm, WatchOptionsVm, XattrFlags,
};
use crate::platform::{PlatformError, VmArray, VmSlice, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

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
pub(crate) fn destack_fs_access(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let _ = (path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.access")).boxed())
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
pub(crate) fn destack_fs_accessat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    mode: AccessMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (dir, path, mode, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.accessat")).boxed())
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
pub(crate) fn destack_fs_chmod(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.chmod")).boxed())
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
pub(crate) fn destack_fs_chown(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (path, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.chown")).boxed())
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
pub(crate) fn destack_fs_fchmod(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.fchmod")).boxed())
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
pub(crate) fn destack_fs_fchmodat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (dir, path, mode, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.fchmodat")).boxed())
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
pub(crate) fn destack_fs_fchown(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (handle, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.fchown")).boxed())
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
pub(crate) fn destack_fs_fchownat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (dir, path, uid, gid, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.fchownat")).boxed())
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
pub(crate) fn destack_fs_futimes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (handle, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.futimes")).boxed())
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
pub(crate) fn destack_fs_lutimes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.lutimes")).boxed())
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
pub(crate) fn destack_fs_utimensat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    atimens: u64,
    mtimens: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (dir, path, atimens, mtimens, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.utimensat")).boxed())
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
pub(crate) fn destack_fs_utimes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.utimes")).boxed())
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
pub(crate) fn destack_fs_closedir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::DirectoryHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.closedir")).boxed())
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
pub(crate) fn destack_fs_dirfd(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::DirectoryHandle,
) -> RuntimeResult<resource::FileHandle> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.dirfd")).boxed())
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
pub(crate) fn destack_fs_mkdir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.mkdir")).boxed())
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
pub(crate) fn destack_fs_mkdirat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (dir, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.mkdirat")).boxed())
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
pub(crate) fn destack_fs_mkdtemp(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _template: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.mkdtemp")).boxed())
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
pub(crate) fn destack_fs_opendir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<resource::DirectoryHandle> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.opendir")).boxed())
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
pub(crate) fn destack_fs_readdir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::DirectoryHandle,
) -> RuntimeResult<VmArray<DirentVm>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.readdir")).boxed())
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
pub(crate) fn destack_fs_readdir_next(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::DirectoryHandle,
) -> RuntimeResult<DirentNextVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.readdirNext")).boxed())
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
pub(crate) fn destack_fs_rewinddir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::DirectoryHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.rewinddir")).boxed())
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
pub(crate) fn destack_fs_rmdir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.rmdir")).boxed())
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
pub(crate) fn destack_fs_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.close")).boxed())
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
pub(crate) fn destack_fs_copy_file_range(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    src: resource::FileHandle,
    srcoffset: FileOffset,
    dst: resource::FileHandle,
    dstoffset: FileOffset,
    length: FileSize,
) -> RuntimeResult<u64> {
    let _ = (src, srcoffset, dst, dstoffset, length);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.file.copyFileRange",
    ))
    .boxed())
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
pub(crate) fn destack_fs_dup(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<resource::FileHandle> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.dup")).boxed())
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
pub(crate) fn destack_fs_dup2(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    target: resource::FileHandle,
) -> RuntimeResult<resource::FileHandle> {
    let _ = (handle, target);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.dup2")).boxed())
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
pub(crate) fn destack_fs_dup3(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    target: resource::FileHandle,
    flags: OpenFlags,
) -> RuntimeResult<resource::FileHandle> {
    let _ = (handle, target, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.dup3")).boxed())
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
pub(crate) fn destack_fs_fadvise(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    offset: FileOffset,
    length: FileSize,
    advice: FileAdvice,
) -> RuntimeResult<()> {
    let _ = (handle, offset, length, advice);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fadvise")).boxed())
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
pub(crate) fn destack_fs_fallocate(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: AllocFlags,
) -> RuntimeResult<()> {
    let _ = (handle, offset, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fallocate")).boxed())
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
pub(crate) fn destack_fs_fdatasync(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fdatasync")).boxed())
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
pub(crate) fn destack_fs_fsync(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fsync")).boxed())
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
pub(crate) fn destack_fs_ftruncate(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (handle, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.ftruncate")).boxed())
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
pub(crate) fn destack_fs_get_fd_flags(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<FdFlags> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.getFdFlags")).boxed())
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
pub(crate) fn destack_fs_get_status_flags(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<StatusFlags> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.file.getStatusFlags",
    ))
    .boxed())
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
pub(crate) fn destack_fs_lock(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    let _ = (handle, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.lock")).boxed())
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
pub(crate) fn destack_fs_open(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<resource::FileHandle> {
    let _ = (path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.open")).boxed())
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
pub(crate) fn destack_fs_openat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<resource::FileHandle> {
    let _ = (dir, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.openat")).boxed())
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
pub(crate) fn destack_fs_openat2(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    how: OpenOptionsVm,
) -> RuntimeResult<resource::FileHandle> {
    let _ = (dir, path, how);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.openat2")).boxed())
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
pub(crate) fn destack_fs_pread(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    buffer: VmSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.pread")).boxed())
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
pub(crate) fn destack_fs_preadv(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.preadv")).boxed())
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
pub(crate) fn destack_fs_preadv2(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers, offset, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.preadv2")).boxed())
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
pub(crate) fn destack_fs_pwrite(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    buffer: VmSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.pwrite")).boxed())
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
pub(crate) fn destack_fs_pwritev(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.pwritev")).boxed())
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
pub(crate) fn destack_fs_pwritev2(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers, offset, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.pwritev2")).boxed())
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
pub(crate) fn destack_fs_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.read")).boxed())
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
pub(crate) fn destack_fs_readv(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.readv")).boxed())
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
pub(crate) fn destack_fs_seek(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    offset: FileOffset,
    whence: SeekWhence,
) -> RuntimeResult<FileOffset> {
    let _ = (handle, offset, whence);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.seek")).boxed())
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
pub(crate) fn destack_fs_sendfile(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    socket: resource::SocketHandle,
    file: resource::FileHandle,
    offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<u64> {
    let _ = (socket, file, offset, length);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.sendfile")).boxed())
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
pub(crate) fn destack_fs_set_fd_flags(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    flags: FdFlags,
) -> RuntimeResult<()> {
    let _ = (handle, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.setFdFlags")).boxed())
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
pub(crate) fn destack_fs_set_status_flags(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    flags: StatusFlags,
) -> RuntimeResult<()> {
    let _ = (handle, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.file.setStatusFlags",
    ))
    .boxed())
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
pub(crate) fn destack_fs_splice(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    source: resource::ResourceId,
    sourcecursor: SpliceCursorVm,
    target: resource::ResourceId,
    targetcursor: SpliceCursorVm,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<u64> {
    let _ = (source, sourcecursor, target, targetcursor, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.splice")).boxed())
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
pub(crate) fn destack_fs_sync_file_range(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: SyncFlags,
) -> RuntimeResult<()> {
    let _ = (handle, offset, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.file.syncFileRange",
    ))
    .boxed())
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
pub(crate) fn destack_fs_syncfs(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.syncfs")).boxed())
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
pub(crate) fn destack_fs_tee(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    sourcepipe: resource::PipeHandle,
    targetpipe: resource::PipeHandle,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<u64> {
    let _ = (sourcepipe, targetpipe, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.tee")).boxed())
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
pub(crate) fn destack_fs_truncate(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (path, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.truncate")).boxed())
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
pub(crate) fn destack_fs_vmsplice(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    pipe: resource::PipeHandle,
    buffers: VmSlice<VmSlice<u8>>,
    flags: SpliceFlags,
) -> RuntimeResult<u64> {
    let _ = (pipe, buffers, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.vmsplice")).boxed())
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
pub(crate) fn destack_fs_write(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.write")).boxed())
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
pub(crate) fn destack_fs_writev(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.writev")).boxed())
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
pub(crate) fn destack_fs_madvise(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    mapping: VmSlice<u8>,
    advice: MmapAdvice,
) -> RuntimeResult<()> {
    let _ = (mapping, advice);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmap.madvise")).boxed())
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
pub(crate) fn destack_fs_mprotect(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    mapping: VmSlice<u8>,
    prot: MmapProt,
) -> RuntimeResult<()> {
    let _ = (mapping, prot);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmap.mprotect")).boxed())
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
pub(crate) fn destack_fs_msync(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    mapping: VmSlice<u8>,
    flags: MmapSyncFlags,
) -> RuntimeResult<()> {
    let _ = (mapping, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmap.msync")).boxed())
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
pub(crate) fn destack_fs_munmap(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _mapping: VmSlice<u8>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmap.munmap")).boxed())
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
pub(crate) fn destack_fs_mmap_anonymous(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (length, prot, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmapAnonymous")).boxed())
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
pub(crate) fn destack_fs_mmap_file(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    offset: FileOffset,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, offset, length, prot, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmapFile")).boxed())
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
pub(crate) fn destack_fs_copyfile(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    from: OsPathVm,
    to: OsPathVm,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    let _ = (from, to, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.copyfile")).boxed())
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
pub(crate) fn destack_fs_link(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    existingpath: OsPathVm,
    newpath: OsPathVm,
) -> RuntimeResult<()> {
    let _ = (existingpath, newpath);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.link")).boxed())
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
pub(crate) fn destack_fs_linkat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    existingdir: resource::DirectoryHandle,
    existingpath: OsPathVm,
    newdir: resource::DirectoryHandle,
    newpath: OsPathVm,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (existingdir, existingpath, newdir, newpath, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.linkat")).boxed())
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
pub(crate) fn destack_fs_mkfifo(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.mkfifo")).boxed())
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
pub(crate) fn destack_fs_mkfifoat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (dir, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.mkfifoat")).boxed())
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
pub(crate) fn destack_fs_mknod(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    let _ = (path, mode, device);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.mknod")).boxed())
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
pub(crate) fn destack_fs_mknodat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    let _ = (dir, path, mode, device);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.mknodat")).boxed())
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
pub(crate) fn destack_fs_readlink(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.readlink")).boxed())
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
pub(crate) fn destack_fs_readlinkat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let _ = (dir, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.readlinkat")).boxed())
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
pub(crate) fn destack_fs_realpath(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.realpath")).boxed())
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
pub(crate) fn destack_fs_rename(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    from: OsPathVm,
    to: OsPathVm,
) -> RuntimeResult<()> {
    let _ = (from, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.rename")).boxed())
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
pub(crate) fn destack_fs_renameat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    fromdir: resource::DirectoryHandle,
    from: OsPathVm,
    todir: resource::DirectoryHandle,
    to: OsPathVm,
) -> RuntimeResult<()> {
    let _ = (fromdir, from, todir, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.renameat")).boxed())
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
pub(crate) fn destack_fs_renameat2(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    fromdir: resource::DirectoryHandle,
    from: OsPathVm,
    todir: resource::DirectoryHandle,
    to: OsPathVm,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    let _ = (fromdir, from, todir, to, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.renameat2")).boxed())
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
pub(crate) fn destack_fs_symlink(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    target: OsPathVm,
    path: OsPathVm,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = (target, path, kind);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.symlink")).boxed())
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
pub(crate) fn destack_fs_symlinkat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    target: OsPathVm,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = (target, dir, path, kind);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.symlinkat")).boxed())
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
pub(crate) fn destack_fs_unlink(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.unlink")).boxed())
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
pub(crate) fn destack_fs_unlinkat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.unlinkat")).boxed())
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
pub(crate) fn destack_fs_fstat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<StatVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.fstat")).boxed())
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
pub(crate) fn destack_fs_fstatfs(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<StatFsVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.fstatfs")).boxed())
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
pub(crate) fn destack_fs_lstat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<StatVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.lstat")).boxed())
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
pub(crate) fn destack_fs_stat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<StatVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.path")).boxed())
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
pub(crate) fn destack_fs_statat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    flags: AtFlags,
) -> RuntimeResult<StatVm> {
    let _ = (dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.pathat")).boxed())
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
pub(crate) fn destack_fs_statfs(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<StatFsVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.pathfs")).boxed())
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
pub(crate) fn destack_fs_statx(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    dir: resource::DirectoryHandle,
    path: OsPathVm,
    flags: StatxFlags,
    mask: StatxMask,
) -> RuntimeResult<StatxVm> {
    let _ = (dir, path, flags, mask);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.pathx")).boxed())
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
pub(crate) fn destack_fs_watch(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    options: WatchOptionsVm,
) -> RuntimeResult<resource::WatchHandle> {
    let _ = (path, options);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watch")).boxed())
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
pub(crate) fn destack_fs_watch_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::WatchHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchClose")).boxed())
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
pub(crate) fn destack_fs_watch_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::WatchHandle,
) -> RuntimeResult<WatchBatchVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchRead")).boxed())
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
pub(crate) fn destack_fs_watchat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    directory: resource::DirectoryHandle,
    path: OsPathVm,
    options: WatchOptionsVm,
) -> RuntimeResult<resource::WatchHandle> {
    let _ = (directory, path, options);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchat")).boxed())
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
pub(crate) fn destack_fs_fgetxattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    name: vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let _ = (handle, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.fgetxattr")).boxed())
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
pub(crate) fn destack_fs_fgetxattr_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    let _ = (handle, name);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.fgetxattrBytes",
    ))
    .boxed())
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
pub(crate) fn destack_fs_flistxattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.flistxattr")).boxed())
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
pub(crate) fn destack_fs_flistxattr_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<VmArray<VmArray<u8>>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.flistxattrBytes",
    ))
    .boxed())
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
pub(crate) fn destack_fs_fremovexattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (handle, name);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.fremovexattr",
    ))
    .boxed())
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
pub(crate) fn destack_fs_fremovexattr_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, name);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.fremovexattrBytes",
    ))
    .boxed())
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
pub(crate) fn destack_fs_fsetxattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    name: vm::StringHandle,
    argument_value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (handle, name, argument_value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.fsetxattr")).boxed())
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
pub(crate) fn destack_fs_fsetxattr_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::FileHandle,
    name: VmSlice<u8>,
    argument_value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (handle, name, argument_value, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.fsetxattrBytes",
    ))
    .boxed())
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
pub(crate) fn destack_fs_getxattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let _ = (path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.getxattr")).boxed())
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
pub(crate) fn destack_fs_getxattr_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    let _ = (path, name);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.getxattrBytes",
    ))
    .boxed())
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
pub(crate) fn destack_fs_lgetxattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let _ = (path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.lgetxattr")).boxed())
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
pub(crate) fn destack_fs_lgetxattr_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    let _ = (path, name);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.lgetxattrBytes",
    ))
    .boxed())
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
pub(crate) fn destack_fs_listxattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.listxattr")).boxed())
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
pub(crate) fn destack_fs_listxattr_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<VmArray<VmArray<u8>>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.listxattrBytes",
    ))
    .boxed())
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
pub(crate) fn destack_fs_llistxattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.llistxattr")).boxed())
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
pub(crate) fn destack_fs_llistxattr_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<VmArray<VmArray<u8>>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.llistxattrBytes",
    ))
    .boxed())
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
pub(crate) fn destack_fs_lremovexattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (path, name);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.lremovexattr",
    ))
    .boxed())
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
pub(crate) fn destack_fs_lremovexattr_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (path, name);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.lremovexattrBytes",
    ))
    .boxed())
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
pub(crate) fn destack_fs_lsetxattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
    argument_value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (path, name, argument_value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.lsetxattr")).boxed())
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
pub(crate) fn destack_fs_lsetxattr_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
    argument_value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (path, name, argument_value, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.lsetxattrBytes",
    ))
    .boxed())
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
pub(crate) fn destack_fs_removexattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.removexattr")).boxed())
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
pub(crate) fn destack_fs_removexattr_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (path, name);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.removexattrBytes",
    ))
    .boxed())
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
pub(crate) fn destack_fs_setxattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
    argument_value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (path, name, argument_value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.setxattr")).boxed())
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
pub(crate) fn destack_fs_setxattr_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
    argument_value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (path, name, argument_value, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.setxattrBytes",
    ))
    .boxed())
}
