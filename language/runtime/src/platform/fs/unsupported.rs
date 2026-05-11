#![allow(dead_code)]
#![allow(unused_variables)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeSlice;
use crate::platform::fs::{
    AccessMode, AllocFlags, AtFlags, CopyFlags, Dirent, FileAdvice, FileLockFlags, FileMode,
    FileOffset, FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags, OpenFlags, OpenOptions,
    PathBytes, PathUtf16, ReadWriteFlags, RenameFlags, SeekWhence, SpliceCursor, SpliceFlags, Stat,
    StatFs, SymlinkType, SyncFlags, XattrFlags,
};
use crate::platform::net::SocketHandle;
use crate::platform::resource::{DirectoryHandle, FileHandle, PipeHandle, ResourceId};
use crate::platform::{NativeArray, PlatformError};
use crate::runtime::BindingCallContext;

pub(crate) use crate::platform::fs::simulation::native::*;

/// Check file access permissions.
pub(crate) unsafe fn destack_fs_access_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let _ = (binding, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.accessBytes")).boxed())
}

/// Check file access permissions.
pub(crate) unsafe fn destack_fs_access_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let _ = (binding, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.accessUtf16")).boxed())
}

/// Change file permissions.
pub(crate) unsafe fn destack_fs_chmod_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chmodBytes")).boxed())
}

/// Change file permissions.
pub(crate) unsafe fn destack_fs_chmod_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chmodUtf16")).boxed())
}

/// Change file permissions relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchmodat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, mode, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchmodatBytes")).boxed())
}

/// Change file permissions relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchmodat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, mode, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchmodatUtf16")).boxed())
}

/// Change file owner and group.
pub(crate) unsafe fn destack_fs_chown_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (binding, path, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chownBytes")).boxed())
}

/// Change file owner and group.
pub(crate) unsafe fn destack_fs_chown_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (binding, path, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chownUtf16")).boxed())
}

/// Change file owner and group relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchownat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, uid, gid, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchownatBytes")).boxed())
}

/// Change file owner and group relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchownat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, uid, gid, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchownatUtf16")).boxed())
}

/// Close an open file handle.
pub(crate) unsafe fn destack_fs_close(
    binding: &BindingCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.close")).boxed())
}

/// Close a directory handle.
pub(crate) unsafe fn destack_fs_closedir(
    binding: &BindingCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.closedir")).boxed())
}

