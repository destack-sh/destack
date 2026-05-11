use std::collections::HashMap;
use std::sync::Arc;

use destack_vm;
use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{
    allocate_vm_read_buffer as allocate_read_buffer,
    allocate_vm_read_buffers as allocate_read_buffers, bytes_array_array_to_vm, bytes_array_to_vm,
    call_out, intern_string_to_vm as string_ref_to_vm, map_native_array_to_vm,
    os_path_to_vm as path_ref_to_vm, store_bytes_from_vm,
    store_os_path_from_vm as path_ref_from_vm, store_string_from_vm as string_ref_from_vm,
    store_vm_byte_slices as buffers_from_vm, string_array_to_vm,
    write_vm_read_buffer as write_read_buffer, write_vm_read_buffers as write_read_buffers,
};
use crate::platform::fs::core::{decode_mmap_flags, validate_mapping_length};
use crate::platform::fs::{
    AccessMode, AllocFlags, AtFlags, CopyFlags, DirectoryHandle, DirentNext, DirentNextEndVm,
    DirentNextEntryVm, DirentNextVm, DirentVm, FdFlags, FileAdvice, FileHandle, FileLockFlags,
    FileMode, FileOffset, FileSize, MmapAdvice, MmapFlags, MmapProt, MmapSyncFlags, NodeDevice,
    OpenFlags, OpenOptions, OpenOptionsVm, OsPath, OsPathVm, ReadWriteFlags, RenameFlags,
    SeekWhence, SpliceCursor, SpliceFlags, Stat, StatFs, StatusFlags, Statx, StatxFlags, StatxMask,
    SymlinkType, SyncFlags, WatchBatchVm, WatchCreateEventVm, WatchEvent, WatchEventMetadataVm,
    WatchEventVm, WatchMetadataEventVm, WatchModifyEventVm, WatchOptions, WatchOptionsVm,
    WatchOverflowEventVm, WatchRemoveEventVm, WatchRenameEventVm, XattrFlags, host as host_fs,
};
use crate::platform::resource::{PipeHandle, ResourceId, SocketHandle, WatchHandle};
use crate::platform::{PlatformError, VmArray, VmSlice};
use crate::runtime::BindingCallContext;

/// Runtime-owned metadata for one VM mmap allocation.
#[derive(Debug, Clone, Copy)]
struct VmMappingEntry {
    /// Backing file handle for file mappings.
    handle: Option<FileHandle>,
    /// Starting file offset for file-backed mappings.
    offset: FileOffset,
    /// Current protection mask.
    prot: MmapProt,
    /// Whether writes should sync back to the file.
    is_shared: bool,
}

/// Runtime-owned mutable state for vm mmap mappings.
#[derive(Debug, Default)]
pub(crate) struct VmMmapRuntimeState {
    /// Mapping metadata keyed by VM raw-pointer offset.
    mappings: Mutex<HashMap<u64, VmMappingEntry>>,
}

/// Return runtime-owned vm mmap mutable state.
fn vm_mmap_runtime_state(binding: &BindingCallContext) -> Arc<VmMmapRuntimeState> {
    binding
        .worker()
        .platform_state
        .fs
        .vm_mmap_runtime_state(VmMmapRuntimeState::default)
}

/// Build a stable vm mapping key from one slice.
fn vm_mapping_key(mapping: VmSlice<u8>) -> u64 {
    let data = mapping.data.as_raw_pointer();

    data.offset() as u64
}

/// Validate common mmap flags.
fn validate_vm_mmap_flags(flags: MmapFlags, allow_anonymous: bool) -> RuntimeResult<bool> {
    let flags = decode_mmap_flags(flags, allow_anonymous)?;

    // reject fixed-address mappings in the vm backend
    if flags.is_fixed {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.mmap MAP_FIXED")).boxed(),
        );
    }

    Ok(flags.is_shared)
}

/// Validate one vm mapping length.
fn validate_vm_mmap_length(length: FileSize) -> RuntimeResult<usize> {
    validate_mapping_length(length)
}

/// Check file access permissions.
pub fn destack_fs_access(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    mode: AccessMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_access(binding, path, mode) }
}

/// Check file access permissions relative to a directory handle.
pub fn destack_fs_accessat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: AccessMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_accessat(binding, dir, path, mode, flags) }
}

/// Change file permissions.
pub fn destack_fs_chmod(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_chmod(binding, path, mode) }
}

/// Change file permissions relative to a directory handle.
pub fn destack_fs_fchmodat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_fchmodat(binding, dir, path, mode, flags) }
}

/// Change file owner and group.
pub fn destack_fs_chown(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_chown(binding, path, uid, gid) }
}

/// Change file owner and group relative to a directory handle.
pub fn destack_fs_fchownat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    uid: u32,
    gid: u32,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_fchownat(binding, dir, path, uid, gid, flags) }
}

/// Update access and modification times.
pub fn destack_fs_utimes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_utimes(binding, path, atime_ns, mtime_ns) }
}

/// Update access and modification times without following symlinks.
pub fn destack_fs_lutimes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_lutimes(binding, path, atime_ns, mtime_ns) }
}

/// Update access and modification times relative to a directory handle.
pub fn destack_fs_utimensat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    atime_ns: u64,
    mtime_ns: u64,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_utimensat(binding, dir, path, atime_ns, mtime_ns, flags) }
}

/// Create a directory.
pub fn destack_fs_mkdir(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_mkdir(binding, path, mode) }
}

