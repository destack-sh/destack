#![allow(dead_code)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{
    AccessMode, AllocFlags, AtFlags, CopyFlags, FileAdvice, FileLockFlags, FileMode, FileOffset,
    FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags, OpenFlags, OpenOptions, PathBytes,
    PathUtf16, RenameFlags, SeekWhence, Stat, StatFs, SymlinkType, SyncFlags, XattrFlags,
};
use crate::platform::net::SocketHandle;
use crate::platform::resource::{DirectoryHandle, FileHandle, PipeHandle, ResourceId};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Reject unsupported fs access bytes.
pub(crate) unsafe fn destack_fs_access_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let _ = (context, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.accessBytes")).boxed())
}

/// Reject unsupported fs access utf16.
pub(crate) unsafe fn destack_fs_access_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let _ = (context, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.accessUtf16")).boxed())
}

/// Reject unsupported fs chmod bytes.
pub(crate) unsafe fn destack_fs_chmod_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chmodBytes")).boxed())
}

/// Reject unsupported fs chmod utf16.
pub(crate) unsafe fn destack_fs_chmod_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chmodUtf16")).boxed())
}

/// Reject unsupported fs fchmodat bytes.
pub(crate) unsafe fn destack_fs_fchmodat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, mode, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchmodatBytes")).boxed())
}

/// Reject unsupported fs fchmodat utf16.
pub(crate) unsafe fn destack_fs_fchmodat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, mode, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchmodatUtf16")).boxed())
}

/// Reject unsupported fs chown bytes.
pub(crate) unsafe fn destack_fs_chown_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (context, path, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chownBytes")).boxed())
}

/// Reject unsupported fs chown utf16.
pub(crate) unsafe fn destack_fs_chown_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (context, path, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chownUtf16")).boxed())
}

/// Reject unsupported fs fchownat bytes.
pub(crate) unsafe fn destack_fs_fchownat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, uid, gid, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchownatBytes")).boxed())
}

/// Reject unsupported fs fchownat utf16.
pub(crate) unsafe fn destack_fs_fchownat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, uid, gid, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchownatUtf16")).boxed())
}

/// Reject unsupported fs close.
pub(crate) unsafe fn destack_fs_close(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.close")).boxed())
}

/// Reject unsupported fs closedir.
pub(crate) unsafe fn destack_fs_closedir(
    context: &RuntimeCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.closedir")).boxed())
}

/// Reject unsupported fs copyfile bytes.
pub(crate) unsafe fn destack_fs_copyfile_bytes(
    context: &RuntimeCallContext,
    from: PathBytes,
    to: PathBytes,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    let _ = (context, from, to, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.copyfileBytes")).boxed())
}

/// Reject unsupported fs copyfile utf16.
pub(crate) unsafe fn destack_fs_copyfile_utf16(
    context: &RuntimeCallContext,
    from: PathUtf16,
    to: PathUtf16,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    let _ = (context, from, to, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.copyfileUtf16")).boxed())
}

/// Reject unsupported fs fchmod.
pub(crate) unsafe fn destack_fs_fchmod(
    context: &RuntimeCallContext,
    handle: FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchmod")).boxed())
}

/// Reject unsupported fs fchown.
pub(crate) unsafe fn destack_fs_fchown(
    context: &RuntimeCallContext,
    handle: FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchown")).boxed())
}

/// Reject unsupported fs fdatasync.
pub(crate) unsafe fn destack_fs_fdatasync(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fdatasync")).boxed())
}

/// Reject unsupported fs fstat.
pub(crate) unsafe fn destack_fs_fstat(
    context: &RuntimeCallContext,
    _out: *mut Stat,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fstat")).boxed())
}

/// Reject unsupported fs fstatfs.
pub(crate) unsafe fn destack_fs_fstatfs(
    context: &RuntimeCallContext,
    _out: *mut StatFs,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fstatfs")).boxed())
}

/// Reject unsupported fs fsync.
pub(crate) unsafe fn destack_fs_fsync(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fsync")).boxed())
}

/// Reject unsupported fs ftruncate.
pub(crate) unsafe fn destack_fs_ftruncate(
    context: &RuntimeCallContext,
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, handle, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.ftruncate")).boxed())
}

/// Reject unsupported fs futimes.
pub(crate) unsafe fn destack_fs_futimes(
    context: &RuntimeCallContext,
    handle: FileHandle,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, handle, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.futimes")).boxed())
}

/// Reject unsupported fs link bytes.
pub(crate) unsafe fn destack_fs_link_bytes(
    context: &RuntimeCallContext,
    existingpath: PathBytes,
    newpath: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, existingpath, newpath);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkBytes")).boxed())
}

/// Reject unsupported fs link utf16.
pub(crate) unsafe fn destack_fs_link_utf16(
    context: &RuntimeCallContext,
    existingpath: PathUtf16,
    newpath: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, existingpath, newpath);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkUtf16")).boxed())
}

