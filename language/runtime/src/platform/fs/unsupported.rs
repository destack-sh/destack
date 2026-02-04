use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{
    AccessMode, AtFlags, FileLockFlags, FileMode, FileOffset, OpenFlags, PathBytes, PathUtf16,
    Stat, StatFs, SymlinkType,
};
use crate::platform::resource::{DirectoryHandle, FileHandle};
use crate::platform::{NativeArray, NativeSlice, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Reject unsupported fs access bytes.
pub(crate) unsafe fn destack_fs_access_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathBytes,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let _ = (context, out, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.accessBytes")).boxed())
}

/// Reject unsupported fs access utf16.
pub(crate) unsafe fn destack_fs_access_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathUtf16,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let _ = (context, out, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.accessUtf16")).boxed())
}

/// Reject unsupported fs chmod bytes.
pub(crate) unsafe fn destack_fs_chmod_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, out, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chmodBytes")).boxed())
}

/// Reject unsupported fs chmod utf16.
pub(crate) unsafe fn destack_fs_chmod_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, out, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chmodUtf16")).boxed())
}

/// Reject unsupported fs chown bytes.
pub(crate) unsafe fn destack_fs_chown_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathBytes,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (context, out, path, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chownBytes")).boxed())
}

/// Reject unsupported fs chown utf16.
pub(crate) unsafe fn destack_fs_chown_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathUtf16,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (context, out, path, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chownUtf16")).boxed())
}

/// Reject unsupported fs close.
pub(crate) unsafe fn destack_fs_close(
    context: &RuntimeCallContext,
    out: *mut (),
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.close")).boxed())
}

/// Reject unsupported fs closedir.
pub(crate) unsafe fn destack_fs_closedir(
    context: &RuntimeCallContext,
    out: *mut (),
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.closedir")).boxed())
}

/// Reject unsupported fs copyfile bytes.
pub(crate) unsafe fn destack_fs_copyfile_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    from: PathBytes,
    to: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, from, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.copyfileBytes")).boxed())
}

/// Reject unsupported fs copyfile utf16.
pub(crate) unsafe fn destack_fs_copyfile_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    from: PathUtf16,
    to: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, from, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.copyfileUtf16")).boxed())
}

/// Reject unsupported fs fchmod.
pub(crate) unsafe fn destack_fs_fchmod(
    context: &RuntimeCallContext,
    out: *mut (),
    handle: FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchmod")).boxed())
}

/// Reject unsupported fs fchown.
pub(crate) unsafe fn destack_fs_fchown(
    context: &RuntimeCallContext,
    out: *mut (),
    handle: FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchown")).boxed())
}

/// Reject unsupported fs fdatasync.
pub(crate) unsafe fn destack_fs_fdatasync(
    context: &RuntimeCallContext,
    out: *mut (),
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fdatasync")).boxed())
}

/// Reject unsupported fs fstat.
pub(crate) unsafe fn destack_fs_fstat(
    context: &RuntimeCallContext,
    out: *mut Stat,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fstat")).boxed())
}

/// Reject unsupported fs fstatfs.
pub(crate) unsafe fn destack_fs_fstatfs(
    context: &RuntimeCallContext,
    out: *mut StatFs,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fstatfs")).boxed())
}

/// Reject unsupported fs fsync.
pub(crate) unsafe fn destack_fs_fsync(
    context: &RuntimeCallContext,
    out: *mut (),
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fsync")).boxed())
}

/// Reject unsupported fs ftruncate.
pub(crate) unsafe fn destack_fs_ftruncate(
    context: &RuntimeCallContext,
    out: *mut (),
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.ftruncate")).boxed())
}

/// Reject unsupported fs futimes.
pub(crate) unsafe fn destack_fs_futimes(
    context: &RuntimeCallContext,
    out: *mut (),
    handle: FileHandle,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.futimes")).boxed())
}

/// Reject unsupported fs link bytes.
pub(crate) unsafe fn destack_fs_link_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    existingpath: PathBytes,
    newpath: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, existingpath, newpath);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkBytes")).boxed())
}

/// Reject unsupported fs link utf16.
pub(crate) unsafe fn destack_fs_link_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    existingpath: PathUtf16,
    newpath: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, existingpath, newpath);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkUtf16")).boxed())
}

/// Reject unsupported fs lstat bytes.
pub(crate) unsafe fn destack_fs_lstat_bytes(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lstatBytes")).boxed())
}

