#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::RuntimeError;
use crate::platform::resource::{DirectoryHandle, FileHandle};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError, RuntimeStatus};

use crate::platform::fs::{
    AccessMode, AtFlags, Dirent, FileLockFlags, FileMode, FileOffset, OpenFlags, Stat, StatFs,
};

/// Stub for destack.fs.access.
#[unsafe(export_name = "destack.fs.access")]
pub unsafe extern "C" fn destack_fs_access(
    path: NativeStringRef,
    mode: AccessMode,
) -> RuntimeStatus {
    let _ = (path, mode);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.access")).boxed(),
        None,
    )
}

/// Stub for destack.fs.chmod.
#[unsafe(export_name = "destack.fs.chmod")]
pub unsafe extern "C" fn destack_fs_chmod(path: NativeStringRef, mode: FileMode) -> RuntimeStatus {
    let _ = (path, mode);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.chmod")).boxed(),
        None,
    )
}

/// Stub for destack.fs.chown.
#[unsafe(export_name = "destack.fs.chown")]
pub unsafe extern "C" fn destack_fs_chown(
    path: NativeStringRef,
    uid: u32,
    gid: u32,
) -> RuntimeStatus {
    let _ = (path, uid, gid);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.chown")).boxed(),
        None,
    )
}

/// Stub for destack.fs.close.
#[unsafe(export_name = "destack.fs.close")]
pub unsafe extern "C" fn destack_fs_close(handle: FileHandle) -> RuntimeStatus {
    let _ = handle;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.close")).boxed(),
        None,
    )
}

/// Stub for destack.fs.closedir.
#[unsafe(export_name = "destack.fs.closedir")]
pub unsafe extern "C" fn destack_fs_closedir(handle: DirectoryHandle) -> RuntimeStatus {
    let _ = handle;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.closedir")).boxed(),
        None,
    )
}

/// Stub for destack.fs.copyfile.
#[unsafe(export_name = "destack.fs.copyfile")]
pub unsafe extern "C" fn destack_fs_copyfile(
    from: NativeStringRef,
    to: NativeStringRef,
    flags: u32,
) -> RuntimeStatus {
    let _ = (from, to, flags);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.copyfile")).boxed(),
        None,
    )
}

/// Stub for destack.fs.fchmod.
#[unsafe(export_name = "destack.fs.fchmod")]
pub unsafe extern "C" fn destack_fs_fchmod(handle: FileHandle, mode: FileMode) -> RuntimeStatus {
    let _ = (handle, mode);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.fchmod")).boxed(),
        None,
    )
}

/// Stub for destack.fs.fchown.
#[unsafe(export_name = "destack.fs.fchown")]
pub unsafe extern "C" fn destack_fs_fchown(
    handle: FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeStatus {
    let _ = (handle, uid, gid);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.fchown")).boxed(),
        None,
    )
}

/// Stub for destack.fs.fdatasync.
#[unsafe(export_name = "destack.fs.fdatasync")]
pub unsafe extern "C" fn destack_fs_fdatasync(handle: FileHandle) -> RuntimeStatus {
    let _ = handle;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.fdatasync")).boxed(),
        None,
    )
}

/// Stub for destack.fs.fstat.
#[unsafe(export_name = "destack.fs.fstat")]
pub unsafe extern "C" fn destack_fs_fstat(out: *mut Stat, handle: FileHandle) -> RuntimeStatus {
    let _ = (out, handle);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.fstat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.fstatfs.
#[unsafe(export_name = "destack.fs.fstatfs")]
pub unsafe extern "C" fn destack_fs_fstatfs(out: *mut StatFs, handle: FileHandle) -> RuntimeStatus {
    let _ = (out, handle);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.fstatfs")).boxed(),
        None,
    )
}

/// Stub for destack.fs.fsync.
#[unsafe(export_name = "destack.fs.fsync")]
pub unsafe extern "C" fn destack_fs_fsync(handle: FileHandle) -> RuntimeStatus {
    let _ = handle;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.fsync")).boxed(),
        None,
    )
}

/// Stub for destack.fs.ftruncate.
#[unsafe(export_name = "destack.fs.ftruncate")]
pub unsafe extern "C" fn destack_fs_ftruncate(
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeStatus {
    let _ = (handle, size);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.ftruncate")).boxed(),
        None,
    )
}

/// Stub for destack.fs.futimes.
#[unsafe(export_name = "destack.fs.futimes")]
pub unsafe extern "C" fn destack_fs_futimes(
    handle: FileHandle,
    atimens: u64,
    mtimens: u64,
) -> RuntimeStatus {
    let _ = (handle, atimens, mtimens);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.futimes")).boxed(),
        None,
    )
}

/// Stub for destack.fs.link.
#[unsafe(export_name = "destack.fs.link")]
pub unsafe extern "C" fn destack_fs_link(
    existingpath: NativeStringRef,
    newpath: NativeStringRef,
) -> RuntimeStatus {
    let _ = (existingpath, newpath);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.link")).boxed(),
        None,
    )
}