/// Reject unsupported fs lstat bytes.
pub(crate) unsafe fn destack_fs_lstat_bytes(
    context: &RuntimeCallContext,
    _out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lstatBytes")).boxed())
}

/// Reject unsupported fs lstat utf16.
pub(crate) unsafe fn destack_fs_lstat_utf16(
    context: &RuntimeCallContext,
    _out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lstatUtf16")).boxed())
}

/// Reject unsupported fs lutimes bytes.
pub(crate) unsafe fn destack_fs_lutimes_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lutimesBytes")).boxed())
}

/// Reject unsupported fs lutimes utf16.
pub(crate) unsafe fn destack_fs_lutimes_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lutimesUtf16")).boxed())
}

/// Reject unsupported fs utimensat bytes.
pub(crate) unsafe fn destack_fs_utimensat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, atimens, mtimens, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimensatBytes")).boxed())
}

/// Reject unsupported fs utimensat utf16.
pub(crate) unsafe fn destack_fs_utimensat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, atimens, mtimens, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimensatUtf16")).boxed())
}

/// Reject unsupported fs mkdtemp bytes.
pub(crate) unsafe fn destack_fs_mkdtemp_bytes(
    context: &RuntimeCallContext,
    _out: *mut PathBytes,
    template: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, template);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdtempBytes")).boxed())
}

/// Reject unsupported fs mkdtemp utf16.
pub(crate) unsafe fn destack_fs_mkdtemp_utf16(
    context: &RuntimeCallContext,
    _out: *mut PathUtf16,
    template: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, template);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdtempUtf16")).boxed())
}

/// Reject unsupported fs open bytes.
pub(crate) unsafe fn destack_fs_open_bytes(
    context: &RuntimeCallContext,
    _out: *mut FileHandle,
    path: PathBytes,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openBytes")).boxed())
}

/// Reject unsupported fs open utf16.
pub(crate) unsafe fn destack_fs_open_utf16(
    context: &RuntimeCallContext,
    _out: *mut FileHandle,
    path: PathUtf16,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openUtf16")).boxed())
}

/// Reject unsupported fs opendir bytes.
pub(crate) unsafe fn destack_fs_opendir_bytes(
    context: &RuntimeCallContext,
    _out: *mut DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.opendirBytes")).boxed())
}

/// Reject unsupported fs opendir utf16.
pub(crate) unsafe fn destack_fs_opendir_utf16(
    context: &RuntimeCallContext,
    _out: *mut DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.opendirUtf16")).boxed())
}

/// Reject unsupported fs read.
pub(crate) unsafe fn destack_fs_read(
    context: &RuntimeCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.read")).boxed())
}

/// Reject unsupported fs readdir.
pub(crate) unsafe fn destack_fs_readdir(
    context: &RuntimeCallContext,
    _out: *mut NativeArray<crate::platform::fs::Dirent>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readdir")).boxed())
}

/// Reject unsupported fs readlink bytes.
pub(crate) unsafe fn destack_fs_readlink_bytes(
    context: &RuntimeCallContext,
    _out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkBytes")).boxed())
}

/// Reject unsupported fs readlink utf16.
pub(crate) unsafe fn destack_fs_readlink_utf16(
    context: &RuntimeCallContext,
    _out: *mut PathUtf16,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkUtf16")).boxed())
}