/// Create a directory relative to a directory handle.
pub fn destack_fs_mkdirat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_mkdirat(binding, dir, path, mode) }
}

/// Remove a directory.
pub fn destack_fs_rmdir(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_rmdir(binding, path) }
}

/// Open a directory and return a handle.
pub fn destack_fs_opendir(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<DirectoryHandle> {
    let path = path_ref_from_vm(binding, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_opendir(binding, out, path) })
}

/// Create a temporary directory.
pub fn destack_fs_mkdtemp(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    template: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let template = path_ref_from_vm(binding, context, template)?;
    let path = call_out(|out| unsafe { host_fs::destack_fs_mkdtemp(binding, out, template) })?;
    path_ref_to_vm(context, path)
}

/// Open a file and return a handle.
pub fn destack_fs_open(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<FileHandle> {
    let path = path_ref_from_vm(binding, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_open(binding, out, path, flags, mode) })
}

/// Open a file relative to a directory handle.
pub fn destack_fs_openat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<FileHandle> {
    let path = path_ref_from_vm(binding, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_openat(binding, out, dir, path, flags, mode) })
}

/// Open a file relative to a directory handle with openat2 semantics.
pub fn destack_fs_openat2(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    how: OpenOptionsVm,
) -> RuntimeResult<FileHandle> {
    let path = path_ref_from_vm(binding, context, path)?;
    let how = OpenOptions {
        flags: how.flags,
        mode: how.mode,
        resolve: how.resolve,
    };
    call_out(|out| unsafe { host_fs::destack_fs_openat2(binding, out, dir, path, how) })
}

/// Close an open file handle.
pub fn destack_fs_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_close(binding, handle) }
}

/// Close a directory handle.
pub fn destack_fs_closedir(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_closedir(binding, handle) }
}

/// Read directory entries from an open directory handle.
pub fn destack_fs_readdir(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<VmArray<DirentVm>> {
    // read the host entries and map them into VM descriptors
    let entries = call_out(|out| unsafe { host_fs::destack_fs_readdir(binding, out, handle) })?;

    map_native_array_to_vm(context, entries, |context, entry| {
        let name = path_ref_to_vm(context, entry.name)?;

        Ok(DirentVm {
            name,
            kind: entry.kind,
        })
    })
}

/// Read a single directory entry from an open directory handle.
pub fn destack_fs_readdir_next(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<DirentNextVm> {
    // read one host entry and map it into the VM shape
    let entry = call_out(|out| unsafe { host_fs::destack_fs_readdir_next(binding, out, handle) })?;

    match entry {
        DirentNext::DirentNextEnd(end_value) => {
            let kind = string_ref_to_vm(context, end_value.kind)?;

            Ok(DirentNextVm::DirentNextEnd(DirentNextEndVm { kind }))
        }
        DirentNext::DirentNextEntry(entry_value) => {
            let kind = string_ref_to_vm(context, entry_value.kind)?;
            let name = path_ref_to_vm(context, entry_value.entry.name)?;
            let entry = DirentVm {
                name,
                kind: entry_value.entry.kind,
            };

            Ok(DirentNextVm::DirentNextEntry(DirentNextEntryVm {
                kind,
                entry,
            }))
        }
    }
}

/// Reset an open directory handle to the first entry.
pub fn destack_fs_rewinddir(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_rewinddir(binding, handle) }
}

/// Resolve the directory descriptor for an open directory handle.
pub fn destack_fs_dirfd(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: DirectoryHandle,
) -> RuntimeResult<FileHandle> {
    call_out(|out| unsafe { host_fs::destack_fs_dirfd(binding, out, handle) })
}

/// Read from a file into the provided slice.
pub fn destack_fs_read(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let native = allocate_read_buffer(binding, buffer);
    let count = call_out(|out| unsafe { host_fs::destack_fs_read(binding, out, handle, native) })?;
    write_read_buffer(context, buffer, native)?;
    Ok(count)
}

/// Read from a file at the given file offset.
pub fn destack_fs_pread(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let native = allocate_read_buffer(binding, buffer);
    let count =
        call_out(|out| unsafe { host_fs::destack_fs_pread(binding, out, handle, native, offset) })?;
    write_read_buffer(context, buffer, native)?;
    Ok(count)
}

/// Write to a file from the provided slice.
pub fn destack_fs_write(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
) -> RuntimeResult<u64> {
    let native = store_bytes_from_vm(binding, context, buffer)?;
    call_out(|out| unsafe { host_fs::destack_fs_write(binding, out, handle, native) })
}

/// Write to a file at the given file offset.
pub fn destack_fs_pwrite(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    buffer: VmSlice<u8>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let native = store_bytes_from_vm(binding, context, buffer)?;
    call_out(|out| unsafe { host_fs::destack_fs_pwrite(binding, out, handle, native, offset) })
}

/// Read into multiple buffers.
pub fn destack_fs_readv(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let (native_buffers, vm_buffers) = allocate_read_buffers(binding, context, buffers, "buffers")?;
    let count =
        call_out(|out| unsafe { host_fs::destack_fs_readv(binding, out, handle, native_buffers) })?;
    write_read_buffers(context, vm_buffers, native_buffers, "buffers")?;
    Ok(count)
}

/// Read into multiple buffers at the given file offset.
pub fn destack_fs_preadv(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let (native_buffers, vm_buffers) = allocate_read_buffers(binding, context, buffers, "buffers")?;
    let count = call_out(|out| unsafe {
        host_fs::destack_fs_preadv(binding, out, handle, native_buffers, offset)
    })?;
    write_read_buffers(context, vm_buffers, native_buffers, "buffers")?;
    Ok(count)
}

/// Read into multiple buffers at the given file offset with explicit read flags.
pub fn destack_fs_preadv2(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<u64> {
    let (native_buffers, vm_buffers) = allocate_read_buffers(binding, context, buffers, "buffers")?;
    let count = call_out(|out| unsafe {
        host_fs::destack_fs_preadv2(binding, out, handle, native_buffers, offset, flags)
    })?;
    write_read_buffers(context, vm_buffers, native_buffers, "buffers")?;
    Ok(count)
}

/// Write from multiple buffers.
pub fn destack_fs_writev(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
) -> RuntimeResult<u64> {
    let native_buffers = buffers_from_vm(binding, context, buffers, "buffers")?;
    call_out(|out| unsafe { host_fs::destack_fs_writev(binding, out, handle, native_buffers) })
}

/// Write from multiple buffers at the given file offset.
pub fn destack_fs_pwritev(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
) -> RuntimeResult<u64> {
    let native_buffers = buffers_from_vm(binding, context, buffers, "buffers")?;
    call_out(|out| unsafe {
        host_fs::destack_fs_pwritev(binding, out, handle, native_buffers, offset)
    })
}

/// Write from multiple buffers at the given file offset with explicit write flags.
pub fn destack_fs_pwritev2(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    buffers: VmSlice<VmSlice<u8>>,
    offset: FileOffset,
    flags: ReadWriteFlags,
) -> RuntimeResult<u64> {
    let native_buffers = buffers_from_vm(binding, context, buffers, "buffers")?;
    call_out(|out| unsafe {
        host_fs::destack_fs_pwritev2(binding, out, handle, native_buffers, offset, flags)
    })
}

/// Change file permissions by handle.
pub fn destack_fs_fchmod(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fchmod(binding, handle, mode) }
}

/// Change file owner and group by handle.
pub fn destack_fs_fchown(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fchown(binding, handle, uid, gid) }
}

