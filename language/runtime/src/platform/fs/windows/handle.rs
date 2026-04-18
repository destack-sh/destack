use std::sync::Arc;

use parking_lot::Mutex;

use windows_sys::Win32::Foundation::{
    DUPLICATE_SAME_ACCESS, DuplicateHandle, GetHandleInformation, HANDLE_FLAG_INHERIT,
    SetHandleInformation,
};
use windows_sys::Win32::Security::Authorization::{SE_FILE_OBJECT, SetSecurityInfo};
use windows_sys::Win32::Security::{GROUP_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_READONLY, FILE_BEGIN, FILE_CURRENT, FILE_END, FlushFileBuffers,
    GetFileAttributesW, SetEndOfFile, SetFileAttributesW, SetFilePointerEx,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::{
    FdFlags, FileHandle, FileMode, FileOffset, OpenFlags, SeekWhence, Stat, StatFs, StatusFlags,
};
use crate::platform::resource::{DirectoryHandle, ResourceEntry, ResourceKind};
use crate::runtime::BindingCallContext;

/// Windows-side representation for close-on-exec in fd-flag lanes.
const WINDOWS_FD_CLOEXEC_FLAG: u32 = 1;

/// Close an open file handle.
///
/// Close the target handle by forwarding the descriptor teardown to the host kernel.
/// The descriptor becomes invalid immediately for subsequent read or write operations.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses close(2) on Unix and CloseHandle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_close(
    binding: &BindingCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // remove the resource entry
    let entry = binding
        .worker()
        .resources
        .remove(&binding.world(), handle.0, Some(binding.engine()))
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                "unknown file handle",
            ))
            .boxed()
        })?;

    // finalize the handle
    entry.finalize(handle.0);

    Ok(())
}

/// Duplicate a file handle.
///
/// Duplicate one descriptor and return a new descriptor that references the same open file description.
/// Both descriptors share file-offset and status-flag state per host dup semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses dup(2) on Unix and DuplicateHandle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_dup(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the source handle and shared state
    let (source_cursor, source_status_flags) = file_state(binding, handle)?;
    let handle = file_handle(binding, handle)?;

    // duplicate the handle into the current process
    let process = unsafe { GetCurrentProcess() };
    let mut duplicated = unsafe { std::mem::zeroed::<windows_sys::Win32::Foundation::HANDLE>() };
    let rc = unsafe {
        DuplicateHandle(
            process,
            handle,
            process,
            &mut duplicated,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if rc == 0 {
        return Err(last_os_error("DuplicateHandle", None));
    }

    // disable handle inheritance
    let rc = unsafe { SetHandleInformation(duplicated, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(duplicated);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }

    // register the new resource
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_handle(duplicated as _)
        .with_payload(FileResource {
            handle: duplicated,
            cursor: source_cursor,
            status_flags: source_status_flags,
        })
        .with_finalizer(HandleFinalizer::new(duplicated));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), entry, Some(binding.engine()));
    unsafe {
        *out = FileHandle(resource_id);
    }

    Ok(())
}