/// Reject unsupported fs lstat utf16.
pub(crate) unsafe fn destack_fs_lstat_utf16(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lstatUtf16")).boxed())
}

/// Reject unsupported fs lutimes bytes.
pub(crate) unsafe fn destack_fs_lutimes_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, out, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lutimesBytes")).boxed())
}

/// Reject unsupported fs lutimes utf16.
pub(crate) unsafe fn destack_fs_lutimes_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, out, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lutimesUtf16")).boxed())
}

/// Reject unsupported fs mkdtemp bytes.
pub(crate) unsafe fn destack_fs_mkdtemp_bytes(
    context: &RuntimeCallContext,
    out: *mut PathBytes,
    template: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, template);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdtempBytes")).boxed())
}

/// Reject unsupported fs mkdtemp utf16.
pub(crate) unsafe fn destack_fs_mkdtemp_utf16(
    context: &RuntimeCallContext,
    out: *mut PathUtf16,
    template: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, template);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdtempUtf16")).boxed())
}

/// Reject unsupported fs open bytes.
pub(crate) unsafe fn destack_fs_open_bytes(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    path: PathBytes,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, out, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openBytes")).boxed())
}

/// Reject unsupported fs open utf16.
pub(crate) unsafe fn destack_fs_open_utf16(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    path: PathUtf16,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, out, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openUtf16")).boxed())
}

/// Reject unsupported fs opendir bytes.
pub(crate) unsafe fn destack_fs_opendir_bytes(
    context: &RuntimeCallContext,
    out: *mut DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.opendirBytes")).boxed())
}

/// Reject unsupported fs opendir utf16.
pub(crate) unsafe fn destack_fs_opendir_utf16(
    context: &RuntimeCallContext,
    out: *mut DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.opendirUtf16")).boxed())
}

/// Reject unsupported fs read.
pub(crate) unsafe fn destack_fs_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffer, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.read")).boxed())
}

/// Reject unsupported fs readdir.
pub(crate) unsafe fn destack_fs_readdir(
    context: &RuntimeCallContext,
    out: *mut NativeArray<crate::platform::fs::Dirent>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readdir")).boxed())
}

/// Reject unsupported fs readlink bytes.
pub(crate) unsafe fn destack_fs_readlink_bytes(
    context: &RuntimeCallContext,
    out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkBytes")).boxed())
}

/// Reject unsupported fs readlink utf16.
pub(crate) unsafe fn destack_fs_readlink_utf16(
    context: &RuntimeCallContext,
    out: *mut PathUtf16,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkUtf16")).boxed())
}

/// Reject unsupported fs readv.
pub(crate) unsafe fn destack_fs_readv(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffers, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readv")).boxed())
}

/// Reject unsupported fs realpath bytes.
pub(crate) unsafe fn destack_fs_realpath_bytes(
    context: &RuntimeCallContext,
    out: *mut PathBytes,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.realpathBytes")).boxed())
}

/// Reject unsupported fs realpath utf16.
pub(crate) unsafe fn destack_fs_realpath_utf16(
    context: &RuntimeCallContext,
    out: *mut PathUtf16,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.realpathUtf16")).boxed())
}

/// Reject unsupported fs rename bytes.
pub(crate) unsafe fn destack_fs_rename_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    from: PathBytes,
    to: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, from, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameBytes")).boxed())
}

/// Reject unsupported fs rename utf16.
pub(crate) unsafe fn destack_fs_rename_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    from: PathUtf16,
    to: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, from, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameUtf16")).boxed())
}

/// Reject unsupported fs rmdir bytes.
pub(crate) unsafe fn destack_fs_rmdir_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.rmdirBytes")).boxed())
}

/// Reject unsupported fs rmdir utf16.
pub(crate) unsafe fn destack_fs_rmdir_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.rmdirUtf16")).boxed())
}

/// Reject unsupported fs stat bytes.
pub(crate) unsafe fn destack_fs_stat_bytes(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statBytes")).boxed())
}

/// Reject unsupported fs stat utf16.
pub(crate) unsafe fn destack_fs_stat_utf16(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statUtf16")).boxed())
}

/// Reject unsupported fs statfs bytes.
pub(crate) unsafe fn destack_fs_statfs_bytes(
    context: &RuntimeCallContext,
    out: *mut StatFs,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statfsBytes")).boxed())
}