/// Synchronize a file's in-core state with storage.
pub fn destack_fs_fsync(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fsync(binding, handle) }
}

/// Synchronize file data only.
pub fn destack_fs_fdatasync(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fdatasync(binding, handle) }
}

/// Truncate a file by handle.
pub fn destack_fs_ftruncate(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_ftruncate(binding, handle, size) }
}

/// Update access and modification times by handle.
pub fn destack_fs_futimes(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_futimes(binding, handle, atime_ns, mtime_ns) }
}

/// Stat a file by handle.
pub fn destack_fs_fstat(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<Stat> {
    call_out(|out| unsafe { host_fs::destack_fs_fstat(binding, out, handle) })
}

/// Stat a filesystem by handle.
pub fn destack_fs_fstatfs(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<StatFs> {
    call_out(|out| unsafe { host_fs::destack_fs_fstatfs(binding, out, handle) })
}

/// Apply file locks to a file handle.
pub fn destack_fs_lock(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    flags: FileLockFlags,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_lock(binding, handle, flags) }
}

/// Read file descriptor flags.
pub fn destack_fs_get_fd_flags(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<FdFlags> {
    call_out(|out| unsafe { host_fs::destack_fs_get_fd_flags(binding, out, handle) })
}

/// Read file status flags.
pub fn destack_fs_get_status_flags(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<StatusFlags> {
    call_out(|out| unsafe { host_fs::destack_fs_get_status_flags(binding, out, handle) })
}

/// Write file descriptor flags.
pub fn destack_fs_set_fd_flags(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    flags: FdFlags,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_set_fd_flags(binding, handle, flags) }
}

/// Write file status flags.
pub fn destack_fs_set_status_flags(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    flags: StatusFlags,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_set_status_flags(binding, handle, flags) }
}

/// Truncate a file.
pub fn destack_fs_truncate(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    size: FileOffset,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_truncate(binding, path, size) }
}

/// Rename or move a file.
pub fn destack_fs_rename(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    from: OsPathVm,
    to: OsPathVm,
) -> RuntimeResult<()> {
    let from = path_ref_from_vm(binding, context, from)?;
    let to = path_ref_from_vm(binding, context, to)?;
    unsafe { host_fs::destack_fs_rename(binding, from, to) }
}

/// Rename or move a file relative to directory handles.
pub fn destack_fs_renameat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    from_dir: DirectoryHandle,
    from: OsPathVm,
    to_dir: DirectoryHandle,
    to: OsPathVm,
) -> RuntimeResult<()> {
    let from = path_ref_from_vm(binding, context, from)?;
    let to = path_ref_from_vm(binding, context, to)?;
    unsafe { host_fs::destack_fs_renameat(binding, from_dir, from, to_dir, to) }
}

/// Rename or move a file relative to directory handles with renameat2 semantics.
pub fn destack_fs_renameat2(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    from_dir: DirectoryHandle,
    from: OsPathVm,
    to_dir: DirectoryHandle,
    to: OsPathVm,
    flags: RenameFlags,
) -> RuntimeResult<()> {
    let from = path_ref_from_vm(binding, context, from)?;
    let to = path_ref_from_vm(binding, context, to)?;
    unsafe { host_fs::destack_fs_renameat2(binding, from_dir, from, to_dir, to, flags) }
}

/// Unlink a file.
pub fn destack_fs_unlink(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_unlink(binding, path) }
}

