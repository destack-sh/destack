#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::{NativeArray, PlatformError};

use crate::runtime::BindingCallContext;

use crate::platform::fs::{
    AccessMode, AllocFlags, AtFlags, CopyFlags, Dirent, DirentNext, FdFlags, FileAdvice,
    FileLockFlags, FileMode, FileOffset, FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags,
    NodeDevice, OpenFlags, OpenOptions, OsPath, ReadWriteFlags, RenameFlags, SeekWhence,
    SpliceCursor, SpliceFlags, Stat, StatFs, StatusFlags, Statx, StatxFlags, StatxMask,
    SymlinkType, SyncFlags, WatchBatch, WatchOptions, XattrFlags,
};
use crate::platform::resource;

/// Check file access permissions.
pub(crate) unsafe fn destack_fs_access(
    _binding: &BindingCallContext,
    path: OsPath,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let _ = (path, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.access")).boxed())
}

/// Check file access permissions relative to a directory handle.
pub(crate) unsafe fn destack_fs_accessat(
    _binding: &BindingCallContext,
    dir: resource::DirectoryHandle,
    path: OsPath,
    mode: AccessMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (dir, path, mode, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.accessat")).boxed())
}

/// Change file permissions.
pub(crate) unsafe fn destack_fs_chmod(
    _binding: &BindingCallContext,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (path, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.chmod")).boxed())
}

/// Change file owner and group.
pub(crate) unsafe fn destack_fs_chown(
    _binding: &BindingCallContext,
    path: OsPath,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (path, uid, gid);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.chown")).boxed())
}

/// Change file permissions by handle.
pub(crate) unsafe fn destack_fs_fchmod(
    _binding: &BindingCallContext,
    handle: resource::FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (handle, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.fchmod")).boxed())
}

/// Change file permissions relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchmodat(
    _binding: &BindingCallContext,
    dir: resource::DirectoryHandle,
    path: OsPath,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (dir, path, mode, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.fchmodat")).boxed())
}

/// Change file owner and group by handle.
pub(crate) unsafe fn destack_fs_fchown(
    _binding: &BindingCallContext,
    handle: resource::FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let _ = (handle, uid, gid);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.fchown")).boxed())
}

/// Change file owner and group relative to a directory handle.
pub(crate) unsafe fn destack_fs_fchownat(
    _binding: &BindingCallContext,
    dir: resource::DirectoryHandle,
    path: OsPath,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (dir, path, uid, gid, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.fchownat")).boxed())
}

/// Update access and modification times by handle.
pub(crate) unsafe fn destack_fs_futimes(
    _binding: &BindingCallContext,
    handle: resource::FileHandle,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (handle, atimens, mtimens);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.futimes")).boxed())
}

/// Update access and modification times without following symlinks.
pub(crate) unsafe fn destack_fs_lutimes(
    _binding: &BindingCallContext,
    path: OsPath,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (path, atimens, mtimens);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.lutimes")).boxed())
}

/// Update access and modification times relative to a directory handle.
pub(crate) unsafe fn destack_fs_utimensat(
    _binding: &BindingCallContext,
    dir: resource::DirectoryHandle,
    path: OsPath,
    atimens: u64,
    mtimens: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (dir, path, atimens, mtimens, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.utimensat")).boxed())
}

/// Update access and modification times.
pub(crate) unsafe fn destack_fs_utimes(
    _binding: &BindingCallContext,
    path: OsPath,
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    let _ = (path, atimens, mtimens);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.attrs.utimes")).boxed())
}

/// Close a directory handle.
pub(crate) unsafe fn destack_fs_closedir(
    _binding: &BindingCallContext,
    _handle: resource::DirectoryHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.closedir")).boxed())
}

/// Resolve the directory descriptor for an open directory handle.
pub(crate) unsafe fn destack_fs_dirfd(
    _binding: &BindingCallContext,
    out: *mut resource::FileHandle,
    handle: resource::DirectoryHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.dirfd")).boxed())
}

/// Create a directory.
pub(crate) unsafe fn destack_fs_mkdir(
    _binding: &BindingCallContext,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (path, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.mkdir")).boxed())
}

/// Create a directory relative to a directory handle.
pub(crate) unsafe fn destack_fs_mkdirat(
    _binding: &BindingCallContext,
    dir: resource::DirectoryHandle,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (dir, path, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.mkdirat")).boxed())
}

/// Create a temporary directory.
pub(crate) unsafe fn destack_fs_mkdtemp(
    _binding: &BindingCallContext,
    out: *mut OsPath,
    template: OsPath,
) -> RuntimeResult<()> {
    let _ = (out, template);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.mkdtemp")).boxed())
}

/// Open a directory and return a handle.
pub(crate) unsafe fn destack_fs_opendir(
    _binding: &BindingCallContext,
    out: *mut resource::DirectoryHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (out, path);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.opendir")).boxed())
}

/// Read directory entries from an open directory handle.
pub(crate) unsafe fn destack_fs_readdir(
    _binding: &BindingCallContext,
    out: *mut NativeArray<Dirent>,
    handle: resource::DirectoryHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.readdir")).boxed())
}

/// Read a single directory entry from an open directory handle.
pub(crate) unsafe fn destack_fs_readdir_next(
    _binding: &BindingCallContext,
    out: *mut DirentNext,
    handle: resource::DirectoryHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.readdirNext")).boxed())
}

/// Reset an open directory handle to the first entry.
pub(crate) unsafe fn destack_fs_rewinddir(
    _binding: &BindingCallContext,
    _handle: resource::DirectoryHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.rewinddir")).boxed())
}

/// Remove a directory.
pub(crate) unsafe fn destack_fs_rmdir(
    _binding: &BindingCallContext,
    _path: OsPath,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dir.rmdir")).boxed())
}

/// Close an open file handle.
pub(crate) unsafe fn destack_fs_close(
    _binding: &BindingCallContext,
    _handle: resource::FileHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.close")).boxed())
}

/// Copy a range between file descriptors.
pub(crate) unsafe fn destack_fs_copy_file_range(
    _binding: &BindingCallContext,
    out: *mut u64,
    src: resource::FileHandle,
    srcoffset: FileOffset,
    dst: resource::FileHandle,
    dstoffset: FileOffset,
    length: FileSize,
) -> RuntimeResult<()> {
    let _ = (out, src, srcoffset, dst, dstoffset, length);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.file.copyFileRange",
    ))
    .boxed())
}

/// Duplicate a file handle.
pub(crate) unsafe fn destack_fs_dup(
    _binding: &BindingCallContext,
    out: *mut resource::FileHandle,
    handle: resource::FileHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.dup")).boxed())
}

/// Duplicate a file handle to a specific target.
pub(crate) unsafe fn destack_fs_dup2(
    _binding: &BindingCallContext,
    out: *mut resource::FileHandle,
    handle: resource::FileHandle,
    target: resource::FileHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle, target);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.dup2")).boxed())
}

/// Duplicate a file handle to a specific target with flags.
pub(crate) unsafe fn destack_fs_dup3(
    _binding: &BindingCallContext,
    out: *mut resource::FileHandle,
    handle: resource::FileHandle,
    target: resource::FileHandle,
    flags: OpenFlags,
) -> RuntimeResult<()> {
    let _ = (out, handle, target, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.dup3")).boxed())
}

/// Advise the kernel about access patterns.
pub(crate) unsafe fn destack_fs_fadvise(
    _binding: &BindingCallContext,
    handle: resource::FileHandle,
    offset: FileOffset,
    length: FileSize,
    advice: FileAdvice,
) -> RuntimeResult<()> {
    let _ = (handle, offset, length, advice);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fadvise")).boxed())
}

/// Allocate or punch file space.
pub(crate) unsafe fn destack_fs_fallocate(
    _binding: &BindingCallContext,
    handle: resource::FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: AllocFlags,
) -> RuntimeResult<()> {
    let _ = (handle, offset, length, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fallocate")).boxed())
}

/// Synchronize file data only.
pub(crate) unsafe fn destack_fs_fdatasync(
    _binding: &BindingCallContext,
    _handle: resource::FileHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fdatasync")).boxed())
}

/// Synchronize a file's in-core state with storage.
pub(crate) unsafe fn destack_fs_fsync(
    _binding: &BindingCallContext,
    _handle: resource::FileHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.fsync")).boxed())
}

/// Truncate a file by handle.
pub(crate) unsafe fn destack_fs_ftruncate(
    _binding: &BindingCallContext,
    handle: resource::FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (handle, size);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.ftruncate")).boxed())
}

/// Read file descriptor flags.
pub(crate) unsafe fn destack_fs_get_fd_flags(
    _binding: &BindingCallContext,
    out: *mut FdFlags,
    handle: resource::FileHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.getFdFlags")).boxed())
}

/// Read file status flags.
pub(crate) unsafe fn destack_fs_get_status_flags(
    _binding: &BindingCallContext,
    out: *mut StatusFlags,
    handle: resource::FileHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.file.getStatusFlags",
    ))
    .boxed())
}

/// Apply file locks to a file handle.
pub(crate) unsafe fn destack_fs_lock(
    _binding: &BindingCallContext,
    handle: resource::FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    let _ = (handle, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.lock")).boxed())
}

/// Open a file and return a handle.
pub(crate) unsafe fn destack_fs_open(
    _binding: &BindingCallContext,
    out: *mut resource::FileHandle,
    path: OsPath,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (out, path, flags, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.open")).boxed())
}

/// Open a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_openat(
    _binding: &BindingCallContext,
    out: *mut resource::FileHandle,
    dir: resource::DirectoryHandle,
    path: OsPath,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (out, dir, path, flags, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.openat")).boxed())
}

/// Open a file relative to a directory handle with openat2 semantics.
pub(crate) unsafe fn destack_fs_openat2(
    _binding: &BindingCallContext,
    out: *mut resource::FileHandle,
    dir: resource::DirectoryHandle,
    path: OsPath,
    how: OpenOptions,
) -> RuntimeResult<()> {
    let _ = (out, dir, path, how);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.openat2")).boxed())
}

/// Read from a file at the given file offset.
pub(crate) unsafe fn destack_fs_pread(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer, offset);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.pread")).boxed())
}

/// Read into multiple buffers at the given file offset.
pub(crate) unsafe fn destack_fs_preadv(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers, offset);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.preadv")).boxed())
}

/// Read into multiple buffers at the given file offset with explicit read flags.
pub(crate) unsafe fn destack_fs_preadv2(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers, offset, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.preadv2")).boxed())
}

/// Write to a file at the given file offset.
pub(crate) unsafe fn destack_fs_pwrite(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::FileHandle,
    buffer: NativeSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer, offset);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.pwrite")).boxed())
}

/// Write from multiple buffers at the given file offset.
pub(crate) unsafe fn destack_fs_pwritev(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers, offset);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.pwritev")).boxed())
}

/// Write from multiple buffers at the given file offset with explicit write flags.
pub(crate) unsafe fn destack_fs_pwritev2(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers, offset, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.pwritev2")).boxed())
}

/// Read from a file into the provided slice.
pub(crate) unsafe fn destack_fs_read(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.read")).boxed())
}

/// Read into multiple buffers.
pub(crate) unsafe fn destack_fs_readv(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.readv")).boxed())
}

/// Seek within a file and return the new offset.
pub(crate) unsafe fn destack_fs_seek(
    _binding: &BindingCallContext,
    out: *mut FileOffset,
    handle: resource::FileHandle,
    offset: FileOffset,
    whence: SeekWhence,
) -> RuntimeResult<()> {
    let _ = (out, handle, offset, whence);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.seek")).boxed())
}

/// Send file data to a socket.
pub(crate) unsafe fn destack_fs_sendfile(
    _binding: &BindingCallContext,
    out: *mut u64,
    socket: resource::SocketHandle,
    file: resource::FileHandle,
    offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<()> {
    let _ = (out, socket, file, offset, length);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.sendfile")).boxed())
}

/// Write file descriptor flags.
pub(crate) unsafe fn destack_fs_set_fd_flags(
    _binding: &BindingCallContext,
    handle: resource::FileHandle,
    flags: FdFlags,
) -> RuntimeResult<()> {
    let _ = (handle, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.setFdFlags")).boxed())
}

/// Write file status flags.
pub(crate) unsafe fn destack_fs_set_status_flags(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_fs_splice(
    _binding: &BindingCallContext,
    out: *mut u64,
    source: resource::ResourceId,
    sourcecursor: SpliceCursor,
    target: resource::ResourceId,
    targetcursor: SpliceCursor,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (
        out,
        source,
        sourcecursor,
        target,
        targetcursor,
        length,
        flags,
    );

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.splice")).boxed())
}

/// Synchronize a file range.
pub(crate) unsafe fn destack_fs_sync_file_range(
    _binding: &BindingCallContext,
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
pub(crate) unsafe fn destack_fs_syncfs(
    _binding: &BindingCallContext,
    _handle: resource::FileHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.syncfs")).boxed())
}

/// Duplicate bytes from one pipe to another without consuming source bytes.
pub(crate) unsafe fn destack_fs_tee(
    _binding: &BindingCallContext,
    out: *mut u64,
    sourcepipe: resource::PipeHandle,
    targetpipe: resource::PipeHandle,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (out, sourcepipe, targetpipe, length, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.tee")).boxed())
}

/// Truncate a file.
pub(crate) unsafe fn destack_fs_truncate(
    _binding: &BindingCallContext,
    path: OsPath,
    size: FileOffset,
) -> RuntimeResult<()> {
    let _ = (path, size);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.truncate")).boxed())
}

/// Map user memory pages into a pipe as queued pipe buffers.
pub(crate) unsafe fn destack_fs_vmsplice(
    _binding: &BindingCallContext,
    out: *mut u64,
    pipe: resource::PipeHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
    flags: SpliceFlags,
) -> RuntimeResult<()> {
    let _ = (out, pipe, buffers, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.vmsplice")).boxed())
}

/// Write to a file from the provided slice.
pub(crate) unsafe fn destack_fs_write(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::FileHandle,
    buffer: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.write")).boxed())
}

/// Write from multiple buffers.
pub(crate) unsafe fn destack_fs_writev(
    _binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::FileHandle,
    buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let _ = (out, handle, buffers);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.file.writev")).boxed())
}

/// Advise the kernel about access patterns.
pub(crate) unsafe fn destack_fs_madvise(
    _binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
    advice: MmapAdvice,
) -> RuntimeResult<()> {
    let _ = (mapping, advice);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmap.madvise")).boxed())
}

/// Change memory protection for a mapping.
pub(crate) unsafe fn destack_fs_mprotect(
    _binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
    prot: MmapProt,
) -> RuntimeResult<()> {
    let _ = (mapping, prot);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmap.mprotect")).boxed())
}

/// Flush a mapping to storage.
pub(crate) unsafe fn destack_fs_msync(
    _binding: &BindingCallContext,
    mapping: NativeSlice<u8>,
    flags: MmapSyncFlags,
) -> RuntimeResult<()> {
    let _ = (mapping, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmap.msync")).boxed())
}

/// Unmap a memory region.
pub(crate) unsafe fn destack_fs_munmap(
    _binding: &BindingCallContext,
    _mapping: NativeSlice<u8>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmap.munmap")).boxed())
}

/// Create an anonymous memory mapping.
pub(crate) unsafe fn destack_fs_mmap_anonymous(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    let _ = (out, length, prot, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmapAnonymous")).boxed())
}

/// Create a file-backed memory mapping.
pub(crate) unsafe fn destack_fs_mmap_file(
    _binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::FileHandle,
    offset: FileOffset,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<()> {
    let _ = (out, handle, offset, length, prot, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmapFile")).boxed())
}

/// Copy a file.
pub(crate) unsafe fn destack_fs_copyfile(
    _binding: &BindingCallContext,
    from: OsPath,
    to: OsPath,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    let _ = (from, to, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.copyfile")).boxed())
}

/// Create a hard link.
pub(crate) unsafe fn destack_fs_link(
    _binding: &BindingCallContext,
    existingpath: OsPath,
    newpath: OsPath,
) -> RuntimeResult<()> {
    let _ = (existingpath, newpath);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.link")).boxed())
}

/// Create a hard link relative to directory handles.
pub(crate) unsafe fn destack_fs_linkat(
    _binding: &BindingCallContext,
    existingdir: resource::DirectoryHandle,
    existingpath: OsPath,
    newdir: resource::DirectoryHandle,
    newpath: OsPath,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (existingdir, existingpath, newdir, newpath, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.linkat")).boxed())
}

/// Create a FIFO special file.
pub(crate) unsafe fn destack_fs_mkfifo(
    _binding: &BindingCallContext,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (path, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.mkfifo")).boxed())
}

/// Create a FIFO special file relative to a directory handle.
pub(crate) unsafe fn destack_fs_mkfifoat(
    _binding: &BindingCallContext,
    dir: resource::DirectoryHandle,
    path: OsPath,
    mode: FileMode,
) -> RuntimeResult<()> {
    let _ = (dir, path, mode);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.mkfifoat")).boxed())
}

/// Create a filesystem node.
pub(crate) unsafe fn destack_fs_mknod(
    _binding: &BindingCallContext,
    path: OsPath,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    let _ = (path, mode, device);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.mknod")).boxed())
}

/// Create a filesystem node relative to a directory handle.
pub(crate) unsafe fn destack_fs_mknodat(
    _binding: &BindingCallContext,
    dir: resource::DirectoryHandle,
    path: OsPath,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    let _ = (dir, path, mode, device);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.mknodat")).boxed())
}

/// Read a symbolic link.
pub(crate) unsafe fn destack_fs_readlink(
    _binding: &BindingCallContext,
    out: *mut OsPath,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (out, path);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.readlink")).boxed())
}

/// Read a symbolic link relative to a directory handle.
pub(crate) unsafe fn destack_fs_readlinkat(
    _binding: &BindingCallContext,
    out: *mut OsPath,
    dir: resource::DirectoryHandle,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (out, dir, path);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.readlinkat")).boxed())
}

/// Resolve a path to its canonical form.
pub(crate) unsafe fn destack_fs_realpath(
    _binding: &BindingCallContext,
    out: *mut OsPath,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (out, path);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.realpath")).boxed())
}

/// Rename or move a file.
pub(crate) unsafe fn destack_fs_rename(
    _binding: &BindingCallContext,
    from: OsPath,
    to: OsPath,
) -> RuntimeResult<()> {
    let _ = (from, to);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.rename")).boxed())
}

/// Rename or move a file relative to directory handles.
pub(crate) unsafe fn destack_fs_renameat(
    _binding: &BindingCallContext,
    fromdir: resource::DirectoryHandle,
    from: OsPath,
    todir: resource::DirectoryHandle,
    to: OsPath,
) -> RuntimeResult<()> {
    let _ = (fromdir, from, todir, to);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.renameat")).boxed())
}

/// Rename or move a file relative to directory handles with renameat2 semantics.
pub(crate) unsafe fn destack_fs_renameat2(
    _binding: &BindingCallContext,
    fromdir: resource::DirectoryHandle,
    from: OsPath,
    todir: resource::DirectoryHandle,
    to: OsPath,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    let _ = (fromdir, from, todir, to, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.renameat2")).boxed())
}

/// Create a symbolic link.
pub(crate) unsafe fn destack_fs_symlink(
    _binding: &BindingCallContext,
    target: OsPath,
    path: OsPath,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = (target, path, kind);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.symlink")).boxed())
}

/// Create a symbolic link relative to a directory handle.
pub(crate) unsafe fn destack_fs_symlinkat(
    _binding: &BindingCallContext,
    target: OsPath,
    dir: resource::DirectoryHandle,
    path: OsPath,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let _ = (target, dir, path, kind);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.symlinkat")).boxed())
}

/// Unlink a file.
pub(crate) unsafe fn destack_fs_unlink(
    _binding: &BindingCallContext,
    _path: OsPath,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.unlink")).boxed())
}

/// Unlink a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_unlinkat(
    _binding: &BindingCallContext,
    dir: resource::DirectoryHandle,
    path: OsPath,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (dir, path, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.path.unlinkat")).boxed())
}

/// Stat a file by handle.
pub(crate) unsafe fn destack_fs_fstat(
    _binding: &BindingCallContext,
    out: *mut Stat,
    handle: resource::FileHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.fstat")).boxed())
}

/// Stat a filesystem by handle.
pub(crate) unsafe fn destack_fs_fstatfs(
    _binding: &BindingCallContext,
    out: *mut StatFs,
    handle: resource::FileHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.fstatfs")).boxed())
}

/// Stat a file without following symlinks.
pub(crate) unsafe fn destack_fs_lstat(
    _binding: &BindingCallContext,
    out: *mut Stat,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (out, path);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.lstat")).boxed())
}

/// Stat a file.
pub(crate) unsafe fn destack_fs_stat(
    _binding: &BindingCallContext,
    out: *mut Stat,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (out, path);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.path")).boxed())
}

/// Stat a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_statat(
    _binding: &BindingCallContext,
    out: *mut Stat,
    dir: resource::DirectoryHandle,
    path: OsPath,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let _ = (out, dir, path, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.pathat")).boxed())
}

/// Stat a filesystem.
pub(crate) unsafe fn destack_fs_statfs(
    _binding: &BindingCallContext,
    out: *mut StatFs,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (out, path);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.pathfs")).boxed())
}

/// Stat a path with statx semantics.
pub(crate) unsafe fn destack_fs_statx(
    _binding: &BindingCallContext,
    out: *mut Statx,
    dir: resource::DirectoryHandle,
    path: OsPath,
    flags: StatxFlags,
    mask: StatxMask,
) -> RuntimeResult<()> {
    let _ = (out, dir, path, flags, mask);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.stat.pathx")).boxed())
}

/// Start watching a path and return a watch handle.
pub(crate) unsafe fn destack_fs_watch(
    _binding: &BindingCallContext,
    out: *mut resource::WatchHandle,
    path: OsPath,
    options: WatchOptions,
) -> RuntimeResult<()> {
    let _ = (out, path, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watch")).boxed())
}

/// Close a watch handle.
pub(crate) unsafe fn destack_fs_watch_close(
    _binding: &BindingCallContext,
    _handle: resource::WatchHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchClose")).boxed())
}

/// Read a batch of events from a watch handle.
pub(crate) unsafe fn destack_fs_watch_read(
    _binding: &BindingCallContext,
    out: *mut WatchBatch,
    handle: resource::WatchHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchRead")).boxed())
}

/// Start watching a path relative to a directory handle.
pub(crate) unsafe fn destack_fs_watchat(
    _binding: &BindingCallContext,
    out: *mut resource::WatchHandle,
    directory: resource::DirectoryHandle,
    path: OsPath,
    options: WatchOptions,
) -> RuntimeResult<()> {
    let _ = (out, directory, path, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchat")).boxed())
}

/// Read an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fgetxattr(
    _binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: resource::FileHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (out, handle, name);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.fgetxattr")).boxed())
}

/// Read an extended attribute by handle with a raw name payload.
pub(crate) unsafe fn destack_fs_fgetxattr_bytes(
    _binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    handle: resource::FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, handle, name);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.fgetxattrBytes",
    ))
    .boxed())
}

/// List extended attribute names by handle.
pub(crate) unsafe fn destack_fs_flistxattr(
    _binding: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    handle: resource::FileHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.flistxattr")).boxed())
}

/// List extended attribute names by handle as raw byte payloads.
pub(crate) unsafe fn destack_fs_flistxattr_bytes(
    _binding: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    handle: resource::FileHandle,
) -> RuntimeResult<()> {
    let _ = (out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.flistxattrBytes",
    ))
    .boxed())
}

/// Remove an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fremovexattr(
    _binding: &BindingCallContext,
    handle: resource::FileHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (handle, name);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.fremovexattr",
    ))
    .boxed())
}

/// Remove an extended attribute by handle with a raw name payload.
pub(crate) unsafe fn destack_fs_fremovexattr_bytes(
    _binding: &BindingCallContext,
    handle: resource::FileHandle,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (handle, name);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.fremovexattrBytes",
    ))
    .boxed())
}

/// Set an extended attribute by handle.
pub(crate) unsafe fn destack_fs_fsetxattr(
    _binding: &BindingCallContext,
    handle: resource::FileHandle,
    name: NativeStringRef,
    argument_value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (handle, name, argument_value, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.fsetxattr")).boxed())
}

/// Set an extended attribute by handle with a raw name payload.
pub(crate) unsafe fn destack_fs_fsetxattr_bytes(
    _binding: &BindingCallContext,
    handle: resource::FileHandle,
    name: NativeSlice<u8>,
    argument_value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (handle, name, argument_value, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.fsetxattrBytes",
    ))
    .boxed())
}

/// Read an extended attribute by path.
pub(crate) unsafe fn destack_fs_getxattr(
    _binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (out, path, name);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.getxattr")).boxed())
}

/// Read an extended attribute by path with a raw name payload.
pub(crate) unsafe fn destack_fs_getxattr_bytes(
    _binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, path, name);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.getxattrBytes",
    ))
    .boxed())
}

