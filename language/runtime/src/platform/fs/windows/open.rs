use windows_sys::Wdk::Storage::FileSystem::{FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE};
use windows_sys::Win32::Foundation::{
    CloseHandle, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE, SetHandleInformation,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::{DirectoryHandle, FileHandle, FileMode, OpenFlags, PathBytes, PathUtf16};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::runtime::RuntimeCallContext;

/// Open flag value for directory-only opens.
const O_DIRECTORY: u32 = 0o200000;

/// Open a file with byte paths.
pub(crate) unsafe fn destack_fs_open_bytes(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    path: PathBytes,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let wide = wide_from_bytes(path, "path")?;
    let access = desired_access_from_flags(flags);
    let creation = creation_from_flags(flags);
    let mut attributes = attributes_from_mode(mode) | FILE_FLAG_OVERLAPPED;
    if flags.0 & O_DIRECTORY != 0 {
        attributes |= FILE_FLAG_BACKUP_SEMANTICS;
    }
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            creation,
            attributes,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_os_error("CreateFileW", None));
    }
    let rc = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            CloseHandle(handle);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_handle(handle as _)
        .with_finalizer(HandleFinalizer::new(handle));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = FileHandle(resource_id);
    }
    Ok(())
}

/// Open a file with UTF-16 paths.
pub(crate) unsafe fn destack_fs_open_utf16(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    path: PathUtf16,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let wide = wide_from_utf16(path, "path")?;
    let access = desired_access_from_flags(flags);
    let creation = creation_from_flags(flags);
    let mut attributes = attributes_from_mode(mode) | FILE_FLAG_OVERLAPPED;
    if flags.0 & O_DIRECTORY != 0 {
        attributes |= FILE_FLAG_BACKUP_SEMANTICS;
    }
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            creation,
            attributes,
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_os_error("CreateFileW", None));
    }
    let rc = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            CloseHandle(handle);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_handle(handle as _)
        .with_finalizer(HandleFinalizer::new(handle));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = FileHandle(resource_id);
    }
    Ok(())
}

/// Open a file relative to a directory with byte paths.
pub(crate) unsafe fn destack_fs_openat_bytes(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathBytes,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // use the absolute path variant when the path is fully qualified
    let pathbuf = pathbuf_from_bytes(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_open_bytes(context, out, path, flags, mode) };
    }

    // resolve the directory handle
    let root = directory_handle(context, dir)?;
    let path = wide_from_pathbuf_no_nul(&pathbuf);

    // map flags into NtCreateFile inputs
    let access = desired_access_from_flags(flags);
    let disposition = nt_disposition_from_flags(flags);
    let options = if flags.0 & O_DIRECTORY != 0 {
        FILE_DIRECTORY_FILE
    } else {
        FILE_NON_DIRECTORY_FILE
    };

    let handle = nt_create_file_at(
        root,
        &path,
        access,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        disposition,
        options,
        attributes_from_mode(mode),
    )?;
    let rc = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            CloseHandle(handle);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }

    let entry = ResourceEntry::new(ResourceKind::File)
        .with_handle(handle as _)
        .with_finalizer(HandleFinalizer::new(handle));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = FileHandle(resource_id);
    }
    Ok(())
}

/// Open a file relative to a directory with UTF-16 paths.
pub(crate) unsafe fn destack_fs_openat_utf16(
    context: &RuntimeCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathUtf16,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // use the absolute path variant when the path is fully qualified
    let pathbuf = pathbuf_from_utf16(path, "path")?;
    if pathbuf.is_absolute() {
        return unsafe { destack_fs_open_utf16(context, out, path, flags, mode) };
    }

    // resolve the directory handle
    let root = directory_handle(context, dir)?;
    let path = wide_from_pathbuf_no_nul(&pathbuf);

    // map flags into NtCreateFile inputs
    let access = desired_access_from_flags(flags);
    let disposition = nt_disposition_from_flags(flags);
    let options = if flags.0 & O_DIRECTORY != 0 {
        FILE_DIRECTORY_FILE
    } else {
        FILE_NON_DIRECTORY_FILE
    };

    let handle = nt_create_file_at(
        root,
        &path,
        access,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        disposition,
        options,
        attributes_from_mode(mode),
    )?;
    let rc = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            CloseHandle(handle);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }

    let entry = ResourceEntry::new(ResourceKind::File)
        .with_handle(handle as _)
        .with_finalizer(HandleFinalizer::new(handle));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = FileHandle(resource_id);
    }
    Ok(())
}
