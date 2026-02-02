#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::RuntimeError;
use crate::platform::{
    PlatformArray, PlatformError, PlatformSlice, PlatformStringRef, RuntimeStatus,
};

use crate::platform::fs::{AccessMode, Dirent, FileMode, FileOffset, OpenFlags, Stat, StatFs};

/// Stub for destack.fs.access.
#[unsafe(export_name = "destack.fs.access")]
pub unsafe extern "C" fn destack_fs_access(
    path: PlatformStringRef,
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
pub unsafe extern "C" fn destack_fs_chmod(
    path: PlatformStringRef,
    mode: FileMode,
) -> RuntimeStatus {
    let _ = (path, mode);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.chmod")).boxed(),
        None,
    )
}

/// Stub for destack.fs.chown.
#[unsafe(export_name = "destack.fs.chown")]
pub unsafe extern "C" fn destack_fs_chown(
    path: PlatformStringRef,
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
pub unsafe extern "C" fn destack_fs_close(
    handle: crate::platform::resource::FileHandle,
) -> RuntimeStatus {
    let _ = handle;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.close")).boxed(),
        None,
    )
}

/// Stub for destack.fs.closedir.
#[unsafe(export_name = "destack.fs.closedir")]
pub unsafe extern "C" fn destack_fs_closedir(
    handle: crate::platform::resource::DirectoryHandle,
) -> RuntimeStatus {
    let _ = handle;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.closedir")).boxed(),
        None,
    )
}

/// Stub for destack.fs.copyfile.
#[unsafe(export_name = "destack.fs.copyfile")]
pub unsafe extern "C" fn destack_fs_copyfile(
    from: PlatformStringRef,
    to: PlatformStringRef,
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
pub unsafe extern "C" fn destack_fs_fchmod(
    handle: crate::platform::resource::FileHandle,
    mode: FileMode,
) -> RuntimeStatus {
    let _ = (handle, mode);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.fchmod")).boxed(),
        None,
    )
}

/// Stub for destack.fs.fchown.
#[unsafe(export_name = "destack.fs.fchown")]
pub unsafe extern "C" fn destack_fs_fchown(
    handle: crate::platform::resource::FileHandle,
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
pub unsafe extern "C" fn destack_fs_fdatasync(
    handle: crate::platform::resource::FileHandle,
) -> RuntimeStatus {
    let _ = handle;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.fdatasync")).boxed(),
        None,
    )
}

/// Stub for destack.fs.fstat.
#[unsafe(export_name = "destack.fs.fstat")]
pub unsafe extern "C" fn destack_fs_fstat(
    out: *mut Stat,
    handle: crate::platform::resource::FileHandle,
) -> RuntimeStatus {
    let _ = (out, handle);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.fstat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.fstatfs.
#[unsafe(export_name = "destack.fs.fstatfs")]
pub unsafe extern "C" fn destack_fs_fstatfs(
    out: *mut StatFs,
    handle: crate::platform::resource::FileHandle,
) -> RuntimeStatus {
    let _ = (out, handle);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.fstatfs")).boxed(),
        None,
    )
}

/// Stub for destack.fs.fsync.
#[unsafe(export_name = "destack.fs.fsync")]
pub unsafe extern "C" fn destack_fs_fsync(
    handle: crate::platform::resource::FileHandle,
) -> RuntimeStatus {
    let _ = handle;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.fsync")).boxed(),
        None,
    )
}

/// Stub for destack.fs.ftruncate.
#[unsafe(export_name = "destack.fs.ftruncate")]
pub unsafe extern "C" fn destack_fs_ftruncate(
    handle: crate::platform::resource::FileHandle,
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
    handle: crate::platform::resource::FileHandle,
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
    existingpath: PlatformStringRef,
    newpath: PlatformStringRef,
) -> RuntimeStatus {
    let _ = (existingpath, newpath);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.link")).boxed(),
        None,
    )
}

/// Stub for destack.fs.lstat.
#[unsafe(export_name = "destack.fs.lstat")]
pub unsafe extern "C" fn destack_fs_lstat(
    out: *mut Stat,
    path: PlatformStringRef,
) -> RuntimeStatus {
    let _ = (out, path);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.lstat")).boxed(),
        None,
    )
}

/// Stub for destack.fs.lutimes.
#[unsafe(export_name = "destack.fs.lutimes")]
pub unsafe extern "C" fn destack_fs_lutimes(
    path: PlatformStringRef,
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
pub unsafe extern "C" fn destack_fs_mkdir(
    path: PlatformStringRef,
    mode: FileMode,
) -> RuntimeStatus {
    let _ = (path, mode);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.mkdir")).boxed(),
        None,
    )
}

/// Stub for destack.fs.mkdtemp.
#[unsafe(export_name = "destack.fs.mkdtemp")]
pub unsafe extern "C" fn destack_fs_mkdtemp(
    out: *mut PlatformStringRef,
    template: PlatformStringRef,
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
    out: *mut crate::platform::resource::FileHandle,
    path: PlatformStringRef,
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
    out: *mut crate::platform::resource::DirectoryHandle,
    path: PlatformStringRef,
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
    handle: crate::platform::resource::FileHandle,
    buffer: PlatformSlice<u8>,
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
    out: *mut PlatformArray<Dirent>,
    handle: crate::platform::resource::DirectoryHandle,
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
    out: *mut PlatformStringRef,
    path: PlatformStringRef,
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
    handle: crate::platform::resource::FileHandle,
    buffers: PlatformSlice<PlatformSlice<u8>>,
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
    out: *mut PlatformStringRef,
    path: PlatformStringRef,
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
    from: PlatformStringRef,
    to: PlatformStringRef,
) -> RuntimeStatus {
    let _ = (from, to);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.rename")).boxed(),
        None,
    )
}

/// Stub for destack.fs.rmdir.
#[unsafe(export_name = "destack.fs.rmdir")]
pub unsafe extern "C" fn destack_fs_rmdir(path: PlatformStringRef) -> RuntimeStatus {
    let _ = path;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.rmdir")).boxed(),
        None,
    )
}

/// Stub for destack.fs.stat.
#[unsafe(export_name = "destack.fs.stat")]
pub unsafe extern "C" fn destack_fs_stat(out: *mut Stat, path: PlatformStringRef) -> RuntimeStatus {
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
    path: PlatformStringRef,
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
    target: PlatformStringRef,
    path: PlatformStringRef,
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
    path: PlatformStringRef,
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
pub unsafe extern "C" fn destack_fs_unlink(path: PlatformStringRef) -> RuntimeStatus {
    let _ = path;
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.unlink")).boxed(),
        None,
    )
}

/// Stub for destack.fs.utimes.
#[unsafe(export_name = "destack.fs.utimes")]
pub unsafe extern "C" fn destack_fs_utimes(
    path: PlatformStringRef,
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
    handle: crate::platform::resource::FileHandle,
    buffer: PlatformSlice<u8>,
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
    handle: crate::platform::resource::FileHandle,
    buffers: PlatformSlice<PlatformSlice<u8>>,
    offset: FileOffset,
) -> RuntimeStatus {
    let _ = (out, handle, buffers, offset);
    RuntimeStatus::from_error(
        RuntimeError::platform(PlatformError::not_supported("destack.fs.writev")).boxed(),
        None,
    )
}
