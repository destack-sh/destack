use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::{core as core_fs, os as os_fs};

use crate::platform::abi::{NativeAbi, VmAbi};
use crate::platform::fs::{
    AccessMode, AllocFlags, AtFlags, CopyFlags, DirectoryHandle, Dirent, DirentNext, DirentNextVm,
    DirentVm, FdFlags, FileAdvice, FileHandle, FileLockFlags, FileMode, FileOffset, FileSize,
    MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags, NodeDevice, OpenFlags, OpenOptions,
    OpenOptionsVm, OsPath, OsPathVm, PathBytes, PathBytesAbi, PathBytesVm, PathEncoding, PathUtf16,
    PathUtf16Abi, PathUtf16Vm, ReadWriteFlags, RenameFlags, SeekWhence, SpliceCursorVm,
    SpliceFlags, Stat, StatFs, StatusFlags, Statx, StatxFlags, StatxMask, SymlinkType, SyncFlags,
    WatchBatchVm, WatchOptionsVm, XattrFlags,
};
use crate::platform::resource::{PipeHandle, ResourceId, SocketHandle};
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError, VmArray, VmSlice};
use crate::runtime::RuntimeCallContext;

fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    let mut out = std::mem::MaybeUninit::<T>::uninit();
    call(out.as_mut_ptr())?;
    Ok(unsafe { out.assume_init() })
}

/// Check filesystem access for a path.
pub fn destack_fs_access(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_access(runtime, path, mode) }
}

/// Check filesystem access for a path relative to a directory handle.
pub fn destack_fs_accessat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: AccessMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_accessat(runtime, dir, path, mode, flags) }
}

/// Change permissions for a path.
pub fn destack_fs_chmod(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_chmod(runtime, path, mode) }
}

/// Change permissions for a path relative to a directory handle.
pub fn destack_fs_fchmodat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_fchmodat(runtime, dir, path, mode, flags) }
}

/// Change ownership for a path.
pub fn destack_fs_chown(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_chown(runtime, path, uid, gid) }
}

/// Change ownership for a path relative to a directory handle.
pub fn destack_fs_fchownat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_fchownat(runtime, dir, path, uid, gid, flags) }
}

/// Update access and modification times for a path.
pub fn destack_fs_utimes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_utimes(runtime, path, atime_ns, mtime_ns) }
}

/// Update access and modification times without following symlinks.
pub fn destack_fs_lutimes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_lutimes(runtime, path, atime_ns, mtime_ns) }
}

/// Update access and modification times relative to a directory handle.
pub fn destack_fs_utimensat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    atime_ns: u64,
    mtime_ns: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_utimensat(runtime, dir, path, atime_ns, mtime_ns, flags) }
}

/// Create a directory.
pub fn destack_fs_mkdir(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_mkdir(runtime, path, mode) }
}

/// Create a directory relative to a directory handle.
pub fn destack_fs_mkdirat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_mkdirat(runtime, dir, path, mode) }
}

/// Remove a directory.
pub fn destack_fs_rmdir(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_rmdir(runtime, path) }
}

/// Open a directory and return a handle.
pub fn destack_fs_opendir(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<DirectoryHandle> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_fs::destack_fs_opendir(runtime, out, path) })
}

/// Create a temporary directory.
pub fn destack_fs_mkdtemp(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    template: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let template = path_ref_from_vm(runtime, context, template)?;
    let path = call_out(|out| unsafe { core_fs::destack_fs_mkdtemp(runtime, out, template) })?;
    path_ref_to_vm(context, path)
}

/// Open a file and return a handle.
pub fn destack_fs_open(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<FileHandle> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_fs::destack_fs_open(runtime, out, path, flags, mode) })
}

/// Open a file relative to a directory handle.
pub fn destack_fs_openat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<FileHandle> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_fs::destack_fs_openat(runtime, out, dir, path, flags, mode) })
}

/// Open a file relative to a directory handle with openat2 semantics.
pub fn destack_fs_openat2(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    how: OpenOptionsVm,
) -> RuntimeResult<FileHandle> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let how = OpenOptions {
        flags: how.flags,
        mode: how.mode,
        resolve: how.resolve,
    };
    call_out(|out| unsafe { core_fs::destack_fs_openat2(runtime, out, dir, path, how) })
}

/// Close an open file handle.
pub fn destack_fs_close(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { os_fs::destack_fs_close(runtime, handle) }
}

/// Close an open directory handle.
pub fn destack_fs_closedir(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    unsafe { os_fs::destack_fs_closedir(runtime, handle) }
}

