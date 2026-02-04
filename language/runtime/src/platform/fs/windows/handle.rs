use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_READONLY, FILE_BEGIN, FlushFileBuffers, GetFileAttributesW, SetEndOfFile,
    SetFileAttributesW, SetFilePointerEx,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::{FileHandle, FileMode, FileOffset, Stat, StatFs};
use crate::platform::resource::DirectoryHandle;
use crate::runtime::RuntimeCallContext;

/// Close a file handle.
pub(crate) unsafe fn destack_fs_close(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
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

    entry.finalize(handle.0);
    Ok(())
}

/// Close a directory handle.
pub(crate) unsafe fn destack_fs_closedir(
    context: &RuntimeCallContext,
    handle: DirectoryHandle,
) -> RuntimeResult<()> {
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

    entry.finalize(handle.0);
    Ok(())
}

/// Change permissions for a file handle.
pub(crate) unsafe fn destack_fs_fchmod(
    _context: &RuntimeCallContext,
    handle: FileHandle,
    mode: FileMode,
) -> RuntimeResult<()> {
    let handle = file_handle(_context, handle)?;
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
    let rc = unsafe { SetFileAttributesW(path.as_ptr(), attrs) };
    if rc == 0 {
        return Err(last_os_error("SetFileAttributesW", None));
    }

    Ok(())
}

/// Change ownership for a file handle.
pub(crate) unsafe fn destack_fs_fchown(
    _context: &RuntimeCallContext,
    _handle: FileHandle,
    _uid: u32,
    _gid: u32,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.fs.fchown")).boxed())
}

/// Flush file data to disk.
pub(crate) unsafe fn destack_fs_fdatasync(
    _context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    let handle = file_handle(_context, handle)?;
    let rc = unsafe { FlushFileBuffers(handle) };
    if rc == 0 {
        return Err(last_os_error("FlushFileBuffers", None));
    }
    Ok(())
}

/// Stat a file handle.
pub(crate) unsafe fn destack_fs_fstat(
    _context: &RuntimeCallContext,
    out: *mut Stat,
    handle: FileHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let handle = file_handle(_context, handle)?;
    let stat = stat_from_handle(handle)?;
    unsafe {
        *out = stat;
    }
    Ok(())
}

/// Statfs a file handle.
pub(crate) unsafe fn destack_fs_fstatfs(
    _context: &RuntimeCallContext,
    out: *mut StatFs,
    handle: FileHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let handle = file_handle(_context, handle)?;
    let path = final_path_from_handle(handle)?;
    let statfs = statfs_from_path(&path)?;
    unsafe {
        *out = statfs;
    }
    Ok(())
}

/// Flush file buffers for a file handle.
pub(crate) unsafe fn destack_fs_fsync(
    context: &RuntimeCallContext,
    handle: FileHandle,
) -> RuntimeResult<()> {
    unsafe { destack_fs_fdatasync(context, handle) }
}

/// Truncate a file handle to a size.
pub(crate) unsafe fn destack_fs_ftruncate(
    _context: &RuntimeCallContext,
    handle: FileHandle,
    size: FileOffset,
) -> RuntimeResult<()> {
    let handle = file_handle(_context, handle)?;
    let distance = size.0 as i64;
    let rc = unsafe { SetFilePointerEx(handle, distance, std::ptr::null_mut(), FILE_BEGIN) };
    if rc == 0 {
        return Err(last_os_error("SetFilePointerEx", None));
    }

    let rc = unsafe { SetEndOfFile(handle) };
    if rc == 0 {
        return Err(last_os_error("SetEndOfFile", None));
    }

    Ok(())
}

/// Update file times for a handle.
pub(crate) unsafe fn destack_fs_futimes(
    _context: &RuntimeCallContext,
    handle: FileHandle,
    atime_ns: u64,
    mtime_ns: u64,
) -> RuntimeResult<()> {
    let handle = file_handle(_context, handle)?;
    set_handle_times(handle, atime_ns, mtime_ns)
}