/// Unlink a file relative to a directory handle.
pub fn destack_fs_unlinkat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_unlinkat(binding, dir, path, flags) }
}

/// Create a hard link.
pub fn destack_fs_link(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    existing_path: OsPathVm,
    new_path: OsPathVm,
) -> RuntimeResult<()> {
    let existing_path = path_ref_from_vm(binding, context, existing_path)?;
    let new_path = path_ref_from_vm(binding, context, new_path)?;
    unsafe { host_fs::destack_fs_link(binding, existing_path, new_path) }
}

/// Create a hard link relative to directory handles.
pub fn destack_fs_linkat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    existing_dir: DirectoryHandle,
    existing_path: OsPathVm,
    new_dir: DirectoryHandle,
    new_path: OsPathVm,
    flags: AtFlags,
) -> RuntimeResult<()> {
    let existing_path = path_ref_from_vm(binding, context, existing_path)?;
    let new_path = path_ref_from_vm(binding, context, new_path)?;
    unsafe {
        host_fs::destack_fs_linkat(
            binding,
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
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    target: OsPathVm,
    path: OsPathVm,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let target = path_ref_from_vm(binding, context, target)?;
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_symlink(binding, target, path, kind) }
}

/// Create a symbolic link relative to a directory handle.
pub fn destack_fs_symlinkat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    target: OsPathVm,
    dir: DirectoryHandle,
    path: OsPathVm,
    kind: SymlinkType,
) -> RuntimeResult<()> {
    let target = path_ref_from_vm(binding, context, target)?;
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_symlinkat(binding, target, dir, path, kind) }
}

/// Read a symbolic link.
pub fn destack_fs_readlink(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let path = path_ref_from_vm(binding, context, path)?;
    let target = call_out(|out| unsafe { host_fs::destack_fs_readlink(binding, out, path) })?;
    path_ref_to_vm(context, target)
}

/// Read a symbolic link relative to a directory handle.
pub fn destack_fs_readlinkat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let path = path_ref_from_vm(binding, context, path)?;
    let target =
        call_out(|out| unsafe { host_fs::destack_fs_readlinkat(binding, out, dir, path) })?;
    path_ref_to_vm(context, target)
}

/// Resolve a path to its canonical form.
pub fn destack_fs_realpath(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<OsPathVm> {
    let path = path_ref_from_vm(binding, context, path)?;
    let resolved = call_out(|out| unsafe { host_fs::destack_fs_realpath(binding, out, path) })?;
    path_ref_to_vm(context, resolved)
}

/// Copy a file.
pub fn destack_fs_copyfile(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    from: OsPathVm,
    to: OsPathVm,
    flags: CopyFlags,
) -> RuntimeResult<()> {
    let from = path_ref_from_vm(binding, context, from)?;
    let to = path_ref_from_vm(binding, context, to)?;
    unsafe { host_fs::destack_fs_copyfile(binding, from, to, flags) }
}

/// Create a FIFO special file.
pub fn destack_fs_mkfifo(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_mkfifo(binding, path, mode) }
}

/// Create a FIFO special file relative to a directory handle.
pub fn destack_fs_mkfifoat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_mkfifoat(binding, dir, path, mode) }
}

/// Create a filesystem node.
pub fn destack_fs_mknod(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_mknod(binding, path, mode, device) }
}

/// Create a filesystem node relative to a directory handle.
pub fn destack_fs_mknodat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    mode: FileMode,
    device: NodeDevice,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    unsafe { host_fs::destack_fs_mknodat(binding, dir, path, mode, device) }
}

/// Stat a file.
pub fn destack_fs_stat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<Stat> {
    let path = path_ref_from_vm(binding, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_stat(binding, out, path) })
}

/// Stat a file relative to a directory handle.
pub fn destack_fs_statat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    flags: AtFlags,
) -> RuntimeResult<Stat> {
    let path = path_ref_from_vm(binding, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_statat(binding, out, dir, path, flags) })
}

/// Stat a file without following symlinks.
pub fn destack_fs_lstat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<Stat> {
    let path = path_ref_from_vm(binding, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_lstat(binding, out, path) })
}

/// Stat a filesystem.
pub fn destack_fs_statfs(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<StatFs> {
    let path = path_ref_from_vm(binding, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_statfs(binding, out, path) })
}

/// Stat a path with statx semantics.
pub fn destack_fs_statx(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    dir: DirectoryHandle,
    path: OsPathVm,
    flags: StatxFlags,
    mask: StatxMask,
) -> RuntimeResult<Statx> {
    let path = path_ref_from_vm(binding, context, path)?;
    call_out(|out| unsafe { host_fs::destack_fs_statx(binding, out, dir, path, flags, mask) })
}

/// Synchronize a filesystem by file handle.
pub fn destack_fs_syncfs(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_syncfs(binding, handle) }
}

/// Start watching a path and return a watch handle.
pub fn destack_fs_watch(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    options: WatchOptionsVm,
) -> RuntimeResult<WatchHandle> {
    // decode vm path and options
    let path = path_ref_from_vm(binding, context, path)?;
    let options = WatchOptions {
        mask: options.mask,
        recursive: options.recursive,
        follow_symlinks: options.follow_symlinks,
    };

    // open one host watch handle
    call_out(|out| unsafe { host_fs::destack_fs_watch(binding, out, path, options) })
}

/// Close a watch handle.
pub fn destack_fs_watch_close(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: WatchHandle,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_watch_close(binding, handle) }
}

