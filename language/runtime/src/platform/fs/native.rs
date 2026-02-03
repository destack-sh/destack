use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{
    AccessMode, AtFlags, Dirent, FileLockFlags, FileMode, FileOffset, OpenFlags, Stat, StatFs,
};
use crate::platform::resource::{DirectoryHandle, FileHandle};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Stub for destack.fs.access.
pub unsafe fn destack_fs_access(
    context: &RuntimeCallContext,
    path: NativeStringRef,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let _ = (context, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.access")).boxed())
}

/// Stub for destack.fs.chmod.
pub unsafe fn destack_fs_chmod(
    context: &RuntimeCallContext,
    path: NativeStringRef,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chmod")).boxed())
}

/// Stub for destack.fs.chown.
pub unsafe fn destack_fs_chown(
    context: &RuntimeCallContext,
    path: NativeStringRef,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (context, path, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.chown")).boxed())
}

/// Stub for destack.fs.close.
pub unsafe fn destack_fs_close(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.close")).boxed())
}

/// Stub for destack.fs.closedir.
pub unsafe fn destack_fs_closedir(
    context: &RuntimeCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.closedir")).boxed())
}

/// Stub for destack.fs.copyfile.
pub unsafe fn destack_fs_copyfile(
    context: &RuntimeCallContext,
    from: NativeStringRef,
    to: NativeStringRef,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = (context, from, to, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.copyfile")).boxed())
}

/// Stub for destack.fs.fchmod.
pub unsafe fn destack_fs_fchmod(
    context: &RuntimeCallContext,
    handle: FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, handle, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchmod")).boxed())
}

/// Stub for destack.fs.fchown.
pub unsafe fn destack_fs_fchown(
    context: &RuntimeCallContext,
    handle: FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (context, handle, uid, gid);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchown")).boxed())
}

/// Stub for destack.fs.fdatasync.
pub unsafe fn destack_fs_fdatasync(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fdatasync")).boxed())
}

/// Stub for destack.fs.fstat.
pub unsafe fn destack_fs_fstat(
    context: &RuntimeCallContext,
    out: *mut Stat,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fstat")).boxed())
}

/// Stub for destack.fs.fstatfs.
pub unsafe fn destack_fs_fstatfs(
    context: &RuntimeCallContext,
    out: *mut StatFs,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fstatfs")).boxed())
}

/// Stub for destack.fs.fsync.
pub unsafe fn destack_fs_fsync(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fsync")).boxed())
}

/// Stub for destack.fs.ftruncate.
pub unsafe fn destack_fs_ftruncate(
    context: &RuntimeCallContext,
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, handle, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.ftruncate")).boxed())
}

/// Stub for destack.fs.futimes.
pub unsafe fn destack_fs_futimes(
    context: &RuntimeCallContext,
    handle: FileHandle,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, handle, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.futimes")).boxed())
}

/// Stub for destack.fs.link.
pub unsafe fn destack_fs_link(
    context: &RuntimeCallContext,
    existingpath: NativeStringRef,
    newpath: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, existingpath, newpath);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.link")).boxed())
}

/// Stub for destack.fs.lstat.
pub unsafe fn destack_fs_lstat(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lstat")).boxed())
}

/// Stub for destack.fs.lutimes.
pub unsafe fn destack_fs_lutimes(
    context: &RuntimeCallContext,
    path: NativeStringRef,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lutimes")).boxed())
}

/// Stub for destack.fs.mkdir.
pub unsafe fn destack_fs_mkdir(
    context: &RuntimeCallContext,
    path: NativeStringRef,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdir")).boxed())
}

/// Stub for destack.fs.mkdtemp.
pub unsafe fn destack_fs_mkdtemp(
    context: &RuntimeCallContext,
    out: *mut NativeStringRef,
    template: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, out, template);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdtemp")).boxed())
}

/// Stub for destack.fs.open.
pub unsafe fn destack_fs_open(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    path: NativeStringRef,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, out, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.open")).boxed())
}

/// Stub for destack.fs.opendir.
pub unsafe fn destack_fs_opendir(
    context: &RuntimeCallContext,
    out: *mut DirectoryHandle,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.opendir")).boxed())
}

/// Stub for destack.fs.read.
pub unsafe fn destack_fs_read(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffer, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.read")).boxed())
}

/// Stub for destack.fs.readdir.
pub unsafe fn destack_fs_readdir(
    context: &RuntimeCallContext,
    out: *mut NativeArray<Dirent>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    let _ = (context, out, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readdir")).boxed())
}

/// Stub for destack.fs.readlink.
pub unsafe fn destack_fs_readlink(
    context: &RuntimeCallContext,
    out: *mut NativeStringRef,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlink")).boxed())
}

/// Stub for destack.fs.readv.
pub unsafe fn destack_fs_readv(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffers, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readv")).boxed())
}

/// Stub for destack.fs.realpath.
pub unsafe fn destack_fs_realpath(
    context: &RuntimeCallContext,
    out: *mut NativeStringRef,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.realpath")).boxed())
}

/// Stub for destack.fs.rename.
pub unsafe fn destack_fs_rename(
    context: &RuntimeCallContext,
    from: NativeStringRef,
    to: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, from, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.rename")).boxed())
}

/// Stub for destack.fs.rmdir.
pub unsafe fn destack_fs_rmdir(
    context: &RuntimeCallContext,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.rmdir")).boxed())
}

/// Stub for destack.fs.stat.
pub unsafe fn destack_fs_stat(
    context: &RuntimeCallContext,
    out: *mut Stat,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat")).boxed())
}

/// Stub for destack.fs.statfs.
pub unsafe fn destack_fs_statfs(
    context: &RuntimeCallContext,
    out: *mut StatFs,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, out, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statfs")).boxed())
}

/// Stub for destack.fs.symlink.
pub unsafe fn destack_fs_symlink(
    context: &RuntimeCallContext,
    target: NativeStringRef,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, target, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlink")).boxed())
}

/// Stub for destack.fs.truncate.
pub unsafe fn destack_fs_truncate(
    context: &RuntimeCallContext,
    path: NativeStringRef,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, path, size);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.truncate")).boxed())
}

/// Stub for destack.fs.unlink.
pub unsafe fn destack_fs_unlink(
    context: &RuntimeCallContext,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlink")).boxed())
}

/// Stub for destack.fs.utimes.
pub unsafe fn destack_fs_utimes(
    context: &RuntimeCallContext,
    path: NativeStringRef,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (context, path, atimens, mtimens);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.utimes")).boxed())
}

/// Stub for destack.fs.write.
pub unsafe fn destack_fs_write(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffer, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.write")).boxed())
}

/// Stub for destack.fs.writev.
pub unsafe fn destack_fs_writev(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (context, out, handle, buffers, offset);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.writev")).boxed())
}

/// Stub for destack.fs.openat.
pub unsafe fn destack_fs_openat(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: NativeStringRef,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path, flags, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openat")).boxed())
}

/// Stub for destack.fs.mkdirat.
pub unsafe fn destack_fs_mkdirat(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: NativeStringRef,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, mode);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mkdirat")).boxed())
}

