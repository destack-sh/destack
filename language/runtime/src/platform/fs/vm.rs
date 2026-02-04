use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::core as core_fs;

use crate::platform::abi::{NativeAbi, VmAbi};
use crate::platform::fs::{
    AccessMode, AtFlags, DirectoryHandle, Dirent, DirentVm, FileHandle, FileLockFlags, FileMode,
    FileOffset, OpenFlags, PathBytes, PathBytesAbi, PathBytesVm, PathUtf16, PathUtf16Abi,
    PathUtf16Vm, Stat, StatFs, SymlinkType,
};
use crate::platform::{NativeArray, NativeSlice, PlatformError, VmArray, VmSlice};
use crate::runtime::RuntimeCallContext;

/// Check filesystem access for a byte path.
pub fn destack_fs_access_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_access_bytes(runtime, path, mode) }
}

/// Check filesystem access for a UTF-16 path.
pub fn destack_fs_access_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let path = path_utf16_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_access_utf16(runtime, path, mode) }
}

/// Change permissions for a byte path.
pub fn destack_fs_chmod_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_chmod_bytes(runtime, path, mode) }
}

/// Change permissions for a UTF-16 path.
pub fn destack_fs_chmod_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_utf16_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_chmod_utf16(runtime, path, mode) }
}

/// Change ownership for a byte path.
pub fn destack_fs_chown_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_chown_bytes(runtime, path, uid, gid) }
}

/// Change ownership for a UTF-16 path.
pub fn destack_fs_chown_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let path = path_utf16_from_vm(runtime, context, path)?;
    unsafe { core_fs::destack_fs_chown_utf16(runtime, path, uid, gid) }
}

/// Close a file handle.
pub fn destack_fs_close(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_close(runtime, handle) }
}

/// Close a directory handle.
pub fn destack_fs_closedir(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_closedir(runtime, handle) }
}

/// Copy a file from one byte path to another.
pub fn destack_fs_copyfile_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from: PathBytesVm,
    to: PathBytesVm,
    flags: u32,
) -> RuntimeResult<()> {
    let from = path_bytes_from_vm(runtime, context, from)?;
    let to = path_bytes_from_vm(runtime, context, to)?;
    unsafe { core_fs::destack_fs_copyfile_bytes(runtime, from, to, flags) }
}

/// Copy a file from one UTF-16 path to another.
pub fn destack_fs_copyfile_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from: PathUtf16Vm,
    to: PathUtf16Vm,
    flags: u32,
) -> RuntimeResult<()> {
    let from = path_utf16_from_vm(runtime, context, from)?;
    let to = path_utf16_from_vm(runtime, context, to)?;
    unsafe { core_fs::destack_fs_copyfile_utf16(runtime, from, to, flags) }
}

/// Change permissions for an open file.
pub fn destack_fs_fchmod(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_fchmod(runtime, handle, mode) }
}

/// Change ownership for an open file.
pub fn destack_fs_fchown(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_fchown(runtime, handle, uid, gid) }
}

/// Flush file data to disk.
pub fn destack_fs_fdatasync(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_fdatasync(runtime, handle) }
}

/// Stat an open file.
pub fn destack_fs_fstat(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<Stat> {
    call_out(|out| unsafe { core_fs::destack_fs_fstat(runtime, out, handle) })
}

/// Statfs an open file.
pub fn destack_fs_fstatfs(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<StatFs> {
    call_out(|out| unsafe { core_fs::destack_fs_fstatfs(runtime, out, handle) })
}

/// Flush file data and metadata to disk.
pub fn destack_fs_fsync(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_fsync(runtime, handle) }
}

/// Truncate an open file.
pub fn destack_fs_ftruncate(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_ftruncate(runtime, handle, size) }
}

/// Update times for an open file.
pub fn destack_fs_futimes(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_futimes(runtime, handle, atime_ns, mtime_ns) }
}

/// Link two byte paths.
pub fn destack_fs_link_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from: PathBytesVm,
    to: PathBytesVm,
) -> RuntimeResult<()> {
    let from = path_bytes_from_vm(runtime, context, from)?;
    let to = path_bytes_from_vm(runtime, context, to)?;

    unsafe { core_fs::destack_fs_link_bytes(runtime, from, to) }
}