/// Copy a file.
pub(crate) unsafe fn destack_fs_copyfile_bytes(
    binding: &BindingCallContext,
    from: PathBytes,
    to: PathBytes,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    let _ = (binding, from, to, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.copyfileBytes")).boxed())
}

/// Copy a file.
pub(crate) unsafe fn destack_fs_copyfile_utf16(
    binding: &BindingCallContext,
    from: PathUtf16,
    to: PathUtf16,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    let _ = (binding, from, to, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.copyfileUtf16")).boxed())
}

/// Change file permissions by handle.
pub(crate) unsafe fn destack_fs_fchmod(
    binding: &BindingCallContext,
    handle: FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchmod")).boxed())
}

/// Change file owner and group by handle.
pub(crate) unsafe fn destack_fs_fchown(
    binding: &BindingCallContext,
    handle: FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (binding, handle, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchown")).boxed())
}

/// Synchronize file data only.
pub(crate) unsafe fn destack_fs_fdatasync(
    binding: &BindingCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fdatasync")).boxed())
}

/// Stat a file by handle.
pub(crate) unsafe fn destack_fs_fstat(
    binding: &BindingCallContext,
    _out: *mut Stat,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fstat")).boxed())
}

/// Stat a filesystem by handle.
pub(crate) unsafe fn destack_fs_fstatfs(
    binding: &BindingCallContext,
    _out: *mut StatFs,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fstatfs")).boxed())
}

/// Synchronize a file's in-core state with storage.
pub(crate) unsafe fn destack_fs_fsync(
    binding: &BindingCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fsync")).boxed())
}

/// Truncate a file by handle.
pub(crate) unsafe fn destack_fs_ftruncate(
    binding: &BindingCallContext,
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (binding, handle, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.ftruncate")).boxed())
}

/// Update access and modification times by handle.
pub(crate) unsafe fn destack_fs_futimes(
    binding: &BindingCallContext,
    handle: FileHandle,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (binding, handle, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.futimes")).boxed())
}

/// Create a hard link.
pub(crate) unsafe fn destack_fs_link_bytes(
    binding: &BindingCallContext,
    existingpath: PathBytes,
    newpath: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, existingpath, newpath);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkBytes")).boxed())
}

/// Create a hard link.
pub(crate) unsafe fn destack_fs_link_utf16(
    binding: &BindingCallContext,
    existingpath: PathUtf16,
    newpath: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, existingpath, newpath);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkUtf16")).boxed())
}

/// Stat a file without following symlinks.
pub(crate) unsafe fn destack_fs_lstat_bytes(
    binding: &BindingCallContext,
    _out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lstatBytes")).boxed())
}

/// Stat a file without following symlinks.
pub(crate) unsafe fn destack_fs_lstat_utf16(
    binding: &BindingCallContext,
    _out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lstatUtf16")).boxed())
}

/// Update access and modification times without following symlinks.
pub(crate) unsafe fn destack_fs_lutimes_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (binding, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lutimesBytes")).boxed())
}

/// Update access and modification times without following symlinks.
pub(crate) unsafe fn destack_fs_lutimes_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (binding, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lutimesUtf16")).boxed())
}

/// Update access and modification times relative to a directory handle.
pub(crate) unsafe fn destack_fs_utimensat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, atimens, mtimens, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimensatBytes")).boxed())
}

/// Update access and modification times relative to a directory handle.
pub(crate) unsafe fn destack_fs_utimensat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, atimens, mtimens, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimensatUtf16")).boxed())
}

/// Create a temporary directory.
pub(crate) unsafe fn destack_fs_mkdtemp_bytes(
    binding: &BindingCallContext,
    _out: *mut PathBytes,
    template: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, template);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdtempBytes")).boxed())
}

/// Create a temporary directory.
pub(crate) unsafe fn destack_fs_mkdtemp_utf16(
    binding: &BindingCallContext,
    _out: *mut PathUtf16,
    template: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, template);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdtempUtf16")).boxed())
}

/// Open a file and return a handle.
pub(crate) unsafe fn destack_fs_open_bytes(
    binding: &BindingCallContext,
    _out: *mut FileHandle,
    path: PathBytes,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openBytes")).boxed())
}

/// Open a file and return a handle.
pub(crate) unsafe fn destack_fs_open_utf16(
    binding: &BindingCallContext,
    _out: *mut FileHandle,
    path: PathUtf16,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openUtf16")).boxed())
}

/// Open a directory and return a handle.
pub(crate) unsafe fn destack_fs_opendir_bytes(
    binding: &BindingCallContext,
    _out: *mut DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.opendirBytes")).boxed())
}

/// Open a directory and return a handle.
pub(crate) unsafe fn destack_fs_opendir_utf16(
    binding: &BindingCallContext,
    _out: *mut DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.opendirUtf16")).boxed())
}

/// Read from a file into the provided slice.
pub(crate) unsafe fn destack_fs_read(
    binding: &BindingCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.read")).boxed())
}

/// Read directory entries from an open directory handle.
pub(crate) unsafe fn destack_fs_readdir(
    binding: &BindingCallContext,
    _out: *mut NativeArray<Dirent>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readdir")).boxed())
}

/// Read a symbolic link.
pub(crate) unsafe fn destack_fs_readlink_bytes(
    binding: &BindingCallContext,
    _out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkBytes")).boxed())
}