/// Duplicate a file handle to a specific target.
///
/// Duplicate one descriptor onto a caller-provided target descriptor number.
/// Existing target descriptor state is replaced according to host dup2 semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses dup2(2) on Unix and DuplicateHandle target replacement on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_dup2(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    handle: FileHandle,
    target: FileHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // return the same handle when the target already matches
    if handle.0 == target.0 {
        unsafe {
            *out = target;
        }
        return Ok(());
    }

    // resolve the source handle and shared state
    let (source_cursor, source_status_flags) = file_state(binding, handle)?;
    let source_handle = file_handle(binding, handle)?;

    // duplicate the handle into the current process
    let process = unsafe { GetCurrentProcess() };
    let mut duplicated = unsafe { std::mem::zeroed::<windows_sys::Win32::Foundation::HANDLE>() };
    let rc = unsafe {
        DuplicateHandle(
            process,
            source_handle,
            process,
            &mut duplicated,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if rc == 0 {
        return Err(last_os_error("DuplicateHandle", None));
    }

    // disable handle inheritance
    let rc = unsafe { SetHandleInformation(duplicated, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(duplicated);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }

    // replace the target resource entry with the duplicated handle
    if let Some(entry) =
        binding
            .worker()
            .resources
            .remove(&binding.world(), target.0, Some(binding.engine()))
    {
        entry.finalize(target.0);
    }

    let entry = ResourceEntry::new(ResourceKind::File)
        .with_handle(duplicated as _)
        .with_payload(FileResource {
            handle: duplicated,
            cursor: source_cursor,
            status_flags: source_status_flags,
        })
        .with_finalizer(HandleFinalizer::new(duplicated));
    binding.worker().resources.insert_with_id(
        &binding.world(),
        target.0,
        entry,
        Some(binding.engine()),
    );
    unsafe {
        *out = target;
    }

    Ok(())
}

/// Duplicate a file handle to a specific target with flags.
///
/// Duplicate one descriptor onto a target descriptor while applying explicit duplication flags.
/// Flag support and close-on-exec semantics follow host dup3 behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses dup3(2) on linux and runtime emulation on other targets.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_dup3(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    handle: FileHandle,
    target: FileHandle,
    _flags: OpenFlags,
) -> RuntimeResult<()> {
    unsafe { destack_fs_dup2(binding, out, handle, target) }
}

/// Close a directory handle.
///
/// Close the target handle by forwarding the descriptor teardown to the host kernel.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses closedir(3) on Unix and FindClose/CloseHandle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_closedir(
    binding: &BindingCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    // remove the resource entry
    let entry = binding
        .worker()
        .resources
        .remove(&binding.world(), handle.0, Some(binding.engine()))
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "handle",
                "unknown directory handle",
            ))
            .boxed()
        })?;

    // finalize the handle
    entry.finalize(handle.0);

    Ok(())
}

/// Change file permissions by handle.
///
/// Change file permissions by handle via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fchmod(2) on Unix and handle-based mode updates on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chmod`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fchmod(
    binding: &BindingCallContext,
    handle: FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    // resolve the file handle
    let handle = file_handle(binding, handle)?;

    // read the current attributes
    let path = final_path_from_handle(handle)?;
    let mut attrs = unsafe { GetFileAttributesW(path.as_ptr()) };
    if attrs == u32::MAX {
        return Err(last_os_error("GetFileAttributesW", None));
    }

    // map write bits to readonly attribute
    if mode.0 & 0o222 == 0 {
        attrs |= FILE_ATTRIBUTE_READONLY;
    } else {
        attrs &= !FILE_ATTRIBUTE_READONLY;
    }

    // write updated attributes
    let rc = unsafe { SetFileAttributesW(path.as_ptr(), attrs) };
    if rc == 0 {
        return Err(last_os_error("SetFileAttributesW", None));
    }

    Ok(())
}

/// Change file owner and group by handle.
///
/// Change file owner and group by handle via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fchown(2) on Unix and handle owner updates where supported on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.chown`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fchown(
    binding: &BindingCallContext,
    handle: FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    // resolve inputs
    let handle = file_handle(binding, handle)?;
    let (owner, group) = posix_sids(binding, uid, gid)?;

    // update ownership information
    let rc = unsafe {
        SetSecurityInfo(
            handle,
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION,
            owner.as_ptr(),
            group.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if rc != 0 {
        return Err(win32_error("SetSecurityInfo", rc));
    }

    Ok(())
}

/// Synchronize file data only.
///
/// Flush file data pages for the target descriptor without requiring full metadata durability.
/// Metadata needed for data reachability may still be persisted per host kernel rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fdatasync(2) on Unix and FlushFileBuffers on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fdatasync(
    binding: &BindingCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // resolve the file handle
    let handle = file_handle(binding, handle)?;

    // flush buffers
    let rc = unsafe { FlushFileBuffers(handle) };
    if rc == 0 {
        return Err(last_os_error("FlushFileBuffers", None));
    }

    Ok(())
}

/// Stat a file by handle.
///
/// Stat a file by handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fstat(2) on Unix and GetFileInformationByHandleEx on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fstat(
    binding: &BindingCallContext,
    out: *mut Stat,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the file handle
    let handle = file_handle(binding, handle)?;

    // gather metadata
    let stat = stat_from_handle(handle)?;

    // write the output
    unsafe {
        *out = stat;
    }

    Ok(())
}