/// Read an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lgetxattr(
    _binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (out, path, name);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.lgetxattr")).boxed())
}

/// Read an extended attribute without following symlinks, using a raw name payload.
pub(crate) unsafe fn destack_fs_lgetxattr_bytes(
    _binding: &BindingCallContext,
    out: *mut NativeArray<u8>,
    path: OsPath,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (out, path, name);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.lgetxattrBytes",
    ))
    .boxed())
}

/// List extended attribute names by path.
pub(crate) unsafe fn destack_fs_listxattr(
    _binding: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (out, path);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.listxattr")).boxed())
}

/// List extended attribute names by path as raw byte payloads.
pub(crate) unsafe fn destack_fs_listxattr_bytes(
    _binding: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (out, path);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.listxattrBytes",
    ))
    .boxed())
}

/// List extended attribute names without following symlinks.
pub(crate) unsafe fn destack_fs_llistxattr(
    _binding: &BindingCallContext,
    out: *mut NativeArray<NativeStringRef>,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (out, path);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.llistxattr")).boxed())
}

/// List extended attribute names without following symlinks as raw byte payloads.
pub(crate) unsafe fn destack_fs_llistxattr_bytes(
    _binding: &BindingCallContext,
    out: *mut NativeArray<NativeArray<u8>>,
    path: OsPath,
) -> RuntimeResult<()> {
    let _ = (out, path);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.llistxattrBytes",
    ))
    .boxed())
}