/// Read a symbolic link.
pub(crate) unsafe fn destack_fs_readlink_utf16(
    binding: &BindingCallContext,
    _out: *mut PathUtf16,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkUtf16")).boxed())
}

/// Read into multiple buffers.
pub(crate) unsafe fn destack_fs_readv(
    binding: &BindingCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (binding, handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readv")).boxed())
}

/// Resolve a path to its canonical form.
pub(crate) unsafe fn destack_fs_realpath_bytes(
    binding: &BindingCallContext,
    _out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.realpathBytes")).boxed())
}

/// Resolve a path to its canonical form.
pub(crate) unsafe fn destack_fs_realpath_utf16(
    binding: &BindingCallContext,
    _out: *mut PathUtf16,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.realpathUtf16")).boxed())
}

/// Rename or move a file.
pub(crate) unsafe fn destack_fs_rename_bytes(
    binding: &BindingCallContext,
    from: PathBytes,
    to: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, from, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameBytes")).boxed())
}

/// Rename or move a file.
pub(crate) unsafe fn destack_fs_rename_utf16(
    binding: &BindingCallContext,
    from: PathUtf16,
    to: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, from, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameUtf16")).boxed())
}

/// Remove a directory.
pub(crate) unsafe fn destack_fs_rmdir_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.rmdirBytes")).boxed())
}

/// Remove a directory.
pub(crate) unsafe fn destack_fs_rmdir_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.rmdirUtf16")).boxed())
}

/// Stat a file.
pub(crate) unsafe fn destack_fs_stat_bytes(
    binding: &BindingCallContext,
    _out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statBytes")).boxed())
}

/// Stat a file.
pub(crate) unsafe fn destack_fs_stat_utf16(
    binding: &BindingCallContext,
    _out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statUtf16")).boxed())
}

/// Stat a filesystem.
pub(crate) unsafe fn destack_fs_statfs_bytes(
    binding: &BindingCallContext,
    _out: *mut StatFs,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statfsBytes")).boxed())
}

/// Stat a filesystem.
pub(crate) unsafe fn destack_fs_statfs_utf16(
    binding: &BindingCallContext,
    _out: *mut StatFs,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statfsUtf16")).boxed())
}

/// Create a symbolic link.
pub(crate) unsafe fn destack_fs_symlink_bytes(
    binding: &BindingCallContext,
    target: PathBytes,
    path: PathBytes,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = (binding, target, path, kind);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlinkBytes")).boxed())
}

/// Create a symbolic link.
pub(crate) unsafe fn destack_fs_symlink_utf16(
    binding: &BindingCallContext,
    target: PathUtf16,
    path: PathUtf16,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = (binding, target, path, kind);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlinkUtf16")).boxed())
}

/// Truncate a file.
pub(crate) unsafe fn destack_fs_truncate_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (binding, path, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.truncateBytes")).boxed())
}

/// Truncate a file.
pub(crate) unsafe fn destack_fs_truncate_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (binding, path, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.truncateUtf16")).boxed())
}

/// Unlink a file.
pub(crate) unsafe fn destack_fs_unlink_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkBytes")).boxed())
}

/// Unlink a file.
pub(crate) unsafe fn destack_fs_unlink_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkUtf16")).boxed())
}

/// Update access and modification times.
pub(crate) unsafe fn destack_fs_utimes_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (binding, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimesBytes")).boxed())
}

/// Update access and modification times.
pub(crate) unsafe fn destack_fs_utimes_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (binding, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimesUtf16")).boxed())
}

/// Write to a file from the provided slice.
pub(crate) unsafe fn destack_fs_write(
    binding: &BindingCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.write")).boxed())
}

/// Write from multiple buffers.
pub(crate) unsafe fn destack_fs_writev(
    binding: &BindingCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (binding, handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.writev")).boxed())
}

/// Open a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_openat_bytes(
    binding: &BindingCallContext,
    _out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openatBytes")).boxed())
}

