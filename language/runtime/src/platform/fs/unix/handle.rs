use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::*;
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

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
    // validate the handle kind
    let is_file = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind == ResourceKind::File)
        .unwrap_or(false);
    if !is_file {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown file handle",
        ))
        .boxed());
    }

    // remove the resource and close the descriptor
    if !binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown file handle",
        ))
        .boxed());
    }

    Ok(())
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
    // validate the handle kind
    let is_directory = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind == ResourceKind::Directory)
        .unwrap_or(false);
    if !is_directory {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown directory handle",
        ))
        .boxed());
    }

    // remove the resource entry
    if !binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            "unknown directory handle",
        ))
        .boxed());
    }

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
    // resolve the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let rc = unsafe { libc::fchmod(fd, mode.0 as libc::mode_t) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("fchmod", None))
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
    // resolve the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let rc = unsafe { libc::fchown(fd, uid, gid) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("fchown", None))
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
    // resolve the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let rc = os::fdatasync(fd);
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("fdatasync", None))
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
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let mut stat: libc::stat = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::fstat(fd, &mut stat) };
    if rc != 0 {
        return Err(core_platform::io_error("fstat", None));
    }

    unsafe {
        *out = stat_from_libc(stat);
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
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let statfs = os::statfs_for_fd(fd)?;
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
    // resolve the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let rc = unsafe { libc::fsync(fd) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("fsync", None))
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
    // resolve the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let offset = offset_to_off_t(size)?;
    let rc = unsafe { libc::ftruncate(fd, offset) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("ftruncate", None))
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

    // resolve the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let offset = offset_to_off_t(offset)?;
    let whence = match whence {
        SeekWhence::Set => libc::SEEK_SET,
        SeekWhence::Cur => libc::SEEK_CUR,
        SeekWhence::End => libc::SEEK_END,
    };

    // seek and return the new position
    let rc = unsafe { libc::lseek(fd, offset, whence) };
    if rc < 0 {
        return Err(core_platform::io_error("lseek", None));
    }

    unsafe {
        *out = FileOffset(rc as i64);
    }

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

    // duplicate the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let dup_fd = unsafe { libc::dup(fd) };
    if dup_fd < 0 {
        return Err(core_platform::io_error("dup", None));
    }

    // register the new handle
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_fd(dup_fd)
        .with_finalizer(FdFinalizer {
            fd: dup_fd,
            directory_stream: None,
        });
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

    // duplicate the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let dup_fd = unsafe { libc::dup(fd) };
    if dup_fd < 0 {
        return Err(core_platform::io_error("dup", None));
    }

    // replace the target resource entry
    if let Some(entry) =
        binding
            .worker()
            .resources
            .remove(&binding.world(), target.0, Some(binding.engine()))
    {
        entry.finalize(target.0);
    }
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_fd(dup_fd)
        .with_finalizer(FdFinalizer {
            fd: dup_fd,
            directory_stream: None,
        });
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
    flags: OpenFlags,
) -> RuntimeResult<()> {
    // ensure the output pointer is valid
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // duplicate the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let dup_fd = unsafe { libc::dup(fd) };
    if dup_fd < 0 {
        return Err(core_platform::io_error("dup", None));
    }

    // apply CLOEXEC when requested
    if flags.0 & libc::O_CLOEXEC as u32 != 0 {
        let rc = unsafe { libc::fcntl(dup_fd, libc::F_SETFD, libc::FD_CLOEXEC) };
        if rc != 0 {
            unsafe {
                libc::close(dup_fd);
            }
            return Err(core_platform::io_error("fcntl", None));
        }
    }

    // replace the target resource entry
    if let Some(entry) =
        binding
            .worker()
            .resources
            .remove(&binding.world(), target.0, Some(binding.engine()))
    {
        entry.finalize(target.0);
    }
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_fd(dup_fd)
        .with_finalizer(FdFinalizer {
            fd: dup_fd,
            directory_stream: None,
        });
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
    atimens: u64,
    mtimens: u64,
) -> RuntimeResult<()> {
    // resolve the file descriptor
    let fd = file_descriptor(binding, handle)?;
    let times = [timespec_from_nanos(atimens), timespec_from_nanos(mtimens)];
    let rc = unsafe { libc::futimens(fd, times.as_ptr()) };
    if rc == 0 {
        return Ok(());
    }

    Err(core_platform::io_error("futimens", None))
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

    let directory_fd = directory_descriptor(binding, handle)?;
    let file_fd = unsafe { libc::dup(directory_fd) };
    if file_fd < 0 {
        return Err(core_platform::io_error("dup", None));
    }

    let resource = ResourceEntry::new(ResourceKind::File)
        .with_fd(file_fd)
        .with_finalizer(FdFinalizer {
            fd: file_fd,
            directory_stream: None,
        });
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

    let fd = file_descriptor(binding, handle)?;
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags < 0 {
        return Err(core_platform::io_error("fcntl", None));
    }
    unsafe {
        *out = FdFlags(flags as u32);
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

    let fd = file_descriptor(binding, handle)?;
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(core_platform::io_error("fcntl", None));
    }
    unsafe {
        *out = StatusFlags(flags as u32);
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
    let fd = file_descriptor(binding, handle)?;
    let result = unsafe { libc::fcntl(fd, libc::F_SETFD, flags.0 as libc::c_int) };
    if result < 0 {
        return Err(core_platform::io_error("fcntl", None));
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
    let fd = file_descriptor(binding, handle)?;
    let result = unsafe { libc::fcntl(fd, libc::F_SETFL, flags.0 as libc::c_int) };
    if result < 0 {
        return Err(core_platform::io_error("fcntl", None));
    }
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
    let fd = file_descriptor(binding, handle)?;
    #[cfg(target_os = "linux")]
    {
        let result = unsafe { libc::syncfs(fd) };
        if result != 0 {
            return Err(core_platform::io_error("syncfs", None));
        }
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let result = unsafe { libc::fsync(fd) };
        if result != 0 {
            return Err(core_platform::io_error("fsync", None));
        }
        Ok(())
    }
}
