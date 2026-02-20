use windows_sys::Wdk::Storage::FileSystem::{FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE};
use windows_sys::Win32::Foundation::{
    CloseHandle, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE, SetHandleInformation,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use std::sync::Arc;

use parking_lot::Mutex;

use super::util::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::fs::{
    DirectoryHandle, FileHandle, FileMode, OpenFlags, OpenOptions, OsPath, PathBytes, PathUtf16,
    core as core_fs,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::runtime::BindingCallContext;

/// Open flag value for directory-only opens.
const O_DIRECTORY: u32 = 0o200000;

/// Open a file and return a handle.
///
/// Open one filesystem entry by path and return a host-backed file handle.
/// Flag interpretation, creation behavior, and inheritance defaults follow host open semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses open(2) on Unix and CreateFileW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_open_bytes(
    context: &BindingCallContext,
    out: *mut FileHandle,
    path: PathBytes,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path and flags
    let wide = wide_from_bytes(path, "path")?;
    let access = desired_access_from_flags(flags);
    let creation = creation_from_flags(flags);
    let mut attributes = attributes_from_mode(mode) | FILE_FLAG_OVERLAPPED;
    if flags.0 & O_DIRECTORY != 0 {
        attributes |= FILE_FLAG_BACKUP_SEMANTICS;
    }

    // open the file handle
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

    // disable handle inheritance
    let rc = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            CloseHandle(handle);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }

    // register the resource handle
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_handle(handle as _)
        .with_payload(FileResource {
            handle: handle as isize,
            cursor: Arc::new(Mutex::new(0)),
        })
        .with_finalizer(HandleFinalizer::new(handle));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = FileHandle(resource_id);
    }

    Ok(())
}

/// Open a file and return a handle.
///
/// Open one filesystem entry by path and return a host-backed file handle.
/// Flag interpretation, creation behavior, and inheritance defaults follow host open semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses open(2) on Unix and CreateFileW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_open_utf16(
    context: &BindingCallContext,
    out: *mut FileHandle,
    path: PathUtf16,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    // validate the output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // decode the path and flags
    let wide = wide_from_utf16(path, "path")?;
    let access = desired_access_from_flags(flags);
    let creation = creation_from_flags(flags);
    let mut attributes = attributes_from_mode(mode) | FILE_FLAG_OVERLAPPED;
    if flags.0 & O_DIRECTORY != 0 {
        attributes |= FILE_FLAG_BACKUP_SEMANTICS;
    }

    // open the file handle
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

    // disable handle inheritance
    let rc = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            CloseHandle(handle);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }

    // register the resource handle
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_handle(handle as _)
        .with_payload(FileResource {
            handle: handle as isize,
            cursor: Arc::new(Mutex::new(0)),
        })
        .with_finalizer(HandleFinalizer::new(handle));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = FileHandle(resource_id);
    }

    Ok(())
}

/// Open a file relative to a directory handle.
///
/// Open one filesystem entry resolved relative to an explicit directory handle.
/// This avoids ambient current-working-directory resolution and keeps caller-controlled base directory scope.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat(2) on Unix and NtCreateFile relative opens on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_openat_bytes(
    context: &BindingCallContext,
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

    // open the file handle
    let handle = nt_create_file_at(
        root,
        &path,
        access,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        disposition,
        options,
        attributes_from_mode(mode),
    )?;

    // disable handle inheritance
    let rc = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            CloseHandle(handle);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }

    // register the resource handle
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_handle(handle as _)
        .with_payload(FileResource {
            handle: handle as isize,
            cursor: Arc::new(Mutex::new(0)),
        })
        .with_finalizer(HandleFinalizer::new(handle));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = FileHandle(resource_id);
    }

    Ok(())
}