/// Link two UTF-16 paths.
pub fn destack_fs_link_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from: PathUtf16Vm,
    to: PathUtf16Vm,
) -> RuntimeResult<()> {
    let from = path_utf16_from_vm(runtime, context, from)?;
    let to = path_utf16_from_vm(runtime, context, to)?;

    unsafe { core_fs::destack_fs_link_utf16(runtime, from, to) }
}

/// Lstat a byte path.
pub fn destack_fs_lstat_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<Stat> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    call_out(|out| unsafe { core_fs::destack_fs_lstat_bytes(runtime, out, path) })
}

/// Lstat a UTF-16 path.
pub fn destack_fs_lstat_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
) -> RuntimeResult<Stat> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    call_out(|out| unsafe { core_fs::destack_fs_lstat_utf16(runtime, out, path) })
}

/// Update times for a byte path without following symlinks.
pub fn destack_fs_lutimes_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_lutimes_bytes(runtime, path, atime_ns, mtime_ns) }
}

/// Update times for a UTF-16 path without following symlinks.
pub fn destack_fs_lutimes_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_lutimes_utf16(runtime, path, atime_ns, mtime_ns) }
}

/// Create a temporary directory with a byte template.
pub fn destack_fs_mkdtemp_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    template: PathBytesVm,
) -> RuntimeResult<PathBytesVm> {
    let template = path_bytes_from_vm(runtime, context, template)?;
    let path =
        call_out(|out| unsafe { core_fs::destack_fs_mkdtemp_bytes(runtime, out, template) })?;
    path_bytes_to_vm(context, path)
}

/// Create a temporary directory with a UTF-16 template.
pub fn destack_fs_mkdtemp_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    template: PathUtf16Vm,
) -> RuntimeResult<PathUtf16Vm> {
    let template = path_utf16_from_vm(runtime, context, template)?;
    let path =
        call_out(|out| unsafe { core_fs::destack_fs_mkdtemp_utf16(runtime, out, template) })?;
    path_utf16_to_vm(context, path)
}

/// Open a file with a byte path.
pub fn destack_fs_open_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<FileHandle> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_fs::destack_fs_open_bytes(runtime, out, path, flags, mode) })
}

/// Open a file with a UTF-16 path.
pub fn destack_fs_open_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<FileHandle> {
    let path = path_utf16_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_fs::destack_fs_open_utf16(runtime, out, path, flags, mode) })
}

/// Open a directory with a byte path.
pub fn destack_fs_opendir_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<DirectoryHandle> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    call_out(|out| unsafe { core_fs::destack_fs_opendir_bytes(runtime, out, path) })
}

/// Open a directory with a UTF-16 path.
pub fn destack_fs_opendir_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
) -> RuntimeResult<DirectoryHandle> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    call_out(|out| unsafe { core_fs::destack_fs_opendir_utf16(runtime, out, path) })
}

/// Read from an open file into a buffer.
pub fn destack_fs_read(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let native_buffer = allocate_read_buffer(runtime, buffer);
    let bytes_read = call_out(|out| unsafe {
        core_fs::destack_fs_read(runtime, out, handle, native_buffer, offset)
    })?;
    write_read_buffer(context, buffer, native_buffer)?;

    Ok(bytes_read)
}

/// Read directory entries.
pub fn destack_fs_readdir(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<VmArray<DirentVm>> {
    let native_entries =
        call_out(|out| unsafe { core_fs::destack_fs_readdir(runtime, out, handle) })?;

    dirent_array_to_vm(context, native_entries)
}

/// Read a symlink target for a byte path.
pub fn destack_fs_readlink_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<PathBytesVm> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    let target = call_out(|out| unsafe { core_fs::destack_fs_readlink_bytes(runtime, out, path) })?;

    path_bytes_to_vm(context, target)
}

/// Read a symlink target for a UTF-16 path.
pub fn destack_fs_readlink_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
) -> RuntimeResult<PathUtf16Vm> {
    let path = path_utf16_from_vm(runtime, context, path)?;
    let target = call_out(|out| unsafe { core_fs::destack_fs_readlink_utf16(runtime, out, path) })?;

    path_utf16_to_vm(context, target)
}