/// Read directory entries from an open directory handle.
pub fn destack_fs_readdir(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<VmArray<DirentVm>> {
    let entries = call_out(|out| unsafe { os_fs::destack_fs_readdir(runtime, out, handle) })?;
    dirent_array_to_vm(context, entries)
}

/// Read the next entry from an open directory handle.
pub fn destack_fs_readdir_next(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<DirentNextVm> {
    let entry = call_out(|out| unsafe { core_fs::destack_fs_readdir_next(runtime, out, handle) })?;
    dirent_next_to_vm(context, entry)
}

/// Rewind an open directory handle to the first entry.
pub fn destack_fs_rewinddir(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_rewinddir(runtime, handle) }
}

/// Resolve the file descriptor for an open directory handle.
pub fn destack_fs_dirfd(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<FileHandle> {
    call_out(|out| unsafe { core_fs::destack_fs_dirfd(runtime, out, handle) })
}

/// Read from a file into a buffer.
pub fn destack_fs_read(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let native = allocate_read_buffer(runtime, buffer);
    let count = call_out(|out| unsafe { os_fs::destack_fs_read(runtime, out, handle, native) })?;
    write_read_buffer(context, buffer, native)?;
    Ok(count)
}

/// Read from a file into a buffer at the given file offset.
pub fn destack_fs_pread(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let native = allocate_read_buffer(runtime, buffer);
    let count =
        call_out(|out| unsafe { os_fs::destack_fs_pread(runtime, out, handle, native, offset) })?;
    write_read_buffer(context, buffer, native)?;
    Ok(count)
}

/// Write to a file from a buffer.
pub fn destack_fs_write(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let native = buffer_from_vm(runtime, context, buffer)?;
    call_out(|out| unsafe { os_fs::destack_fs_write(runtime, out, handle, native) })
}

/// Write to a file from a buffer at the given file offset.
pub fn destack_fs_pwrite(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let native = buffer_from_vm(runtime, context, buffer)?;
    call_out(|out| unsafe { os_fs::destack_fs_pwrite(runtime, out, handle, native, offset) })
}

/// Read into multiple buffers at the given file offset.
pub fn destack_fs_readv(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let (native_buffers, vm_buffers) = allocate_read_buffers(runtime, context, buffers)?;
    let count =
        call_out(|out| unsafe { os_fs::destack_fs_readv(runtime, out, handle, native_buffers) })?;
    write_read_buffers(context, vm_buffers, native_buffers)?;
    Ok(count)
}

/// Read into multiple buffers at the given file offset.
pub fn destack_fs_preadv(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let (native_buffers, vm_buffers) = allocate_read_buffers(runtime, context, buffers)?;
    let count = call_out(|out| unsafe {
        os_fs::destack_fs_preadv(runtime, out, handle, native_buffers, offset)
    })?;
    write_read_buffers(context, vm_buffers, native_buffers)?;
    Ok(count)
}

/// Read into multiple buffers with explicit read flags.
pub fn destack_fs_preadv2(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<u64> {
    // NOTE #Incomplete: honor read flags in vm mode
    let _ = flags;

    destack_fs_preadv(runtime, context, handle, buffers, offset)
}

/// Write from multiple buffers.
pub fn destack_fs_writev(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let native_buffers = buffers_from_vm(runtime, context, buffers)?;
    call_out(|out| unsafe { os_fs::destack_fs_writev(runtime, out, handle, native_buffers) })
}

/// Write from multiple buffers at the given file offset.
pub fn destack_fs_pwritev(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let native_buffers = buffers_from_vm(runtime, context, buffers)?;
    call_out(|out| unsafe {
        os_fs::destack_fs_pwritev(runtime, out, handle, native_buffers, offset)
    })
}

/// Write from multiple buffers with explicit write flags.
pub fn destack_fs_pwritev2(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<u64> {
    // NOTE #Incomplete: honor write flags in vm mode
    let _ = flags;

    destack_fs_pwritev(runtime, context, handle, buffers, offset)
}

/// Change file permissions by handle.
pub fn destack_fs_fchmod(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    unsafe { os_fs::destack_fs_fchmod(runtime, handle, mode) }
}

/// Change file owner and group by handle.
pub fn destack_fs_fchown(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    unsafe { os_fs::destack_fs_fchown(runtime, handle, uid, gid) }
}

/// Synchronize a file's in-core state with storage.
pub fn destack_fs_fsync(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { os_fs::destack_fs_fsync(runtime, handle) }
}

/// Synchronize file data only.
pub fn destack_fs_fdatasync(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { os_fs::destack_fs_fdatasync(runtime, handle) }
}

/// Truncate a file by handle.
pub fn destack_fs_ftruncate(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    unsafe { os_fs::destack_fs_ftruncate(runtime, handle, size) }
}

/// Update access and modification times by handle.
pub fn destack_fs_futimes(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    unsafe { os_fs::destack_fs_futimes(runtime, handle, atime_ns, mtime_ns) }
}

/// Stat a file by handle.
pub fn destack_fs_fstat(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<Stat> {
    call_out(|out| unsafe { os_fs::destack_fs_fstat(runtime, out, handle) })
}

/// Stat a filesystem by handle.
pub fn destack_fs_fstatfs(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<StatFs> {
    call_out(|out| unsafe { os_fs::destack_fs_fstatfs(runtime, out, handle) })
}

/// Apply file locks to a file handle.
pub fn destack_fs_lock(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    unsafe { os_fs::destack_fs_lock(runtime, handle, flags) }
}

/// Read file descriptor flags.
pub fn destack_fs_get_fd_flags(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<FdFlags> {
    call_out(|out| unsafe { core_fs::destack_fs_get_fd_flags(runtime, out, handle) })
}

/// Read file status flags.
pub fn destack_fs_get_status_flags(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<StatusFlags> {
    call_out(|out| unsafe { core_fs::destack_fs_get_status_flags(runtime, out, handle) })
}

/// Write file descriptor flags.
pub fn destack_fs_set_fd_flags(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    flags: FdFlags,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_set_fd_flags(runtime, handle, flags) }
}

/// Write file status flags.
pub fn destack_fs_set_status_flags(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    flags: StatusFlags,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_set_status_flags(runtime, handle, flags) }
}

/// Truncate a file.
pub fn destack_fs_truncate(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    size: FileOffset,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_truncate(runtime, path, size) }
}

/// Rename or move a file.
pub fn destack_fs_rename(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from: OsPathVm,
    to: OsPathVm,
) -> RuntimeResult<()> {
    let from = path_ref_from_vm(runtime, context, from)?;
    let to = path_ref_from_vm(runtime, context, to)?;
    unsafe { core_fs::destack_fs_rename(runtime, from, to) }
}

/// Rename or move a file relative to directory handles.
pub fn destack_fs_renameat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from_dir: DirectoryHandle,
    from: OsPathVm,
    to_dir: DirectoryHandle,
    to: OsPathVm,
) -> RuntimeResult<()> {
    let from = path_ref_from_vm(runtime, context, from)?;
    let to = path_ref_from_vm(runtime, context, to)?;
    unsafe { core_fs::destack_fs_renameat(runtime, from_dir, from, to_dir, to) }
}

/// Rename or move a file relative to directory handles with renameat2 semantics.
pub fn destack_fs_renameat2(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from_dir: DirectoryHandle,
    from: OsPathVm,
    to_dir: DirectoryHandle,
    to: OsPathVm,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    let from = path_ref_from_vm(runtime, context, from)?;
    let to = path_ref_from_vm(runtime, context, to)?;
    unsafe { core_fs::destack_fs_renameat2(runtime, from_dir, from, to_dir, to, flags) }
}

/// Unlink a file.
pub fn destack_fs_unlink(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_unlink(runtime, path) }
}

/// Unlink a file relative to a directory handle.
pub fn destack_fs_unlinkat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_unlinkat(runtime, dir, path, flags) }
}

/// Create a hard link.
pub fn destack_fs_link(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    existing_path: OsPathVm,
    new_path: OsPathVm,
) -> RuntimeResult<()> {
    let existing_path = path_ref_from_vm(runtime, context, existing_path)?;
    let new_path = path_ref_from_vm(runtime, context, new_path)?;
    unsafe { core_fs::destack_fs_link(runtime, existing_path, new_path) }
}

/// Create a hard link relative to directory handles.
pub fn destack_fs_linkat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    existing_dir: DirectoryHandle,
    existing_path: OsPathVm,
    new_dir: DirectoryHandle,
    new_path: OsPathVm,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let existing_path = path_ref_from_vm(runtime, context, existing_path)?;
    let new_path = path_ref_from_vm(runtime, context, new_path)?;
    unsafe {
        core_fs::destack_fs_linkat(
            runtime,
            existing_dir,
            existing_path,
            new_dir,
            new_path,
            flags,
        )
    }
}