/// Read a batch of events from a watch handle.
pub fn destack_fs_watch_read(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: WatchHandle,
) -> RuntimeResult<WatchBatchVm> {
    // read one watch batch from the host binding
    let batch = call_out(|out| unsafe { host_fs::destack_fs_watch_read(binding, out, handle) })?;

    let events = map_native_array_to_vm(context, batch.events, |context, event| match *event {
        WatchEvent::WatchCreateEvent(event_create) => {
            let kind = string_ref_to_vm(context, event_create.kind)?;
            let path = path_ref_to_vm(context, event_create.path)?;

            Ok(WatchEventVm::WatchCreateEvent(WatchCreateEventVm {
                kind,
                metadata: WatchEventMetadataVm {
                    cookie: event_create.metadata.cookie,
                },
                path,
            }))
        }
        WatchEvent::WatchMetadataEvent(event_metadata) => {
            let kind = string_ref_to_vm(context, event_metadata.kind)?;
            let path = path_ref_to_vm(context, event_metadata.path)?;

            Ok(WatchEventVm::WatchMetadataEvent(WatchMetadataEventVm {
                kind,
                metadata: WatchEventMetadataVm {
                    cookie: event_metadata.metadata.cookie,
                },
                path,
            }))
        }
        WatchEvent::WatchModifyEvent(event_modify) => {
            let kind = string_ref_to_vm(context, event_modify.kind)?;
            let path = path_ref_to_vm(context, event_modify.path)?;

            Ok(WatchEventVm::WatchModifyEvent(WatchModifyEventVm {
                kind,
                metadata: WatchEventMetadataVm {
                    cookie: event_modify.metadata.cookie,
                },
                path,
            }))
        }
        WatchEvent::WatchOverflowEvent(event_overflow) => {
            let kind = string_ref_to_vm(context, event_overflow.kind)?;

            Ok(WatchEventVm::WatchOverflowEvent(WatchOverflowEventVm {
                kind,
                metadata: WatchEventMetadataVm {
                    cookie: event_overflow.metadata.cookie,
                },
            }))
        }
        WatchEvent::WatchRemoveEvent(event_remove) => {
            let kind = string_ref_to_vm(context, event_remove.kind)?;
            let path = path_ref_to_vm(context, event_remove.path)?;

            Ok(WatchEventVm::WatchRemoveEvent(WatchRemoveEventVm {
                kind,
                metadata: WatchEventMetadataVm {
                    cookie: event_remove.metadata.cookie,
                },
                path,
            }))
        }
        WatchEvent::WatchRenameEvent(event_rename) => {
            let kind = string_ref_to_vm(context, event_rename.kind)?;
            let path = path_ref_to_vm(context, event_rename.path)?;
            let related_path = path_ref_to_vm(context, event_rename.related_path)?;

            Ok(WatchEventVm::WatchRenameEvent(WatchRenameEventVm {
                kind,
                metadata: WatchEventMetadataVm {
                    cookie: event_rename.metadata.cookie,
                },
                path,
                related_path,
            }))
        }
    })?;

    Ok(WatchBatchVm {
        events,
        overflowed: batch.overflowed,
    })
}

/// Start watching a path relative to a directory handle.
pub fn destack_fs_watchat(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    directory: DirectoryHandle,
    path: OsPathVm,
    options: WatchOptionsVm,
) -> RuntimeResult<WatchHandle> {
    // decode vm path and options
    let path = path_ref_from_vm(binding, context, path)?;
    let options = WatchOptions {
        mask: options.mask,
        recursive: options.recursive,
        follow_symlinks: options.follow_symlinks,
    };

    // open one host watch handle relative to a directory
    call_out(|out| unsafe { host_fs::destack_fs_watchat(binding, out, directory, path, options) })
}

/// Duplicate a file handle.
pub fn destack_fs_dup(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<FileHandle> {
    call_out(|out| unsafe { host_fs::destack_fs_dup(binding, out, handle) })
}

/// Duplicate a file handle to a specific target.
pub fn destack_fs_dup2(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    target: FileHandle,
) -> RuntimeResult<FileHandle> {
    call_out(|out| unsafe { host_fs::destack_fs_dup2(binding, out, handle, target) })
}

/// Duplicate a file handle to a specific target with flags.
pub fn destack_fs_dup3(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    target: FileHandle,
    flags: OpenFlags,
) -> RuntimeResult<FileHandle> {
    call_out(|out| unsafe { host_fs::destack_fs_dup3(binding, out, handle, target, flags) })
}

/// Copy a range between file descriptors.
pub fn destack_fs_copy_file_range(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    src: FileHandle,
    src_offset: FileOffset,
    dst: FileHandle,
    dst_offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe {
        host_fs::destack_fs_copy_file_range(binding, out, src, src_offset, dst, dst_offset, length)
    })
}

/// Send file data to a socket.
pub fn destack_fs_sendfile(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    socket: SocketHandle,
    file: FileHandle,
    offset: FileOffset,
    length: FileSize,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe {
        host_fs::destack_fs_sendfile(binding, out, socket, file, offset, length)
    })
}