/// Stub for destack.fs.lstat.
#[unsafe(export_name = "destack.fs.lstat")]
pub unsafe extern "C" fn destack_fs_lstat(out: *mut Stat, path: NativeStringRef) -> RuntimeStatus {
    let _ = (out, path);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.lstat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.lutimes.
#[unsafe(export_name = "destack.fs.lutimes")]
pub unsafe extern "C" fn destack_fs_lutimes(
    path: NativeStringRef,
    atimens: u64,
    mtimens: u64,
) -> RuntimeStatus {
    let _ = (path, atimens, mtimens);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.lutimes")).boxed(),
        None,
    )
}

/// Stub for destack.fs.mkdir.
#[unsafe(export_name = "destack.fs.mkdir")]
pub unsafe extern "C" fn destack_fs_mkdir(path: NativeStringRef, mode: FileMode) -> RuntimeStatus {
    let _ = (path, mode);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.mkdir")).boxed(),
        None,
    )
}

/// Stub for destack.fs.mkdtemp.
#[unsafe(export_name = "destack.fs.mkdtemp")]
pub unsafe extern "C" fn destack_fs_mkdtemp(
    out: *mut NativeStringRef,
    template: NativeStringRef,
) -> RuntimeStatus {
    let _ = (out, template);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.mkdtemp")).boxed(),
        None,
    )
}

/// Stub for destack.fs.open.
#[unsafe(export_name = "destack.fs.open")]
pub unsafe extern "C" fn destack_fs_open(
    out: *mut FileHandle,
    path: NativeStringRef,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeStatus {
    let _ = (out, path, flags, mode);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.open")).boxed(),
        None,
    )
}

/// Stub for destack.fs.opendir.
#[unsafe(export_name = "destack.fs.opendir")]
pub unsafe extern "C" fn destack_fs_opendir(
    out: *mut DirectoryHandle,
    path: NativeStringRef,
) -> RuntimeStatus {
    let _ = (out, path);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.opendir")).boxed(),
        None,
    )
}

/// Stub for destack.fs.read.
#[unsafe(export_name = "destack.fs.read")]
pub unsafe extern "C" fn destack_fs_read(
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeStatus {
    let _ = (out, handle, buffer, offset);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.read")).boxed(),
        None,
    )
}

/// Stub for destack.fs.readdir.
#[unsafe(export_name = "destack.fs.readdir")]
pub unsafe extern "C" fn destack_fs_readdir(
    out: *mut NativeArray<Dirent>,
    handle: DirectoryHandle,
) -> RuntimeStatus {
    let _ = (out, handle);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.readdir")).boxed(),
        None,
    )
}

/// Stub for destack.fs.readlink.
#[unsafe(export_name = "destack.fs.readlink")]
pub unsafe extern "C" fn destack_fs_readlink(
    out: *mut NativeStringRef,
    path: NativeStringRef,
) -> RuntimeStatus {
    let _ = (out, path);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.readlink")).boxed(),
        None,
    )
}

/// Stub for destack.fs.readv.
#[unsafe(export_name = "destack.fs.readv")]
pub unsafe extern "C" fn destack_fs_readv(
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeStatus {
    let _ = (out, handle, buffers, offset);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.readv")).boxed(),
        None,
    )
}

/// Stub for destack.fs.realpath.
#[unsafe(export_name = "destack.fs.realpath")]
pub unsafe extern "C" fn destack_fs_realpath(
    out: *mut NativeStringRef,
    path: NativeStringRef,
) -> RuntimeStatus {
    let _ = (out, path);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.realpath")).boxed(),
        None,
    )
}

/// Stub for destack.fs.rename.
#[unsafe(export_name = "destack.fs.rename")]
pub unsafe extern "C" fn destack_fs_rename(
    from: NativeStringRef,
    to: NativeStringRef,
) -> RuntimeStatus {
    let _ = (from, to);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.rename")).boxed(),
        None,
    )
}

/// Stub for destack.fs.rmdir.
#[unsafe(export_name = "destack.fs.rmdir")]
pub unsafe extern "C" fn destack_fs_rmdir(path: NativeStringRef) -> RuntimeStatus {
    let _ = path;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.rmdir")).boxed(),
        None,
    )
}

/// Stub for destack.fs.stat.
#[unsafe(export_name = "destack.fs.stat")]
pub unsafe extern "C" fn destack_fs_stat(out: *mut Stat, path: NativeStringRef) -> RuntimeStatus {
    let _ = (out, path);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.stat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.statfs.
#[unsafe(export_name = "destack.fs.statfs")]
pub unsafe extern "C" fn destack_fs_statfs(
    out: *mut StatFs,
    path: NativeStringRef,
) -> RuntimeStatus {
    let _ = (out, path);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.statfs")).boxed(),
        None,
    )
}