/// Create a symbolic link.
pub fn destack_fs_symlink(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    target: OsPathVm,
    path: OsPathVm,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let target = path_ref_from_vm(runtime, context, target)?;
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_symlink(runtime, target, path, kind) }
}

/// Create a symbolic link relative to a directory handle.
pub fn destack_fs_symlinkat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    target: OsPathVm,
    dir: DirectoryHandle,
    path: OsPathVm,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let target = path_ref_from_vm(runtime, context, target)?;
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_symlinkat(runtime, target, dir, path, kind) }
}

/// Read a symlink target.
pub fn destack_fs_readlink(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let target = call_out(|out| unsafe { core_fs::destack_fs_readlink(runtime, out, path) })?;
    path_ref_to_vm(context, target)
}

/// Read a symlink target relative to a directory handle.
pub fn destack_fs_readlinkat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let target =
        call_out(|out| unsafe { core_fs::destack_fs_readlinkat(runtime, out, dir, path) })?;
    path_ref_to_vm(context, target)
}

/// Resolve the real path.
pub fn destack_fs_realpath(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let resolved = call_out(|out| unsafe { core_fs::destack_fs_realpath(runtime, out, path) })?;
    path_ref_to_vm(context, resolved)
}

/// Copy a file.
pub fn destack_fs_copyfile(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from: OsPathVm,
    to: OsPathVm,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    let from = path_ref_from_vm(runtime, context, from)?;
    let to = path_ref_from_vm(runtime, context, to)?;
    unsafe { core_fs::destack_fs_copyfile(runtime, from, to, flags) }
}