/// Transfer bytes between descriptors using kernel splice pipelines.
pub fn destack_fs_splice(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    source: ResourceId,
    sourcecursor: SpliceCursor,
    target: ResourceId,
    targetcursor: SpliceCursor,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<u64> {
    // forward splice to the host lane
    call_out(|out| unsafe {
        host_fs::destack_fs_splice(
            binding,
            out,
            source,
            sourcecursor,
            target,
            targetcursor,
            length,
            flags,
        )
    })
}

/// Duplicate bytes from one pipe to another without consuming source bytes.
pub fn destack_fs_tee(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    sourcepipe: PipeHandle,
    targetpipe: PipeHandle,
    length: FileSize,
    flags: SpliceFlags,
) -> RuntimeResult<u64> {
    // forward tee to the host lane
    call_out(|out| unsafe {
        host_fs::destack_fs_tee(binding, out, sourcepipe, targetpipe, length, flags)
    })
}

/// Map user memory pages into a pipe as queued pipe buffers.
pub fn destack_fs_vmsplice(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    pipe: PipeHandle,
    buffers: VmSlice<VmSlice<u8>>,
    flags: SpliceFlags,
) -> RuntimeResult<u64> {
    // decode VM buffer slices into native host buffers
    let native_buffers = buffers_from_vm(binding, context, buffers, "buffers")?;

    // forward vmsplice to the host lane
    call_out(|out| unsafe {
        host_fs::destack_fs_vmsplice(binding, out, pipe, native_buffers, flags)
    })
}

/// Seek within a file and return the new offset.
pub fn destack_fs_seek(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    whence: SeekWhence,
) -> RuntimeResult<FileOffset> {
    call_out(|out| unsafe { host_fs::destack_fs_seek(binding, out, handle, offset, whence) })
}

/// Advise the kernel about access patterns.
pub fn destack_fs_fadvise(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    advice: FileAdvice,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fadvise(binding, handle, offset, length, advice) }
}

/// Allocate or punch file space.
pub fn destack_fs_fallocate(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: AllocFlags,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_fallocate(binding, handle, offset, length, flags) }
}

/// Synchronize a file range.
pub fn destack_fs_sync_file_range(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    flags: SyncFlags,
) -> RuntimeResult<()> {
    unsafe { host_fs::destack_fs_sync_file_range(binding, handle, offset, length, flags) }
}

/// Read an extended attribute by path.
pub fn destack_fs_getxattr(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    name: destack_vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let path = path_ref_from_vm(binding, context, path)?;
    let name = string_ref_from_vm(binding, context, name)?;
    let values = call_out(|out| unsafe { host_fs::destack_fs_getxattr(binding, out, path, name) })?;
    bytes_array_to_vm(context, values)
}

/// Read an extended attribute by path with a raw name payload.
pub fn destack_fs_getxattr_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    // decode path and raw name inputs
    let path = path_ref_from_vm(binding, context, path)?;
    let name = store_bytes_from_vm(binding, context, name)?;

    // dispatch by path encoding
    let values = call_out(|out| match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_getxattr_bytes(binding, out, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_getxattr_utf16(binding, out, path_utf16.utf16, name)
        },
    })?;

    bytes_array_to_vm(context, values)
}

/// Read an extended attribute without following symlinks.
pub fn destack_fs_lgetxattr(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    name: destack_vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let path = path_ref_from_vm(binding, context, path)?;
    let name = string_ref_from_vm(binding, context, name)?;
    let values =
        call_out(|out| unsafe { host_fs::destack_fs_lgetxattr(binding, out, path, name) })?;
    bytes_array_to_vm(context, values)
}

/// Read an extended attribute without following symlinks, using a raw name payload.
pub fn destack_fs_lgetxattr_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    // decode path and raw name inputs
    let path = path_ref_from_vm(binding, context, path)?;
    let name = store_bytes_from_vm(binding, context, name)?;

    // dispatch by path encoding
    let values = call_out(|out| match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_lgetxattr_bytes(binding, out, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_lgetxattr_utf16(binding, out, path_utf16.utf16, name)
        },
    })?;

    bytes_array_to_vm(context, values)
}

/// Read an extended attribute by handle.
pub fn destack_fs_fgetxattr(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    name: destack_vm::StringHandle,
) -> RuntimeResult<VmArray<u8>> {
    let name = string_ref_from_vm(binding, context, name)?;
    let values =
        call_out(|out| unsafe { host_fs::destack_fs_fgetxattr(binding, out, handle, name) })?;
    bytes_array_to_vm(context, values)
}

/// Read an extended attribute by handle with a raw name payload.
pub fn destack_fs_fgetxattr_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    name: VmSlice<u8>,
) -> RuntimeResult<VmArray<u8>> {
    // decode the raw name input
    let name = store_bytes_from_vm(binding, context, name)?;

    // dispatch through the raw handle lane
    let values = call_out(|out| unsafe {
        host_fs::destack_fs_fgetxattr_handle(binding, out, handle, name)
    })?;

    bytes_array_to_vm(context, values)
}

/// Set an extended attribute by path.
pub fn destack_fs_setxattr(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    name: destack_vm::StringHandle,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    let name = string_ref_from_vm(binding, context, name)?;
    let value = store_bytes_from_vm(binding, context, value)?;
    unsafe { host_fs::destack_fs_setxattr(binding, path, name, value, flags) }
}

/// Set an extended attribute by path with a raw name payload.
pub fn destack_fs_setxattr_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // decode path, raw name, and value inputs
    let path = path_ref_from_vm(binding, context, path)?;
    let name = store_bytes_from_vm(binding, context, name)?;
    let value = store_bytes_from_vm(binding, context, value)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_setxattr_bytes(binding, path_bytes.bytes, name, value, flags)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_setxattr_utf16(binding, path_utf16.utf16, name, value, flags)
        },
    }
}