/// Reject unsupported fs readv.
pub(crate) unsafe fn destack_fs_readv(
    context: &RuntimeCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (context, handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readv")).boxed())
}

/// Reject unsupported fs realpath bytes.
pub(crate) unsafe fn destack_fs_realpath_bytes(
    context: &RuntimeCallContext,
    _out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.realpathBytes")).boxed())
}

/// Reject unsupported fs realpath utf16.
pub(crate) unsafe fn destack_fs_realpath_utf16(
    context: &RuntimeCallContext,
    _out: *mut PathUtf16,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.realpathUtf16")).boxed())
}

/// Reject unsupported fs rename bytes.
pub(crate) unsafe fn destack_fs_rename_bytes(
    context: &RuntimeCallContext,
    from: PathBytes,
    to: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, from, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameBytes")).boxed())
}

/// Reject unsupported fs rename utf16.
pub(crate) unsafe fn destack_fs_rename_utf16(
    context: &RuntimeCallContext,
    from: PathUtf16,
    to: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, from, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameUtf16")).boxed())
}

/// Reject unsupported fs rmdir bytes.
pub(crate) unsafe fn destack_fs_rmdir_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.rmdirBytes")).boxed())
}

/// Reject unsupported fs rmdir utf16.
pub(crate) unsafe fn destack_fs_rmdir_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.rmdirUtf16")).boxed())
}

/// Reject unsupported fs stat bytes.
pub(crate) unsafe fn destack_fs_stat_bytes(
    context: &RuntimeCallContext,
    _out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statBytes")).boxed())
}

/// Reject unsupported fs stat utf16.
pub(crate) unsafe fn destack_fs_stat_utf16(
    context: &RuntimeCallContext,
    _out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statUtf16")).boxed())
}

/// Reject unsupported fs statfs bytes.
pub(crate) unsafe fn destack_fs_statfs_bytes(
    context: &RuntimeCallContext,
    _out: *mut StatFs,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statfsBytes")).boxed())
}

/// Reject unsupported fs statfs utf16.
pub(crate) unsafe fn destack_fs_statfs_utf16(
    context: &RuntimeCallContext,
    _out: *mut StatFs,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statfsUtf16")).boxed())
}

/// Reject unsupported fs symlink bytes.
pub(crate) unsafe fn destack_fs_symlink_bytes(
    context: &RuntimeCallContext,
    target: PathBytes,
    path: PathBytes,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = (context, target, path, kind);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlinkBytes")).boxed())
}

/// Reject unsupported fs symlink utf16.
pub(crate) unsafe fn destack_fs_symlink_utf16(
    context: &RuntimeCallContext,
    target: PathUtf16,
    path: PathUtf16,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = (context, target, path, kind);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlinkUtf16")).boxed())
}

/// Reject unsupported fs truncate bytes.
pub(crate) unsafe fn destack_fs_truncate_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, path, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.truncateBytes")).boxed())
}

/// Reject unsupported fs truncate utf16.
pub(crate) unsafe fn destack_fs_truncate_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, path, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.truncateUtf16")).boxed())
}

/// Reject unsupported fs unlink bytes.
pub(crate) unsafe fn destack_fs_unlink_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkBytes")).boxed())
}

/// Reject unsupported fs unlink utf16.
pub(crate) unsafe fn destack_fs_unlink_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkUtf16")).boxed())
}

/// Reject unsupported fs utimes bytes.
pub(crate) unsafe fn destack_fs_utimes_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimesBytes")).boxed())
}

/// Reject unsupported fs utimes utf16.
pub(crate) unsafe fn destack_fs_utimes_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimesUtf16")).boxed())
}

/// Reject unsupported fs write.
pub(crate) unsafe fn destack_fs_write(
    context: &RuntimeCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, handle, buffer);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.write")).boxed())
}

/// Reject unsupported fs writev.
pub(crate) unsafe fn destack_fs_writev(
    context: &RuntimeCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (context, handle, buffers);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.writev")).boxed())
}

/// Reject unsupported fs openat bytes.
pub(crate) unsafe fn destack_fs_openat_bytes(
    context: &RuntimeCallContext,
    _out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openatBytes")).boxed())
}

/// Reject unsupported fs openat utf16.
pub(crate) unsafe fn destack_fs_openat_utf16(
    context: &RuntimeCallContext,
    _out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openatUtf16")).boxed())
}