/// Create a fifo special file.
pub fn destack_fs_mkfifo(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_mkfifo(runtime, path, mode) }
}

/// Create a fifo special file relative to a directory handle.
pub fn destack_fs_mkfifoat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_mkfifoat(runtime, dir, path, mode) }
}

/// Create a filesystem node.
pub fn destack_fs_mknod(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_mknod(runtime, path, mode, device) }
}

/// Create a filesystem node relative to a directory handle.
pub fn destack_fs_mknodat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_mknodat(runtime, dir, path, mode, device) }
}

/// Stat a file.
pub fn destack_fs_stat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<Stat> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_fs::destack_fs_stat(runtime, out, path) })
}

/// Stat a file relative to a directory handle.
pub fn destack_fs_statat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    flags: AtFlags,
) -> RuntimeResult<Stat> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_fs::destack_fs_statat(runtime, out, dir, path, flags) })
}

/// Stat a file without following symlinks.
pub fn destack_fs_lstat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<Stat> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_fs::destack_fs_lstat(runtime, out, path) })
}

/// Stat a filesystem.
pub fn destack_fs_statfs(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<StatFs> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_fs::destack_fs_statfs(runtime, out, path) })
}

/// Stat a path with statx semantics.
pub fn destack_fs_statx(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    flags: StatxFlags,
    mask: StatxMask,
) -> RuntimeResult<Statx> {
    let path = path_ref_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_fs::destack_fs_statx(runtime, out, dir, path, flags, mask) })
}

/// Synchronize a filesystem by file handle.
pub fn destack_fs_syncfs(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_syncfs(runtime, handle) }
}

/// Open a filesystem watch for a path.
pub fn destack_fs_watch(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    options: WatchOptionsVm,
) -> RuntimeResult<crate::platform::resource::WatchHandle> {
    // NOTE #Incomplete: implement vm filesystem watch open
    let _ = (runtime, context, path, options);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watch")).boxed())
}

