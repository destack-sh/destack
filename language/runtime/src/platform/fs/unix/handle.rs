use super::core::*;
use super::os;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::fs::*;
use crate::platform::resource::*;
use crate::platform::{core as core_platform, *};
use crate::runtime::BindingCallContext;

/// Close an open file handle.
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