/// Open a file relative to a directory handle.
///
/// Open one filesystem entry resolved relative to an explicit directory handle.
/// This avoids ambient current-working-directory resolution and keeps caller-controlled base directory scope.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat(2) on Unix and NtCreateFile relative opens on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_openat_utf16(
    context: &BindingCallContext,
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

    // open the file handle
    let handle = nt_create_file_at(
        root,
        &path,
        access,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        disposition,
        options,
        attributes_from_mode(mode),
    )?;

    // disable handle inheritance
    let rc = unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0) };
    if rc == 0 {
        unsafe {
            CloseHandle(handle);
        }
        return Err(last_os_error("SetHandleInformation", None));
    }

    // register the resource handle
    let entry = ResourceEntry::new(ResourceKind::File)
        .with_handle(handle as _)
        .with_payload(FileResource {
            handle: handle as isize,
            cursor: Arc::new(Mutex::new(0)),
        })
        .with_finalizer(HandleFinalizer::new(handle));
    let resource_id = context.runtime().resources.insert(entry);
    unsafe {
        *out = FileHandle(resource_id);
    }

    Ok(())
}

/// Open a file relative to a directory handle with openat2 semantics.
///
/// Open one filesystem entry relative to an explicit directory handle with resolve policy flags.
/// Resolve behavior is passed through to supported hosts and rejected when the host backend cannot honor requested guarantees.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat2(2) on Linux and Android, falls back to openat semantics on other Unix targets when resolve flags are empty, and maps to openat semantics on Windows with resolve flags rejected.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_openat2_bytes(
    context: &BindingCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathBytes,
    how: OpenOptions,
) -> RuntimeResult<()> {
    // reject unsupported resolve flags on windows
    if how.resolve.0 != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openat2")).boxed());
    }

    // delegate to openat with the provided flags
    unsafe { destack_fs_openat_bytes(context, out, dir, path, how.flags, how.mode) }
}

/// Open a file relative to a directory handle with openat2 semantics.
///
/// Open one filesystem entry relative to an explicit directory handle with resolve policy flags.
/// Resolve behavior is passed through to supported hosts and rejected when the host backend cannot honor requested guarantees.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat2(2) on Linux and Android, falls back to openat semantics on other Unix targets when resolve flags are empty, and maps to openat semantics on Windows with resolve flags rejected.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_openat2_utf16(
    context: &BindingCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathUtf16,
    how: OpenOptions,
) -> RuntimeResult<()> {
    // reject unsupported resolve flags on windows
    if how.resolve.0 != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported("destack.fs.openat2")).boxed());
    }

    // delegate to openat with the provided flags
    unsafe { destack_fs_openat_utf16(context, out, dir, path, how.flags, how.mode) }
}

/// Open a file and return a handle.
///
/// Open one filesystem entry by path and return a host-backed file handle.
/// Flag interpretation, creation behavior, and inheritance defaults follow host open semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses open(2) on Unix and CreateFileW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_open(
    context: &BindingCallContext,
    out: *mut FileHandle,
    path: OsPath,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_open_bytes(context, out, path, flags, mode) },
        |path| unsafe { destack_fs_open_utf16(context, out, path, flags, mode) },
    )
}

/// Open a file relative to a directory handle.
///
/// Open one filesystem entry resolved relative to an explicit directory handle.
/// This avoids ambient current-working-directory resolution and keeps caller-controlled base directory scope.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat(2) on Unix and NtCreateFile relative opens on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_openat(
    context: &BindingCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: OsPath,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_openat_bytes(context, out, dir, path, flags, mode) },
        |path| unsafe { destack_fs_openat_utf16(context, out, dir, path, flags, mode) },
    )
}

/// Open a file relative to a directory handle with openat2 semantics.
///
/// Open one filesystem entry relative to an explicit directory handle with resolve policy flags.
/// Resolve behavior is passed through to supported hosts and rejected when the host backend cannot honor requested guarantees.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses openat2(2) on Linux and Android, falls back to openat semantics on other Unix targets when resolve flags are empty, and maps to openat semantics on Windows with resolve flags rejected.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `fs.read`, `fs.write`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_fs_openat2(
    context: &BindingCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: OsPath,
    how: OpenOptions,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_openat2_bytes(context, out, dir, path, how) },
        |path| unsafe { destack_fs_openat2_utf16(context, out, dir, path, how) },
    )
}