/// Close a filesystem watch handle.
pub fn destack_fs_watch_close(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::WatchHandle,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement vm filesystem watch close
    let _ = (runtime, context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchClose")).boxed())
}

/// Read pending filesystem watch events.
pub fn destack_fs_watch_read(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: crate::platform::resource::WatchHandle,
) -> RuntimeResult<WatchBatchVm> {
    // NOTE #Incomplete: implement vm filesystem watch read
    let _ = (runtime, context, handle);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchRead")).boxed())
}

/// Open a filesystem watch relative to a directory handle.
pub fn destack_fs_watchat(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    directory: DirectoryHandle,
    path: OsPathVm,
    options: WatchOptionsVm,
) -> RuntimeResult<crate::platform::resource::WatchHandle> {
    // NOTE #Incomplete: implement vm filesystem watch open at
    let _ = (runtime, context, directory, path, options);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.watchat")).boxed())
}

/// Duplicate a file handle.
pub fn destack_fs_dup(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<FileHandle> {
    call_out(|out| unsafe { os_fs::destack_fs_dup(runtime, out, handle) })
}

/// Duplicate a file handle into a target handle.
pub fn destack_fs_dup2(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    target: FileHandle,
) -> RuntimeResult<FileHandle> {
    call_out(|out| unsafe { os_fs::destack_fs_dup2(runtime, out, handle, target) })
}

/// Duplicate a file handle into a target handle with flags.
pub fn destack_fs_dup3(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    target: FileHandle,
    flags: OpenFlags,
) -> RuntimeResult<FileHandle> {
    call_out(|out| unsafe { os_fs::destack_fs_dup3(runtime, out, handle, target, flags) })
}

/// Copy a range between file descriptors.
pub fn destack_fs_copy_file_range(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    src: FileHandle,
    src_offset: FileOffset,
    dst: FileHandle,
    dst_offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe {
        os_fs::destack_fs_copy_file_range(runtime, out, src, src_offset, dst, dst_offset, length)
    })
}

/// Send file data to a socket.
pub fn destack_fs_sendfile(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    socket: SocketHandle,
    file: FileHandle,
    offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe {
        os_fs::destack_fs_sendfile(runtime, out, socket, file, offset, length)
    })
}

/// Move data between file descriptors in-kernel.
pub fn destack_fs_splice(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    source: ResourceId,
    sourcecursor: SpliceCursorVm,
    target: ResourceId,
    targetcursor: SpliceCursorVm,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<u64> {
    // NOTE #Incomplete: implement vm splice support
    let _ = (source, sourcecursor, target, targetcursor, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.splice")).boxed())
}

/// Duplicate pipe data in-kernel.
pub fn destack_fs_tee(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    sourcepipe: PipeHandle,
    targetpipe: PipeHandle,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<u64> {
    // NOTE #Incomplete: implement vm tee support
    let _ = (sourcepipe, targetpipe, length, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.tee")).boxed())
}

/// Move user buffers into a pipe.
pub fn destack_fs_vmsplice(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pipe: PipeHandle,
    buffers: VmSlice<VmSlice<u8>>,
    flags: SpliceFlags,
) -> RuntimeResult<u64> {
    // NOTE #Incomplete: implement vm vmsplice support
    let _ = (pipe, buffers, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.vmsplice")).boxed())
}

/// Seek within a file and return the new offset.
pub fn destack_fs_seek(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    whence: SeekWhence,
) -> RuntimeResult<FileOffset> {
    call_out(|out| unsafe { os_fs::destack_fs_seek(runtime, out, handle, offset, whence) })
}

/// Advise the kernel about access patterns.
pub fn destack_fs_fadvise(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    advice: FileAdvice,
) -> RuntimeResult<()> {
    unsafe { os_fs::destack_fs_fadvise(runtime, handle, offset, length, advice) }
}

/// Allocate storage for a file range.
pub fn destack_fs_fallocate(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: AllocFlags,
) -> RuntimeResult<()> {
    unsafe { os_fs::destack_fs_fallocate(runtime, handle, offset, length, flags) }
}

/// Synchronize a range of a file to storage.
pub fn destack_fs_sync_file_range(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: SyncFlags,
) -> RuntimeResult<()> {
    unsafe { os_fs::destack_fs_sync_file_range(runtime, handle, offset, length, flags) }
}

/// Read an extended attribute by path.
pub fn destack_fs_getxattr(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    let values = call_out(|out| unsafe { core_fs::destack_fs_getxattr(runtime, out, path, name) })?;
    array_u8_to_vm(context, values)
}

/// Read an extended attribute by byte path.
pub fn destack_fs_getxattr_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    name: vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    let values =
        call_out(|out| unsafe { os_fs::destack_fs_getxattr_bytes(runtime, out, path, name) })?;
    array_u8_to_vm(context, values)
}

/// Read an extended attribute without following symlinks.
pub fn destack_fs_lgetxattr(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    let values =
        call_out(|out| unsafe { core_fs::destack_fs_lgetxattr(runtime, out, path, name) })?;
    array_u8_to_vm(context, values)
}

/// Read an extended attribute by byte path without following symlinks.
pub fn destack_fs_lgetxattr_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    name: vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    let values =
        call_out(|out| unsafe { os_fs::destack_fs_lgetxattr_bytes(runtime, out, path, name) })?;
    array_u8_to_vm(context, values)
}

/// Read an extended attribute by handle.
pub fn destack_fs_fgetxattr(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    name: vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let name = string_ref_from_vm(runtime, context, name)?;
    let values =
        call_out(|out| unsafe { core_fs::destack_fs_fgetxattr(runtime, out, handle, name) })?;
    array_u8_to_vm(context, values)
}

/// Set an extended attribute by path.
pub fn destack_fs_setxattr(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    let value = buffer_from_vm(runtime, context, value)?;
    unsafe { core_fs::destack_fs_setxattr(runtime, path, name, value, flags) }
}

/// Set an extended attribute by byte path.
pub fn destack_fs_setxattr_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    name: vm::StringHandle,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    let value = buffer_from_vm(runtime, context, value)?;
    unsafe { os_fs::destack_fs_setxattr_bytes(runtime, path, name, value, flags) }
}

/// Set an extended attribute without following symlinks.
pub fn destack_fs_lsetxattr(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    let value = buffer_from_vm(runtime, context, value)?;
    unsafe { core_fs::destack_fs_lsetxattr(runtime, path, name, value, flags) }
}

/// Set an extended attribute by byte path without following symlinks.
pub fn destack_fs_lsetxattr_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    name: vm::StringHandle,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    let value = buffer_from_vm(runtime, context, value)?;
    unsafe { os_fs::destack_fs_lsetxattr_bytes(runtime, path, name, value, flags) }
}

/// Set an extended attribute by handle.
pub fn destack_fs_fsetxattr(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    name: vm::StringHandle,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let name = string_ref_from_vm(runtime, context, name)?;
    let value = buffer_from_vm(runtime, context, value)?;
    unsafe { core_fs::destack_fs_fsetxattr(runtime, handle, name, value, flags) }
}

/// List extended attribute names by path.
pub fn destack_fs_listxattr(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let names = call_out(|out| unsafe { core_fs::destack_fs_listxattr(runtime, out, path) })?;
    string_array_to_vm(context, names)
}

/// List extended attribute names by byte path.
pub fn destack_fs_listxattr_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    let names = call_out(|out| unsafe { os_fs::destack_fs_listxattr_bytes(runtime, out, path) })?;
    string_array_to_vm(context, names)
}

