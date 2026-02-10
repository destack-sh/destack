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
use crate::platform::fs::{FileHandle, FileMode, FileOffset, OpenFlags, SeekWhence, Stat, StatFs};
use crate::platform::resource::{DirectoryHandle, ResourceEntry, ResourceKind};
use crate::runtime::RuntimeCallContext;

/// Close a file handle.
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

/// Duplicate a file handle, closing the target handle if needed.
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

/// Duplicate a file handle with flags.
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

/// Change permissions for a file handle.
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

/// Change ownership for a file handle.
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

/// Flush file data to disk.
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

/// Stat a file handle.
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

/// Statfs a file handle.
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

/// Seek within a file handle.
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

/// Update file times for a handle.
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