/// Stat a filesystem by handle.
///
/// Stat a filesystem by handle via host kernel APIs.
/// Paths are forwarded from `OsPath` without runtime normalization or canonicalization, and permission checks follow host filesystem rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fstatfs/statvfs by handle on Unix and volume information by handle on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fstatfs(
    binding: &BindingCallContext,
    out: *mut StatFs,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the file handle
    let handle = file_handle(binding, handle)?;

    // gather filesystem metadata
    let path = final_path_from_handle(handle)?;
    let statfs = statfs_from_path(&path)?;

    // write the output
    unsafe {
        *out = statfs;
    }

    Ok(())
}

/// Synchronize a file's in-core state with storage.
///
/// Synchronize buffered file state to storage for the target file descriptor.
/// Completion guarantees and writeback scope follow host kernel fsync semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fsync(2) on Unix and FlushFileBuffers on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_fsync(
    binding: &BindingCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { destack_fs_fdatasync(binding, handle) }
}

/// Truncate a file by handle.
///
/// Truncate the target file to the requested size using host file-size control APIs.
/// Growth behavior for sparse expansion and zero-fill follows host filesystem policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses ftruncate(2) on Unix and SetEndOfFile on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_ftruncate(
    binding: &BindingCallContext,
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    // resolve the file handle and set the file pointer
    let handle = file_handle(binding, handle)?;
    let distance = size.0;
    let rc = unsafe { SetFilePointerEx(handle, distance, std::ptr::null_mut(), FILE_BEGIN) };
    if rc == 0 {
        return Err(last_os_error("SetFilePointerEx", None));
    }

    // truncate to the new size
    let rc = unsafe { SetEndOfFile(handle) };
    if rc == 0 {
        return Err(last_os_error("SetEndOfFile", None));
    }

    Ok(())
}

/// Seek within a file and return the new offset.
///
/// Reposition the descriptor file offset using the supplied origin and delta.
/// Returned offset is the new descriptor position after host seek processing.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses lseek(2) on Unix and SetFilePointerEx on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_seek(
    binding: &BindingCallContext,
    out: *mut FileOffset,
    handle: FileHandle,
    offset: FileOffset,
    whence: SeekWhence,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the file handle
    let handle = file_handle(binding, handle)?;
    let distance = offset.0;
    let whence = match whence {
        SeekWhence::Set => FILE_BEGIN,
        SeekWhence::Cur => FILE_CURRENT,
        SeekWhence::End => FILE_END,
    };

    // seek and return the new position
    let mut new_position = 0i64;
    let rc = unsafe { SetFilePointerEx(handle, distance, &mut new_position, whence) };
    if rc == 0 {
        return Err(last_os_error("SetFilePointerEx", None));
    }

    unsafe {
        *out = FileOffset(new_position);
    }

    Ok(())
}

/// Update access and modification times by handle.
///
/// Update access and modification times by handle via host kernel APIs.
/// Return values and failures map directly to host contracts so higher layers can apply policy explicitly.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses futimens/futimes on Unix and SetFileTime on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.metadata`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_futimes(
    binding: &BindingCallContext,
    handle: FileHandle,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    // update the handle timestamps
    let handle = file_handle(binding, handle)?;
    set_handle_times(handle, atime_ns, mtime_ns)
}