/// List extended attribute names without following symlinks.
pub fn destack_fs_llistxattr(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let names = call_out(|out| unsafe { core_fs::destack_fs_llistxattr(runtime, out, path) })?;
    string_array_to_vm(context, names)
}

/// List extended attribute names by byte path without following symlinks.
pub fn destack_fs_llistxattr_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    let names = call_out(|out| unsafe { os_fs::destack_fs_llistxattr_bytes(runtime, out, path) })?;
    string_array_to_vm(context, names)
}

/// List extended attribute names by handle.
pub fn destack_fs_flistxattr(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let names = call_out(|out| unsafe { core_fs::destack_fs_flistxattr(runtime, out, handle) })?;
    string_array_to_vm(context, names)
}

/// Remove an extended attribute by path.
pub fn destack_fs_removexattr(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    unsafe { core_fs::destack_fs_removexattr(runtime, path, name) }
}

/// Remove an extended attribute by byte path.
pub fn destack_fs_removexattr_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    unsafe { os_fs::destack_fs_removexattr_bytes(runtime, path, name) }
}

/// Remove an extended attribute without following symlinks.
pub fn destack_fs_lremovexattr(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    unsafe { core_fs::destack_fs_lremovexattr(runtime, path, name) }
}

/// Remove an extended attribute by byte path without following symlinks.
pub fn destack_fs_lremovexattr_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    let name = string_ref_from_vm(runtime, context, name)?;
    unsafe { os_fs::destack_fs_lremovexattr_bytes(runtime, path, name) }
}

/// Remove an extended attribute by handle.
pub fn destack_fs_fremovexattr(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    name: vm::StringHandle,
) -> RuntimeResult<()> {
    let name = string_ref_from_vm(runtime, context, name)?;
    unsafe { core_fs::destack_fs_fremovexattr(runtime, handle, name) }
}

/// Create a file-backed memory mapping.
pub fn destack_fs_mmap_file(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _handle: FileHandle,
    _offset: FileOffset,
    _length: FileSize,
    _prot: MmapProt,
    _flags: MmapFlags,
) -> RuntimeResult<VmSlice<u8>> {
    // NOTE #Incomplete: implement VM-safe mmap by mapping into shared/foreign memory
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmapFile")).boxed())
}