/// Read from an open file into multiple buffers.
pub fn destack_fs_readv(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let (native_buffers, vm_buffers) = allocate_read_buffers(runtime, context, buffers)?;
    let bytes_read = call_out(|out| unsafe {
        core_fs::destack_fs_readv(runtime, out, handle, native_buffers, offset)
    })?;
    write_read_buffers(context, vm_buffers, native_buffers)?;

    Ok(bytes_read)
}

/// Resolve the real path for a byte path.
pub fn destack_fs_realpath_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<PathBytesVm> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    let resolved =
        call_out(|out| unsafe { core_fs::destack_fs_realpath_bytes(runtime, out, path) })?;

    path_bytes_to_vm(context, resolved)
}

/// Resolve the real path for a UTF-16 path.
pub fn destack_fs_realpath_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
) -> RuntimeResult<PathUtf16Vm> {
    let path = path_utf16_from_vm(runtime, context, path)?;
    let resolved =
        call_out(|out| unsafe { core_fs::destack_fs_realpath_utf16(runtime, out, path) })?;

    path_utf16_to_vm(context, resolved)
}

/// Rename a byte path.
pub fn destack_fs_rename_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from: PathBytesVm,
    to: PathBytesVm,
) -> RuntimeResult<()> {
    let from = path_bytes_from_vm(runtime, context, from)?;
    let to = path_bytes_from_vm(runtime, context, to)?;

    unsafe { core_fs::destack_fs_rename_bytes(runtime, from, to) }
}

/// Rename a UTF-16 path.
pub fn destack_fs_rename_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from: PathUtf16Vm,
    to: PathUtf16Vm,
) -> RuntimeResult<()> {
    let from = path_utf16_from_vm(runtime, context, from)?;
    let to = path_utf16_from_vm(runtime, context, to)?;

    unsafe { core_fs::destack_fs_rename_utf16(runtime, from, to) }
}

/// Remove a directory for a byte path.
pub fn destack_fs_rmdir_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_rmdir_bytes(runtime, path) }
}

/// Remove a directory for a UTF-16 path.
pub fn destack_fs_rmdir_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
) -> RuntimeResult<()> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_rmdir_utf16(runtime, path) }
}

/// Stat a byte path.
pub fn destack_fs_stat_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<Stat> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    call_out(|out| unsafe { core_fs::destack_fs_stat_bytes(runtime, out, path) })
}

/// Stat a UTF-16 path.
pub fn destack_fs_stat_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
) -> RuntimeResult<Stat> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    call_out(|out| unsafe { core_fs::destack_fs_stat_utf16(runtime, out, path) })
}

/// Statfs a byte path.
pub fn destack_fs_statfs_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<StatFs> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    call_out(|out| unsafe { core_fs::destack_fs_statfs_bytes(runtime, out, path) })
}

/// Statfs a UTF-16 path.
pub fn destack_fs_statfs_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
) -> RuntimeResult<StatFs> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    call_out(|out| unsafe { core_fs::destack_fs_statfs_utf16(runtime, out, path) })
}

/// Create a symlink using byte paths.
pub fn destack_fs_symlink_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    target: PathBytesVm,
    link: PathBytesVm,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let target = path_bytes_from_vm(runtime, context, target)?;
    let link = path_bytes_from_vm(runtime, context, link)?;

    unsafe { core_fs::destack_fs_symlink_bytes(runtime, target, link, kind) }
}

/// Create a symlink using UTF-16 paths.
pub fn destack_fs_symlink_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    target: PathUtf16Vm,
    link: PathUtf16Vm,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let target = path_utf16_from_vm(runtime, context, target)?;
    let link = path_utf16_from_vm(runtime, context, link)?;

    unsafe { core_fs::destack_fs_symlink_utf16(runtime, target, link, kind) }
}

/// Truncate a byte path.
pub fn destack_fs_truncate_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    size: FileOffset,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_truncate_bytes(runtime, path, size) }
}

/// Truncate a UTF-16 path.
pub fn destack_fs_truncate_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
    size: FileOffset,
) -> RuntimeResult<()> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_truncate_utf16(runtime, path, size) }
}