/// Set an extended attribute without following symlinks.
pub fn destack_fs_lsetxattr(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    name: destack_vm::StringHandle,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    let name = string_ref_from_vm(binding, context, name)?;
    let value = store_bytes_from_vm(binding, context, value)?;
    unsafe { host_fs::destack_fs_lsetxattr(binding, path, name, value, flags) }
}

/// Set an extended attribute without following symlinks, using a raw name payload.
pub fn destack_fs_lsetxattr_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // decode path, raw name, and value inputs
    let path = path_ref_from_vm(binding, context, path)?;
    let name = store_bytes_from_vm(binding, context, name)?;
    let value = store_bytes_from_vm(binding, context, value)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_lsetxattr_bytes(binding, path_bytes.bytes, name, value, flags)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_lsetxattr_utf16(binding, path_utf16.utf16, name, value, flags)
        },
    }
}

/// Set an extended attribute by handle.
pub fn destack_fs_fsetxattr(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    name: destack_vm::StringHandle,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    let name = string_ref_from_vm(binding, context, name)?;
    let value = store_bytes_from_vm(binding, context, value)?;
    unsafe { host_fs::destack_fs_fsetxattr(binding, handle, name, value, flags) }
}

/// Set an extended attribute by handle with a raw name payload.
pub fn destack_fs_fsetxattr_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    name: VmSlice<u8>,
    value: VmSlice<u8>,
    flags: XattrFlags,
) -> RuntimeResult<()> {
    // decode raw name and value inputs
    let name = store_bytes_from_vm(binding, context, name)?;
    let value = store_bytes_from_vm(binding, context, value)?;

    // dispatch through the raw handle lane
    unsafe { host_fs::destack_fs_fsetxattr_handle(binding, handle, name, value, flags) }
}

/// List extended attribute names by path.
pub fn destack_fs_listxattr(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<VmArray<destack_vm::StringHandle>> {
    let path = path_ref_from_vm(binding, context, path)?;
    let names = call_out(|out| unsafe { host_fs::destack_fs_listxattr(binding, out, path) })?;
    string_array_to_vm(context, names)
}

/// List extended attribute names by path as raw byte payloads.
pub fn destack_fs_listxattr_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<VmArray<VmArray<u8>>> {
    // decode the path input
    let path = path_ref_from_vm(binding, context, path)?;

    // query host names and encode them as vm byte arrays
    let names = call_out(|out| match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_listxattr_bytes(binding, out, path_bytes.bytes)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_listxattr_utf16(binding, out, path_utf16.utf16)
        },
    })?;

    bytes_array_array_to_vm(context, names)
}

/// List extended attribute names without following symlinks.
pub fn destack_fs_llistxattr(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<VmArray<destack_vm::StringHandle>> {
    let path = path_ref_from_vm(binding, context, path)?;
    let names = call_out(|out| unsafe { host_fs::destack_fs_llistxattr(binding, out, path) })?;
    string_array_to_vm(context, names)
}

/// List extended attribute names without following symlinks as raw byte payloads.
pub fn destack_fs_llistxattr_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
) -> RuntimeResult<VmArray<VmArray<u8>>> {
    // decode the path input
    let path = path_ref_from_vm(binding, context, path)?;

    // query host names and encode them as vm byte arrays
    let names = call_out(|out| match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_llistxattr_bytes(binding, out, path_bytes.bytes)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_llistxattr_utf16(binding, out, path_utf16.utf16)
        },
    })?;

    bytes_array_array_to_vm(context, names)
}

/// List extended attribute names by handle.
pub fn destack_fs_flistxattr(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<VmArray<destack_vm::StringHandle>> {
    let names = call_out(|out| unsafe { host_fs::destack_fs_flistxattr(binding, out, handle) })?;
    string_array_to_vm(context, names)
}

/// List extended attribute names by handle as raw byte payloads.
pub fn destack_fs_flistxattr_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
) -> RuntimeResult<VmArray<VmArray<u8>>> {
    // query host names and encode them as vm byte arrays
    let names =
        call_out(|out| unsafe { host_fs::destack_fs_flistxattr_handle(binding, out, handle) })?;

    bytes_array_array_to_vm(context, names)
}

/// Remove an extended attribute by path.
pub fn destack_fs_removexattr(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    name: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    let name = string_ref_from_vm(binding, context, name)?;
    unsafe { host_fs::destack_fs_removexattr(binding, path, name) }
}

/// Remove an extended attribute by path with a raw name payload.
pub fn destack_fs_removexattr_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    // decode path and raw name inputs
    let path = path_ref_from_vm(binding, context, path)?;
    let name = store_bytes_from_vm(binding, context, name)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_removexattr_bytes(binding, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_removexattr_utf16(binding, path_utf16.utf16, name)
        },
    }
}

/// Remove an extended attribute without following symlinks.
pub fn destack_fs_lremovexattr(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    name: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    let path = path_ref_from_vm(binding, context, path)?;
    let name = string_ref_from_vm(binding, context, name)?;
    unsafe { host_fs::destack_fs_lremovexattr(binding, path, name) }
}

/// Remove an extended attribute without following symlinks, using a raw name payload.
pub fn destack_fs_lremovexattr_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    path: OsPathVm,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    // decode path and raw name inputs
    let path = path_ref_from_vm(binding, context, path)?;
    let name = store_bytes_from_vm(binding, context, name)?;

    // dispatch by path encoding
    match path {
        OsPath::OsPathBytes(path_bytes) => unsafe {
            host_fs::destack_fs_lremovexattr_bytes(binding, path_bytes.bytes, name)
        },
        OsPath::OsPathUtf16(path_utf16) => unsafe {
            host_fs::destack_fs_lremovexattr_utf16(binding, path_utf16.utf16, name)
        },
    }
}