/// Create an anonymous memory mapping.
pub fn destack_fs_mmap_anonymous(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _length: FileSize,
    _prot: MmapProt,
    _flags: MmapFlags,
) -> RuntimeResult<VmSlice<u8>> {
    // NOTE #Incomplete: implement VM-safe mmap by mapping into shared/foreign memory
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mmapAnonymous")).boxed())
}

/// Unmap a memory region.
pub fn destack_fs_munmap(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _mapping: VmSlice<u8>,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement VM-safe mmap by mapping into shared/foreign memory
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.munmap")).boxed())
}

/// Change memory protection for a mapping.
pub fn destack_fs_mprotect(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _mapping: VmSlice<u8>,
    _prot: MmapProt,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement VM-safe mmap by mapping into shared/foreign memory
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.mprotect")).boxed())
}

/// Flush a mapping to storage.
pub fn destack_fs_msync(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _mapping: VmSlice<u8>,
    _flags: MmapSyncFlags,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement VM-safe mmap by mapping into shared/foreign memory
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.msync")).boxed())
}

/// Advise the kernel about access patterns.
pub fn destack_fs_madvise(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    _mapping: VmSlice<u8>,
    _advice: MmapAdvice,
) -> RuntimeResult<()> {
    // NOTE #Incomplete: implement VM-safe mmap by mapping into shared/foreign memory
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.madvise")).boxed())
}

fn path_bytes_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<PathBytes> {
    let bytes = path.0.read_bytes(context)?;
    Ok(PathBytesAbi::<NativeAbi>(runtime.store_array(bytes)))
}

fn path_utf16_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
) -> RuntimeResult<PathUtf16> {
    let units = path.0.read_values(context)?;
    Ok(PathUtf16Abi::<NativeAbi>(runtime.store_array(units)))
}

fn path_ref_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<OsPath> {
    match path.encoding {
        PathEncoding::Bytes => {
            let bytes = path_bytes_from_vm(runtime, context, path.bytes)?;
            Ok(core_fs::path_ref_from_bytes(bytes))
        }
        PathEncoding::Utf16 => {
            let utf16 = path_utf16_from_vm(runtime, context, path.utf16)?;
            Ok(core_fs::path_ref_from_utf16(utf16))
        }
    }
}

fn path_bytes_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytes,
) -> RuntimeResult<PathBytesVm> {
    let bytes = unsafe { path.0.as_slice()? };
    let array = VmArray::from_bytes(context, bytes);
    Ok(PathBytesAbi::<VmAbi>(array))
}

fn path_utf16_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16,
) -> RuntimeResult<PathUtf16Vm> {
    let units = unsafe { path.0.as_slice()? };
    let array = VmArray::from_values(context, units)?;
    Ok(PathUtf16Abi::<VmAbi>(array))
}

fn path_ref_to_vm(context: &mut vm::RuntimeContext<'_>, path: OsPath) -> RuntimeResult<OsPathVm> {
    match path.encoding {
        PathEncoding::Bytes => {
            let bytes = path_bytes_to_vm(context, path.bytes)?;
            let utf16_inner = VmArray::from_values(context, &[])?;
            let utf16 = PathUtf16Abi::<VmAbi>(utf16_inner);
            Ok(OsPathVm {
                encoding: PathEncoding::Bytes,
                bytes,
                utf16,
            })
        }
        PathEncoding::Utf16 => {
            let utf16 = path_utf16_to_vm(context, path.utf16)?;
            let bytes_inner = VmArray::from_bytes(context, &[]);
            let bytes = PathBytesAbi::<VmAbi>(bytes_inner);
            Ok(OsPathVm {
                encoding: PathEncoding::Utf16,
                bytes,
                utf16,
            })
        }
    }
}

fn string_ref_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<NativeStringRef> {
    let string_ref = context
        .string_ref(value)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    Ok(runtime.store_string(string_ref.as_str()))
}

fn array_u8_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    array: NativeArray<u8>,
) -> RuntimeResult<VmArray<u8>> {
    let bytes = unsafe { array.as_slice()? };
    Ok(VmArray::from_bytes(context, bytes))
}

fn string_array_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    array: NativeArray<NativeStringRef>,
) -> RuntimeResult<VmArray<vm::StringHandle>> {
    let names = unsafe { array.as_slice()? };
    let mut handles = Vec::with_capacity(names.len());
    for name in names {
        let value = unsafe { name.as_str()? };
        let handle = vm::StringHandle::new(context.intern_string(value));
        handles.push(handle);
    }
    VmArray::from_values(context, &handles)
}