/// Remove a byte path.
pub fn destack_fs_unlink_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_unlink_bytes(runtime, path) }
}

/// Remove a UTF-16 path.
pub fn destack_fs_unlink_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
) -> RuntimeResult<()> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_unlink_utf16(runtime, path) }
}

/// Update times for a byte path.
pub fn destack_fs_utimes_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_utimes_bytes(runtime, path, atime_ns, mtime_ns) }
}

/// Update times for a UTF-16 path.
pub fn destack_fs_utimes_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_utimes_utf16(runtime, path, atime_ns, mtime_ns) }
}

/// Write to an open file from a buffer.
pub fn destack_fs_write(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let native_buffer = buffer_from_vm(runtime, context, buffer)?;

    call_out(|out| unsafe {
        core_fs::destack_fs_write(runtime, out, handle, native_buffer, offset)
    })
}

/// Write to an open file from multiple buffers.
pub fn destack_fs_writev(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let native_buffers = buffers_from_vm(runtime, context, buffers)?;

    call_out(|out| unsafe {
        core_fs::destack_fs_writev(runtime, out, handle, native_buffers, offset)
    })
}

/// Open a file relative to a directory with a byte path.
pub fn destack_fs_openat_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: PathBytesVm,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<FileHandle> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    call_out(|out| unsafe {
        core_fs::destack_fs_openat_bytes(runtime, out, dir, path, flags, mode)
    })
}

/// Open a file relative to a directory with a UTF-16 path.
pub fn destack_fs_openat_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: PathUtf16Vm,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<FileHandle> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    call_out(|out| unsafe {
        core_fs::destack_fs_openat_utf16(runtime, out, dir, path, flags, mode)
    })
}

/// Create a directory from a byte path.
pub fn destack_fs_mkdir_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathBytesVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_mkdir_bytes(runtime, path, mode) }
}

/// Create a directory from a UTF-16 path.
pub fn destack_fs_mkdir_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    path: PathUtf16Vm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_mkdir_utf16(runtime, path, mode) }
}

/// Create a directory relative to a directory handle with a byte path.
pub fn destack_fs_mkdirat_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: PathBytesVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_mkdirat_bytes(runtime, dir, path, mode) }
}

/// Create a directory relative to a directory handle with a UTF-16 path.
pub fn destack_fs_mkdirat_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: PathUtf16Vm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_mkdirat_utf16(runtime, dir, path, mode) }
}

/// Rename a path relative to a directory handle with byte paths.
pub fn destack_fs_renameat_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from_dir: DirectoryHandle,
    from_path: PathBytesVm,
    to_dir: DirectoryHandle,
    to_path: PathBytesVm,
) -> RuntimeResult<()> {
    let from_path = path_bytes_from_vm(runtime, context, from_path)?;
    let to_path = path_bytes_from_vm(runtime, context, to_path)?;

    unsafe { core_fs::destack_fs_renameat_bytes(runtime, from_dir, from_path, to_dir, to_path) }
}

/// Rename a path relative to a directory handle with UTF-16 paths.
pub fn destack_fs_renameat_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from_dir: DirectoryHandle,
    from_path: PathUtf16Vm,
    to_dir: DirectoryHandle,
    to_path: PathUtf16Vm,
) -> RuntimeResult<()> {
    let from_path = path_utf16_from_vm(runtime, context, from_path)?;
    let to_path = path_utf16_from_vm(runtime, context, to_path)?;

    unsafe { core_fs::destack_fs_renameat_utf16(runtime, from_dir, from_path, to_dir, to_path) }
}

/// Remove a path relative to a directory handle with a byte path.
pub fn destack_fs_unlinkat_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: PathBytesVm,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_unlinkat_bytes(runtime, dir, path, flags) }
}

/// Remove a path relative to a directory handle with a UTF-16 path.
pub fn destack_fs_unlinkat_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: PathUtf16Vm,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    unsafe { core_fs::destack_fs_unlinkat_utf16(runtime, dir, path, flags) }
}