/// Remove an extended attribute by handle.
pub fn destack_fs_fremovexattr(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    name: destack_vm::StringHandle,
) -> RuntimeResult<()> {
    let name = string_ref_from_vm(binding, context, name)?;
    unsafe { host_fs::destack_fs_fremovexattr(binding, handle, name) }
}

/// Remove an extended attribute by handle with a raw name payload.
pub fn destack_fs_fremovexattr_bytes(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    name: VmSlice<u8>,
) -> RuntimeResult<()> {
    // decode the raw name input
    let name = store_bytes_from_vm(binding, context, name)?;

    // dispatch through the raw handle lane
    unsafe { host_fs::destack_fs_fremovexattr_handle(binding, handle, name) }
}

/// Create a file-backed memory mapping.
pub fn destack_fs_mmap_file(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    handle: FileHandle,
    offset: FileOffset,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<VmSlice<u8>> {
    let runtime_state = vm_mmap_runtime_state(binding);

    // validate the mapping shape
    let is_shared = validate_vm_mmap_flags(flags, false)?;
    let length = validate_vm_mmap_length(length)?;
    if offset.0 < 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "offset",
            "offset must be non-negative",
        ))
        .boxed());
    }

    // allocate one zeroed vm slice
    let mapping = VmSlice::from_bytes(&mut context.write(), &vec![0u8; length])?;

    // seed the mapping from the host file
    let _ = destack_fs_pread(binding, context, handle, mapping, offset)?;

    // record the mapping metadata
    let key = vm_mapping_key(mapping);
    let entry = VmMappingEntry {
        handle: Some(handle),
        offset,
        prot,
        is_shared,
    };
    runtime_state.mappings.lock().insert(key, entry);

    Ok(mapping)
}

/// Create an anonymous memory mapping.
pub fn destack_fs_mmap_anonymous(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    length: FileSize,
    prot: MmapProt,
    flags: MmapFlags,
) -> RuntimeResult<VmSlice<u8>> {
    let runtime_state = vm_mmap_runtime_state(binding);

    // validate the mapping shape
    let is_shared = validate_vm_mmap_flags(flags, true)?;
    let length = validate_vm_mmap_length(length)?;

    // allocate one zeroed vm slice
    let mapping = VmSlice::from_bytes(&mut context.write(), &vec![0u8; length])?;

    // record the mapping metadata
    let key = vm_mapping_key(mapping);
    let entry = VmMappingEntry {
        handle: None,
        offset: FileOffset(0),
        prot,
        is_shared,
    };
    runtime_state.mappings.lock().insert(key, entry);

    Ok(mapping)
}

/// Unmap a memory region.
pub fn destack_fs_munmap(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    mapping: VmSlice<u8>,
) -> RuntimeResult<()> {
    let runtime_state = vm_mmap_runtime_state(binding);
    let key = vm_mapping_key(mapping);
    let removed = runtime_state.mappings.lock().remove(&key);
    if removed.is_none() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "mapping",
            "mapping is not active",
        ))
        .boxed());
    }

    Ok(())
}

/// Change memory protection for a mapping.
pub fn destack_fs_mprotect(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    mapping: VmSlice<u8>,
    prot: MmapProt,
) -> RuntimeResult<()> {
    let runtime_state = vm_mmap_runtime_state(binding);
    let key = vm_mapping_key(mapping);
    let mut mappings = runtime_state.mappings.lock();
    let Some(entry) = mappings.get_mut(&key) else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "mapping",
            "mapping is not active",
        ))
        .boxed());
    };

    entry.prot = prot;

    Ok(())
}

/// Flush a mapping to storage.
pub fn destack_fs_msync(
    binding: &BindingCallContext,
    context: &mut destack_vm::BindingContext<'_>,
    mapping: VmSlice<u8>,
    _flags: MmapSyncFlags,
) -> RuntimeResult<()> {
    let runtime_state = vm_mmap_runtime_state(binding);
    let key = vm_mapping_key(mapping);
    let Some(entry) = runtime_state.mappings.lock().get(&key).copied() else {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "mapping",
            "mapping is not active",
        ))
        .boxed());
    };

    // anonymous and private mappings have no writeback target
    let Some(handle) = entry.handle else {
        return Ok(());
    };
    if !entry.is_shared {
        return Ok(());
    }

    // flush the current vm bytes back into the host file
    let written = destack_fs_pwrite(binding, context, handle, mapping, entry.offset)?;
    if written != u64::from(mapping.len) {
        return Err(
            RuntimeError::from(PlatformError::io("vm mmap writeback completed short")).boxed(),
        );
    }

    Ok(())
}

/// Advise the kernel about access patterns.
pub fn destack_fs_madvise(
    binding: &BindingCallContext,
    _context: &mut destack_vm::BindingContext<'_>,
    mapping: VmSlice<u8>,
    _advice: MmapAdvice,
) -> RuntimeResult<()> {
    let runtime_state = vm_mmap_runtime_state(binding);
    let key = vm_mapping_key(mapping);
    if !runtime_state.mappings.lock().contains_key(&key) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "mapping",
            "mapping is not active",
        ))
        .boxed());
    }

    Ok(())
}