/// Reject unsupported fs statfs utf16.
pub(crate) unsafe fn destack_fs_statfs_utf16(
    context: &RuntimeCallContext,
    out: *mut StatFs,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
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
    out: *mut (),
    path: PathBytes,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, out, path, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.truncateBytes")).boxed())
}

/// Reject unsupported fs truncate utf16.
pub(crate) unsafe fn destack_fs_truncate_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathUtf16,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, out, path, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.truncateUtf16")).boxed())
}

/// Reject unsupported fs unlink bytes.
pub(crate) unsafe fn destack_fs_unlink_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkBytes")).boxed())
}

/// Reject unsupported fs unlink utf16.
pub(crate) unsafe fn destack_fs_unlink_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkUtf16")).boxed())
}

/// Reject unsupported fs utimes bytes.
pub(crate) unsafe fn destack_fs_utimes_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathBytes,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, out, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimesBytes")).boxed())
}

/// Reject unsupported fs utimes utf16.
pub(crate) unsafe fn destack_fs_utimes_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    path: PathUtf16,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, out, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimesUtf16")).boxed())
}

/// Reject unsupported fs write.
pub(crate) unsafe fn destack_fs_write(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffer, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.write")).boxed())
}

/// Reject unsupported fs writev.
pub(crate) unsafe fn destack_fs_writev(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffers, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.writev")).boxed())
}

/// Reject unsupported fs openat bytes.
pub(crate) unsafe fn destack_fs_openat_bytes(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openatBytes")).boxed())
}

/// Reject unsupported fs openat utf16.
pub(crate) unsafe fn destack_fs_openat_utf16(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openatUtf16")).boxed())
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
    out: *mut (),
    dir: DirectoryHandle,
    path: PathBytes,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdiratBytes")).boxed())
}

/// Reject unsupported fs mkdirat utf16.
pub(crate) unsafe fn destack_fs_mkdirat_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    dir: DirectoryHandle,
    path: PathUtf16,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdiratUtf16")).boxed())
}

/// Reject unsupported fs renameat bytes.
pub(crate) unsafe fn destack_fs_renameat_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    from_dir: DirectoryHandle,
    from: PathBytes,
    to_dir: DirectoryHandle,
    to: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, from_dir, from, to_dir, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameatBytes")).boxed())
}

/// Reject unsupported fs renameat utf16.
pub(crate) unsafe fn destack_fs_renameat_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    from_dir: DirectoryHandle,
    from: PathUtf16,
    to_dir: DirectoryHandle,
    to: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, from_dir, from, to_dir, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameatUtf16")).boxed())
}

/// Reject unsupported fs unlinkat bytes.
pub(crate) unsafe fn destack_fs_unlinkat_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkatBytes")).boxed())
}

/// Reject unsupported fs unlinkat utf16.
pub(crate) unsafe fn destack_fs_unlinkat_utf16(
    context: &RuntimeCallContext,
    out: *mut (),
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkatUtf16")).boxed())
}

/// Reject unsupported fs linkat bytes.
pub(crate) unsafe fn destack_fs_linkat_bytes(
    context: &RuntimeCallContext,
    out: *mut (),
    existing_dir: DirectoryHandle,
    existing_path: PathBytes,
    new_dir: DirectoryHandle,
    new_path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (
        context,
        out,
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
    out: *mut (),
    existing_dir: DirectoryHandle,
    existing_path: PathUtf16,
    new_dir: DirectoryHandle,
    new_path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (
        context,
        out,
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
    out: *mut PathBytes,
    dir: DirectoryHandle,
    path: PathBytes,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkatBytes")).boxed())
}

/// Reject unsupported fs readlinkat utf16.
pub(crate) unsafe fn destack_fs_readlinkat_utf16(
    context: &RuntimeCallContext,
    out: *mut PathUtf16,
    dir: DirectoryHandle,
    path: PathUtf16,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkatUtf16")).boxed())
}

/// Reject unsupported fs statat bytes.
pub(crate) unsafe fn destack_fs_statat_bytes(
    context: &RuntimeCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statatBytes")).boxed())
}

/// Reject unsupported fs statat utf16.
pub(crate) unsafe fn destack_fs_statat_utf16(
    context: &RuntimeCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statatUtf16")).boxed())
}

/// Reject unsupported fs lock.
pub(crate) unsafe fn destack_fs_lock(
    context: &RuntimeCallContext,
    out: *mut (),
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lock")).boxed())
}