/// Open a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_openat_utf16(
    binding: &BindingCallContext,
    _out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openatUtf16")).boxed())
}

/// Open a file relative to a directory handle with openat2 semantics.
pub(crate) unsafe fn destack_fs_openat2_bytes(
    binding: &BindingCallContext,
    _out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathBytes,
    how: OpenOptions,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, how);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openat2Bytes")).boxed())
}

/// Open a file relative to a directory handle with openat2 semantics.
pub(crate) unsafe fn destack_fs_openat2_utf16(
    binding: &BindingCallContext,
    _out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathUtf16,
    how: OpenOptions,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, how);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openat2Utf16")).boxed())
}

/// Create a directory.
pub(crate) unsafe fn destack_fs_mkdir_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdirBytes")).boxed())
}

/// Create a directory.
pub(crate) unsafe fn destack_fs_mkdir_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdirUtf16")).boxed())
}

/// Create a directory relative to a directory handle.
pub(crate) unsafe fn destack_fs_mkdirat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdiratBytes")).boxed())
}

/// Create a directory relative to a directory handle.
pub(crate) unsafe fn destack_fs_mkdirat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdiratUtf16")).boxed())
}

/// Rename or move a file relative to directory handles.
pub(crate) unsafe fn destack_fs_renameat_bytes(
    binding: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathBytes,
    to_dir: DirectoryHandle,
    to: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, from_dir, from, to_dir, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameatBytes")).boxed())
}

/// Rename or move a file relative to directory handles.
pub(crate) unsafe fn destack_fs_renameat_utf16(
    binding: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathUtf16,
    to_dir: DirectoryHandle,
    to: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, from_dir, from, to_dir, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameatUtf16")).boxed())
}

/// Rename or move a file relative to directory handles with renameat2 semantics.
pub(crate) unsafe fn destack_fs_renameat2_bytes(
    binding: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathBytes,
    to_dir: DirectoryHandle,
    to: PathBytes,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    let _ = (binding, from_dir, from, to_dir, to, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameat2Bytes")).boxed())
}

/// Rename or move a file relative to directory handles with renameat2 semantics.
pub(crate) unsafe fn destack_fs_renameat2_utf16(
    binding: &BindingCallContext,
    from_dir: DirectoryHandle,
    from: PathUtf16,
    to_dir: DirectoryHandle,
    to: PathUtf16,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    let _ = (binding, from_dir, from, to_dir, to, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameat2Utf16")).boxed())
}

/// Unlink a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_unlinkat_bytes(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkatBytes")).boxed())
}

/// Unlink a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_unlinkat_utf16(
    binding: &BindingCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkatUtf16")).boxed())
}

/// Create a hard link relative to directory handles.
pub(crate) unsafe fn destack_fs_linkat_bytes(
    binding: &BindingCallContext,
    existing_dir: DirectoryHandle,
    existing_path: PathBytes,
    new_dir: DirectoryHandle,
    new_path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (
        binding,
        existing_dir,
        existing_path,
        new_dir,
        new_path,
        flags,
    );
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkatBytes")).boxed())
}

/// Create a hard link relative to directory handles.
pub(crate) unsafe fn destack_fs_linkat_utf16(
    binding: &BindingCallContext,
    existing_dir: DirectoryHandle,
    existing_path: PathUtf16,
    new_dir: DirectoryHandle,
    new_path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (
        binding,
        existing_dir,
        existing_path,
        new_dir,
        new_path,
        flags,
    );
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkatUtf16")).boxed())
}

/// Create a symbolic link relative to a directory handle.
pub(crate) unsafe fn destack_fs_symlinkat_bytes(
    binding: &BindingCallContext,
    target: PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = (binding, target, dir, path, kind);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlinkatBytes")).boxed())
}

/// Create a symbolic link relative to a directory handle.
pub(crate) unsafe fn destack_fs_symlinkat_utf16(
    binding: &BindingCallContext,
    target: PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = (binding, target, dir, path, kind);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlinkatUtf16")).boxed())
}