/// Remove an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lremovexattr(
    _binding: &BindingCallContext,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (path, name);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.lremovexattr",
    ))
    .boxed())
}

/// Remove an extended attribute without following symlinks, using a raw name payload.
pub(crate) unsafe fn destack_fs_lremovexattr_bytes(
    _binding: &BindingCallContext,
    path: OsPath,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (path, name);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.lremovexattrBytes",
    ))
    .boxed())
}

/// Set an extended attribute without following symlinks.
pub(crate) unsafe fn destack_fs_lsetxattr(
    _binding: &BindingCallContext,
    path: OsPath,
    name: NativeStringRef,
    argument_value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (path, name, argument_value, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.lsetxattr")).boxed())
}

/// Set an extended attribute without following symlinks, using a raw name payload.
pub(crate) unsafe fn destack_fs_lsetxattr_bytes(
    _binding: &BindingCallContext,
    path: OsPath,
    name: NativeSlice<u8>,
    argument_value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (path, name, argument_value, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.lsetxattrBytes",
    ))
    .boxed())
}

/// Remove an extended attribute by path.
pub(crate) unsafe fn destack_fs_removexattr(
    _binding: &BindingCallContext,
    path: OsPath,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (path, name);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.removexattr")).boxed())
}

/// Remove an extended attribute by path with a raw name payload.
pub(crate) unsafe fn destack_fs_removexattr_bytes(
    _binding: &BindingCallContext,
    path: OsPath,
    name: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (path, name);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.removexattrBytes",
    ))
    .boxed())
}

/// Set an extended attribute by path.
pub(crate) unsafe fn destack_fs_setxattr(
    _binding: &BindingCallContext,
    path: OsPath,
    name: NativeStringRef,
    argument_value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (path, name, argument_value, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.xattr.setxattr")).boxed())
}

/// Set an extended attribute by path with a raw name payload.
pub(crate) unsafe fn destack_fs_setxattr_bytes(
    _binding: &BindingCallContext,
    path: OsPath,
    name: NativeSlice<u8>,
    argument_value: NativeSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let _ = (path, name, argument_value, flags);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.fs.xattr.setxattrBytes",
    ))
    .boxed())
}
