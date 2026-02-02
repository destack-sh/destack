use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{
    AccessMode, DirentVm, FileMode, FileOffset, OpenFlags, StatFsVm, StatVm,
};
use crate::platform::{PlatformError, VmArray, VmSlice};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.fs.access.
pub(super) fn destack_fs_access(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let _ = (path, mode);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.access is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.chmod.
pub(super) fn destack_fs_chmod(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (path, mode);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.chmod is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.chown.
pub(super) fn destack_fs_chown(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (path, uid, gid);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.chown is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.close.
pub(super) fn destack_fs_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.closedir.
pub(super) fn destack_fs_closedir(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::DirectoryHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.closedir is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.copyfile.
pub(super) fn destack_fs_copyfile(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    from: vm::StringHandle,
    to: vm::StringHandle,
    flags: u32,
) -> RuntimeResult<()> {
    let _ = (from, to, flags);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.copyfile is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.fchmod.
pub(super) fn destack_fs_fchmod(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.fchmod is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.fchown.
pub(super) fn destack_fs_fchown(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (handle, uid, gid);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.fchown is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.fdatasync.
pub(super) fn destack_fs_fdatasync(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.fdatasync is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.fstat.
pub(super) fn destack_fs_fstat(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
) -> RuntimeResult<StatVm> {
    let _ = handle;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.fstat is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.fstatfs.
pub(super) fn destack_fs_fstatfs(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
) -> RuntimeResult<StatFsVm> {
    let _ = handle;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.fstatfs is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.fsync.
pub(super) fn destack_fs_fsync(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.fsync is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.ftruncate.
pub(super) fn destack_fs_ftruncate(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (handle, size);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.ftruncate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.futimes.
pub(super) fn destack_fs_futimes(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (handle, atimens, mtimens);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.futimes is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.link.
pub(super) fn destack_fs_link(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    existingpath: vm::StringHandle,
    newpath: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (existingpath, newpath);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.link is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.lstat.
pub(super) fn destack_fs_lstat(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
) -> RuntimeResult<StatVm> {
    let _ = path;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.lstat is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.lutimes.
pub(super) fn destack_fs_lutimes(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (path, atimens, mtimens);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.lutimes is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.mkdir.
pub(super) fn destack_fs_mkdir(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (path, mode);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.mkdir is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.mkdtemp.
pub(super) fn destack_fs_mkdtemp(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    template: vm::StringHandle,
) -> RuntimeResult<vm::StringHandle> {
    let _ = template;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.mkdtemp is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.open.
pub(super) fn destack_fs_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<crate::platform::resource::FileHandle> {
    let _ = (path, flags, mode);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.opendir.
pub(super) fn destack_fs_opendir(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
) -> RuntimeResult<crate::platform::resource::DirectoryHandle> {
    let _ = path;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.opendir is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.read.
pub(super) fn destack_fs_read(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
    buffer: VmSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer, offset);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.read is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.readdir.
pub(super) fn destack_fs_readdir(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::DirectoryHandle,
) -> RuntimeResult<VmArray<DirentVm>> {
    let _ = handle;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.readdir is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.readlink.
pub(super) fn destack_fs_readlink(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
) -> RuntimeResult<vm::StringHandle> {
    let _ = path;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.readlink is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.readv.
pub(super) fn destack_fs_readv(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers, offset);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.readv is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.realpath.
pub(super) fn destack_fs_realpath(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
) -> RuntimeResult<vm::StringHandle> {
    let _ = path;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.realpath is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.rename.
pub(super) fn destack_fs_rename(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    from: vm::StringHandle,
    to: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (from, to);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.rename is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.rmdir.
pub(super) fn destack_fs_rmdir(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.rmdir is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.stat.
pub(super) fn destack_fs_stat(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
) -> RuntimeResult<StatVm> {
    let _ = path;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.stat is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.statfs.
pub(super) fn destack_fs_statfs(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
) -> RuntimeResult<StatFsVm> {
    let _ = path;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.statfs is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.symlink.
pub(super) fn destack_fs_symlink(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    target: vm::StringHandle,
    path: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (target, path);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.symlink is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.truncate.
pub(super) fn destack_fs_truncate(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (path, size);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.truncate is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.unlink.
pub(super) fn destack_fs_unlink(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.unlink is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.utimes.
pub(super) fn destack_fs_utimes(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: vm::StringHandle,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (path, atimens, mtimens);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.utimes is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.write.
pub(super) fn destack_fs_write(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
    buffer: VmSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let _ = (handle, buffer, offset);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.write is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.fs.writev.
pub(super) fn destack_fs_writev(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let _ = (handle, buffers, offset);
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.fs.writev is not available in the VM yet",
    ))
    .boxed())
}