/// Reject unsupported fs openat2 bytes.
pub(crate) unsafe fn destack_fs_openat2_bytes(
    context: &RuntimeCallContext,
    _out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathBytes,
    how: OpenOptions,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, how);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openat2Bytes")).boxed())
}

/// Reject unsupported fs openat2 utf16.
pub(crate) unsafe fn destack_fs_openat2_utf16(
    context: &RuntimeCallContext,
    _out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathUtf16,
    how: OpenOptions,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, how);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openat2Utf16")).boxed())
}

/// Reject unsupported fs mkdir bytes.
pub(crate) unsafe fn destack_fs_mkdir_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdirBytes")).boxed())
}

/// Reject unsupported fs mkdir utf16.
pub(crate) unsafe fn destack_fs_mkdir_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdirUtf16")).boxed())
}

/// Reject unsupported fs mkdirat bytes.
pub(crate) unsafe fn destack_fs_mkdirat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdiratBytes")).boxed())
}

/// Reject unsupported fs mkdirat utf16.
pub(crate) unsafe fn destack_fs_mkdirat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdiratUtf16")).boxed())
}

/// Reject unsupported fs renameat bytes.
pub(crate) unsafe fn destack_fs_renameat_bytes(
    context: &RuntimeCallContext,
    from_dir: DirectoryHandle,
    from: PathBytes,
    to_dir: DirectoryHandle,
    to: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, from_dir, from, to_dir, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameatBytes")).boxed())
}

/// Reject unsupported fs renameat utf16.
pub(crate) unsafe fn destack_fs_renameat_utf16(
    context: &RuntimeCallContext,
    from_dir: DirectoryHandle,
    from: PathUtf16,
    to_dir: DirectoryHandle,
    to: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, from_dir, from, to_dir, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameatUtf16")).boxed())
}