/// Link paths relative to directory handles with byte paths.
pub fn destack_fs_linkat_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from_dir: DirectoryHandle,
    from_path: PathBytesVm,
    to_dir: DirectoryHandle,
    to_path: PathBytesVm,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let from_path = path_bytes_from_vm(runtime, context, from_path)?;
    let to_path = path_bytes_from_vm(runtime, context, to_path)?;

    unsafe {
        core_fs::destack_fs_linkat_bytes(runtime, from_dir, from_path, to_dir, to_path, flags)
    }
}

/// Link paths relative to directory handles with UTF-16 paths.
pub fn destack_fs_linkat_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    from_dir: DirectoryHandle,
    from_path: PathUtf16Vm,
    to_dir: DirectoryHandle,
    to_path: PathUtf16Vm,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let from_path = path_utf16_from_vm(runtime, context, from_path)?;
    let to_path = path_utf16_from_vm(runtime, context, to_path)?;

    unsafe {
        core_fs::destack_fs_linkat_utf16(runtime, from_dir, from_path, to_dir, to_path, flags)
    }
}

/// Create a symlink relative to directory handles with byte paths.
pub fn destack_fs_symlinkat_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    target: PathBytesVm,
    dir: DirectoryHandle,
    link: PathBytesVm,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let target = path_bytes_from_vm(runtime, context, target)?;
    let link = path_bytes_from_vm(runtime, context, link)?;

    unsafe { core_fs::destack_fs_symlinkat_bytes(runtime, target, dir, link, kind) }
}

/// Create a symlink relative to directory handles with UTF-16 paths.
pub fn destack_fs_symlinkat_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    target: PathUtf16Vm,
    dir: DirectoryHandle,
    link: PathUtf16Vm,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let target = path_utf16_from_vm(runtime, context, target)?;
    let link = path_utf16_from_vm(runtime, context, link)?;

    unsafe { core_fs::destack_fs_symlinkat_utf16(runtime, target, dir, link, kind) }
}

/// Read a symlink relative to a directory handle with a byte path.
pub fn destack_fs_readlinkat_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: PathBytesVm,
) -> RuntimeResult<PathBytesVm> {
    let path = path_bytes_from_vm(runtime, context, path)?;
    let target =
        call_out(|out| unsafe { core_fs::destack_fs_readlinkat_bytes(runtime, out, dir, path) })?;

    path_bytes_to_vm(context, target)
}

/// Read a symlink relative to a directory handle with a UTF-16 path.
pub fn destack_fs_readlinkat_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: PathUtf16Vm,
) -> RuntimeResult<PathUtf16Vm> {
    let path = path_utf16_from_vm(runtime, context, path)?;
    let target =
        call_out(|out| unsafe { core_fs::destack_fs_readlinkat_utf16(runtime, out, dir, path) })?;

    path_utf16_to_vm(context, target)
}

/// Stat a path relative to a directory handle with a byte path.
pub fn destack_fs_statat_bytes(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: PathBytesVm,
    flags: AtFlags,
) -> RuntimeResult<Stat> {
    let path = path_bytes_from_vm(runtime, context, path)?;

    call_out(|out| unsafe { core_fs::destack_fs_statat_bytes(runtime, out, dir, path, flags) })
}

/// Stat a path relative to a directory handle with a UTF-16 path.
pub fn destack_fs_statat_utf16(
    runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    dir: DirectoryHandle,
    path: PathUtf16Vm,
    flags: AtFlags,
) -> RuntimeResult<Stat> {
    let path = path_utf16_from_vm(runtime, context, path)?;

    call_out(|out| unsafe { core_fs::destack_fs_statat_utf16(runtime, out, dir, path, flags) })
}

/// Lock an open file.
pub fn destack_fs_lock(
    runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    unsafe { core_fs::destack_fs_lock(runtime, handle, flags) }
}

fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    let mut value = std::mem::MaybeUninit::<T>::uninit();
    call(value.as_mut_ptr())?;
    Ok(unsafe { value.assume_init() })
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
        let name = unsafe { entry.name.as_str()? };
        let name_value = context.intern_string(name);
        let name_handle = vm::StringHandle::new(name_value);
        let value = DirentVm {
            name: name_handle,
            kind: entry.kind,
        };
        let encoded = context.allocate_aggregate(vec![
            value.name.value(),
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