/// Read a symbolic link relative to a directory handle.
pub(crate) unsafe fn destack_fs_readlinkat_bytes(
    binding: &BindingCallContext,
    _out: *mut PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkatBytes")).boxed())
}

/// Read a symbolic link relative to a directory handle.
pub(crate) unsafe fn destack_fs_readlinkat_utf16(
    binding: &BindingCallContext,
    _out: *mut PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkatUtf16")).boxed())
}

/// Stat a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_statat_bytes(
    binding: &BindingCallContext,
    _out: *mut Stat,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statatBytes")).boxed())
}

/// Stat a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_statat_utf16(
    binding: &BindingCallContext,
    _out: *mut Stat,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (binding, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statatUtf16")).boxed())
}

/// Apply file locks to a file handle.
pub(crate) unsafe fn destack_fs_lock(
    binding: &BindingCallContext,
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    let _ = (binding, handle, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lock")).boxed())
}

/// Copy a range between file descriptors.
pub(crate) unsafe fn destack_fs_copy_file_range(
    binding: &BindingCallContext,
    _out: *mut u64,
    src: FileHandle,
    src_offset: FileOffset,
    dst: FileHandle,
    dst_offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<()> {
    let _ = (binding, src, src_offset, dst, dst_offset, length);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.copyFileRange")).boxed())
}

/// Duplicate a file handle.
pub(crate) unsafe fn destack_fs_dup(
    binding: &BindingCallContext,
    _out: *mut FileHandle,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dup")).boxed())
}

/// Duplicate a file handle to a specific target.
pub(crate) unsafe fn destack_fs_dup2(
    binding: &BindingCallContext,
    _out: *mut FileHandle,
    handle: FileHandle,
    target: FileHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle, target);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dup2")).boxed())
}

/// Duplicate a file handle to a specific target with flags.
pub(crate) unsafe fn destack_fs_dup3(
    binding: &BindingCallContext,
    _out: *mut FileHandle,
    handle: FileHandle,
    target: FileHandle,
    flags: OpenFlags,
) -> RuntimeResult<()> {
    let _ = (binding, handle, target, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dup3")).boxed())
}

/// Advise the kernel about access patterns.
pub(crate) unsafe fn destack_fs_fadvise(
    binding: &BindingCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    advice: FileAdvice,
) -> RuntimeResult<()> {
    let _ = (binding, handle, offset, length, advice);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fadvise")).boxed())
}

/// Allocate or punch file space.
pub(crate) unsafe fn destack_fs_fallocate(
    binding: &BindingCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: AllocFlags,
) -> RuntimeResult<()> {
    let _ = (binding, handle, offset, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fallocate")).boxed())
}

/// Advise the kernel about access patterns.
pub(crate) unsafe fn destack_fs_madvise(
    binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
    advice: MmapAdvice,
) -> RuntimeResult<()> {
    let _ = (binding, mapping, advice);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.madvise")).boxed())
}

/// Create an anonymous memory mapping.
pub(crate) unsafe fn destack_fs_mmap_anonymous(
    binding: &BindingCallContext,
    _out: *mut NativeSlice<u8>,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    let _ = (binding, length, prot, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmapAnonymous")).boxed())
}

/// Create a file-backed memory mapping.
pub(crate) unsafe fn destack_fs_mmap_file(
    binding: &BindingCallContext,
    _out: *mut NativeSlice<u8>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    let _ = (binding, handle, offset, length, prot, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmapFile")).boxed())
}

/// Change memory protection for a mapping.
pub(crate) unsafe fn destack_fs_mprotect(
    binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
    prot: MmapProt,
) -> RuntimeResult<()> {
    let _ = (binding, mapping, prot);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mprotect")).boxed())
}

/// Flush a mapping to storage.
pub(crate) unsafe fn destack_fs_msync(
    binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
    flags: MmapSyncFlags,
) -> RuntimeResult<()> {
    let _ = (binding, mapping, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.msync")).boxed())
}

/// Unmap a memory region.
pub(crate) unsafe fn destack_fs_munmap(
    binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, mapping);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.munmap")).boxed())
}

/// Read from a file at the given file offset.
pub(crate) unsafe fn destack_fs_pread(
    binding: &BindingCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (binding, handle, buffer, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.pread")).boxed())
}

/// Read into multiple buffers at the given file offset.
pub(crate) unsafe fn destack_fs_preadv(
    binding: &BindingCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (binding, handle, buffers, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.preadv")).boxed())
}

/// Read into multiple buffers at the given file offset with explicit read flags.
pub(crate) unsafe fn destack_fs_preadv2(
    binding: &BindingCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    let _ = (binding, handle, buffers, offset, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.preadv2")).boxed())
}

/// Write to a file at the given file offset.
pub(crate) unsafe fn destack_fs_pwrite(
    binding: &BindingCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (binding, handle, buffer, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.pwrite")).boxed())
}

/// Write from multiple buffers at the given file offset.
pub(crate) unsafe fn destack_fs_pwritev(
    binding: &BindingCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (binding, handle, buffers, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.pwritev")).boxed())
}

/// Write from multiple buffers at the given file offset with explicit write flags.
pub(crate) unsafe fn destack_fs_pwritev2(
    binding: &BindingCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    let _ = (binding, handle, buffers, offset, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.pwritev2")).boxed())
}

/// Seek within a file and return the new offset.
pub(crate) unsafe fn destack_fs_seek(
    binding: &BindingCallContext,
    _out: *mut FileOffset,
    handle: FileHandle,
    offset: FileOffset,
    whence: SeekWhence,
) -> RuntimeResult<()> {
    let _ = (binding, handle, offset, whence);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.seek")).boxed())
}

/// Send file data to a socket.
pub(crate) unsafe fn destack_fs_sendfile(
    binding: &BindingCallContext,
    _out: *mut u64,
    socket: SocketHandle,
    file: FileHandle,
    offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<()> {
    let _ = (binding, socket, file, offset, length);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.sendfile")).boxed())
}

/// Transfer bytes between descriptors using kernel splice pipelines.
pub(crate) unsafe fn destack_fs_splice(
    binding: &BindingCallContext,
    _out: *mut u64,
    source: ResourceId,
    sourcecursor: SpliceCursor,
    target: ResourceId,
    targetcursor: SpliceCursor,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (
        binding,
        source,
        sourcecursor,
        target,
        targetcursor,
        length,
        flags,
    );
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.splice")).boxed())
}

/// Duplicate bytes from one pipe to another without consuming source bytes.
pub(crate) unsafe fn destack_fs_tee(
    binding: &BindingCallContext,
    _out: *mut u64,
    sourcepipe: PipeHandle,
    targetpipe: PipeHandle,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (binding, sourcepipe, targetpipe, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.tee")).boxed())
}

/// Map user memory pages into a pipe as queued pipe buffers.
pub(crate) unsafe fn destack_fs_vmsplice(
    binding: &BindingCallContext,
    _out: *mut u64,
    pipe: PipeHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (binding, pipe, buffers, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.vmsplice")).boxed())
}

/// Synchronize a file range.
pub(crate) unsafe fn destack_fs_sync_file_range(
    binding: &BindingCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: SyncFlags,
) -> RuntimeResult<()> {
    let _ = (binding, handle, offset, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.syncFileRange")).boxed())
}

/// Read an extended attribute by path with a raw name payload.
pub(crate) unsafe fn destack_fs_getxattr_bytes(
    binding: &BindingCallContext,
    _out: *mut NativeArray<u8>,
    path: PathBytes,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.getxattrBytes")).boxed())
}

/// Read an extended attribute by path.
pub(crate) unsafe fn destack_fs_getxattr_utf16(
    binding: &BindingCallContext,
    _out: *mut NativeArray<u8>,
    path: PathUtf16,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.getxattrUtf16")).boxed())
}

/// Read an extended attribute without following symlinks, using a raw name payload.
pub(crate) unsafe fn destack_fs_lgetxattr_bytes(
    binding: &BindingCallContext,
    _out: *mut NativeArray<u8>,
    path: PathBytes,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lgetxattrBytes")).boxed())
}

/// Read an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lgetxattr_utf16(
    binding: &BindingCallContext,
    _out: *mut NativeArray<u8>,
    path: PathUtf16,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lgetxattrUtf16")).boxed())
}

/// Read an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fgetxattr_handle(
    binding: &BindingCallContext,
    _out: *mut NativeArray<u8>,
    handle: FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, handle, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fgetxattr")).boxed())
}

/// Set an extended attribute by path with a raw name payload.
pub(crate) unsafe fn destack_fs_setxattr_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (binding, path, name, value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.setxattrBytes")).boxed())
}

/// Set an extended attribute by path.
pub(crate) unsafe fn destack_fs_setxattr_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (binding, path, name, value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.setxattrUtf16")).boxed())
}

/// Set an extended attribute without following symlinks, using a raw name payload.
pub(crate) unsafe fn destack_fs_lsetxattr_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (binding, path, name, value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lsetxattrBytes")).boxed())
}

/// Set an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lsetxattr_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (binding, path, name, value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lsetxattrUtf16")).boxed())
}

/// Set an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fsetxattr_handle(
    binding: &BindingCallContext,
    handle: FileHandle,
    name: NativeSlice<u8>,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (binding, handle, name, value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fsetxattr")).boxed())
}

/// List extended attribute names by path as raw byte payloads.
pub(crate) unsafe fn destack_fs_listxattr_bytes(
    binding: &BindingCallContext,
    _out: *mut NativeArray<NativeArray<u8>>,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.listxattrBytes")).boxed())
}

/// List extended attribute names by path.
pub(crate) unsafe fn destack_fs_listxattr_utf16(
    binding: &BindingCallContext,
    _out: *mut NativeArray<NativeArray<u8>>,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.listxattrUtf16")).boxed())
}

/// List extended attribute names without following symlinks as raw byte payloads.
pub(crate) unsafe fn destack_fs_llistxattr_bytes(
    binding: &BindingCallContext,
    _out: *mut NativeArray<NativeArray<u8>>,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.llistxattrBytes")).boxed())
}

/// List extended attribute names without following symlinks.
pub(crate) unsafe fn destack_fs_llistxattr_utf16(
    binding: &BindingCallContext,
    _out: *mut NativeArray<NativeArray<u8>>,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (binding, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.llistxattrUtf16")).boxed())
}

/// List extended attribute names by handle.
pub(crate) unsafe fn destack_fs_flistxattr_handle(
    binding: &BindingCallContext,
    _out: *mut NativeArray<NativeArray<u8>>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (binding, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.flistxattr")).boxed())
}

/// Remove an extended attribute by path with a raw name payload.
pub(crate) unsafe fn destack_fs_removexattr_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.removexattrBytes")).boxed())
}

/// Remove an extended attribute by path.
pub(crate) unsafe fn destack_fs_removexattr_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.removexattrUtf16")).boxed())
}

/// Remove an extended attribute without following symlinks, using a raw name payload.
pub(crate) unsafe fn destack_fs_lremovexattr_bytes(
    binding: &BindingCallContext,
    path: PathBytes,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lremovexattrBytes")).boxed())
}

/// Remove an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lremovexattr_utf16(
    binding: &BindingCallContext,
    path: PathUtf16,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lremovexattrUtf16")).boxed())
}

/// Remove an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fremovexattr_handle(
    binding: &BindingCallContext,
    handle: FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (binding, handle, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fremovexattr")).boxed())
}
