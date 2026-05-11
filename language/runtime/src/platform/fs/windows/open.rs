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
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::{
    DirectoryHandle, FileHandle, FileMode, OpenFlags, OpenOptions, OsPath, PathBytes, PathUtf16,
    core as core_fs,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::runtime::BindingCallContext;

/// Open flag value for directory-only opens.
const O_DIRECTORY: u32 = 0o200000;
/// Known Linux `openat2` resolve flag bits.
const OPENAT2_RESOLVE_KNOWN_BITS: u64 = 0x01 | 0x02 | 0x04 | 0x08 | 0x10 | 0x20;

/// Require one opened handle to describe a directory when `O_DIRECTORY` was requested.
fn require_directory_handle(handle: isize) -> RuntimeResult<()> {
    // stat the opened handle and reject non-directory targets
    let stat = stat_from_handle(handle)?;
    if stat.mode.0 & libc::S_IFMT as u32 == libc::S_IFDIR as u32 {
        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotDirectory),
        None,
        None,
        Some("open".to_string()),
        None,
        "path is not a directory",
    ))
    .boxed())
}

/// Validate one Windows `openat2` resolve payload.
fn validate_openat2_resolve_flags(resolve: u64) -> RuntimeResult<()> {
    // reject unknown resolve bits explicitly
    let unknown_bits = resolve & !OPENAT2_RESOLVE_KNOWN_BITS;
    if unknown_bits != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "how.resolve",
            format!("unknown openat2 resolve bits: {unknown_bits:#x}"),
        ))
        .boxed());
    }

    // windows does not yet honor Linux resolve semantics
    if resolve != 0 {
        return Err(
            RuntimeError::from(PlatformError::not_supported("destack.fs.file.openat2")).boxed(),
        );
    }

    Ok(())
}

/// Open a file and return a handle.
pub(crate) unsafe fn destack_fs_open_bytes(
    binding: &BindingCallContext,
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

    // enforce directory only semantics for plain path opens
    if flags.0 & O_DIRECTORY != 0
        && let Err(error) = require_directory_handle(handle)
    {
        unsafe {
            CloseHandle(handle);
        }
        return Err(error);
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
            status_flags: Arc::new(Mutex::new(tracked_status_flags_from_open_flags(flags))),
        })
        .with_finalizer(HandleFinalizer::new(handle));
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

/// Open a file and return a handle.
pub(crate) unsafe fn destack_fs_open_utf16(
    binding: &BindingCallContext,
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

    // enforce directory only semantics for plain path opens
    if flags.0 & O_DIRECTORY != 0
        && let Err(error) = require_directory_handle(handle)
    {
        unsafe {
            CloseHandle(handle);
        }
        return Err(error);
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
            status_flags: Arc::new(Mutex::new(tracked_status_flags_from_open_flags(flags))),
        })
        .with_finalizer(HandleFinalizer::new(handle));
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

/// Open a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_openat_bytes(
    binding: &BindingCallContext,
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
        return unsafe { destack_fs_open_bytes(binding, out, path, flags, mode) };
    }

    // resolve the directory handle
    let root = directory_handle(binding, dir)?;
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
            status_flags: Arc::new(Mutex::new(tracked_status_flags_from_open_flags(flags))),
        })
        .with_finalizer(HandleFinalizer::new(handle));
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

/// Open a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_openat_utf16(
    binding: &BindingCallContext,
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
        return unsafe { destack_fs_open_utf16(binding, out, path, flags, mode) };
    }

    // resolve the directory handle
    let root = directory_handle(binding, dir)?;
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
            status_flags: Arc::new(Mutex::new(tracked_status_flags_from_open_flags(flags))),
        })
        .with_finalizer(HandleFinalizer::new(handle));
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

/// Open a file relative to a directory handle with openat2 semantics.
pub(crate) unsafe fn destack_fs_openat2_bytes(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathBytes,
    how: OpenOptions,
) -> RuntimeResult<()> {
    // validate resolve flags before falling back to openat
    validate_openat2_resolve_flags(how.resolve.0)?;

    // delegate to openat with the provided flags
    unsafe { destack_fs_openat_bytes(binding, out, dir, path, how.flags, how.mode) }
}

/// Open a file relative to a directory handle with openat2 semantics.
pub(crate) unsafe fn destack_fs_openat2_utf16(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: PathUtf16,
    how: OpenOptions,
) -> RuntimeResult<()> {
    // validate resolve flags before falling back to openat
    validate_openat2_resolve_flags(how.resolve.0)?;

    // delegate to openat with the provided flags
    unsafe { destack_fs_openat_utf16(binding, out, dir, path, how.flags, how.mode) }
}

/// Open a file and return a handle.
pub(crate) unsafe fn destack_fs_open(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    path: OsPath,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_open_bytes(binding, out, path, flags, mode) },
        |path| unsafe { destack_fs_open_utf16(binding, out, path, flags, mode) },
    )
}

/// Open a file relative to a directory handle.
pub(crate) unsafe fn destack_fs_openat(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: OsPath,
    flags: OpenFlags,
    mode: FileMode,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_openat_bytes(binding, out, dir, path, flags, mode) },
        |path| unsafe { destack_fs_openat_utf16(binding, out, dir, path, flags, mode) },
    )
}

/// Open a file relative to a directory handle with openat2 semantics.
pub(crate) unsafe fn destack_fs_openat2(
    binding: &BindingCallContext,
    out: *mut FileHandle,
    dir: DirectoryHandle,
    path: OsPath,
    how: OpenOptions,
) -> RuntimeResult<()> {
    core_fs::with_path_ref(
        path,
        "path",
        |path| unsafe { destack_fs_openat2_bytes(binding, out, dir, path, how) },
        |path| unsafe { destack_fs_openat2_utf16(binding, out, dir, path, how) },
    )
}