/// Stub for destack.fs.symlink.
#[unsafe(export_name = "destack.fs.symlink")]
pub unsafe extern "C" fn destack_fs_symlink(
    target: NativeStringRef,
    path: NativeStringRef,
) -> RuntimeStatus {
    let _ = (target, path);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.symlink")).boxed(),
        None,
    )
}

/// Stub for destack.fs.truncate.
#[unsafe(export_name = "destack.fs.truncate")]
pub unsafe extern "C" fn destack_fs_truncate(
    path: NativeStringRef,
    size: FileOffset,
) -> RuntimeStatus {
    let _ = (path, size);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.truncate")).boxed(),
        None,
    )
}

/// Stub for destack.fs.unlink.
#[unsafe(export_name = "destack.fs.unlink")]
pub unsafe extern "C" fn destack_fs_unlink(path: NativeStringRef) -> RuntimeStatus {
    let _ = path;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.unlink")).boxed(),
        None,
    )
}

/// Stub for destack.fs.utimes.
#[unsafe(export_name = "destack.fs.utimes")]
pub unsafe extern "C" fn destack_fs_utimes(
    path: NativeStringRef,
    atimens: u64,
    mtimens: u64,
) -> RuntimeStatus {
    let _ = (path, atimens, mtimens);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.utimes")).boxed(),
        None,
    )
}

/// Stub for destack.fs.write.
#[unsafe(export_name = "destack.fs.write")]
pub unsafe extern "C" fn destack_fs_write(
    out: *mut u64,
    handle: FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeStatus {
    let _ = (out, handle, buffer, offset);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.write")).boxed(),
        None,
    )
}

/// Stub for destack.fs.writev.
#[unsafe(export_name = "destack.fs.writev")]
pub unsafe extern "C" fn destack_fs_writev(
    out: *mut u64,
    handle: FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeStatus {
    let _ = (out, handle, buffers, offset);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.writev")).boxed(),
        None,
    )
}

/// Stub for destack.fs.openat.
#[unsafe(export_name = "destack.fs.openat")]
pub unsafe extern "C" fn destack_fs_openat(
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: NativeStringRef,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeStatus {
    let _ = (out, dir, path, flags, mode);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.openat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.mkdirat.
#[unsafe(export_name = "destack.fs.mkdirat")]
pub unsafe extern "C" fn destack_fs_mkdirat(
    dir: DirectoryHandle,
    path: NativeStringRef,
    mode: FileMode,
) -> RuntimeStatus {
    let _ = (dir, path, mode);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.mkdirat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.renameat.
#[unsafe(export_name = "destack.fs.renameat")]
pub unsafe extern "C" fn destack_fs_renameat(
    from_dir: DirectoryHandle,
    from: NativeStringRef,
    to_dir: DirectoryHandle,
    to: NativeStringRef,
) -> RuntimeStatus {
    let _ = (from_dir, from, to_dir, to);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.renameat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.unlinkat.
#[unsafe(export_name = "destack.fs.unlinkat")]
pub unsafe extern "C" fn destack_fs_unlinkat(
    dir: DirectoryHandle,
    path: NativeStringRef,
    flags: AtFlags,
) -> RuntimeStatus {
    let _ = (dir, path, flags);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.unlinkat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.linkat.
#[unsafe(export_name = "destack.fs.linkat")]
pub unsafe extern "C" fn destack_fs_linkat(
    existing_dir: DirectoryHandle,
    existing_path: NativeStringRef,
    new_dir: DirectoryHandle,
    new_path: NativeStringRef,
    flags: AtFlags,
) -> RuntimeStatus {
    let _ = (existing_dir, existing_path, new_dir, new_path, flags);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.linkat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.symlinkat.
#[unsafe(export_name = "destack.fs.symlinkat")]
pub unsafe extern "C" fn destack_fs_symlinkat(
    target: NativeStringRef,
    dir: DirectoryHandle,
    path: NativeStringRef,
) -> RuntimeStatus {
    let _ = (target, dir, path);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.symlinkat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.readlinkat.
#[unsafe(export_name = "destack.fs.readlinkat")]
pub unsafe extern "C" fn destack_fs_readlinkat(
    out: *mut NativeStringRef,
    dir: DirectoryHandle,
    path: NativeStringRef,
) -> RuntimeStatus {
    let _ = (out, dir, path);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.readlinkat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.statat.
#[unsafe(export_name = "destack.fs.statat")]
pub unsafe extern "C" fn destack_fs_statat(
    out: *mut Stat,
    dir: DirectoryHandle,
    path: NativeStringRef,
    flags: AtFlags,
) -> RuntimeStatus {
    let _ = (out, dir, path, flags);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.statat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.lock.
#[unsafe(export_name = "destack.fs.lock")]
pub unsafe extern "C" fn destack_fs_lock(
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeStatus {
    let _ = (handle, flags);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.lock")).boxed(),
        None,
    )
}