fn buffer_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<NativeSlice<u8>> {
    let bytes = buffer.read_bytes(context)?;
    Ok(runtime.store_slice(bytes))
}

fn allocate_read_buffer(runtime: &RuntimeCallContext, buffer: VmSlice<u8>) -> NativeSlice<u8> {
    let length = buffer.len as usize;
    runtime.store_slice(vec![0u8; length])
}

fn write_read_buffer(
    context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
    native: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let bytes = unsafe { native.as_slice()? };
    buffer.write_bytes(context, bytes)
}

fn decode_buffer_slices(
    context: &mut vm::RuntimeContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<Vec<VmSlice<u8>>> {
    let values = buffers.raw_values(context)?;
    let mut decoded = Vec::with_capacity(values.len());
    for value in values {
        decoded.push(VmSlice::from_value(
            context,
            value,
            "buffers",
            "Slice<uint8>",
        )?);
    }
    Ok(decoded)
}

/// Allocate native read buffers for a set of VM slices.
#[allow(clippy::type_complexity)]
fn allocate_read_buffers(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<(NativeSlice<NativeSlice<u8>>, Vec<VmSlice<u8>>)> {
    let vm_buffers = decode_buffer_slices(context, buffers)?;
    let mut native_buffers = Vec::with_capacity(vm_buffers.len());
    for buffer in vm_buffers.iter() {
        let length = buffer.len as usize;
        native_buffers.push(runtime.store_slice(vec![0u8; length]));
    }
    let native_slice = runtime.store_slice(native_buffers);

    Ok((native_slice, vm_buffers))
}

/// Copy native buffer data back into VM slices.
fn write_read_buffers(
    context: &mut vm::RuntimeContext<'_>,
    vm_buffers: Vec<VmSlice<u8>>,
    native_buffers: NativeSlice<NativeSlice<u8>>,
) -> RuntimeResult<()> {
    let native_buffers = unsafe { native_buffers.as_slice()? };
    if native_buffers.len() != vm_buffers.len() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "buffers",
            "buffer length mismatch",
        ))
        .boxed());
    }

    for (vm_buffer, native_buffer) in vm_buffers.into_iter().zip(native_buffers.iter()) {
        let bytes = unsafe { native_buffer.as_slice()? };
        vm_buffer.write_bytes(context, bytes)?;
    }

    Ok(())
}

/// Convert VM slice buffers into native slices.
fn buffers_from_vm(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<NativeSlice<NativeSlice<u8>>> {
    let vm_buffers = decode_buffer_slices(context, buffers)?;
    let mut native_buffers = Vec::with_capacity(vm_buffers.len());
    for buffer in vm_buffers {
        let bytes = buffer.read_bytes(context)?;
        native_buffers.push(runtime.store_slice(bytes));
    }

    Ok(runtime.store_slice(native_buffers))
}

fn dirent_array_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    entries: NativeArray<Dirent>,
) -> RuntimeResult<VmArray<DirentVm>> {
    let entries = unsafe { entries.as_slice()? };
    let mut values = Vec::with_capacity(entries.len());
    for entry in entries {
        let name = path_ref_to_vm(context, entry.name)?;
        let name_value = path_ref_vm_to_value(context, name);
        let value = DirentVm {
            name,
            kind: entry.kind,
        };
        let encoded = context.allocate_aggregate(vec![
            name_value,
            vm::Value::uint(value.kind as u8 as u64, 8),
        ]);
        values.push(encoded);
    }

    let data = context.allocate_raw_values(values);
    Ok(VmArray {
        data,
        len: entries.len() as u32,
        capacity: entries.len() as u32,
        _marker: std::marker::PhantomData,
    })
}

fn dirent_next_to_vm(
    context: &mut vm::RuntimeContext<'_>,
    value: DirentNext,
) -> RuntimeResult<DirentNextVm> {
    let name = path_ref_to_vm(context, value.entry.name)?;
    let entry = DirentVm {
        name,
        kind: value.entry.kind,
    };

    Ok(DirentNextVm {
        has_entry: value.has_entry,
        entry,
    })
}

fn path_ref_vm_to_value(context: &mut vm::RuntimeContext<'_>, value: OsPathVm) -> vm::Value {
    let bytes_value = value.bytes.0.to_value(context);
    let utf16_value = value.utf16.0.to_value(context);
    context.allocate_aggregate(vec![
        vm::Value::uint(value.encoding as u8 as u64, 8),
        bytes_value,
        utf16_value,
    ])
}