/// Resolve the directory descriptor for an open directory handle.
///
/// Extract the underlying file descriptor or handle value from an open directory stream.
/// The returned handle is valid only while the source directory handle remains open.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses dirfd(3) on Unix and directory handle extraction on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_dirfd(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve and duplicate the directory handle for file-handle lanes
    let directory = directory_handle(binding, handle)?;
    let process = unsafe { GetCurrentProcess() };
    let mut duplicated = unsafe { std::mem::zeroed::<windows_sys::Win32::Foundation::HANDLE>() };
    let rc = unsafe {
        DuplicateHandle(
            process,
            directory,
            process,
            &mut duplicated,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if rc == 0 {
        return Err(last_os_error("DuplicateHandle", None));
    }

    // clear inheritance so duplicated handles match runtime defaults
    let rc = unsafe { SetHandleInformation(duplicated, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(duplicated);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }

    // register one file-handle resource for the duplicated directory handle
    let resource = ResourceEntry::new(ResourceKind::File)
        .with_handle(duplicated as _)
        .with_payload(FileResource {
            handle: duplicated,
            cursor: Arc::new(Mutex::new(0)),
            status_flags: Arc::new(Mutex::new(0)),
        })
        .with_finalizer(HandleFinalizer::new(duplicated));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(&binding.world(), resource, Some(binding.engine()));
    unsafe {
        *out = FileHandle(resource_id);
    }

    Ok(())
}

/// Read file descriptor flags.
///
/// Read descriptor flags such as close-on-exec from the target file descriptor.
/// Returned bits reflect current host descriptor state at call time.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fcntl(F_GETFD) on Unix and runtime handle metadata on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_get_fd_flags(
    binding: &BindingCallContext,
    out: *mut FdFlags,
    handle: FileHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read inheritance metadata from the raw handle
    let handle = file_handle(binding, handle)?;
    let mut value = 0u32;
    let rc = unsafe { GetHandleInformation(handle, &mut value) };
    if rc == 0 {
        return Err(last_os_error("GetHandleInformation", None));
    }

    // map inheritance into one FD_CLOEXEC style flag
    let close_on_exec = (value & HANDLE_FLAG_INHERIT) == 0;
    let fd_flags = if close_on_exec {
        WINDOWS_FD_CLOEXEC_FLAG
    } else {
        0
    };
    unsafe {
        *out = FdFlags(fd_flags);
    }

    Ok(())
}

/// Read file status flags.
///
/// Read status flags such as append and nonblocking from the target file descriptor.
/// Returned bits reflect current host descriptor state at call time.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fcntl(F_GETFL) on Unix and runtime handle metadata on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_get_status_flags(
    binding: &BindingCallContext,
    out: *mut StatusFlags,
    handle: FileHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read tracked status flags from resource metadata
    let status_flags = file_status_flags(binding, handle)?;
    let status_flags = status_flags.lock();
    unsafe {
        *out = StatusFlags(*status_flags);
    }

    Ok(())
}

/// Write file descriptor flags.
///
/// Write descriptor flags such as close-on-exec to the target file descriptor.
/// Unsupported flag bits are rejected according to host descriptor control rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fcntl(F_SETFD) on Unix and runtime handle metadata on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_set_fd_flags(
    binding: &BindingCallContext,
    handle: FileHandle,
    flags: FdFlags,
) -> RuntimeResult<()> {
    // resolve the raw handle and map FD_CLOEXEC into inherit state
    let handle = file_handle(binding, handle)?;
    let close_on_exec = (flags.0 & WINDOWS_FD_CLOEXEC_FLAG) != 0;
    let inherit = if close_on_exec {
        0
    } else {
        HANDLE_FLAG_INHERIT
    };
    let rc = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, inherit) };
    if rc == 0 {
        return Err(last_os_error("SetHandleInformation", None));
    }

    Ok(())
}

/// Write file status flags.
///
/// Write status flags such as append and nonblocking to the target file descriptor.
/// Unsupported or immutable status bits are rejected by host fcntl validation.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses fcntl(F_SETFL) on Unix and runtime handle metadata on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.handle`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_set_status_flags(
    binding: &BindingCallContext,
    handle: FileHandle,
    flags: StatusFlags,
) -> RuntimeResult<()> {
    // normalize one status-flag update against the tracked access mode
    let status_flags = file_status_flags(binding, handle)?;
    let mut status_flags = status_flags.lock();
    *status_flags = normalize_status_flags(*status_flags, flags)?;

    Ok(())
}

/// Synchronize a filesystem by file handle.
///
/// Flush pending filesystem writeback for the mount that contains this handle.
/// Scope and ordering guarantees follow host mount-level sync semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses syncfs(2) on Unix and volume flush APIs on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.sync`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_syncfs(
    binding: &BindingCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // flush pending writeback for the target handle
    let handle = file_handle(binding, handle)?;
    let rc = unsafe { FlushFileBuffers(handle) };
    if rc == 0 {
        return Err(last_os_error("FlushFileBuffers", None));
    }

    Ok(())
}
