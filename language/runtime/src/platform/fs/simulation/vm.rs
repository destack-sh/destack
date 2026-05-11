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
pub(crate) fn destack_fs_closedir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::DirectoryHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.closedir")).boxed())
}

/// Resolve the directory descriptor for an open directory handle.
pub(crate) fn destack_fs_dirfd(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::DirectoryHandle,
) -> RuntimeResult<resource::FileHandle> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.dirfd")).boxed())
}

/// Create a directory.
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
pub(crate) fn destack_fs_mkdtemp(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _template: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.mkdtemp")).boxed())
}

/// Open a directory and return a handle.
pub(crate) fn destack_fs_opendir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<resource::DirectoryHandle> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.opendir")).boxed())
}

/// Read directory entries from an open directory handle.
pub(crate) fn destack_fs_readdir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::DirectoryHandle,
) -> RuntimeResult<VmArray<DirentVm>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.readdir")).boxed())
}

/// Read a single directory entry from an open directory handle.
pub(crate) fn destack_fs_readdir_next(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::DirectoryHandle,
) -> RuntimeResult<DirentNextVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.readdirNext")).boxed())
}

/// Reset an open directory handle to the first entry.
pub(crate) fn destack_fs_rewinddir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::DirectoryHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.rewinddir")).boxed())
}

/// Remove a directory.
pub(crate) fn destack_fs_rmdir(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.rmdir")).boxed())
}

/// Close an open file handle.
pub(crate) fn destack_fs_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.close")).boxed())
}

/// Copy a range between file descriptors.
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
pub(crate) fn destack_fs_dup(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<resource::FileHandle> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.dup")).boxed())
}

/// Duplicate a file handle to a specific target.
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
pub(crate) fn destack_fs_fdatasync(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fdatasync")).boxed())
}

/// Synchronize a file's in-core state with storage.
pub(crate) fn destack_fs_fsync(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fsync")).boxed())
}

/// Truncate a file by handle.
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
pub(crate) fn destack_fs_get_fd_flags(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<FdFlags> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.getFdFlags")).boxed())
}

/// Read file status flags.
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
pub(crate) fn destack_fs_syncfs(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.syncfs")).boxed())
}

/// Duplicate bytes from one pipe to another without consuming source bytes.
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
pub(crate) fn destack_fs_munmap(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _mapping: VmSlice<u8>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmap.munmap")).boxed())
}

/// Create an anonymous memory mapping.
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
pub(crate) fn destack_fs_readlink(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.readlink")).boxed())
}

/// Read a symbolic link relative to a directory handle.
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
pub(crate) fn destack_fs_realpath(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.realpath")).boxed())
}

/// Rename or move a file.
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
pub(crate) fn destack_fs_unlink(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.unlink")).boxed())
}

/// Unlink a file relative to a directory handle.
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
pub(crate) fn destack_fs_fstat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<StatVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.fstat")).boxed())
}

/// Stat a filesystem by handle.
pub(crate) fn destack_fs_fstatfs(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<StatFsVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.fstatfs")).boxed())
}

/// Stat a file without following symlinks.
pub(crate) fn destack_fs_lstat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<StatVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.lstat")).boxed())
}

/// Stat a file.
pub(crate) fn destack_fs_stat(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<StatVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.path")).boxed())
}

/// Stat a file relative to a directory handle.
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
pub(crate) fn destack_fs_statfs(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<StatFsVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.pathfs")).boxed())
}

/// Stat a path with statx semantics.
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
pub(crate) fn destack_fs_watch_close(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::WatchHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchClose")).boxed())
}

/// Read a batch of events from a watch handle.
pub(crate) fn destack_fs_watch_read(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::WatchHandle,
) -> RuntimeResult<WatchBatchVm> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchRead")).boxed())
}

/// Start watching a path relative to a directory handle.
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
pub(crate) fn destack_fs_flistxattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _handle: resource::FileHandle,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.flistxattr")).boxed())
}

/// List extended attribute names by handle as raw byte payloads.
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
pub(crate) fn destack_fs_listxattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.listxattr")).boxed())
}

/// List extended attribute names by path as raw byte payloads.
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
pub(crate) fn destack_fs_llistxattr(
    _binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    _path: OsPathVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.llistxattr")).boxed())
}

/// List extended attribute names without following symlinks as raw byte payloads.
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
