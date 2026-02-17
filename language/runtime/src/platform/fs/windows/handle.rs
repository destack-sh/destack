use std::sync::Arc;

use parking_lot::Mutex;

use windows_sys::Win32::Foundation::{
    DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE_FLAG_INHERIT, SetHandleInformation,
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
use crate::runtime::RuntimeCallContext;

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
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // remove the resource entry
    let entry = context
        .runtime()
        .resources
        .remove(handle.0)
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
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the source handle
    let handle = file_handle(context, handle)?;

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
            cursor: Arc::new(Mutex::new(0)),
        })
        .with_finalizer(HandleFinalizer::new(duplicated));
    let resource_id = context.runtime().resources.insert(entry);
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
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    handle: FileHandle,
    target: FileHandle,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // close the target handle if it exists
    if handle.0 != target.0
        && let Some(entry) = context.runtime().resources.remove(target.0)
    {
        entry.finalize(target.0);
    }

    // duplicate the handle
    unsafe { destack_fs_dup(context, out, handle) }
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
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    handle: FileHandle,
    target: FileHandle,
    _flags: OpenFlags,
) -> RuntimeResult<()> {
    unsafe { destack_fs_dup2(context, out, handle, target) }
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
    context: &RuntimeCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    // remove the resource entry
    let entry = context
        .runtime()
        .resources
        .remove(handle.0)
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
    _context: &RuntimeCallContext,
    handle: FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    // resolve the file handle
    let handle = file_handle(_context, handle)?;

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
    context: &RuntimeCallContext,
    handle: FileHandle,
    uid: u32,
    gid: u32,
) -> RuntimeResult<()> {
    // resolve inputs
    let handle = file_handle(context, handle)?;
    let (owner, group) = posix_sids(context, uid, gid)?;

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
    _context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // resolve the file handle
    let handle = file_handle(_context, handle)?;

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
    _context: &RuntimeCallContext,
    out: *mut Stat,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the file handle
    let handle = file_handle(_context, handle)?;

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
    _context: &RuntimeCallContext,
    out: *mut StatFs,
    handle: FileHandle,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the file handle
    let handle = file_handle(_context, handle)?;

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
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { destack_fs_fdatasync(context, handle) }
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
    _context: &RuntimeCallContext,
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    // resolve the file handle and set the file pointer
    let handle = file_handle(_context, handle)?;
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
    _context: &RuntimeCallContext,
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
    let handle = file_handle(_context, handle)?;
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
    _context: &RuntimeCallContext,
    handle: FileHandle,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    // update the handle timestamps
    let handle = file_handle(_context, handle)?;
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
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    #[cfg(unix)]
    {
        let directory_fd = directory_descriptor(context, handle)?;
        let file_fd = unsafe { libc::dup(directory_fd) };
        if file_fd < 0 {
            return Err(RuntimeError::from(PlatformError::io("dup failed".to_string())).boxed());
        }

        let resource = ResourceEntry::new(ResourceKind::File)
            .with_fd(file_fd)
            .with_finalizer(DescriptorFinalizer { fd: file_fd });
        let resource_id = context.runtime().resources.insert(resource);
        unsafe {
            *out = FileHandle(resource_id);
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.dirfd")).boxed())
    }
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
    context: &RuntimeCallContext,
    out: *mut FdFlags,
    handle: FileHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    #[cfg(unix)]
    {
        let fd = file_descriptor(context, handle)?;
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        if flags < 0 {
            return Err(RuntimeError::from(PlatformError::io("fcntl failed".to_string())).boxed());
        }
        unsafe {
            *out = FdFlags(flags as u32);
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.getFdFlags")).boxed())
    }
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
    context: &RuntimeCallContext,
    out: *mut StatusFlags,
    handle: FileHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    #[cfg(unix)]
    {
        let fd = file_descriptor(context, handle)?;
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(RuntimeError::from(PlatformError::io("fcntl failed".to_string())).boxed());
        }
        unsafe {
            *out = StatusFlags(flags as u32);
        }

        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.getStatusFlags")).boxed())
    }
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
    context: &RuntimeCallContext,
    handle: FileHandle,
    flags: FdFlags,
) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let fd = file_descriptor(context, handle)?;
        let result = unsafe { libc::fcntl(fd, libc::F_SETFD, flags.0 as libc::c_int) };
        if result < 0 {
            return Err(RuntimeError::from(PlatformError::io("fcntl failed".to_string())).boxed());
        }
        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.setFdFlags")).boxed())
    }
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
    context: &RuntimeCallContext,
    handle: FileHandle,
    flags: StatusFlags,
) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let fd = file_descriptor(context, handle)?;
        let result = unsafe { libc::fcntl(fd, libc::F_SETFL, flags.0 as libc::c_int) };
        if result < 0 {
            return Err(RuntimeError::from(PlatformError::io("fcntl failed".to_string())).boxed());
        }
        Ok(())
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle, flags);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.setStatusFlags")).boxed())
    }
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
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    #[cfg(unix)]
    {
        let fd = file_descriptor(context, handle)?;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            let result = unsafe { libc::syncfs(fd) };
            if result != 0 {
                return Err(
                    RuntimeError::from(PlatformError::io("syncfs failed".to_string())).boxed(),
                );
            }
            Ok(())
        }
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            let result = unsafe { libc::fsync(fd) };
            if result != 0 {
                return Err(
                    RuntimeError::from(PlatformError::io("fsync failed".to_string())).boxed(),
                );
            }
            Ok(())
        }
    }

    #[cfg(not(unix))]
    {
        let _ = (context, handle);
        Err(RuntimeError::from(PlatformError::not_supported("destack.fs.syncfs")).boxed())
    }
}