/// Stub for destack.fs.renameat.
pub unsafe fn destack_fs_renameat(
    context: &RuntimeCallContext,
    from_dir: DirectoryHandle,
    from: NativeStringRef,
    to_dir: DirectoryHandle,
    to: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, from_dir, from, to_dir, to);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.renameat")).boxed())
}

/// Stub for destack.fs.unlinkat.
pub unsafe fn destack_fs_unlinkat(
    context: &RuntimeCallContext,
    dir: DirectoryHandle,
    path: NativeStringRef,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.unlinkat")).boxed())
}

/// Stub for destack.fs.linkat.
pub unsafe fn destack_fs_linkat(
    context: &RuntimeCallContext,
    existing_dir: DirectoryHandle,
    existing_path: NativeStringRef,
    new_dir: DirectoryHandle,
    new_path: NativeStringRef,
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
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.linkat")).boxed())
}

/// Stub for destack.fs.symlinkat.
pub unsafe fn destack_fs_symlinkat(
    context: &RuntimeCallContext,
    target: NativeStringRef,
    dir: DirectoryHandle,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, target, dir, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.symlinkat")).boxed())
}

/// Stub for destack.fs.readlinkat.
pub unsafe fn destack_fs_readlinkat(
    context: &RuntimeCallContext,
    out: *mut NativeStringRef,
    dir: DirectoryHandle,
    path: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.readlinkat")).boxed())
}

/// Stub for destack.fs.statat.
pub unsafe fn destack_fs_statat(
    context: &RuntimeCallContext,
    out: *mut Stat,
    dir: DirectoryHandle,
    path: NativeStringRef,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (context, out, dir, path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.statat")).boxed())
}

/// Stub for destack.fs.lock.
pub unsafe fn destack_fs_lock(
    context: &RuntimeCallContext,
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    let _ = (context, handle, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.lock")).boxed())
}