/// Reject unsupported fs renameat2 bytes.
pub(crate) unsafe fn destack_fs_renameat2_bytes(
    context: &RuntimeCallContext,
    from_dir: DirectoryHandle,
    from: PathBytes,
    to_dir: DirectoryHandle,
    to: PathBytes,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    let _ = (context, from_dir, from, to_dir, to, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameat2Bytes")).boxed())
}

/// Reject unsupported fs renameat2 utf16.
pub(crate) unsafe fn destack_fs_renameat2_utf16(
    context: &RuntimeCallContext,
    from_dir: DirectoryHandle,
    from: PathUtf16,
    to_dir: DirectoryHandle,
    to: PathUtf16,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    let _ = (context, from_dir, from, to_dir, to, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameat2Utf16")).boxed())
}

/// Reject unsupported fs unlinkat bytes.
pub(crate) unsafe fn destack_fs_unlinkat_bytes(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkatBytes")).boxed())
}

/// Reject unsupported fs unlinkat utf16.
pub(crate) unsafe fn destack_fs_unlinkat_utf16(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkatUtf16")).boxed())
}

/// Reject unsupported fs linkat bytes.
pub(crate) unsafe fn destack_fs_linkat_bytes(
    context: &RuntimeCallContext,
    existing_dir: DirectoryHandle,
    existing_path: PathBytes,
    new_dir: DirectoryHandle,
    new_path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (
        context,
        existing_dir,
        existing_path,
        new_dir,
        new_path,
        flags,
    );
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkatBytes")).boxed())
}

/// Reject unsupported fs linkat utf16.
pub(crate) unsafe fn destack_fs_linkat_utf16(
    context: &RuntimeCallContext,
    existing_dir: DirectoryHandle,
    existing_path: PathUtf16,
    new_dir: DirectoryHandle,
    new_path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (
        context,
        existing_dir,
        existing_path,
        new_dir,
        new_path,
        flags,
    );
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkatUtf16")).boxed())
}

/// Reject unsupported fs symlinkat bytes.
pub(crate) unsafe fn destack_fs_symlinkat_bytes(
    context: &RuntimeCallContext,
    target: PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = (context, target, dir, path, kind);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlinkatBytes")).boxed())
}

/// Reject unsupported fs symlinkat utf16.
pub(crate) unsafe fn destack_fs_symlinkat_utf16(
    context: &RuntimeCallContext,
    target: PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = (context, target, dir, path, kind);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlinkatUtf16")).boxed())
}

/// Reject unsupported fs readlinkat bytes.
pub(crate) unsafe fn destack_fs_readlinkat_bytes(
    context: &RuntimeCallContext,
    _out: *mut PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, dir, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkatBytes")).boxed())
}

/// Reject unsupported fs readlinkat utf16.
pub(crate) unsafe fn destack_fs_readlinkat_utf16(
    context: &RuntimeCallContext,
    _out: *mut PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, dir, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkatUtf16")).boxed())
}

/// Reject unsupported fs statat bytes.
pub(crate) unsafe fn destack_fs_statat_bytes(
    context: &RuntimeCallContext,
    _out: *mut Stat,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statatBytes")).boxed())
}

/// Reject unsupported fs statat utf16.
pub(crate) unsafe fn destack_fs_statat_utf16(
    context: &RuntimeCallContext,
    _out: *mut Stat,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statatUtf16")).boxed())
}

/// Reject unsupported fs lock.
pub(crate) unsafe fn destack_fs_lock(
    context: &RuntimeCallContext,
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    let _ = (context, handle, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lock")).boxed())
}

/// Reject unsupported fs copyFileRange.
pub(crate) unsafe fn destack_fs_copy_file_range(
    context: &RuntimeCallContext,
    _out: *mut u64,
    src: FileHandle,
    src_offset: FileOffset,
    dst: FileHandle,
    dst_offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<()> {
    let _ = (context, src, src_offset, dst, dst_offset, length);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.copyFileRange")).boxed())
}

/// Reject unsupported fs dup.
pub(crate) unsafe fn destack_fs_dup(
    context: &RuntimeCallContext,
    _out: *mut FileHandle,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dup")).boxed())
}

/// Reject unsupported fs dup2.
pub(crate) unsafe fn destack_fs_dup2(
    context: &RuntimeCallContext,
    _out: *mut FileHandle,
    handle: FileHandle,
    target: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle, target);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dup2")).boxed())
}

/// Reject unsupported fs dup3.
pub(crate) unsafe fn destack_fs_dup3(
    context: &RuntimeCallContext,
    _out: *mut FileHandle,
    handle: FileHandle,
    target: FileHandle,
    flags: OpenFlags,
) -> RuntimeResult<()> {
    let _ = (context, handle, target, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dup3")).boxed())
}

/// Reject unsupported fs fadvise.
pub(crate) unsafe fn destack_fs_fadvise(
    context: &RuntimeCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    advice: FileAdvice,
) -> RuntimeResult<()> {
    let _ = (context, handle, offset, length, advice);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fadvise")).boxed())
}

/// Reject unsupported fs fallocate.
pub(crate) unsafe fn destack_fs_fallocate(
    context: &RuntimeCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: AllocFlags,
) -> RuntimeResult<()> {
    let _ = (context, handle, offset, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fallocate")).boxed())
}

/// Reject unsupported fs madvise.
pub(crate) unsafe fn destack_fs_madvise(
    context: &RuntimeCallContext,
    mapping: NativeSlice<u8>,
    advice: MmapAdvice,
) -> RuntimeResult<()> {
    let _ = (context, mapping, advice);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.madvise")).boxed())
}

/// Reject unsupported fs mmapAnonymous.
pub(crate) unsafe fn destack_fs_mmap_anonymous(
    context: &RuntimeCallContext,
    _out: *mut NativeSlice<u8>,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    let _ = (context, length, prot, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmapAnonymous")).boxed())
}

/// Reject unsupported fs mmapFile.
pub(crate) unsafe fn destack_fs_mmap_file(
    context: &RuntimeCallContext,
    _out: *mut NativeSlice<u8>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    let _ = (context, handle, offset, length, prot, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmapFile")).boxed())
}

/// Reject unsupported fs mprotect.
pub(crate) unsafe fn destack_fs_mprotect(
    context: &RuntimeCallContext,
    mapping: NativeSlice<u8>,
    prot: MmapProt,
) -> RuntimeResult<()> {
    let _ = (context, mapping, prot);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mprotect")).boxed())
}

/// Reject unsupported fs msync.
pub(crate) unsafe fn destack_fs_msync(
    context: &RuntimeCallContext,
    mapping: NativeSlice<u8>,
    flags: MmapSyncFlags,
) -> RuntimeResult<()> {
    let _ = (context, mapping, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.msync")).boxed())
}

/// Reject unsupported fs munmap.
pub(crate) unsafe fn destack_fs_munmap(
    context: &RuntimeCallContext,
    mapping: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, mapping);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.munmap")).boxed())
}

/// Reject unsupported fs pread.
pub(crate) unsafe fn destack_fs_pread(
    context: &RuntimeCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, handle, buffer, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.pread")).boxed())
}

/// Reject unsupported fs preadv.
pub(crate) unsafe fn destack_fs_preadv(
    context: &RuntimeCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, handle, buffers, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.preadv")).boxed())
}

/// Reject unsupported fs preadv2.
pub(crate) unsafe fn destack_fs_preadv2(
    context: &RuntimeCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, buffers, offset, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.preadv2")).boxed())
}

/// Reject unsupported fs pwrite.
pub(crate) unsafe fn destack_fs_pwrite(
    context: &RuntimeCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, handle, buffer, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.pwrite")).boxed())
}

/// Reject unsupported fs pwritev.
pub(crate) unsafe fn destack_fs_pwritev(
    context: &RuntimeCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, handle, buffers, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.pwritev")).boxed())
}

/// Reject unsupported fs pwritev2.
pub(crate) unsafe fn destack_fs_pwritev2(
    context: &RuntimeCallContext,
    _out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, buffers, offset, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.pwritev2")).boxed())
}

/// Reject unsupported fs seek.
pub(crate) unsafe fn destack_fs_seek(
    context: &RuntimeCallContext,
    _out: *mut FileOffset,
    handle: FileHandle,
    offset: FileOffset,
    whence: SeekWhence,
) -> RuntimeResult<()> {
    let _ = (context, handle, offset, whence);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.seek")).boxed())
}

/// Reject unsupported fs sendfile.
pub(crate) unsafe fn destack_fs_sendfile(
    context: &RuntimeCallContext,
    _out: *mut u64,
    socket: SocketHandle,
    file: FileHandle,
    offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<()> {
    let _ = (context, socket, file, offset, length);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.sendfile")).boxed())
}

/// Reject unsupported fs splice.
pub(crate) unsafe fn destack_fs_splice(
    context: &RuntimeCallContext,
    _out: *mut u64,
    source: ResourceId,
    sourcecursor: i64,
    target: ResourceId,
    targetcursor: i64,
    length: FileSize,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = (
        context,
        source,
        sourcecursor,
        target,
        targetcursor,
        length,
        flags,
    );
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.splice")).boxed())
}

/// Reject unsupported fs tee.
pub(crate) unsafe fn destack_fs_tee(
    context: &RuntimeCallContext,
    _out: *mut u64,
    sourcepipe: PipeHandle,
    targetpipe: PipeHandle,
    length: FileSize,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = (context, sourcepipe, targetpipe, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.tee")).boxed())
}

/// Reject unsupported fs vmsplice.
pub(crate) unsafe fn destack_fs_vmsplice(
    context: &RuntimeCallContext,
    _out: *mut u64,
    pipe: PipeHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = (context, pipe, buffers, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.vmsplice")).boxed())
}

/// Reject unsupported fs syncFileRange.
pub(crate) unsafe fn destack_fs_sync_file_range(
    context: &RuntimeCallContext,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: SyncFlags,
) -> RuntimeResult<()> {
    let _ = (context, handle, offset, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.syncFileRange")).boxed())
}

/// Reject unsupported fs getxattr bytes.
pub(crate) unsafe fn destack_fs_getxattr_bytes(
    context: &RuntimeCallContext,
    _out: *mut NativeArray<u8>,
    path: PathBytes,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.getxattrBytes")).boxed())
}

/// Reject unsupported fs getxattr utf16.
pub(crate) unsafe fn destack_fs_getxattr_utf16(
    context: &RuntimeCallContext,
    _out: *mut NativeArray<u8>,
    path: PathUtf16,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.getxattrUtf16")).boxed())
}

/// Reject unsupported fs lgetxattr bytes.
pub(crate) unsafe fn destack_fs_lgetxattr_bytes(
    context: &RuntimeCallContext,
    _out: *mut NativeArray<u8>,
    path: PathBytes,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lgetxattrBytes")).boxed())
}

/// Reject unsupported fs lgetxattr utf16.
pub(crate) unsafe fn destack_fs_lgetxattr_utf16(
    context: &RuntimeCallContext,
    _out: *mut NativeArray<u8>,
    path: PathUtf16,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lgetxattrUtf16")).boxed())
}

/// Reject unsupported fs fgetxattr.
pub(crate) unsafe fn destack_fs_fgetxattr_handle(
    context: &RuntimeCallContext,
    _out: *mut NativeArray<u8>,
    handle: FileHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, handle, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fgetxattr")).boxed())
}

/// Reject unsupported fs setxattr bytes.
pub(crate) unsafe fn destack_fs_setxattr_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (context, path, name, value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.setxattrBytes")).boxed())
}

/// Reject unsupported fs setxattr utf16.
pub(crate) unsafe fn destack_fs_setxattr_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (context, path, name, value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.setxattrUtf16")).boxed())
}

/// Reject unsupported fs lsetxattr bytes.
pub(crate) unsafe fn destack_fs_lsetxattr_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (context, path, name, value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lsetxattrBytes")).boxed())
}

/// Reject unsupported fs lsetxattr utf16.
pub(crate) unsafe fn destack_fs_lsetxattr_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (context, path, name, value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lsetxattrUtf16")).boxed())
}

/// Reject unsupported fs fsetxattr.
pub(crate) unsafe fn destack_fs_fsetxattr_handle(
    context: &RuntimeCallContext,
    handle: FileHandle,
    name: NativeStringRef,
    value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (context, handle, name, value, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fsetxattr")).boxed())
}

/// Reject unsupported fs listxattr bytes.
pub(crate) unsafe fn destack_fs_listxattr_bytes(
    context: &RuntimeCallContext,
    _out: *mut NativeArray<NativeStringRef>,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.listxattrBytes")).boxed())
}

/// Reject unsupported fs listxattr utf16.
pub(crate) unsafe fn destack_fs_listxattr_utf16(
    context: &RuntimeCallContext,
    _out: *mut NativeArray<NativeStringRef>,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.listxattrUtf16")).boxed())
}

/// Reject unsupported fs llistxattr bytes.
pub(crate) unsafe fn destack_fs_llistxattr_bytes(
    context: &RuntimeCallContext,
    _out: *mut NativeArray<NativeStringRef>,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.llistxattrBytes")).boxed())
}

/// Reject unsupported fs llistxattr utf16.
pub(crate) unsafe fn destack_fs_llistxattr_utf16(
    context: &RuntimeCallContext,
    _out: *mut NativeArray<NativeStringRef>,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.llistxattrUtf16")).boxed())
}

/// Reject unsupported fs flistxattr.
pub(crate) unsafe fn destack_fs_flistxattr_handle(
    context: &RuntimeCallContext,
    _out: *mut NativeArray<NativeStringRef>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.flistxattr")).boxed())
}

/// Reject unsupported fs removexattr bytes.
pub(crate) unsafe fn destack_fs_removexattr_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.removexattrBytes")).boxed())
}

/// Reject unsupported fs removexattr utf16.
pub(crate) unsafe fn destack_fs_removexattr_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.removexattrUtf16")).boxed())
}

/// Reject unsupported fs lremovexattr bytes.
pub(crate) unsafe fn destack_fs_lremovexattr_bytes(
    context: &RuntimeCallContext,
    path: PathBytes,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lremovexattrBytes")).boxed())
}

/// Reject unsupported fs lremovexattr utf16.
pub(crate) unsafe fn destack_fs_lremovexattr_utf16(
    context: &RuntimeCallContext,
    path: PathUtf16,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, path, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lremovexattrUtf16")).boxed())
}

/// Reject unsupported fs fremovexattr.
pub(crate) unsafe fn destack_fs_fremovexattr_handle(
    context: &RuntimeCallContext,
    handle: FileHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, handle, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fremovexattr")).boxed())
}
